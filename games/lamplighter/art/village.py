#!/usr/bin/env python3
"""Builds `textures/villagers.png` and its `.sheet` from the Pixel Lab villagers: the baker, the
child, the mender and the elder, each in the four directions, aligned by the feet on a cell
like Wick's. The village itself is `village_backdrop.py`.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'village' / 'people'
TEXTURES = HERE.parent / 'assets' / 'textures'
PEOPLE = ['baker', 'child', 'mender', 'elder']
DIRECTIONS = ['south', 'west', 'north', 'east']
CELL = (48, 44)
FEET_ON_ROTATION = 41


def build():
    names = [f'{person}-{direction}' for person in PEOPLE for direction in DIRECTIONS]
    sheet = Image.new('RGBA', (CELL[0] * len(DIRECTIONS), CELL[1] * len(PEOPLE)))
    for row, person in enumerate(PEOPLE):
        for column, direction in enumerate(DIRECTIONS):
            picture = Image.open(SOURCE / person / f'{direction}.png').convert('RGBA')
            at = (column * CELL[0] + (CELL[0] - picture.width) // 2, row * CELL[1] + CELL[1] - 3 - FEET_ON_ROTATION)
            sheet.alpha_composite(picture, at)
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'villagers.png', optimize=True)
    (TEXTURES / 'villagers.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom',
         'grid': {'columns': len(DIRECTIONS), 'rows': len(PEOPLE), 'names': names}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
