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
        colour = spots[int(noise(i, salt, 44) * len(spots)) % len(spots)]
        c.ellipse(x, y, r, r * (0.6 + noise(i, salt, 45) * 0.6), colour)


def salt(variant):
    def draw(c):
        speckle(c, SALT, [SALT_SHADE], 6 + variant * 2, 100 + variant)
        if variant == 2:
            points = [(0, 10), (8, 14), (13, 8), (21, 17), (CELL, 13)]
            for (ax, ay), (bx, by) in zip(points, points[1:]):
                c.line(ax, ay, bx, by, 1.3, CRACK)
            c.line(13, 8, 16, 0, 1.0, CRACK)

    return draw


def crust(variant):
    def draw(c):
        speckle(c, SALT_PINK, [SALT, SALT_SHADE], 14, 110 + variant, (1.0, 3.0))

    return draw


def mud(variant):
    def draw(c):
        speckle(c, MUD, [MUD_WET, MUD_SHINE], 10, 120 + variant, (1.2, 3.4))
        if variant == 1:
            c.line(4, 20, 28, 16, 1.2, MUD_WET)

    return draw


def sand(variant):
    def draw(c):
        c.rect(0, 0, CELL, CELL, SAND)
        for i in range(4):
            y = i * 8 + 3 + variant * 2
            for x in range(0, CELL, 2):
                wave = math.sin((x + i * 7 + variant * 5) / CELL * math.tau) * 1.6
                c.ellipse(x + 1, y + wave, 1.1, 0.8, SAND_RIPPLE)

    return draw


def reef(variant):
    def draw(c):
        c.rect(0, 0, CELL, CELL, REEF_DEEP)
        for i in range(7):
            x = noise(i, 130 + variant, 51) * CELL
            y = noise(i, 130 + variant, 52) * CELL
            r = 3 + noise(i, 130 + variant, 53) * 4
            c.ellipse(x, y, r, r, REEF)
            c.ring(x, y, r, r - 1.2, REEF_DEEP)
        x = noise(9, 130 + variant, 54) * CELL
        y = noise(9, 130 + variant, 55) * CELL
        c.ellipse(x, y, 2.5, 2.5, REEF_PINK)

    return draw


def kelp(variant):
    def draw(c):
        c.rect(0, 0, CELL, CELL, KELP_DARK)
        for i in range(9):
            x = noise(i, 140 + variant, 61) * CELL
            y = noise(i, 140 + variant, 62) * CELL
            a = noise(i, 140 + variant, 63) * math.tau
            colour = KELP_DRY if i % 3 == 0 else KELP
            c.line(x, y, x + math.cos(a) * 9, y + math.sin(a) * 9, 2.2, colour)

    return draw


def rock(variant):
    def draw(c):
        speckle(c, ROCK, [ROCK_DARK], 8, 150 + variant, (2.0, 5.0))

    return draw


def grass(variant):
    def draw(c):
        speckle(c, GRASS, [GRASS_DARK, GRASS_LIGHT], 16, 160 + variant, (0.8, 2.0))

    return draw


def dirt(c):
    speckle(c, DIRT, [MUD], 8, 170)


def brine(variant):
    def draw(c):
        c.rect(0, 0, CELL, CELL, BRINE)
        for i in range(3):
            y = 6 + i * 10 + variant * 3
            x = noise(i, 180 + variant, 71) * 16
            c.line(x, y, x + 10, y, 1.3, BRINE_LIGHT)

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
            colour = LEAVES_LIGHT if i % 2 else LEAVES_DARK
            c.ellipse(x, y, r, r, colour)

    return draw


# Each block: its pictures (the first is its own face, the rest variants) and
# what it is to the game. `soft` slows the crawler, `rough` slows it more and
# wears it; `liquid` stops it; `salvage` is worth stopping for.
BLOCKS = {
    "salt": ([salt(0), salt(1), salt(2)], {}),
    "crust": ([crust(0), crust(1)], {"tags": ["soft"]}),
    "mud": ([mud(0), mud(1)], {"tags": ["soft", "wet"]}),
    "sand": ([sand(0), sand(1)], {"tags": ["soft"]}),
    "reef": ([reef(0), reef(1)], {"tags": ["rough"]}),
    "kelp": ([kelp(0), kelp(1)], {"tags": ["soft"]}),
    "rock": ([rock(0), rock(1)], {"tags": ["rough"]}),
    "grass": ([grass(0), grass(1)], {}),
    "dirt": ([dirt], {}),
    "brine": ([brine(0), brine(1)], {"occludes": False, "walkable": False, "tags": ["liquid", "wet"]}),
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
