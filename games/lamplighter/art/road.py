#!/usr/bin/env python3
"""Builds `textures/road.png` and `road.sheet` from the Pixel Lab paving tileset.

`art/pixellab/road/` is `tileset --lower "worn grey cobblestone paving, night"
--upper "packed dirt path" --transition "dirt packed between the stones"
--seed 7`. The service made the plain ground between the stones one flat color
(81, 77, 75), not our dirt, so every pixel of that color is made transparent: the
paving then lies on whatever ground is under it, dirt or grass, and the tile with
no stone in it is empty. It is a second `corners` terrain, `stone`, painted on a
layer above the ground, so a lamp can stand on a pad of paving.

The labels cannot be trusted (here the textured cobblestones were `upper`), so the
stones are found by their pixels: the tile with 21 colors and no flat ones.

    id = NW * 8 + NE * 4 + SW * 2 + SE      (1 where the corner is stone)

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'road'
TEXTURES = HERE.parent / 'assets' / 'textures'
TILE = 16
FLAT = (81, 77, 75, 255)


def pixels(picture):
    return [picture.getpixel((x, y)) for y in range(TILE) for x in range(TILE)]


def build():
    doc = json.loads((SOURCE / 'tileset.json').read_text())
    tiles = [(tile['corners'], Image.open(SOURCE / tile['image']['saved']).convert('RGBA'))
             for tile in doc['tileset']['tiles']]
    flat = [corners for corners, picture in tiles if all(p == FLAT for p in pixels(picture))]
    textured = [corners for corners, picture in tiles if FLAT not in pixels(picture)]
    assert len(flat) == 1 and len(textured) == 1, (len(flat), len(textured))
    # The all-stone tile's label says which of the two names is the stone.
    stone_label = next(iter(textured[0].values()))
    assert all(label == stone_label for label in textured[0].values())
    assert all(label != stone_label for label in flat[0].values())

    sheet = Image.new('RGBA', (4 * TILE, 4 * TILE))
    for corners, picture in tiles:
        ident = sum(value for name, value in {'NW': 8, 'NE': 4, 'SW': 2, 'SE': 1}.items()
                    if corners[name] == stone_label)
        shown = picture.copy()
        for y in range(TILE):
            for x in range(TILE):
                if shown.getpixel((x, y)) == FLAT:
                    shown.putpixel((x, y), (0, 0, 0, 0))
        sheet.paste(shown, ((ident % 4) * TILE, (ident // 4) * TILE))
    TEXTURES.mkdir(parents=True, exist_ok=True)
    sheet.save(TEXTURES / 'road.png', optimize=True)
    names = [f'road-{i}' for i in range(16)]
    (TEXTURES / 'road.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'center', 'grid': {'columns': 4, 'rows': 4, 'names': names},
         'terrains': {'stone': {'kind': 'corners', 'sprites': names}}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
