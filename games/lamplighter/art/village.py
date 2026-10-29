#!/usr/bin/env python3
"""Builds the village's sheets from the Pixel Lab art: `textures/village.png` (the
buildings, the well and the signpost) and `textures/villagers.png` (the four villagers
in the four directions), each with its `.sheet`.

A building is cropped to what it draws and set on a 112 pixel cell, centered and
standing on the cell's bottom edge, so with the sheet anchored at the bottom an entity's
position is the foot of the building. Each house comes twice: lit, as it was drawn, and
unlit, with its warm windows and glow dulled, which is how it stands until its lamp is
lit. A villager is aligned by the feet on a cell like Wick's.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

import objects

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'village'
TEXTURES = HERE.parent / 'assets' / 'textures'
CELL = 112
HOUSES = ['bakery', 'child-house', 'workshop', 'keeper', 'chapel']
PROPS = sorted(path.stem for path in (SOURCE / 'props').glob('*.png'))
PEOPLE = ['baker', 'child', 'mender', 'elder']
DIRECTIONS = ['south', 'west', 'north', 'east']
PERSON_CELL = (48, 44)
FEET_ON_ROTATION = 41


def unlit(picture):
    """The house with its lights out: the warm pixels dulled, the rest a touch darker."""
    out = objects.unlit(picture)
    pixels = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = pixels[x, y]
            if a:
                pixels[x, y] = (r * 9 // 10, g * 9 // 10, b * 9 // 10, a)
    return out


def sprites():
    """Every house twice, lit and unlit, the lamp too, then each prop once."""
    out = []
    for name in HOUSES:
        picture = Image.open(SOURCE / 'buildings' / f'{name}.png').convert('RGBA')
        out += [(f'{name}-unlit', unlit(picture)), (f'{name}-lit', picture)]
    for name in PROPS:
        picture = Image.open(SOURCE / 'props' / f'{name}.png').convert('RGBA')
        if name == 'lamp':
            out += [('lamp-unlit', unlit(picture)), ('lamp-lit', picture)]
        else:
            out.append((name, picture))
    return out


def write(sheet, names, columns, rows, anchor='bottom'):
    TEXTURES.mkdir(parents=True, exist_ok=True)
    return {'format_version': 1, 'anchor': anchor, 'grid': {'columns': columns, 'rows': rows, 'names': names}}


def buildings():
    items = sprites()
    sheet = Image.new('RGBA', (CELL * len(items), CELL))
    for column, (_, picture) in enumerate(items):
        shown = picture.crop(picture.getbbox())
        assert shown.width <= CELL and shown.height <= CELL, shown.size
        sheet.alpha_composite(shown, (column * CELL + (CELL - shown.width) // 2, CELL - shown.height))
    sheet.save(TEXTURES / 'village.png', optimize=True)
    (TEXTURES / 'village.sheet').write_text(json.dumps(
        write(sheet, [name for name, _ in items], len(items), 1), indent=2) + '\n')


def villagers():
    names = [f'{person}-{direction}' for person in PEOPLE for direction in DIRECTIONS]
    sheet = Image.new('RGBA', (PERSON_CELL[0] * len(DIRECTIONS), PERSON_CELL[1] * len(PEOPLE)))
    for row, person in enumerate(PEOPLE):
        for column, direction in enumerate(DIRECTIONS):
            picture = Image.open(SOURCE / 'people' / person / f'{direction}.png').convert('RGBA')
            at = (column * PERSON_CELL[0] + (PERSON_CELL[0] - picture.width) // 2,
                  row * PERSON_CELL[1] + PERSON_CELL[1] - 3 - FEET_ON_ROTATION)
            sheet.alpha_composite(picture, at)
    sheet.save(TEXTURES / 'villagers.png', optimize=True)
    (TEXTURES / 'villagers.sheet').write_text(json.dumps(
        write(sheet, names, len(DIRECTIONS), len(PEOPLE)), indent=2) + '\n')


def build():
    buildings()
    villagers()


if __name__ == '__main__':
    build()
