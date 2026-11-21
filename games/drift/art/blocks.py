#!/usr/bin/env python3
"""Builds DRIFT's block set: `assets/blocks.tileset` and its textures.

Ships in DRIFT are built of half-metre blocks (a built `sindri.voxel_world`
scaled by 0.5). These are their materials, drawn to read as engineered metal
rather than toy cubes: painted hull plate chipped back to bare metal at its
edges, tread plate for decks, interior bulkhead panels with vents, dark
structural frame, hazard striping and a glowing light panel. Each face is a
128-pixel texture for half a metre, about four pixels a centimetre.

Every texture is drawn from fixed seeds, so re-running this changes nothing.
"""
import json
import math
import random
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ASSETS = Path(__file__).resolve().parent.parent / "assets"
TEXTURES = ASSETS / "textures" / "blocks"
SIZE = 128


def noise(seed, scale, size=SIZE):
    """Smooth value noise in 0..255, tiling at the texture's edge."""
    rng = random.Random(seed)
    cells = max(1, size // scale)
    grid = [[rng.random() for _ in range(cells)] for _ in range(cells)]
    image = Image.new("L", (size, size))
    pixels = image.load()
    for y in range(size):
        for x in range(size):
            fx, fy = x / scale, y / scale
            x0, y0 = int(fx) % cells, int(fy) % cells
            x1, y1 = (x0 + 1) % cells, (y0 + 1) % cells
            tx, ty = fx - int(fx), fy - int(fy)
            tx, ty = tx * tx * (3 - 2 * tx), ty * ty * (3 - 2 * ty)
            top = grid[y0][x0] * (1 - tx) + grid[y0][x1] * tx
            bottom = grid[y1][x0] * (1 - tx) + grid[y1][x1] * tx
            pixels[x, y] = int(255 * (top * (1 - ty) + bottom * ty))
    return image


def fractal(seed, octaves=((32, 0.5), (16, 0.3), (6, 0.2))):
    """Several octaves of noise summed: the base of grime and wear."""
    out = Image.new("L", (SIZE, SIZE), 0)
    for i, (scale, weight) in enumerate(octaves):
        layer = noise(seed * 31 + i, scale).point(lambda v, w=weight: int(v * w))
        out = ImageChops.add(out, layer)
    return out


def tint(grey, colour):
    """A greyscale image as shades of a colour."""
    return Image.merge("RGB", [grey.point(lambda v, c=c: int(v * c / 255)) for c in colour])


def blend(base, top, mask):
    return Image.composite(top, base, mask)


def edge_mask(width, softness=6):
    """White towards the block's edges, where paint wears and grime gathers."""
    mask = Image.new("L", (SIZE, SIZE), 0)
    draw = ImageDraw.Draw(mask)
    for i in range(width):
        value = int(255 * (1 - i / width) ** 1.6)
        draw.rectangle([i, i, SIZE - 1 - i, SIZE - 1 - i], outline=value)
    return mask.filter(ImageFilter.GaussianBlur(softness / 3))


def finish(image, seed, grime=0.3, edge_dark=0.4):
    """Grime in the noise, and the block's edges darkened as contact shadow."""
    dirt = fractal(seed + 900, ((24, 0.4), (8, 0.35), (3, 0.25))).point(lambda v: int(max(0, v - 90) * 1.6))
    dark = Image.new("RGB", (SIZE, SIZE), (18, 18, 20))
    image = blend(image, dark, dirt.point(lambda v: int(v * grime)))
    image = blend(image, dark, edge_mask(10).point(lambda v: int(v * edge_dark)))
    return image


def painted_plate(seed, paint, bare=(150, 152, 156)):
    """Paint over metal, chipped back at the edges and in scuffs."""
    metal = tint(fractal(seed).point(lambda v: 150 + v // 3), bare)
    coat = tint(fractal(seed + 1, ((40, 0.6), (10, 0.4))).point(lambda v: 200 + v // 5), paint)
    wear = ImageChops.multiply(fractal(seed + 2, ((8, 0.6), (3, 0.4))), edge_mask(9))
    chips = wear.point(lambda v: 255 if v > 95 else 0).filter(ImageFilter.GaussianBlur(0.5))
    scuffs = fractal(seed + 3, ((10, 0.4), (2, 0.6))).point(lambda v: 255 if v > 205 else 0)
    image = blend(coat, metal, ImageChops.lighter(chips, scuffs))
    return image


def seams(image, step, colour=(26, 27, 30), width=2):
    draw = ImageDraw.Draw(image)
    for i in range(0, SIZE + 1, step):
        draw.line([(i, 0), (i, SIZE)], fill=colour, width=width)
        draw.line([(0, i), (SIZE, i)], fill=colour, width=width)
    return image


def rivets(image, positions, radius=2):
    draw = ImageDraw.Draw(image)
    for x, y in positions:
        draw.ellipse([x - radius, y - radius, x + radius, y + radius], fill=(60, 62, 66))
        draw.point((x - 1, y - 1), fill=(190, 192, 196))
    return image


def hull():
    image = painted_plate(11, (214, 210, 200))
    image = seams(image, SIZE)
    image = rivets(image, [(x, y) for x in (8, SIZE - 8) for y in range(8, SIZE, 24)])
    # A faded registration stripe on some plates is left to variants.
    return finish(image, 11)


def hull_stripe():
    image = painted_plate(12, (214, 210, 200))
    stripe = painted_plate(13, (176, 68, 32))
    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rectangle([0, 76, SIZE, 100], fill=255)
    image = blend(image, stripe, mask)
    return finish(seams(image, SIZE), 12)


def deck():
    """Tread plate: raised diamonds on bare steel, worn bright on top."""
    base = tint(fractal(21).point(lambda v: 110 + v // 3), (128, 130, 134))
    draw = ImageDraw.Draw(base)
    for row in range(0, SIZE, 16):
        for col in range(0, SIZE, 16):
            cx, cy = col + (8 if (row // 16) % 2 else 0), row + 8
            angle = math.radians(45 if (row // 16) % 2 else -45)
            dx, dy = 6 * math.cos(angle), 6 * math.sin(angle)
            draw.line([(cx - dx, cy - dy), (cx + dx, cy + dy)], fill=(176, 178, 182), width=3)
            draw.line([(cx - dx + 1, cy - dy + 2), (cx + dx + 1, cy + dy + 2)], fill=(70, 72, 76), width=1)
    return finish(seams(base, SIZE, width=3), 21, grime=0.45)


def bulkhead():
    """Interior panelling: pale panels, a vent grille, a recessed seam."""
    image = painted_plate(31, (178, 184, 186), bare=(120, 124, 128))
    draw = ImageDraw.Draw(image)
    draw.rectangle([6, 6, SIZE - 7, SIZE - 7], outline=(96, 100, 104), width=2)
    for y in range(40, 88, 6):
        draw.line([(30, y), (98, y)], fill=(40, 42, 46), width=3)
    return finish(image, 31, grime=0.25, edge_dark=0.35)


def frame():
    """Structural frame: dark primer, heavy bolts, a lightening hole."""
    image = painted_plate(41, (58, 62, 68), bare=(120, 122, 126))
    draw = ImageDraw.Draw(image)
    draw.ellipse([40, 40, 88, 88], fill=(16, 17, 19), outline=(90, 94, 100), width=3)
    image = rivets(image, [(16, 16), (112, 16), (16, 112), (112, 112)], radius=4)
    return finish(image, 41, grime=0.3)


def hazard():
    image = painted_plate(51, (226, 176, 28))
    black = painted_plate(52, (26, 26, 28), bare=(140, 142, 146))
    mask = Image.new("L", (SIZE, SIZE), 0)
    draw = ImageDraw.Draw(mask)
    for offset in range(-SIZE, SIZE * 2, 32):
        draw.polygon([(offset, 0), (offset + 16, 0), (offset + 16 - SIZE, SIZE), (offset - SIZE, SIZE)], fill=255)
    return finish(blend(image, black, mask), 51)


def light():
    """A light panel: a diffuser in a dark bezel."""
    image = Image.new("RGB", (SIZE, SIZE), (24, 26, 30))
    draw = ImageDraw.Draw(image)
    draw.rectangle([14, 14, SIZE - 15, SIZE - 15], fill=(238, 236, 226))
    glow = fractal(61, ((20, 1.0),)).point(lambda v: 225 + v // 10)
    panel = tint(glow, (255, 250, 236))
    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rectangle([18, 18, SIZE - 19, SIZE - 19], fill=255)
    return blend(image, panel, mask)


BLOCKS = {
    "hull": hull,
    "hull-stripe": hull_stripe,
    "deck": deck,
    "bulkhead": bulkhead,
    "frame": frame,
    "hazard": hazard,
    "light": light,
}

# Blocks that light themselves: shadow takes none of it, and bloom picks it up.
GLOWING = {"light"}


def tile(name):
    face = {"sprite": f"textures/blocks/{name}.png", "size": [1.0, 1.0]}
    out = {"faces": {"top": face, "bottom": face, "south": face, "east": face}}
    if name in GLOWING:
        out["glow"] = 1.0
    return out


def main():
    TEXTURES.mkdir(parents=True, exist_ok=True)
    for name, draw in BLOCKS.items():
        draw().save(TEXTURES / f"{name}.png", optimize=True)
    tileset = {"format_version": 1, "tiles": {name: tile(name) for name in BLOCKS}}
    (ASSETS / "blocks.tileset").write_text(json.dumps(tileset, indent=2) + "\n")


if __name__ == "__main__":
    main()
