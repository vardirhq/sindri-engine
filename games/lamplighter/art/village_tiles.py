#!/usr/bin/env python3
"""Builds the village's ground sheets from its four Pixel Lab tilesets.

`art/pixellab/village/tiles/` holds `tileset --lower "lush dark village grass with clover" --upper
<surface> --view "high top-down"` for packed earth (`dirt`), cobblestone (`cobble`), tilled soil
(`soil`) and flagstones (`flag`). Each is sixteen 16 pixel tiles named by the terrain at their four
corners, and the service's labels cannot be trusted, so which label is the grass is found by color:
the greener of the two.

    id = NW * 8 + NE * 4 + SW * 2 + SE      (1 where the corner is the surface)

The earth sheet is the whole ground, grass and all (`village-ground`). The other three lie on a layer
above it, so every pixel that is the grass of their own tileset is made transparent and they show the
ground beneath, with the dark moss that grew over their edges still on them.

Needs Pillow; deterministic, so CI can run it again and fail on any drift.
"""
import json
import pathlib

from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
SOURCE = HERE / 'pixellab' / 'village' / 'tiles'
TEXTURES = HERE.parent / 'assets' / 'textures'
TILE = 16
SHEETS = [('dirt', 'village-ground', 'dirt', False), ('cobble', 'village-cobble', 'cobble', True),
          ('soil', 'village-soil', 'soil', True), ('flag', 'village-flag', 'flag', True)]


def mean(picture):
    rgb = picture.convert('RGB')
    pixels = list(rgb.get_flattened_data() if hasattr(rgb, 'get_flattened_data') else rgb.getdata())
    return [sum(p[k] for p in pixels) / len(pixels) for k in range(3)]


def build_one(source, name, terrain, overlay):
    doc = json.loads((SOURCE / source / 'tileset.json').read_text())
    tiles = [(tile['corners'], Image.open(SOURCE / source / tile['image']['saved']).convert('RGBA'))
             for tile in doc['tileset']['tiles']]
    pure = {}
    for corners, picture in tiles:
        labels = set(corners.values())
        if len(labels) == 1:
            pure[next(iter(labels))] = picture
    assert len(pure) == 2, (source, sorted(pure))
    grass_label = max(pure, key=lambda label: mean(pure[label])[1] - mean(pure[label])[0])
    surface_label = next(label for label in pure if label != grass_label)
    grass = pure[grass_label]
    grass_colors = set(grass.get_flattened_data() if hasattr(grass, 'get_flattened_data') else grass.getdata())
    sheet = Image.new('RGBA', (4 * TILE, 4 * TILE))
    seen = set()
    for corners, picture in tiles:
        ident = sum(value for key, value in {'NW': 8, 'NE': 4, 'SW': 2, 'SE': 1}.items()
                    if corners[key] == surface_label)
        seen.add(ident)
        shown = picture.copy()
        if overlay:
            pixels = shown.load()
            for y in range(TILE):
                for x in range(TILE):
                    if pixels[x, y] in grass_colors:
                        pixels[x, y] = (0, 0, 0, 0)
        sheet.paste(shown, ((ident % 4) * TILE, (ident // 4) * TILE))
    assert seen == set(range(16)), (source, sorted(set(range(16)) - seen))
    sheet.save(TEXTURES / f'{name}.png', optimize=True)
    names = [f'{name}-{i}' for i in range(16)]
    (TEXTURES / f'{name}.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'center', 'grid': {'columns': 4, 'rows': 4, 'names': names},
         'terrains': {terrain: {'kind': 'corners', 'sprites': names}}}, indent=2) + '\n')


def build():
    TEXTURES.mkdir(parents=True, exist_ok=True)
    for source, name, terrain, overlay in SHEETS:
        build_one(source, name, terrain, overlay)


if __name__ == '__main__':
    build()
