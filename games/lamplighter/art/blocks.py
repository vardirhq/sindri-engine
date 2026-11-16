#!/usr/bin/env python3
"""Draws the wilds' ground blocks and writes the block set that names them.

The wilds are a voxel world viewed as a map (`docs/lamplighter-plan.md`), so a
block shows only the face seen from above; that picture is used for every face,
which keeps the set valid for a world drawn as blocks too.

Most blocks are the game's own ground, cut from the terrain sheets the field and
the village are painted with: the plain tile of each (index 0 is a sheet's lower
terrain, 15 its upper), so the wilds and the places in them are one palette.
What no sheet has yet (water, sand, moss, mud and rock) is drawn here in the same
palette, the water from the village pond's. Each block has four variants, the
picture turned and mirrored, chosen cell by cell by the world's variant seed, so
a meadow a hundred cells wide is not one tile stamped a hundred times.

Beside it, `blocks_n.png` is the set's normal map, laid out cell for cell like
`blocks.png`, so a low light (the lantern, a sunfall's pool) rakes across the
ground and shows its relief: each picture's brightness read as height, dark
gaps between stones sunk and pale tufts standing proud, with soft lumps of
its own where a ground is too even for its brightness to say anything (a
meadow's grass is one colour), wrapping round its own cell so it tiles as the
picture does, and turned and mirrored with each variant.

Deterministic: running it again changes nothing.
"""

import json
import math
import random
from pathlib import Path

from PIL import Image

ASSETS = Path(__file__).resolve().parent.parent / 'assets'
TEXTURES = ASSETS / 'textures'
CELL = 16

# name, sheet, index: a block cut from a terrain sheet.
CUT = [
    ('grass', 'village-ground', 0),
    ('deadgrass', 'ground', 0),
    ('dirt', 'ground', 15),
    ('earth', 'village-ground', 15),
    ('soil', 'village-soil', 15),
    ('cobble', 'village-cobble', 15),
    ('flag', 'village-flag', 15),
    ('stone', 'road', 15),
]

# name, base, flecks (colour, share), tags, extra block fields: a block drawn here.
DRAWN = [
    ('water', (14, 34, 38), [((20, 48, 52), 0.25), ((60, 120, 120), 0.03)], ['liquid', 'wet'],
     {'occludes': False, 'walkable': False}),
    ('sand', (128, 112, 84), [((110, 96, 72), 0.3), ((146, 130, 98), 0.15)], ['soft'], {}),
    ('moss', (40, 58, 40), [((30, 46, 32), 0.35), ((58, 78, 50), 0.12)], ['soft', 'wet'], {}),
    ('mud', (58, 46, 36), [((46, 36, 28), 0.35), ((72, 60, 46), 0.1)], ['soft', 'wet'], {}),
    ('rock', (70, 70, 74), [((54, 54, 60), 0.35), ((92, 92, 96), 0.12)], ['rough'], {}),
]

VARIANTS = 4
# How much relief each ground has, as how steeply brightness becomes height:
# stone and rock most, grass and soil some, sand and mud a little, water none
# (`map_water` moves it).
# Brightness's share of the height, and the lumps': clumps of grass, ripples in
# sand, the give of mud.
RELIEF = {'grass': 2.0, 'deadgrass': 2.0, 'dirt': 1.8, 'earth': 1.8, 'soil': 1.8, 'cobble': 3.2,
          'flag': 3.0, 'stone': 3.0, 'water': 0.0, 'sand': 1.0, 'moss': 1.6, 'mud': 1.0, 'rock': 3.2}
LUMPS = {'grass': 0.3, 'deadgrass': 0.22, 'dirt': 0.18, 'earth': 0.18, 'soil': 0.18, 'sand': 0.18,
         'moss': 0.28, 'mud': 0.22, 'rock': 0.12}


