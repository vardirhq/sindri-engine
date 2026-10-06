#!/usr/bin/env python3
"""Draws the Basin's blocks and writes the block set that names them.

The Basin is a voxel world viewed as a map, so each block needs only the face
that is seen from above. That one picture is used for every face, which keeps
the set valid for a world drawn as blocks too.

Each block has a few variants, chosen cell by cell by the world's variant
seed, so a salt flat a hundred columns wide does not read as one tile
stamped a hundred times. Run by `draw.py`; deterministic like it.
"""

import json
import math

from draw import CELL, OUT, Canvas, noise, sheet

SALT = (234, 230, 218)
SALT_SHADE = (218, 212, 196)
SALT_PINK = (238, 224, 214)
CRACK = (170, 160, 142)
MUD = (122, 104, 84)
MUD_WET = (104, 88, 70)
MUD_SHINE = (146, 128, 104)
SAND = (214, 196, 160)
SAND_RIPPLE = (192, 172, 134)
REEF = (214, 200, 184)
REEF_DEEP = (170, 150, 136)
REEF_PINK = (210, 160, 150)
KELP = (84, 88, 54)
KELP_DARK = (60, 64, 40)
KELP_DRY = (130, 120, 72)
ROCK = (134, 120, 108)
ROCK_DARK = (104, 92, 84)
GRASS = (122, 142, 82)
GRASS_DARK = (98, 118, 66)
GRASS_LIGHT = (150, 166, 96)
DIRT = (132, 104, 76)
BRINE = (58, 140, 136)
BRINE_LIGHT = (96, 176, 166)
TRUNK = (110, 82, 56)
LEAVES = (82, 120, 62)
LEAVES_DARK = (60, 96, 48)
LEAVES_LIGHT = (112, 150, 78)


def speckle(c, base, spots, count, salt, size=(0.8, 2.2)):
    c.rect(0, 0, CELL, CELL, base)
    for i in range(count):
        x = noise(i, salt, 41) * CELL
        y = noise(i, salt, 42) * CELL
        r = size[0] + noise(i, salt, 43) * (size[1] - size[0])
        color = spots[int(noise(i, salt, 44) * len(spots)) % len(spots)]
        c.ellipse(x, y, r, r * (0.6 + noise(i, salt, 45) * 0.6), color)


def dust(c, base, variant, seed):
    """Low contrast grains; the ground is a material, not a field of dots."""
    c.rect(0, 0, CELL, CELL, base)
    for i in range(38):
        x = noise(i, seed + variant, 41) * CELL
        y = noise(i, seed + variant, 42) * CELL
        r = 0.2 + noise(i, seed + variant, 43) * 0.6
        shift = int(noise(i, seed + variant, 44) * 16) - 8
        color = tuple(max(0, min(255, channel + shift)) for channel in base)
        c.ellipse(x, y, r, r * 0.65, color)


def fissure(c, variant, color):
    points = [(0, 12), (7, 15 + variant % 3), (16, 11), (23, 17), (32, 12)]
    for (ax, ay), (bx, by) in zip(points, points[1:]):
        c.line(ax, ay, bx, by, 0.7, color)
        c.line(ax, ay - 0.8, bx, by - 0.8, 0.45, (248, 242, 220))
    c.line(16, 11, 18, 4, 0.55, color)
    c.line(23, 17, 20, 24, 0.5, color)


def salt(variant):
    def draw(c):
        dust(c, SALT, variant, 100)
        if variant in (2, 5):
            fissure(c, variant, (199, 192, 173))
        if variant == 7:
            c.ellipse(20, 9, 2.2, 1.2, (218, 211, 192))
            c.line(18, 8, 21, 8, 0.6, (250, 245, 230))
    return draw


def crust(variant):
    def draw(c):
        dust(c, (230, 222, 204), variant, 110)
        for i in range(5):
            x = noise(i, 110 + variant, 51) * CELL
            y = noise(i, 110 + variant, 52) * CELL
            c.ellipse(x, y, 2.5, 1.5, (240, 232, 213, 100))
            c.line(x - 1, y, x + 1, y - 0.5, 0.5, (207, 195, 169))
        if variant in (1, 4, 7):
            fissure(c, variant, (182, 168, 141))
    return draw


def mud(variant):
    def draw(c):
        dust(c, MUD, variant, 120)
        for i in range(4):
            x = noise(i, 120 + variant, 51) * CELL
            y = noise(i, 120 + variant, 52) * CELL
            c.ellipse(x, y, 4, 1.5, (97, 84, 69, 100))
            c.line(x - 2, y - 1, x + 2, y - 1, 0.6, (149, 131, 106, 120))
        if variant == 3:
            fissure(c, variant, (85, 76, 65))
    return draw


def sand(variant):
    def draw(c):
        dust(c, SAND, variant, 130)
        # A periodic wave meets at both sides; its phase changes the ridges
        # without changing the base color at a tile boundary.
        for i in range(-1, 5):
            y = i * 9 + variant * 0.35
            for x in range(CELL):
                wave = math.sin(x / CELL * math.tau) * 1.4
                c.line(x, y + wave, x + 1, y + math.sin((x + 1) / CELL * math.tau) * 1.4,
                       0.65, (196, 177, 139, 160))
                c.ellipse(x + 0.5, y + wave - 1.2, 0.55, 0.3, (236, 217, 178, 150))
    return draw


