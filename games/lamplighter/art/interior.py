#!/usr/bin/env python3
"""Builds `textures/room.png`: the shell of a house's room, drawn in the village's palette.

14 by 11 world units at 16 pixels to the unit. A wall face along the top (timber over stone, with
two window slots), plank flooring below it, a thin wall down both sides and along the bottom with a
doorway in the middle. Each house is the same shell with its own floor colour, so the four rooms
share one palette and differ by their furniture (`interior_scene.py`).

Deterministic; needs Pillow."""
import json
import pathlib
import random

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
TEXTURES = HERE.parent / 'assets' / 'textures'
W, H = 224, 176
WALL_H, SIDE, BOTTOM = 48, 16, 16
DOOR = (96, 128)
FLOORS = {
    'bakery': [(98, 65, 40), (123, 72, 36), (76, 64, 40)],
    'child-house': [(123, 72, 36), (156, 92, 39), (98, 65, 40)],
    'workshop': [(76, 64, 40), (98, 65, 40), (61, 45, 43)],
    'keeper': [(61, 45, 43), (76, 64, 40), (98, 65, 40)],
}
STONE = [(61, 45, 43), (75, 80, 88), (122, 127, 134), (44, 40, 45)]
TIMBER = [(44, 32, 28), (76, 48, 30), (98, 65, 40)]
GLASS = [(10, 28, 34), (47, 111, 115), (22, 44, 47)]


def room(name):
    rng = random.Random(name)
    out = Image.new('RGBA', (W, H))
    px = out.load()
    floor = FLOORS[name]
    for y in range(WALL_H, H - BOTTOM):                       # planks, each a few tiles long, laid in rows
        row = (y - WALL_H) // 8
        for x in range(SIDE, W - SIDE):
            seam = (y - WALL_H) % 8 == 0 or (x + row * 37) % 56 == 0
            colour = floor[(row * 3 + (x + row * 37) // 56) % 3]
            px[x, y] = (*(STONE[3] if seam else colour), 255)
            if not seam and rng.random() < 0.03:
                px[x, y] = (*TIMBER[0], 255)
    for y in range(0, WALL_H):                                # the wall face: stone below, timber above
        for x in range(0, W):
            if y < 6:
                px[x, y] = (*TIMBER[0], 255)
            elif y < WALL_H - 14:
                px[x, y] = (*TIMBER[1 + ((x // 8) % 2)], 255)
                if x % 8 == 0:
                    px[x, y] = (*TIMBER[0], 255)
            else:
                brick = (y // 7 + x // 14) % 2
                px[x, y] = (*STONE[1 + brick], 255)
                if y % 7 == 0 or (x + (y // 7) * 7) % 14 == 0:
                    px[x, y] = (*STONE[0], 255)
    for x0 in (48, 160):                                      # a window each side of the middle
        for y in range(10, 28):
            for x in range(x0, x0 + 18):
                edge = x in (x0, x0 + 17) or y in (10, 27)
                bar = x in (x0 + 8, x0 + 9) or y in (18, 19)
                px[x, y] = (*(TIMBER[0] if edge or bar else GLASS[(x + y) % 3 == 0 and 2 or 0]), 255)
    for y in range(0, H):                                     # the side walls
        for x in list(range(0, SIDE)) + list(range(W - SIDE, W)):
            if y >= WALL_H - 14 or x in (0, W - 1):
                px[x, y] = (*STONE[1 + (y // 7 + x // 14) % 2], 255)
                if y % 7 == 0:
                    px[x, y] = (*STONE[0], 255)
    for y in range(H - BOTTOM, H):                            # the bottom wall, with its doorway
        for x in range(0, W):
            if DOOR[0] <= x < DOOR[1]:
                px[x, y] = (*TIMBER[0], 255) if y == H - BOTTOM else (*TIMBER[1], 255)
                continue
            px[x, y] = (*STONE[1 + (y // 7 + x // 14) % 2], 255)
            if y % 7 == 0 or x % 14 == 0:
                px[x, y] = (*STONE[0], 255)
    return out


def build():
    names = list(FLOORS)
    sheet = Image.new('RGBA', (W * len(names), H))
    for column, name in enumerate(names):
        sheet.alpha_composite(room(name), (column * W, 0))
    sheet.save(TEXTURES / 'room.png', optimize=True)
    (TEXTURES / 'room.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'center', 'grid': {'columns': len(names), 'rows': 1, 'names': names}},
        indent=2) + '\n')


if __name__ == '__main__':
    build()
