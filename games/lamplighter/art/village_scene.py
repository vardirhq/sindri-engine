#!/usr/bin/env python3
"""Builds `assets/village.scene`: the square Wick reaches once the fields are lit.

Four lamps stand round a well, and each opens one house when it is lit and brings its villager
out. The art is `village_atlas.py`'s: one set of parts in one style, at 16 pixels to a world
unit. The ground is a tilemap of 1.75 unit tiles (28 pixels) laid here: grass, with earth roads,
a cobbled square and a lane. Everything standing on it is placed by hand along the roads and
scattered, thicker towards the edge, clear of the roads and of what is placed.

Positions are world units, the origin the middle of the square, up positive."""
import json
import math
import pathlib
import random

import scenes
from scenes import (ALL, WALL, at, effects, entity, environment, footprint, gloom, hud, input_actions,
                    lantern, depth, shadow, static_body, wall, wick, wisps)

HERE = pathlib.Path(__file__).resolve().parent
SIZES = json.loads((HERE / 'village_sizes.json').read_text())
T = 1.75                      # a ground tile, in world units
COLUMNS, ROWS = 28, 22
LEFT, TOP = -COLUMNS * T / 2, ROWS * T / 2
CELL = 8.5                    # the sheet's 136 pixel cell, in world units
TILE_NAMES = ['grass-0', 'grass-1', 'grass-2', 'grass-3', 'grass-4', 'grass-5', 'dirt-0', 'dirt-1', 'stone-0',
              'stone-1']
FLAME = 3.4                   # how high a lamp's lantern hangs

# name, sprite, x, foot y, lamp, villager, where the villager waits
HOUSES = [
    ('bakery', 'house-teal', 9.625, 5.25, 'lantern-2', 'baker', (9.625, 4.2)),
    ('child-house', 'house-keeper', -9.625, 5.25, 'lantern-1', 'child', (-9.625, 4.2)),
    ('workshop', 'house-workshop', 9.625, -8.75, 'lantern-4', 'mender', (9.625, -9.8)),
    ('keeper', 'house-teal', -9.625, -8.75, 'lantern-3', 'elder', (-9.625, -9.8)),
]
FLIPPED = {'keeper'}
CHIMNEYS = {'bakery': (2.4, 5.6), 'child-house': (1.6, 4.3), 'keeper': (-2.4, 5.6), 'workshop': (-1.4, 6.6)}
LAMPS = [(-3.5, 3.5), (3.5, 3.5), (-3.5, -3.5), (3.5, -3.5)]
DIRECTIONS = ['south', 'west', 'north', 'east']
GLOOM_AT = [(-6.0, -17.0), (8.0, 15.0)]

# What stands by hand: ident, sprite, x, foot y, flip.
BY_HAND = [
    ('well', 'well-roofed', 0.0, 1.4, False),
    ('sign-tools', 'sign-tools', 13.6, 5.6, False),
    ('banner', 'banner', -13.4, 5.6, False),
    ('noticeboard', 'noticeboard', -16.8, -2.9, False),
    ('bench', 'bench', -7.2, 3.8, False),
    ('cart', 'cart', 5.2, -7.6, False),
    ('woodpile', 'woodpile', 14.2, -6.6, False),
    ('barrels', 'barrel-2', 15.4, -7.2, False),
    ('crate', 'crate', 3.7, -7.4, False),
    ('table', 'table-goods', -16.6, 3.0, False),
    ('sacks', 'sacks-1', -17.6, 3.2, False),
    ('barrel-b', 'barrel-1', 14.6, 4.6, False),
    ('pots', 'pots', 7.2, 3.8, False),
    ('gate-wood', 'gate-wood', 15.6, 1.2, False),
    ('workbench', 'workbench', 14.6, -10.8, False),
]
# Fences and hedges that frame the yards: ident, sprite, x, foot y.
FRAMING = [
    ('fence-n1', 'fence-1', 6.5, 5.5), ('fence-n2', 'fence-1', 12.9, 5.5), ('fence-n3', 'fence-1', -12.9, 5.5),
    ('fence-n4', 'fence-1', -6.4, 5.5), ('fence-s1', 'fence-2', 6.6, -8.2), ('fence-s2', 'fence-2', 12.9, -8.2),
    ('fence-s3', 'fence-2', -12.9, -8.2), ('fence-s4', 'fence-2', -6.6, -8.2),
    ('hedge-1', 'bush-2', -6.0, 6.4), ('hedge-2', 'bush-1', 6.0, 6.4), ('hedge-3', 'bush-2', -6.0, -7.0),
    ('hedge-4', 'bush-1', 6.0, -7.0),
]