def reef(variant):
    def draw(c):
        dust(c, (188, 176, 155), variant, 140)
        for i in range(5):
            x = noise(i, 140 + variant, 51) * CELL
            y = noise(i, 140 + variant, 52) * CELL
            r = 2 + noise(i, 140 + variant, 53) * 4
            c.ellipse(x, y + 0.8, r + 0.5, r * 0.8, (136, 122, 104, 110))
            c.ellipse(x, y, r, r * 0.8, (224, 210, 186))
            c.ring(x, y, r * 0.65, r * 0.45, (178, 159, 136))
            c.ellipse(x - r * 0.3, y - r * 0.2, 1, 0.6, (242, 229, 205))
    return draw


def kelp(variant):
    def draw(c):
        dust(c, (112, 112, 77), variant, 150)
        for i in range(3 + variant % 3):
            x = noise(i, 150 + variant, 61) * CELL
            y = noise(i, 150 + variant, 62) * CELL
            a = noise(i, 150 + variant, 63) * math.tau
            color = (86, 91, 59) if i % 2 else (145, 133, 86)
            for j in range(5):
                bend = a + math.sin(j * 0.8) * 0.45
                nx, ny = x + math.cos(bend) * 2.5, y + math.sin(bend) * 2.5
                c.line(x, y, nx, ny, 0.85, color)
                if j % 2:
                    c.ellipse(nx + math.sin(bend) * 1.2, ny - math.cos(bend) * 1.2, 1.6, 0.8, color)
                x, y = nx, ny
        if variant == 6:
            c.ellipse(8, 23, 1.8, 1, (184, 169, 124))
    return draw


def rock(variant):
    def draw(c):
        dust(c, ROCK, variant, 160)
        for i in range(4):
            x = noise(i, 160 + variant, 51) * CELL
            y = noise(i, 160 + variant, 52) * CELL
            r = 2 + noise(i, 160 + variant, 53) * 3
            c.ellipse(x, y + 1, r, r * 0.7, (86, 78, 69, 100))
            c.ellipse(x, y, r, r * 0.7, (160, 145, 125))
            c.line(x - r * 0.6, y - r * 0.3, x + r * 0.5, y - r * 0.3, 0.7, (185, 170, 146))
    return draw


def grass(variant):
    def draw(c):
        dust(c, GRASS, variant, 170)
        for i in range(10):
            x = noise(i, 170 + variant, 61) * CELL
            y = noise(i, 170 + variant, 62) * CELL
            c.line(x, y, x - 1.3, y - 2, 0.6, GRASS_LIGHT)
            c.line(x, y, x + 1, y - 2.5, 0.6, GRASS_DARK)
    return draw


def dirt(c):
    dust(c, DIRT, 0, 180)


def brine(variant):
    def draw(c):
        dust(c, (57, 125, 119), variant, 190)
        for i in range(3):
            x = noise(i, 190 + variant, 71) * 28
            y = noise(i, 190 + variant, 72) * 32
            length = 3 + noise(i, 190 + variant, 73) * 6
            c.line(x, y, x + length, y - 0.7, 0.5, (107, 169, 156, 100))
            c.line(x + 1, y + 1, x + length - 1, y + 0.5, 0.5, (30, 101, 96, 80))
    return draw


def trunk(c):
    c.rect(0, 0, CELL, CELL, LEAVES_DARK)
    c.ellipse(16, 16, 7, 7, TRUNK)


def leaves(variant):
    def draw(c):
        c.rect(0, 0, CELL, CELL, LEAVES)
        for i in range(8):
            x = noise(i, 190 + variant, 81) * CELL
            y = noise(i, 190 + variant, 82) * CELL
            r = 3 + noise(i, 190 + variant, 83) * 4
            color = LEAVES_LIGHT if i % 2 else LEAVES_DARK
            c.ellipse(x, y, r, r, color)

    return draw


# Each block: its pictures (the first is its own face, the rest variants) and
# what it is to the game. `soft` slows the crawler, `rough` slows it more and
# wears it; `liquid` stops it; `salvage` is worth stopping for.
BLOCKS = {
    "salt": ([salt(i) for i in range(8)], {}),
    "crust": ([crust(i) for i in range(8)], {"tags": ["soft"]}),
    "mud": ([mud(i) for i in range(8)], {"tags": ["soft", "wet"]}),
    "sand": ([sand(i) for i in range(8)], {"tags": ["soft"]}),
    "reef": ([reef(i) for i in range(8)], {"tags": ["rough"]}),
    "kelp": ([kelp(i) for i in range(8)], {"tags": ["soft"]}),
    "rock": ([rock(i) for i in range(8)], {"tags": ["rough"]}),
    "grass": ([grass(i) for i in range(8)], {}),
    "dirt": ([dirt], {}),
    "brine": ([brine(i) for i in range(8)], {"occludes": False, "walkable": False, "tags": ["liquid", "wet"]}),
    "trunk": ([trunk], {"tags": ["rough"]}),
    "leaves": ([leaves(0), leaves(1)], {"occludes": False, "tags": ["rough"]}),
}


def main():
    cells = []
    for name, (pictures, _) in BLOCKS.items():
        for index, picture in enumerate(pictures):
            cells.append((f"{name}-{index}", lambda c, p=picture: p(c)))
    sheet("terrain", cells, 6, CELL, CELL)

    def faces(sprite):
        face = {"sprite": f"textures/terrain.png#{sprite}", "size": [1, 1]}
        return {"top": face, "bottom": face, "south": face, "east": face}

    tiles = {}
    for name, (pictures, flags) in BLOCKS.items():
        tile = {"faces": faces(f"{name}-0"), **flags}
        if len(pictures) > 1:
            tile["variants"] = [{"faces": faces(f"{name}-{i}")} for i in range(1, len(pictures))]
        tiles[name] = tile
    document = {"format_version": 1, "tiles": tiles}
    (OUT.parent / "basin.tileset").write_text(json.dumps(document, indent=1) + "\n")


if __name__ == "__main__":
    main()
