#!/usr/bin/env python3
"""Draws Low Tide's art: the crawler's tiles, the salt flats, the crew and a wreck.

Run from anywhere; it writes into ../assets/textures. Deterministic, so running
it again changes nothing unless this file changed. Pure Python, no imaging
library, like the platformer's `draw.py`.

Everything is drawn straight down from above, as a floor plan: walls are thick
strips with no front faces and nothing has a shadow baked in. That is what lets
the crawler turn to any angle and still look right. Shapes are drawn four times
too large and averaged down, so their edges stay smooth when the crawler is
turned and the picture is resampled.
"""

import json
import math
import pathlib
import struct
import zlib

OUT = pathlib.Path(__file__).resolve().parent.parent / "assets" / "textures"
CELL = 32
SS = 4

# The palette. Salt and sand for the world, rust and iron for the crawler,
# warm lamp light inside, and no blue anywhere: the Tide owns the one cool
# colour, so it is always the thing your eye finds.
SALT = (232, 228, 216)
SALT_DIM = (226, 221, 207)
SAND = (212, 196, 164)
SAND_DARK = (188, 168, 132)
CRACK = (156, 146, 128)
ROCK = (150, 130, 112)
ROCK_DARK = (118, 100, 86)
SCRUB = (128, 140, 92)
DECK = (150, 112, 80)
DECK_LINE = (122, 88, 62)
WALL = (52, 42, 40)
WALL_EDGE = (84, 66, 58)
IRON = (96, 92, 90)
IRON_DARK = (62, 60, 60)
RUST = (168, 84, 48)
LAMP = (255, 214, 140)
GLASS = (250, 226, 170)
WOOD = (176, 124, 72)
WOOD_DARK = (128, 86, 48)
CLOTH = (190, 92, 70)
LINEN = (236, 222, 196)
STRIPE = (232, 176, 52)
INK = (34, 28, 28)
SKIN = (226, 176, 136)
HAIR = (70, 46, 36)
COAT = (176, 96, 52)
TROUSERS = (82, 70, 64)


