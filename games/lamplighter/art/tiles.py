#!/usr/bin/env python3
"""Builds `textures/ground.png` and `ground.sheet` from the Pixel Lab tileset.

`art/pixellab/tileset/` is what `tools/pixellab/pixellab.py tileset` saved for
`--lower "dark mossy grass, night" --upper "packed dirt path" --seed 7`: sixteen
16 by 16 tiles, each described by the terrain at its four corners. This orders
them by that corner pattern so a tile is found from the corners it must meet:

    id = NW * 8 + NE * 4 + SW * 2 + SE      (1 for dirt, 0 for grass)

`assets/scripts/ground.decay` paints the dirt path onto that sheet's `dirt`
corners terrain when the world opens. Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'tileset'
TEXTURES = HERE.parent / 'assets' / 'textures'
TILE = 16


def corner_id(corners):
    bit = {'NW': 8, 'NE': 4, 'SW': 2, 'SE': 1}
    return sum(value for name, value in bit.items() if corners[name] == 'upper')


def build():
    doc = json.loads((SOURCE / 'tileset.json').read_text())
    sheet = Image.new('RGBA', (4 * TILE, 4 * TILE))
    seen = set()
    for tile in doc['tileset']['tiles']:
        ident = corner_id(tile['corners'])
        seen.add(ident)
        picture = Image.open(SOURCE / tile['image']['saved']).convert('RGBA')
        assert picture.size == (TILE, TILE), picture.size
        sheet.paste(picture, ((ident % 4) * TILE, (ident // 4) * TILE))
    assert seen == set(range(16)), f'the tileset is missing corner patterns: {sorted(set(range(16)) - seen)}'
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'ground.png', optimize=True)
    names = [f'ground-{i}' for i in range(16)]
    # One `corners` terrain: the grass is the ground it lies on (sprite 0) and
    # dirt is what `Grid.set_terrain_corner` sets, one corner at a time.
    (TEXTURES / 'ground.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'center', 'grid': {'columns': 4, 'rows': 4, 'names': names},
         'terrains': {'dirt': {'kind': 'corners', 'sprites': names}}},
        indent=2) + '\n')


if __name__ == '__main__':
    build()
