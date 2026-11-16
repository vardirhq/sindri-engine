#!/usr/bin/env python3
"""Builds `textures/wick.png` and `wick.sheet` from Wick's frame strips.

Rows are the four directions (south, west, north, east). Each row holds Wick's
idle (eight frames), walk (eight), hurt (eight) and death (twelve), from the
strips in `pixellab/wick/`, 90 pixels a frame. West is east mirrored: the strips
delivered as facing left faced right.

Each frame is put on its cell aligned by its feet, its lowest drawn row, so he
stands on the ground in every frame rather than bobbing above his shadow as he
walks; anchored at the bottom, the entity's position is where he stands.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'wick'
TEXTURES = HERE.parent / 'assets' / 'textures'
DIRECTIONS = ['south', 'west', 'north', 'east']
# Each animation and how many frames its strip holds.
ANIMATIONS = [('idle', 8), ('walk', 8), ('hurt', 8), ('death', 12)]
FRAME = 90
CELL = (FRAME, FRAME)
# Where the feet stand on a cell: the lowest drawn row of every frame.
FEET_ON_CELL = (CELL[0] // 2, CELL[1] - 3)


def feet_row(picture):
    """The lowest row anything is drawn on: where the planted foot is."""
    return picture.getchannel('A').getbbox()[3] - 1


def place(sheet, picture, column, row):
    cell_x, cell_y = column * CELL[0], row * CELL[1]
    sheet.alpha_composite(picture, (cell_x, cell_y + FEET_ON_CELL[1] - feet_row(picture)))


def frames(direction, animation, count):
    """One animation's frames for one direction, west being east mirrored."""
    source = 'east' if direction == 'west' else direction
    strip = Image.open(SOURCE / f'{source}-{animation}.png').convert('RGBA')
    out = [strip.crop((n * FRAME, 0, (n + 1) * FRAME, FRAME)) for n in range(count)]
    if direction == 'west':
        out = [frame.transpose(Image.FLIP_LEFT_RIGHT) for frame in out]
    return out


def names():
    return [f'{animation}-{direction}-{n}' for direction in DIRECTIONS
            for animation, count in ANIMATIONS for n in range(count)]


def build():
    columns = sum(count for _, count in ANIMATIONS)
    sheet = Image.new('RGBA', (columns * CELL[0], len(DIRECTIONS) * CELL[1]))
    for row, direction in enumerate(DIRECTIONS):
        column = 0
        for animation, count in ANIMATIONS:
            for picture in frames(direction, animation, count):
                place(sheet, picture, column, row)
                column += 1
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'wick.png', optimize=True)
    (TEXTURES / 'wick.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom',
         'grid': {'columns': columns, 'rows': len(DIRECTIONS), 'names': names()}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