def cut(sheet, index):
    image = Image.open(TEXTURES / f'{sheet}.png').convert('RGBA')
    x, y = (index % 4) * CELL, (index // 4) * CELL
    return image.crop((x, y, x + CELL, y + CELL))


def drawn(name, base, flecks):
    """A plain ground: its base colour with flecks scattered by a seed of its own name."""
    rng = random.Random(name)
    image = Image.new('RGBA', (CELL, CELL), base + (255,))
    for y in range(CELL):
        for x in range(CELL):
            roll = rng.random()
            for colour, share in flecks:
                if roll < share:
                    image.putpixel((x, y), colour + (255,))
                    break
                roll -= share
    return image


def variants(image):
    """The picture as it is, mirrored, turned half round, and flipped: four that tile alike."""
    return [image, image.transpose(Image.FLIP_LEFT_RIGHT), image.rotate(180),
            image.transpose(Image.FLIP_TOP_BOTTOM)]


def heights(name, picture):
    """How high each pixel of a ground stands, 0 to 1, as a greyscale picture:
    its brightness, and lumps scattered by a seed of its name, wrapped round
    the cell."""
    rng = random.Random(f'{name}-lumps')
    lumps = [(rng.uniform(0, CELL), rng.uniform(0, CELL), rng.uniform(1.2, 2.6), rng.uniform(0.5, 1.0))
             for _ in range(9)]
    rgb = picture.convert('RGB')
    out = Image.new('L', picture.size)
    for y in range(CELL):
        for x in range(CELL):
            bright = sum(rgb.getpixel((x, y))) / (3 * 255)
            lump = 0.0
            for lx, ly, radius, tall in lumps:
                dx = min(abs(x - lx), CELL - abs(x - lx))
                dy = min(abs(y - ly), CELL - abs(y - ly))
                lump = max(lump, tall * max(0.0, 1.0 - (dx * dx + dy * dy) / (radius * radius)))
            level = bright * RELIEF[name] / 3.2 + lump * LUMPS.get(name, 0.0)
            out.putpixel((x, y), round(255 * min(1.0, level)))
    return out


def relief(lift_picture):
    """A ground's normal map from its heights, wrapped round the cell. Red is
    right, green is up the world, blue is out of the ground."""
    width, height = lift_picture.size
    strength = 2.0
    lift = [[lift_picture.getpixel((x, y)) / 255 for x in range(width)] for y in range(height)]
    out = Image.new('RGBA', lift_picture.size)
    for y in range(height):
        for x in range(width):
            across = lift[y][(x + 1) % width] - lift[y][(x - 1) % width]
            down = lift[(y + 1) % height][x] - lift[(y - 1) % height][x]
            # Up the world is up the picture, against the rows.
            nx, ny, nz = -across * strength, down * strength, 1.0
            length = math.sqrt(nx * nx + ny * ny + nz * nz)
            out.putpixel((x, y), tuple(round(127.5 + 127.5 * c / length) for c in (nx, ny, nz)) + (255,))
    return out


def face(sprite):
    return {'sprite': f'textures/blocks.png#{sprite}', 'size': [1, 1]}


def faces(sprite):
    return {side: face(sprite) for side in ('top', 'bottom', 'south', 'east')}


def main():
    blocks = [(name, cut(sheet, index), [], {}) for name, sheet, index in CUT]
    blocks += [(name, drawn(name, base, flecks), tags, extra) for name, base, flecks, tags, extra in DRAWN]
    sheet = Image.new('RGBA', (CELL * VARIANTS, CELL * len(blocks)))
    normals = Image.new('RGBA', sheet.size)
    names = []
    tiles = {}
    for row, (name, image, tags, extra) in enumerate(blocks):
        sprites = []
        lifts = variants(heights(name, image))
        for column, variant in enumerate(variants(image)):
            sheet.paste(variant, (column * CELL, row * CELL))
            normals.paste(relief(lifts[column]), (column * CELL, row * CELL))
            sprites.append(f'{name}-{column}')
        names += sprites
        tile = {'faces': faces(sprites[0]), 'variants': [{'faces': faces(s)} for s in sprites[1:]]}
        if tags:
            tile['tags'] = tags
        tile.update(extra)
        tiles[name] = tile
    sheet.save(TEXTURES / 'blocks.png')
    normals.save(TEXTURES / 'blocks_n.png')
    (TEXTURES / 'blocks.sheet').write_text(json.dumps({
        'format_version': 1, 'anchor': 'center',
        'grid': {'columns': VARIANTS, 'rows': len(blocks), 'names': names}}) + '\n')
    (ASSETS / 'blocks.tileset').write_text(json.dumps({'format_version': 1, 'tiles': tiles}, indent=1) + '\n')


if __name__ == '__main__':
    main()
