#!/usr/bin/env python3
"""Builds `textures/decor.png` and `decor.sheet` from the Pixel Lab ground cover.

Tufts of grass, night flowers, glowing mushrooms and pebbles, each cropped to what
it draws and standing on the bottom edge of a 24 pixel cell, scattered over the
grass by `scenes.py` so the ground is not one texture for a hundred yards.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'decor'
TEXTURES = HERE.parent / 'assets' / 'textures'
CELL = 24
NAMES = ['tufts', 'flowers', 'mushrooms', 'pebbles']


def build():
    sheet = Image.new('RGBA', (CELL * len(NAMES), CELL))
    for column, name in enumerate(NAMES):
        picture = Image.open(SOURCE / f'{name}.png').convert('RGBA')
        shown = picture.crop(picture.getbbox())
        assert shown.width <= CELL and shown.height <= CELL, (name, shown.size)
        sheet.alpha_composite(shown, (column * CELL + (CELL - shown.width) // 2, CELL - shown.height))
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'decor.png', optimize=True)
    (TEXTURES / 'decor.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom',
         'grid': {'columns': len(NAMES), 'rows': 1, 'names': NAMES}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