def tile_index(x, y):
    return int((x - LEFT) // T), int((TOP - y) // T)


def ground_tiles():
    """Grass everywhere; earth roads, a cobbled square, a lane and the paths to the doors."""
    rng = random.Random(3)
    tiles = [[rng.randrange(6) for _ in range(COLUMNS)] for _ in range(ROWS)]

    def fill(x0, x1, y0, y1, kind):
        i0, j1 = tile_index(x0 + 0.01, y0 + 0.01)
        i1, j0 = tile_index(x1 - 0.01, y1 - 0.01)
        for j in range(j0, j1 + 1):
            for i in range(i0, i1 + 1):
                if 0 <= i < COLUMNS and 0 <= j < ROWS:
                    tiles[j][i] = rng.choice(kind)

    dirt, stone = [6, 7], [8, 9]
    fill(-24.5, 24.5, -1.75, 1.75, dirt)              # the road across
    fill(-1.75, 1.75, -19.25, 19.25, dirt)            # the road up and down
    fill(-14.0, 14.0, -14.0, -10.5, dirt)             # the lane the south houses face
    for x in (9.625, -9.625):
        fill(x - 0.87, x + 0.87, 1.75, 5.25, dirt)    # to the north doors
        fill(x - 0.87, x + 0.87, -10.5, -8.75, dirt)  # to the south doors
    fill(-5.25, 5.25, -5.25, 5.25, stone)             # the square
    return [tile for row in tiles for tile in row]


def ground():
    return entity('ground', 'Ground', at(LEFT, TOP, -1.0, T, T), **{'sindri.tilemap': {
        'texture': 'textures/village-ground.png', 'palette': [f'village-ground-{i}' for i in range(len(TILE_NAMES))],
        'columns': COLUMNS, 'rows': ROWS, 'tiles': ground_tiles(), 'layer': -10}})


def size_of(sprite):
    w, h = SIZES[sprite]
    return w / 16.0, h / 16.0


def standing(ident, sprite, x, y, flip=False, block=True, rx=None, ry=0.4, **more):
    """A thing standing on its foot at (x, y). It blocks Wick and light across `rx` either side of its
    middle (by default most of its width) and `ry` deep, unless `block` is off."""
    w, h = size_of(sprite)
    rx = rx if rx is not None else max(0.3, w * 0.4)
    parts = {
        'sindri.sprite': {'texture': f'textures/village.png#{sprite}', 'layer': 5},
        'sindri.shadow2d': shadow(rx * 2.0, ry * 1.2, cast=True),
        **more}
    if block:
        centre = ry
        parts['sindri.physics2d.rigid_body'] = static_body()
        parts['sindri.physics2d.collider'] = {
            'shape': {'shape': 'box', 'half_extents': [rx / CELL, ry / CELL]}, 'offset': [0, centre / CELL],
            'rotation': 0, 'sensor': False, 'layers': {'memberships': WALL, 'filter': ALL},
            'friction': 0, 'restitution': 0}
        parts['sindri.light2d.occluder'] = {
            'shape': 'polygon', 'points': [[px / CELL, py / CELL] for px, py in footprint(rx, ry, centre, 12)]}
    return entity(ident, ident, at(x, y, depth(y), -CELL if flip else CELL, CELL), **parts)


def smoke(ident, x, y):
    return entity(ident, ident, at(x, y, 0.2), **{'sindri.particles': scenes.motes(
        5.0, 3.0, 0.5, [[0.7, 0.7, 0.78, 0.0], [0.7, 0.7, 0.78, 0.5], [0.5, 0.5, 0.6, 0.0]], 0.3, 1.1,
        playing=False, spread=25.0, shape_size=[0.2, 0.1], gravity=[0.25, 0.7], drag=0.3, layer=6)})


def moths(ident, x, y):
    return entity(ident, ident, at(x, y + FLAME - 0.4, 0.2), **{'sindri.particles': scenes.motes(
        3.0, 2.4, 0.9, [[1.0, 0.95, 0.8, 0.0], [1.0, 0.95, 0.8, 0.9], [1.0, 0.95, 0.8, 0.0]], 0.09, 0.07,
        playing=False, spread=360.0, shape='circle', shape_size=[1.6, 1.0], gravity=[0.0, 0.0], drag=0.8, layer=7)})


def house(name, sprite, x, y, lamp, villager, place):
    """A house in its two states, and the warm light inside it that comes on with its lamp."""
    ident = f'house-{name}'
    clips = {'unlit': {'frames': [f'{sprite}-unlit'], 'seconds_per_frame': 0.1, 'looping': False},
             'lit': {'frames': [sprite], 'seconds_per_frame': 0.1, 'looping': False}}
    w, _ = size_of(sprite)
    body = standing(ident, f'{sprite}-unlit', x, y, flip=name in FLIPPED, rx=w * 0.42, ry=1.3, **{
        'sindri.animation.sprite': {'clips': clips, 'playing': 'unlit', 'speed': 1},
        'sindri.script': {'source': 'scripts/house.decay', 'script': 'House',
                          'properties': {'lamp_name': lamp, 'glow_name': ident + '-glow',
                                         'smoke_name': ident + '-smoke', 'moths_name': ident + '-moths'}}})
    glow = entity(ident + '-glow', ident + '-glow', at(x, y + 1.2, 0.1), **{
        'sindri.light2d': {'enabled': True, 'color': [1.0, 0.72, 0.38, 1.0], 'intensity': 0.0, 'radius': 6.5,
                           'falloff': 1.5, 'height': 1.0, 'shadows': False}})
    dx, dy = CHIMNEYS[name]
    lamp_at = LAMPS[int(lamp.split('-')[1]) - 1]
    return [body, glow, smoke(ident + '-smoke', x + dx, y + dy), moths(ident + '-moths', *lamp_at)]


def villager(person, lamp, home, place):
    clips = {d: {'frames': [f'{person}-{d}'], 'seconds_per_frame': 0.1, 'looping': False} for d in DIRECTIONS}
    x, y = place
    return entity(f'villager-{person}', person, at(x, y, depth(y), 3.0, 2.75), **{
        'sindri.sprite': {'texture': f'textures/villagers.png#{person}-south', 'layer': 5,
                          'tint': [1.0, 1.0, 1.0, 0.0]},
        'sindri.shadow2d': shadow(0.9, 0.35),
        'sindri.animation.sprite': {'clips': clips, 'playing': 'south', 'speed': 1},
        'sindri.script': {'source': 'scripts/villager.decay', 'script': 'Villager',
                          'properties': {'lamp_name': lamp}}})


ROADS = [(-24.5, 24.5, -2.4, 2.4), (-2.4, 2.4, -19.25, 19.25), (-14.5, 14.5, -14.6, -9.9), (-6.0, 6.0, -6.0, 6.0),
         (8.4, 10.9, 1.5, 5.6), (-10.9, -8.4, 1.5, 5.6), (8.4, 10.9, -10.9, -8.4), (-10.9, -8.4, -10.9, -8.4)]


def clear_of_roads(x, y, margin):
    return not any(x0 - margin <= x <= x1 + margin and y0 - margin <= y <= y1 + margin for x0, x1, y0, y1 in ROADS)


def scatter():
    """Trees and bushes, thicker towards the edge of the map, clear of the roads, the houses, what
    is placed by hand and where the gloom start; and small flowers and stones on the grass."""
    rng = random.Random(7)
    taken = ([(x, y, 4.6) for _, _, x, y, *_ in HOUSES] + [(x, y, 1.8) for _, _, x, y, _ in BY_HAND]
             + [(x, y, 1.6) for _, _, x, y in FRAMING] + [(x, y, 3.5) for x, y in GLOOM_AT]
             + [(-22.5, 0.0, 3.5)] + [(x, y, 2.2) for x, y in LAMPS])
    trees = ['tree-oak', 'tree-broad', 'tree-tall', 'tree-small-1', 'tree-small-2', 'tree-small-3', 'tree-small-4']
    bushes = ['bush-1', 'bush-2', 'bush-3', 'bush-flower', 'bush-4', 'bush-5', 'bush-6', 'bush-7', 'bush-8']
    out, placed, tries = [], [], 0
    while len(placed) < 95 and tries < 8000:
        tries += 1
        x, y = rng.uniform(-24.0, 24.0), rng.uniform(-18.8, 18.8)
        edge = max(abs(x) / 24.5, abs(y) / 19.25)
        if rng.random() > edge ** 2.4 + 0.05:
            continue
        if not clear_of_roads(x, y, 1.2) or any(math.hypot(x - tx, y - ty) < r for tx, ty, r in taken):
            continue
        if any(math.hypot(x - px, y - py) < 2.3 for px, py in placed):
            continue
        tree = rng.random() < 0.55
        name = rng.choice(trees if tree else bushes)
        placed.append((x, y))
        out.append(standing(f'plant-{len(placed)}', name, round(x, 2), round(y, 2), flip=rng.random() < 0.5,
                            rx=0.5 if tree else None, ry=0.35))
    flowers = [f'flowers-{n}' for n in range(1, 12)]
    count = 0
    while count < 70:
        x, y = rng.uniform(-24.0, 24.0), rng.uniform(-18.8, 18.8)
        if not clear_of_roads(x, y, 0.8) or any(math.hypot(x - tx, y - ty) < r for tx, ty, r in taken):
            continue
        count += 1
        out.append(standing(f'bloom-{count}', rng.choice(flowers), round(x, 2), round(y, 2),
                            flip=rng.random() < 0.5, block=False))
    return out


def camera():
    half_w, half_h = COLUMNS * T / 2, ROWS * T / 2
    return entity('camera', 'Camera', at(-18.0, 0.0, 10.0), **{
        'sindri.camera': {'projection': 'orthographic', 'vertical_size': 16.875, 'near': 0.1, 'far': 100.0,
                          'fit': 'height'},
        'sindri.camera.behavior': {
            'follow': {'target': 'wick', 'offset': [0, 1.2, 0], 'dead_zone': [0.5, 0.5], 'smoothing': 8,
                       'max_speed': 30},
            'confine': {'min': [-half_w, -half_h], 'max': [half_w, half_h], 'contain_view': True},
            'shake': {'strength': 0.55, 'decay': 3.2, 'frequency': 42.0}}})


def village():
    stone = [0.02, 0.02, 0.04, 1.0]
    half_w, half_h = COLUMNS * T / 2, ROWS * T / 2
    ents = [
        camera(),
        environment([0.05, 0.06, 0.12]),
        entity('physics', 'Physics', **{
            'sindri.physics2d.world': {'gravity': [0, 0], 'layers': ['wall', 'wick', 'lantern', 'target']},
            'sindri.input.actions': {'actions': input_actions()}}),
        ground(),
        wall('edge-north', 0, half_h + 0.5, COLUMNS * T + 2, 1, stone),
        wall('edge-south', 0, -half_h - 0.5, COLUMNS * T + 2, 1, stone),
        wall('edge-west', -half_w - 0.5, 0, 1, ROWS * T, stone),
        wall('edge-east', half_w + 0.5, 0, 1, ROWS * T, stone),
    ]
    for args in HOUSES:
        ents += house(*args)
    for ident, sprite, x, y, flip in BY_HAND:
        ents.append(standing(ident, sprite, x, y, flip=flip))
    for ident, sprite, x, y in FRAMING:
        ents.append(standing(ident, sprite, x, y, block=sprite.startswith('fence') is False))
    for n, (x, y) in enumerate(LAMPS, 1):
        ents += lantern(f'lantern-{n}', x, y, sheet='textures/village.png', unlit='lamp-tall-unlit', lit='lamp-tall',
                        size=CELL, flame=FLAME)
    for args in HOUSES:
        ents.append(villager(args[5], args[4], args[0], args[6]))
    ents += scatter()
    ents += [*wick(), *effects()]
    for e in ents:
        if e.get('id') == 'wick':
            e['transform_3d']['position'][:2] = [-22.0, 0.0]
    for n, (x, y) in enumerate(GLOOM_AT, 1):
        ents += [gloom(f'gloom-{n}', x, y, 1 if n == 1 else 3), *wisps(f'gloom-{n}')]
    ents += hud('title.scene')
    return ents


def main():
    scenes.write('village.scene', 'The Lamplit Square', village())


if __name__ == '__main__':
    main()
