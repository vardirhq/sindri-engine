#!/usr/bin/env python3
"""The wilds' screen furniture, in the same pixels as the world under it.

`hud-frame.png` is a dark, weathered slate in a thin brass rim: sliced, so any
size keeps its rim, which `hud-frame.sheet` records as the outer 6 pixels. The
slate is flat inside, so stretching it shows nothing; the rim is worn, darker
along the bottom and right and catching the lamplight along the top and left,
with a rivet at each corner. `hud-well.png` is the same slate sunk without a
rim, for the slot a meter sits in. `hud-icons.png` is a row of 16 pixel cells
for what the screen shows that the world does not: Wick's satchel. Deterministic.
"""
import json
from pathlib import Path

from PIL import Image

TEXTURES = Path(__file__).resolve().parent.parent / 'assets' / 'textures'

CLEAR = (0, 0, 0, 0)
OUTLINE = (14, 10, 8, 255)
BRASS = [(92, 64, 34, 255), (134, 96, 48, 255), (176, 132, 70, 255), (214, 172, 104, 255)]
SLATE = (12, 13, 20, 226)
SLATE_EDGE = (6, 6, 10, 236)
RIVET = (236, 204, 140, 255)
LEATHER, LEATHER_DARK, LEATHER_LIGHT = (104, 66, 38, 255), (66, 40, 24, 255), (146, 98, 56, 255)
BUCKLE = (214, 172, 104, 255)


def worn(x, y):
    """A little wear along the rim, the same every run."""
    return ((x * 73 + y * 151) ^ (x * y * 31)) % 7 == 0


def frame():
    size, rim = 24, 6
    image = Image.new('RGBA', (size, size), CLEAR)
    for y in range(size):
        for x in range(size):
            edge = min(x, y, size - 1 - x, size - 1 - y)
            corner = min(x, size - 1 - x) + min(y, size - 1 - y)
            if corner < 1:
                continue
            if edge == 0 or corner == 1:
                colour = OUTLINE
            elif edge < 3:
                # Lit from above and the left, as the lantern is held.
                lit = y == edge or (x == edge and y < size - 1 - edge)
                shade = y == size - 1 - edge or x == size - 1 - edge
                level = 3 if lit and edge == 1 else 2 if lit else 0 if shade else 1
                if worn(x, y) and level > 0:
                    level -= 1
                colour = BRASS[level]
            elif edge == 3:
                colour = OUTLINE
            elif edge < rim:
                colour = SLATE_EDGE
            else:
                colour = SLATE
            image.putpixel((x, y), colour)
    for x, y in ((2, 2), (size - 3, 2), (2, size - 3), (size - 3, size - 3)):
        image.putpixel((x, y), RIVET)
    return image


def well():
    size = 8
    image = Image.new('RGBA', (size, size), SLATE_EDGE)
    for x in range(size):
        image.putpixel((x, 0), OUTLINE)
        image.putpixel((x, size - 1), BRASS[0])
    for y in range(size):
        image.putpixel((0, y), OUTLINE)
        image.putpixel((size - 1, y), BRASS[0])
    return image


SATCHEL = [
    '................',
    '................',
    '.....oooooo.....',
    '....o......o....',
    '...o........o...',
    '..oooooooooooo..',
    '.oLLLLLLLLLLLLo.',
    '.oLlllllllllLLo.',
    '.oddddddbbddddo.',
    '.ollllllbblllLo.',
    '.ollllllllllllo.',
    '.ollllllllllllo.',
    '.oddddddddddddo.',
    '..oooooooooooo..',
    '................',
    '................',
]


def icons():
    image = Image.new('RGBA', (16, 16), CLEAR)
    colours = {'o': OUTLINE, 'l': LEATHER, 'd': LEATHER_DARK, 'L': LEATHER_LIGHT, 'b': BUCKLE}
    for y, row in enumerate(SATCHEL):
        for x, letter in enumerate(row):
            if letter != '.':
                image.putpixel((x, y), colours[letter])
    return image


def sheet(path, size, names, borders=None):
    data = {'format_version': 1, 'size': list(size),
            'grid': {'columns': len(names), 'rows': 1, 'names': names}}
    if borders is not None:
        data['borders'] = borders
    path.write_text(json.dumps(data, indent=2) + '\n')


def main():
    frame().save(TEXTURES / 'hud-frame.png')
    sheet(TEXTURES / 'hud-frame.sheet', (24, 24), ['frame'], {'': [6, 6, 6, 6]})
    well().save(TEXTURES / 'hud-well.png')
    sheet(TEXTURES / 'hud-well.sheet', (8, 8), ['well'], {'': [2, 2, 2, 2]})
    icons().save(TEXTURES / 'hud-icons.png')
    sheet(TEXTURES / 'hud-icons.sheet', (16, 16), ['satchel'])


if __name__ == '__main__':
    main()