def png(path, width, height, pixels):
    """Writes RGBA `pixels` (rows of (r, g, b, a) tuples) as a PNG."""
    raw = b"".join(
        b"\x00" + b"".join(bytes(pixel) for pixel in row) for row in pixels
    )

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def noise(x, y, salt):
    """A repeatable scatter in [0, 1)."""
    value = (x * 374761393 + y * 668265263 + salt * 2246822519) & 0xFFFFFFFF
    value = ((value ^ (value >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((value ^ (value >> 16)) & 0xFFFF) / 65536.0


class Canvas:
    """A supersampled RGBA canvas, measured in output pixels."""

    def __init__(self, width, height, fill=None):
        self.width = width
        self.height = height
        self.w = width * SS
        self.h = height * SS
        base = (fill[0], fill[1], fill[2], 255) if fill else (0, 0, 0, 0)
        self.px = [list(base) for _ in range(self.w * self.h)]

    def _blend(self, index, colour, alpha):
        if alpha <= 0:
            return
        under = self.px[index]
        a = alpha * (colour[3] / 255 if len(colour) > 3 else 1.0)
        out_a = a + under[3] / 255 * (1 - a)
        if out_a <= 0:
            return
        for channel in range(3):
            under[channel] = (
                colour[channel] * a + under[channel] * (under[3] / 255) * (1 - a)
            ) / out_a
        under[3] = out_a * 255

    def paint(self, inside, colour, box=None):
        """Fills every sub-pixel whose centre `inside(x, y)` accepts.

        Coordinates are output pixels, with fractions."""
        x0, y0, x1, y1 = box or (0, 0, self.width, self.height)
        for sy in range(max(0, int(y0 * SS)), min(self.h, int(math.ceil(y1 * SS)))):
            y = (sy + 0.5) / SS
            for sx in range(max(0, int(x0 * SS)), min(self.w, int(math.ceil(x1 * SS)))):
                x = (sx + 0.5) / SS
                if inside(x, y):
                    self._blend(sy * self.w + sx, colour, 1.0)

    def rect(self, x0, y0, x1, y1, colour, radius=0.0):
        def inside(x, y):
            if not (x0 <= x <= x1 and y0 <= y <= y1):
                return False
            if radius <= 0:
                return True
            cx = min(max(x, x0 + radius), x1 - radius)
            cy = min(max(y, y0 + radius), y1 - radius)
            return (x - cx) ** 2 + (y - cy) ** 2 <= radius * radius

        self.paint(inside, colour, (x0, y0, x1, y1))

    def ellipse(self, cx, cy, rx, ry, colour):
        self.paint(
            lambda x, y: ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0,
            colour,
            (cx - rx, cy - ry, cx + rx, cy + ry),
        )

    def ring(self, cx, cy, r_out, r_in, colour):
        def inside(x, y):
            d = (x - cx) ** 2 + (y - cy) ** 2
            return r_in * r_in <= d <= r_out * r_out

        self.paint(inside, colour, (cx - r_out, cy - r_out, cx + r_out, cy + r_out))

    def line(self, ax, ay, bx, by, width, colour):
        dx, dy = bx - ax, by - ay
        length2 = dx * dx + dy * dy or 1.0
        half = width / 2

        def inside(x, y):
            t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / length2))
            px, py = ax + t * dx, ay + t * dy
            return (x - px) ** 2 + (y - py) ** 2 <= half * half

        pad = half + 1
        self.paint(
            inside,
            colour,
            (min(ax, bx) - pad, min(ay, by) - pad, max(ax, bx) + pad, max(ay, by) + pad),
        )

    def pixels(self):
        """Averages the sub-pixels down to output pixels."""
        rows = []
        for y in range(self.height):
            row = []
            for x in range(self.width):
                total = [0.0, 0.0, 0.0, 0.0]
                for sy in range(SS):
                    for sx in range(SS):
                        p = self.px[(y * SS + sy) * self.w + x * SS + sx]
                        a = p[3] / 255
                        total[0] += p[0] * a
                        total[1] += p[1] * a
                        total[2] += p[2] * a
                        total[3] += a
                count = SS * SS
                alpha = total[3] / count
                if total[3] > 0:
                    rgb = [round(total[c] / total[3]) for c in range(3)]
                else:
                    rgb = [0, 0, 0]
                row.append((*rgb, round(alpha * 255)))
            rows.append(row)
        return rows


def sheet(name, cells, columns, cell_w, cell_h):
    """Lays `cells` (name, draw) out in a grid and writes the PNG and sheet."""
    rows_needed = (len(cells) + columns - 1) // columns
    width, height = columns * cell_w, rows_needed * cell_h
    out = [[(0, 0, 0, 0)] * width for _ in range(height)]
    for index, (_, draw) in enumerate(cells):
        canvas = Canvas(cell_w, cell_h)
        draw(canvas)
        pixels = canvas.pixels()
        ox, oy = (index % columns) * cell_w, (index // columns) * cell_h
        for y in range(cell_h):
            out[oy + y][ox : ox + cell_w] = pixels[y]
    png(OUT / f"{name}.png", width, height, out)
    document = {
        "format_version": 1,
        "anchor": "center",
        "grid": {
            "columns": columns,
            "rows": rows_needed,
            "names": [cell_name for cell_name, _ in cells],
        },
    }
    (OUT / f"{name}.sheet").write_text(json.dumps(document, indent=2) + "\n")


# --- The crawler's tiles -------------------------------------------------


def deck(c, salt=0):
    """Warm, worn boards. Grain stays quiet at phone scale."""
    c.rect(0, 0, CELL, CELL, (157, 119, 85))
    for i in range(4):
        y = i * 8
        tone = 145 + int(noise(i, salt, 3) * 18)
        c.rect(0, y + 1, CELL, y + 8, (tone, tone - 37, tone - 68))
        c.line(0, y + 1, CELL, y + 1, 0.6, (188, 148, 107))
        c.line(0, y, CELL, y, 0.8, (105, 77, 55))
        offset = 6 + int(noise(i, salt, 4) * 18)
        c.line(offset, y + 1, offset, y + 8, 0.7, DECK_LINE)
        c.line(2, y + 5, offset - 2, y + 4.5, 0.35, (132, 97, 67))
        c.ellipse(offset + 2, y + 3, 0.5, 0.5, WOOD_DARK)


def steel(c, hold=False):
    base = (97, 109, 104) if not hold else (84, 86, 77)
    c.rect(0, 0, CELL, CELL, base)
    c.line(0, 0.7, CELL, 0.7, 1.0, (132, 141, 129))
    c.line(0.7, 0, 0.7, CELL, 1.0, (132, 141, 129))
    c.line(31, 0, 31, CELL, 0.9, (57, 63, 57))
    c.line(0, 31, CELL, 31, 0.9, (57, 63, 57))
    for x in (3, 29):
        for y in (3, 29):
            c.ellipse(x, y, 0.8, 0.8, (55, 60, 54))
    if hold:
        for y in range(7, 29, 7):
            for x in range(6, 30, 7):
                c.line(x, y, x + 2, y - 2, 0.65, (112, 114, 99))
    else:
        c.line(8, 24, 21, 24, 0.5, (83, 96, 88))


def wall(c):
    c.rect(0, 0, CELL, CELL, (45, 42, 37))
    c.rect(1, 1, CELL - 1, CELL - 1, (119, 99, 76), 1)
    c.rect(3, 3, CELL - 3, CELL - 3, (68, 62, 51), 1)
    c.rect(5, 5, CELL - 5, CELL - 5, (76, 68, 54), 1)
    c.line(5, 5, 27, 5, 0.8, (99, 87, 68))
    for x, y in ((3, 3), (29, 3), (3, 29), (29, 29)):
        c.ellipse(x, y, 0.8, 0.8, (179, 151, 111))


def window(c):
    wall(c)
    c.rect(3, 10, CELL - 3, 22, INK, 1)
    c.rect(5, 12, CELL - 5, 20, (187, 169, 124))
    c.rect(6, 12, 26, 14, LAMP)
    c.line(7, 18, 14, 14, 1.2, GLASS)
    c.line(19, 18, 24, 15, 0.8, GLASS)
    c.rect(15, 11, 17, 21, WALL_EDGE)


def door(c):
    deck(c, 1)
    c.rect(0, 0, 4, CELL, WALL)
    c.rect(CELL - 4, 0, CELL, CELL, WALL)
    c.line(4, 14, CELL - 4, 14, 1.2, (193, 153, 97))
    c.line(4, 17, CELL - 4, 17, 1.2, WOOD_DARK)


def hatch(c):
    steel(c, True)
    for i in range(-2, 6):
        x = i * 8
        c.line(x, CELL, x + CELL / 2, 0, 3.0, STRIPE)
    c.rect(4, 4, CELL - 4, CELL - 4, IRON_DARK, 1)
    c.rect(6, 6, CELL - 6, CELL - 6, IRON, 1)
    c.line(11, 13, 11, 20, 2, LINEN)
    c.line(11, 20, 20, 20, 2, LINEN)


def helm(c):
    steel(c)
    c.rect(3, 1, 29, 10, INK, 2)
    c.rect(4, 2, 28, 9, (110, 94, 63), 1)
    for x in (8, 16, 24):
        c.ellipse(x, 5.5, 2.5, 2.5, LINEN)
        c.line(x, 5.5, x + 1, 4, 0.7, INK)
    c.ellipse(16, 20, 10.5, 10.5, (36, 36, 30, 100))
    c.ring(16, 19, 10, 7.5, (201, 151, 84))
    for angle in range(0, 360, 60):
        a = math.radians(angle)
        c.line(16, 19, 16 + 11 * math.cos(a), 19 + 11 * math.sin(a), 1.5, WOOD_DARK)
    c.ellipse(16, 19, 3, 3, (198, 172, 109))


def table(c):
    deck(c, 3)
    c.rect(2, 5, 30, 29, (38, 29, 23, 90), 3)
    c.rect(3, 4, CELL - 3, CELL - 5, WOOD_DARK, 3)
    c.rect(4, 5, CELL - 4, CELL - 7, (194, 150, 97), 2)
    c.line(5, 8, 27, 8, 0.7, (225, 183, 127))
    c.rect(7, 10, 23, 22, LINEN, 0.6)
    c.line(12, 11, 14, 17, 0.7, (152, 133, 93))
    c.line(14, 17, 21, 19, 0.7, (152, 133, 93))
    c.ellipse(24, 10, 2.4, 2.4, INK)
    c.ellipse(24, 10, 1.6, 1.6, LINEN)
    c.ellipse(24, 10, 1, 1, (85, 58, 38))
    c.rect(6, 24, 11, 27, (132, 76, 52), 0.5)


def bunk(c):
    deck(c, 4)
    c.rect(2, 2, 30, 31, (45, 30, 22, 100), 2)
    c.rect(3, 1, CELL - 3, CELL - 2, WOOD_DARK, 2)
    c.rect(5, 3, CELL - 5, CELL - 4, (155, 73, 51), 1)
    c.rect(6, 4, CELL - 6, 11, LINEN, 2)
    c.line(8, 6, 24, 6, 0.6, (255, 241, 205))
    c.rect(6, 14, 26, 17, (190, 108, 66))
    c.line(8, 18, 8, 26, 0.5, (208, 135, 86))
    c.line(24, 18, 24, 26, 0.5, (208, 135, 86))
    c.rect(6, 26, 26, 28, (110, 56, 41))


def engine(c):
    steel(c, True)
    c.rect(2, 3, 30, 30, (26, 29, 26, 130), 3)
    c.rect(3, 2, CELL - 3, CELL - 4, IRON_DARK, 3)
    c.rect(6, 3, 26, 26, (118, 122, 103), 2)
    for y in range(7, 23, 3):
        c.line(7, y, 25, y, 1.1, (59, 66, 59))
    c.rect(11, 7, 21, 23, (168, 84, 48), 2)
    for y in (10, 15, 20):
        c.ellipse(16, y, 2, 2, (202, 129, 67))
    c.line(3, 26, 27, 26, 2, (168, 145, 100))
    c.rect(24, 3, 27, 6, LAMP, 0.7)
    c.rect(4, 3, 8, 6, (52, 54, 48))


def upgraded_engine(c):
    engine(c)
    c.rect(9, 6, 23, 24, (91, 133, 107), 2)
    for y in (10, 15, 20):
        c.ellipse(16, y, 2, 2, LAMP)


def tread(c, phase=0, ramp_gap=False):
    c.rect(0, 0, CELL, c.height, (42, 43, 38))
    c.rect(2, 0, CELL - 2, c.height, (65, 66, 58))
    for i in range(-1, c.height // 8 + 1):
        y = i * 8 + 2 + phase
        c.rect(3, y, CELL - 3, y + 5, (96, 98, 84), 0.5)
        c.line(4, y + 1, 28, y + 1, 0.7, (129, 130, 109))
        c.line(4, y + 4, 28, y + 4, 0.7, (53, 56, 49))
        c.ellipse(8, y + 2.5, 1, 1, IRON_DARK)
        c.ellipse(24, y + 2.5, 1, 1, IRON_DARK)
    c.line(1, 0, 1, c.height, 1, (148, 124, 85))
    c.line(31, 0, 31, c.height, 1, (30, 33, 29))
    if ramp_gap:
        # The starboard belt is interrupted by the physical boarding ramp.
        for y in range(CELL * 4 * SS, CELL * 5 * SS):
            for x in range(c.w):
                c.px[y * c.w + x] = [0, 0, 0, 0]


def ramp(c):
    c.rect(0, 0, CELL, CELL, IRON)
    for i in range(4):
        x = i * 8 + 4
        c.line(x, 8, x + 4, 16, 2.0, IRON_DARK)
        c.line(x + 4, 16, x, 24, 2.0, IRON_DARK)


def crate(c):
    deck(c, 6)
    c.rect(3, 3, CELL - 3, CELL - 3, WOOD_DARK, 2)
    c.rect(5, 5, CELL - 5, CELL - 5, WOOD, 1)
    c.line(6, 6, CELL - 6, CELL - 6, 2.5, WOOD_DARK)
    c.line(CELL - 6, 6, 6, CELL - 6, 2.5, WOOD_DARK)
    c.rect(12, 13, 20, 19, RUST, 1)


def resource_crate(c, kind):
    c.rect(3, 3, 29, 29, WOOD_DARK, 2)
    c.rect(5, 5, 27, 27, WOOD, 1)
    c.rect(7, 7, 25, 25, INK, 1)
    if kind == "wood":
        for y in (11, 17, 23):
            c.line(9, y, 23, y - 2, 4, WOOD)
            c.ellipse(9, y, 2, 2, WOOD_DARK)
    elif kind in ("stone", "ore"):
        for x, y in ((11, 12), (20, 11), (16, 21)):
            c.ellipse(x, y, 4, 4, IRON)
            c.ellipse(x - 1, y - 1, 2, 2, RUST if kind == "ore" else IRON_DARK)
    elif kind == "fibre":
        for x in (10, 14, 18, 22):
            c.line(x, 9, x - 2, 23, 3, WOOD)
        c.line(7, 16, 25, 16, 2, RUST)
    else:
        for x, y in ((12, 13), (20, 20)):
            c.ellipse(x, y, 5, 6, SALT)
            c.line(x - 3, y - 3, x + 3, y - 3, 1, WOOD_DARK)
    c.rect(4, 25, 28, 28, WOOD_DARK)


def workbench(c):
    c.rect(3, 3, 29, 29, WOOD_DARK, 2)
    c.rect(4, 4, 28, 25, WOOD, 1)
    for y in (10, 17, 23):
        c.line(5, y, 27, y, 0.8, WOOD_DARK)
    c.rect(7, 7, 17, 19, LINEN, 0.5)
    c.line(9, 10, 15, 10, 0.7, IRON_DARK)
    c.line(9, 13, 14, 13, 0.7, IRON_DARK)
    c.line(21, 10, 21, 22, 2.5, WOOD_DARK)
    c.rect(17, 8, 25, 12, IRON, 1)
    c.ellipse(23, 23, 2.5, 2.5, RUST)


def mast(c):
    deck(c, 7)
    c.ellipse(16, 16, 9, 9, WOOD_DARK)
    c.ellipse(16, 16, 6, 6, WOOD)
    c.line(2, 16, CELL - 2, 16, 2.5, INK)


# --- The salt flats ------------------------------------------------------


def salt(variant):
    def draw(c):
        c.rect(0, 0, CELL, CELL, SALT)
        for i in range(4 + variant * 3):
            x = noise(i, variant, 11) * CELL
            y = noise(i, variant, 12) * CELL
            size = 0.8 + noise(i, variant, 13) * 1.4
            c.ellipse(x, y, size, size, SALT_DIM)

    return draw


def crack(c):
    salt(0)(c)
    points = [(0, 12), (9, 15), (15, 9), (22, 18), (CELL, 14)]
    for (ax, ay), (bx, by) in zip(points, points[1:]):
        c.line(ax, ay, bx, by, 1.6, CRACK)
    c.line(15, 9, 18, 0, 1.2, CRACK)
    c.line(22, 18, 26, CELL, 1.2, CRACK)


def dune(c):
    c.rect(0, 0, CELL, CELL, SAND)
    for i in range(4):
        y = i * 8 + 4
        for x in range(0, CELL, 2):
            wave = math.sin((x + i * 5) / CELL * math.tau) * 1.8
            c.ellipse(x + 1, y + wave, 1.2, 0.9, SAND_DARK)


def rise(c):
    c.rect(0, 0, CELL, CELL, ROCK)
    for i in range(6):
        x = noise(i, 7, 21) * CELL
        y = noise(i, 7, 22) * CELL
        c.ellipse(x, y, 3 + noise(i, 7, 23) * 4, 2 + noise(i, 7, 24) * 3, ROCK_DARK)
    for i in range(3):
        x = noise(i, 9, 25) * CELL
        y = noise(i, 9, 26) * CELL
        c.ellipse(x, y, 2.5, 2.5, SCRUB)


# --- The crew ------------------------------------------------------------
#
# Paper dolls rather than overhead people: a front, a back and a side, each
# with a step. The game picks which to show from the way they move on screen,
# so a crew member never lies on their side when the crawler turns.


def person(view, step):
    def draw(c):
        stride = 2.5 if step else -2.5
        c.ellipse(16, 29, 8, 2.5, (0, 0, 0, 60))
        if view == "side":
            c.rect(13 + stride, 22, 17 + stride, 29, TROUSERS, 1.5)
            c.rect(15 - stride, 22, 19 - stride, 29, TROUSERS, 1.5)
            c.rect(10, 12, 22, 24, COAT, 4)
            c.rect(10, 12, 22, 24, (0, 0, 0, 0))
            c.ellipse(16, 9, 6.5, 6.5, SKIN)
            c.paint(lambda x, y: (x - 15) ** 2 + (y - 7.5) ** 2 <= 42 and x < 17.5, HAIR)
            c.ellipse(19.5, 9, 1, 1, INK)
            c.rect(14, 15, 19, 21, (150, 80, 42), 2)
        else:
            c.rect(10.5, 22 + (stride if view == "down" else -stride) * 0.3, 15, 29, TROUSERS, 1.5)
            c.rect(17, 22 - (stride if view == "down" else -stride) * 0.3, 21.5, 29, TROUSERS, 1.5)
            c.rect(8, 12, 24, 24, COAT, 4)
            c.ellipse(16, 9, 6.5, 6.5, SKIN)
            if view == "down":
                c.paint(lambda x, y: (x - 16) ** 2 + (y - 7.5) ** 2 <= 42 and y < 6.5, HAIR)
                c.ellipse(13.5, 9.5, 1, 1, INK)
                c.ellipse(18.5, 9.5, 1, 1, INK)
                c.rect(15, 15.5, 17, 24, (150, 80, 42))
            else:
                c.ellipse(16, 8.5, 6.5, 6.2, HAIR)
                c.rect(8, 14, 24, 15.5, (150, 80, 42))

    return draw


# --- A wreck -------------------------------------------------------------


def wreck(c):
    w, h = c.width, c.height
    c.ellipse(w / 2, h / 2 + 4, w / 2 - 4, h / 2 - 8, (180, 170, 150, 160))
    c.paint(
        lambda x, y: ((x - w / 2) / (w / 2 - 10)) ** 2 + ((y - h / 2) / (h / 2 - 14)) ** 2 <= 1
        and x < w * 0.78,
        RUST,
    )
    c.paint(
        lambda x, y: ((x - w / 2) / (w / 2 - 16)) ** 2 + ((y - h / 2) / (h / 2 - 20)) ** 2 <= 1
        and x < w * 0.74,
        (120, 66, 42),
    )
    for i in range(7):
        x = 24 + i * 11
        c.line(x, 18, x, h - 18, 3.0, (92, 54, 38))
    c.line(16, h / 2, w * 0.74, h / 2, 4.0, (92, 54, 38))
    for i in range(5):
        x = w * 0.78 + noise(i, 1, 31) * 18
        y = 20 + noise(i, 1, 32) * (h - 40)
        c.rect(x, y, x + 5, y + 4, RUST, 1)
    c.rect(36, 28, 52, 40, (200, 180, 120), 2)


def cabin_art():
    rug = Canvas(96, 128)
    rug.rect(3, 3, 93, 125, (60, 38, 26, 55), 1)
    rug.rect(4, 2, 92, 124, (125, 67, 48), 1)
    rug.rect(8, 6, 88, 120, (173, 109, 70))
    rug.rect(12, 10, 84, 116, (139, 82, 55))
    for y in range(16, 116, 20):
        rug.line(20, y, 48, y + 7, 1.5, (202, 153, 94))
        rug.line(48, y + 7, 76, y, 1.5, (202, 153, 94))
        rug.line(20, y + 1, 48, y + 9, 0.5, (95, 57, 39))
    for x in range(8, 90, 4):
        rug.line(x, 0, x, 3, 0.7, LINEN)
        rug.line(x, 123, x, 127, 0.7, LINEN)
    for i in range(160):
        x = noise(i, 76, 11) * 80 + 8
        y = noise(i, 76, 12) * 110 + 6
        rug.line(x, y, x + 2, y, 0.4, (214, 165, 107, 65))
    png(OUT / "runner.png", 96, 128, rug.pixels())
    pipe = Canvas(32, 128)
    pipe.rect(11, 0, 23, 128, (22, 22, 18, 70))
    pipe.rect(11, 0, 20, 128, (86, 65, 42))
    pipe.rect(12, 0, 19, 128, (153, 115, 66))
    pipe.rect(13, 0, 15, 128, (202, 160, 95))
    for y in (12, 46, 82, 116):
        pipe.rect(8, y, 23, y + 4, (64, 68, 56), 0.5)
        pipe.ellipse(10, y + 2, 1, 1, (188, 172, 121))
        pipe.ellipse(21, y + 2, 1, 1, (188, 172, 121))
    pipe.ring(16, 66, 7, 5, (162, 77, 41))
    pipe.line(10, 66, 22, 66, 1, (112, 59, 35))
    pipe.ellipse(16, 66, 2, 2, (194, 142, 75))
    png(OUT / "pipe.png", 32, 128, pipe.pixels())


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    cabin_art()
    sheet(
        "ship",
        [
            ("deck", lambda c: None),
            ("wall", wall),
            ("window", window),
            ("door", door),
            ("hatch", hatch),
            ("helm", helm),
            ("table", table),
            ("bunk", bunk),
            ("engine", engine),
            ("tread", lambda c: None),
            ("ramp", ramp),
            ("crate", crate),
            ("mast", mast),
            ("floor-home", deck),
            ("floor-bridge", steel),
            ("floor-hold", lambda c: steel(c, True)),
            *[(f"crate-{kind}", lambda c, k=kind: resource_crate(c, k))
              for kind in ("wood", "stone", "ore", "fibre", "salt")],
            ("workbench", workbench),
            ("engine-upgraded", upgraded_engine),
        ],
        4,
        CELL,
        CELL,
    )
    sheet("treads", [(f"{side}-{i}", lambda c, phase=i, gap=False: tread(c, phase, gap))
                     for side in ("port", "starboard") for i in range(8)],
          8, CELL, CELL)
    sheet(
        "ground",
        [
            ("salt", salt(0)),
            ("salt-dim", salt(1)),
            ("salt-fleck", salt(2)),
            ("crack", crack),
            ("dune", dune),
            ("rise", rise),
        ],
        3,
        CELL,
        CELL,
    )
    sheet(
        "crew",
        [
            (f"{view}-{step}", person(view, step))
            for view in ("down", "up", "side")
            for step in (0, 1)
        ],
        6,
        CELL,
        CELL,
    )
    canvas = Canvas(112, 72)
    wreck(canvas)
    png(OUT / "wreck.png", 112, 72, canvas.pixels())

    import terrain

    terrain.main()


if __name__ == "__main__":
    main()
