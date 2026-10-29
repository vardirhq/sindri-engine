#!/usr/bin/env python3
"""Builds `textures/furniture.png` from the Pixel Lab furniture in `art/pixellab/village/furniture/`.

Each piece is cropped to what it draws and set on a 64 pixel cell, centered and standing on the
cell's bottom edge, so with the sheet anchored at the bottom an entity's position is the foot of the
piece and its scale is the cell's size in world units. Needs Pillow; deterministic."""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'village' / 'furniture'
TEXTURES = HERE.parent / 'assets' / 'textures'
CELL = 64


def build():
    names = sorted(path.stem for path in SOURCE.glob('*.png'))
    sheet = Image.new('RGBA', (CELL * len(names), CELL))
    for column, name in enumerate(names):
        picture = Image.open(SOURCE / f'{name}.png').convert('RGBA')
        shown = picture.crop(picture.getbbox())
        assert shown.width <= CELL and shown.height <= CELL, (name, shown.size)
        sheet.alpha_composite(shown, (column * CELL + (CELL - shown.width) // 2, CELL - shown.height))
    sheet.save(TEXTURES / 'furniture.png', optimize=True)
    (TEXTURES / 'furniture.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom', 'grid': {'columns': len(names), 'rows': 1, 'names': names}},
        indent=2) + '\n')


if __name__ == '__main__':
    build()
