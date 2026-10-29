#!/usr/bin/env python3
"""Builds `textures/pond.png`: the pond at the village's south-west corner, drawn rather than
generated so its water, stones and lily pads are in the village's palette and its shape is ours.

A wobbly oval of black-green water that is a little lighter towards the middle, a rim of stones,
lily pads, a few ripples and reeds on the near bank. Deterministic; needs Pillow."""
import json
import math
import pathlib
import random

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
TEXTURES = HERE.parent / 'assets' / 'textures'
W, H = 144, 104
WATER = [(2, 21, 28), (10, 28, 34), (14, 35, 34), (22, 44, 47), (47, 111, 115)]
STONE = [(61, 45, 43), (75, 80, 88), (122, 127, 134), (154, 160, 166)]
MOSS = [(44, 62, 35), (79, 122, 46)]


def build():
    rng = random.Random(5)
    out = Image.new('RGBA', (W, H))
    px = out.load()
    cx, cy, rx, ry = W / 2, H / 2 + 2, 60, 38
    phases = [rng.uniform(0, 6.28) for _ in range(4)]

    def edge(angle):
        return 1 + 0.07 * math.sin(3 * angle + phases[0]) + 0.05 * math.sin(5 * angle + phases[1])

    for y in range(H):
        for x in range(W):
            dx, dy = (x - cx) / rx, (y - cy) / ry
            d = math.hypot(dx, dy) / edge(math.atan2(dy, dx))
            if d > 1.12:
                continue
            if d > 1.0:
                px[x, y] = STONE[0]                                       # the rim's shadowed outer edge
            elif d > 0.9:
                px[x, y] = STONE[1 + (x // 3 * 7 + y // 3 * 13) % 3]      # a ring of stones
                if rng.random() < 0.12:
                    px[x, y] = MOSS[rng.randint(0, 1)]
            else:
                depth = 0 if d > 0.75 else 1 if d > 0.5 else 2
                px[x, y] = (*WATER[min(depth + (1 if y < cy - 8 and d < 0.8 else 0), 3)], 255)
    for _ in range(18):                                                  # ripples and glints
        x, y = int(rng.uniform(cx - 40, cx + 40)), int(rng.uniform(cy - 22, cy + 22))
        if px[x, y][3] and px[x, y][:3] in WATER[:4]:
            for k in range(rng.randint(3, 6)):
                px[x + k, y] = (*WATER[4], 255) if k % 2 == 0 else (*WATER[3], 255)
    for _ in range(9):                                                   # lily pads
        x, y = int(rng.uniform(cx - 45, cx + 45)), int(rng.uniform(cy - 24, cy + 24))
        if math.hypot((x - cx) / rx, (y - cy) / ry) < 0.7:
            for yy in range(-2, 3):
                for xx in range(-3, 4):
                    if xx * xx / 9 + yy * yy / 4 <= 1 and (xx, yy) != (1, 0):
                        px[x + xx, y + yy] = (*MOSS[1 if (xx + yy) % 2 else 0], 255)
    for x0 in (cx - 52, cx - 30, cx + 38, cx + 55):                      # reeds on the rim
        for k in range(rng.randint(3, 5)):
            bx, by = int(x0) + k * 2, int(cy + ry * 0.86 - abs(k - 2))
            for h in range(rng.randint(5, 9)):
                if 0 <= by - h < H:
                    px[bx, by - h] = (*MOSS[1 if h > 3 else 0], 255)
    out.save(TEXTURES / 'pond.png', optimize=True)
    (TEXTURES / 'pond.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'center', 'grid': {'columns': 1, 'rows': 1, 'names': ['pond']}}, indent=2) + '\n')


WOOD = [(44, 32, 28), (76, 64, 40), (98, 65, 40), (123, 72, 36)]


def dock():
    """A jetty running north from the bank over the water: planks, side beams, posts, a lantern on one."""
    w, h = 24, 64
    out = Image.new('RGBA', (w, h))
    px = out.load()
    for y in range(14, h):
        for x in range(2, w - 2):
            beam = x in (2, 3, w - 4, w - 3)
            px[x, y] = (*WOOD[0 if y % 5 == 0 else (1 if beam else 2 + (x // 6 + y // 5) % 2)], 255)
    for y in (18, 38, 60):
        for x in (0, 1, w - 2, w - 1):
            for k in range(-3, 4):
                px[x, y + k] = (*WOOD[1 + (k > 0)], 255)
    for k in range(-12, 4):                                              # the lantern post
        px[w - 2, 18 + k] = (*WOOD[1], 255)
        px[w - 1, 18 + k] = (*WOOD[0], 255)
    for dx, dy, c in ((-3, -17, (255, 223, 138)), (-2, -17, (242, 196, 109)), (-3, -16, (232, 145, 58)),
                      (-2, -16, (232, 145, 58)), (-3, -15, (156, 92, 39)), (-2, -15, (156, 92, 39))):
        px[w + dx, 18 + dy] = (*c, 255)
    out.save(TEXTURES / 'dock.png', optimize=True)
    (TEXTURES / 'dock.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom', 'grid': {'columns': 1, 'rows': 1, 'names': ['dock']}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
    dock()
