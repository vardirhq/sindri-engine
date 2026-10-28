#!/usr/bin/env python3
"""Builds `textures/gloom.png` and `gloom.sheet` from four Pixel Lab candidates.

The four are different shapes of the one creature (a smoking blob, a hunched one,
one with floating motes, a low crouching one), each 32 pixels square and standing
on the cell's bottom edge, so a group of gloom is not one creature repeated.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'gloom'
TEXTURES = HERE.parent / 'assets' / 'textures'
CELL = 32


def build():
    paths = sorted(SOURCE.glob('gloom-*.png'))
    sheet = Image.new('RGBA', (CELL * len(paths), CELL))
    for column, path in enumerate(paths):
        picture = Image.open(path).convert('RGBA')
        shown = picture.crop(picture.getbbox())
        assert shown.width <= CELL and shown.height <= CELL, (path, shown.size)
        sheet.alpha_composite(shown, (column * CELL + (CELL - shown.width) // 2, CELL - shown.height))
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'gloom.png', optimize=True)
    names = [path.stem for path in paths]
    (TEXTURES / 'gloom.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom',
         'grid': {'columns': len(names), 'rows': 1, 'names': names}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
