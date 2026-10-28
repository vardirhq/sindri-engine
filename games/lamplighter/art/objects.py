#!/usr/bin/env python3
"""Builds `textures/objects.png` and `objects.sheet` from the Pixel Lab objects.

Each object is cropped to what it draws and set on a 64 pixel cell, centered and
standing on the cell's bottom edge, so with the sheet anchored at the bottom an
entity's position is the foot of the thing it shows. The unlit lamp post is the
lit one with its warm pixels (the flame and the glow on the glass) dulled to the
metal around them, so lighting a lamp changes its lantern and nothing else.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'objects'
TEXTURES = HERE.parent / 'assets' / 'textures'
CELL = 64


def unlit(picture):
    """The lamp post with its flame put out: warm pixels dulled to dark brass."""
    out = picture.copy()
    pixels = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = pixels[x, y]
            if a and r > b + 40 and r >= g:
                level = (r + g + b) // 3
                dim = max(24, level // 3)
                pixels[x, y] = (dim + 6, dim + 4, dim - 2 if dim > 2 else 0, a)
    return out


def sprites():
    lamp = Image.open(SOURCE / 'lamp-post.png').convert('RGBA')
    return [
        ('lamp-post-unlit', unlit(lamp)),
        ('lamp-post-lit', lamp),
        ('dead-tree', Image.open(SOURCE / 'dead-tree.png').convert('RGBA')),
        ('boulder', Image.open(SOURCE / 'boulder.png').convert('RGBA')),
    ]


def build():
    items = sprites()
    sheet = Image.new('RGBA', (CELL * len(items), CELL))
    for column, (_, picture) in enumerate(items):
        shown = picture.crop(picture.getbbox())
        assert shown.width <= CELL and shown.height <= CELL, shown.size
        at = (column * CELL + (CELL - shown.width) // 2, CELL - shown.height)
        sheet.alpha_composite(shown, at)
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'objects.png', optimize=True)
    (TEXTURES / 'objects.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom',
         'grid': {'columns': len(items), 'rows': 1, 'names': [name for name, _ in items]}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
