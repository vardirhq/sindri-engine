#!/usr/bin/env python3
"""Builds `textures/wick.png` and `wick.sheet` from the Pixel Lab character.

Rows are the four directions (south, west, north, east); columns are the idle
pose, the eight walk frames and the eight of the sword's slash. Pixel Lab's canvases differ (rotations 48 square,
the walk 60, its east frames 56), so each frame is put on one cell aligned by the
feet and the middle of its canvas, where the generator keeps the body. Anchored
at the bottom, the entity's position is where Wick stands.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'wick'
TEXTURES = HERE.parent / 'assets' / 'textures'
DIRECTIONS = ['south', 'west', 'north', 'east']
WALK = 8
SLASH = 8
# Wide enough for the blade, which reaches 24 pixels either side of the body.
CELL = (48, 44)
# Where the feet stand on a cell, and on each source canvas.
FEET_ON_CELL = (CELL[0] // 2, CELL[1] - 3)
FEET_ON_ROTATION = 41
FEET_ON_WALK = 47


def place(sheet, picture, column, row, feet_row):
    cell_x, cell_y = column * CELL[0], row * CELL[1]
    at = (cell_x + FEET_ON_CELL[0] - picture.width // 2, cell_y + FEET_ON_CELL[1] - feet_row)
    sheet.alpha_composite(picture, at)


def names():
    out = []
    for direction in DIRECTIONS:
        out.append(f'idle-{direction}')
        out.extend(f'walk-{direction}-{frame}' for frame in range(WALK))
        out.extend(f'slash-{direction}-{frame}' for frame in range(SLASH))
    return out


def build():
    columns = 1 + WALK + SLASH
    sheet = Image.new('RGBA', (columns * CELL[0], len(DIRECTIONS) * CELL[1]))
    for row, direction in enumerate(DIRECTIONS):
        idle = Image.open(SOURCE / 'rotations' / f'{direction}.png').convert('RGBA')
        place(sheet, idle, 0, row, FEET_ON_ROTATION)
        for frame in range(WALK):
            picture = Image.open(SOURCE / 'walk' / direction / f'frame_{frame + 1:03d}.png').convert('RGBA')
            place(sheet, picture, 1 + frame, row, FEET_ON_WALK)
        for frame in range(SLASH):
            picture = Image.open(SOURCE / 'slash' / direction / f'frame_{frame + 1:03d}.png').convert('RGBA')
            place(sheet, picture, 1 + WALK + frame, row, FEET_ON_WALK)
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'wick.png', optimize=True)
    (TEXTURES / 'wick.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom',
         'grid': {'columns': columns, 'rows': len(DIRECTIONS), 'names': names()}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
