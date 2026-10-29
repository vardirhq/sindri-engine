#!/usr/bin/env python3
"""Builds `assets/village.scene`: the square Wick reaches once the fields are lit.

Four lamps stand round a well, and each of them is one house's: lighting it opens the house
(its windows light, its light comes on) and brings its villager out. The gloom keep to the
dark beyond the lamps' pools. Everything in the Dim Fields that Wick carries (his sprite, the
lantern, the bursts, the HUD) is the same here; what this adds is the place.

The art is cropped and set on one cell by `village.py`, 16 art pixels to a world unit, so a
house is about 5 to 6 units wide and a door about a unit and a half tall. The ground is four
layers (`village_tiles.py`): grass and earth roads, cobblestones, flagstones and tilled soil.
Trees and bushes are scattered, thicker towards the edge, clear of the roads and of what is
placed by hand; the rest is placed by hand, so the street reads."""
import math
import random

import scenes
from scenes import (ALL, WALL, at, camera, effects, entity, environment, footprint, gloom, hud, input_actions,
                    lantern, depth, shadow, static_body, wall, wick, wisps)

COLUMNS, ROWS = 46, 38
LEFT, TOP = -COLUMNS // 2, ROWS // 2
CELL = 7.0          # a 112 pixel cell is seven world units across

# Where each house's chimney is, from its foot, for the smoke.
CHIMNEYS = {'bakery': (2.3, 4.9), 'child-house': (0.9, 4.9), 'keeper': (1.9, 3.5), 'workshop': (2.2, 4.4)}
HOUSES = [
    # name, sprite, x, y, lamp, villager, villager's place
    # The north houses face the main road, a few steps off; the south houses face a lane of their own.
    ('bakery', 'bakery', 10.5, 3.0, 'lantern-2', 'baker', (10.5, 1.4)),
    ('child-house', 'child-house', -10.5, 3.0, 'lantern-1', 'child', (-10.5, 1.4)),
    ('workshop', 'workshop', 10.5, -9.0, 'lantern-4', 'mender', (10.5, -10.6)),
    ('keeper', 'keeper', -10.5, -9.0, 'lantern-3', 'elder', (-10.5, -10.6)),
]
LAMPS = [(-3.5, 3.5), (3.5, 3.5), (-3.5, -3.5), (3.5, -3.5)]
DIRECTIONS = ['south', 'west', 'north', 'east']

# The roads, as `village_ground.decay` has them (left, right, bottom, top): the scatter keeps clear.
ROADS = [(-24, -5, -1.3, 1.3), (5, 24, -1.3, 1.3), (-6, 6, -6, 6), (-1.3, 1.3, 0, 13.5), (-1.3, 1.3, -13, 0),
         (-15, 15, -12.8, -10.2), (9.8, 11.2, 0, 3.2), (-11.2, -9.8, 0, 3.2), (9.8, 11.2, -10.2, -8.8),
         (-11.2, -9.8, -10.2, -8.8)]

# What stands by hand, outside the houses: name, sprite, x, y, (rx, ry, centre) of what blocks or None.
# A footprint is in world units, from the foot.
BY_HAND = [
    ('well', 'well', 0.0, 0.5, (1.0, 0.55, 0.55)),
    ('chapel', 'chapel-lit', 0.0, 14.0, (2.5, 1.2, 1.3)),
    ('graves-1', 'graves', -5.8, 12.4, (1.0, 0.4, 0.4)),
    ('graves-2', 'graves', 5.8, 12.0, (1.0, 0.4, 0.4)),
    ('graves-3', 'graves', -8.4, 15.6, (1.0, 0.4, 0.4)),
    ('graves-4', 'graves', 8.2, 15.4, (1.0, 0.4, 0.4)),
    ('stall-red', 'stall-red', -17.4, 2.9, (1.2, 0.5, 0.5)),
    ('stall-blue', 'stall-blue', 17.4, 2.9, (1.2, 0.5, 0.5)),
    ('notice', 'notice', -17.6, -2.4, (0.8, 0.3, 0.3)),
    ('bench', 'bench', -7.4, 2.6, (0.9, 0.4, 0.4)),
    ('garden-1', 'garden-cabbage', -16.4, 5.4, None),
    ('garden-2', 'garden-cabbage-2', -19.0, 5.6, None),
    ('scarecrow', 'scarecrow', -17.6, 7.8, (0.4, 0.3, 0.3)),
    ('woodpile', 'woodpile', 15.6, -8.2, (1.0, 0.5, 0.5)),
    ('crates-1', 'crates', 4.2, -8.6, (0.9, 0.5, 0.5)),
    ('barrels-1', 'barrels', 16.6, -9.6, (0.7, 0.4, 0.4)),
    ('cart', 'cart', -4.6, -9.2, (1.0, 0.5, 0.5)),
    ('laundry', 'laundry', -19.0, -8.4, None),
    ('flowers-1', 'flowers-white', -8.2, 3.2, None),
    ('flowers-2', 'flowers-purple', -12.9, 3.0, None),
    ('flowers-3', 'flowers-purple-2', 8.2, 3.0, None),
    ('flowers-4', 'flowers-soil', 12.9, 3.0, None),
    ('flowers-5', 'flowers-white', 4.0, 8.0, None),
    ('flowers-6', 'flowers-purple', -4.0, 8.2, None),
    ('flowers-7', 'flowers-purple-2', -4.2, -8.6, None),
]
GLOOM_AT = [(-17.0, -15.0), (17.0, 15.0)]


def standing(ident, sprite, x, y, block, occlude=True, flip=False, **more):
    """A thing of the village that stands on its position, and if `block` (its footprint's two
    radii and the height of its middle, in world units) stops Wick and light."""
    sx = -CELL if flip else CELL
    parts = {
        'sindri.sprite': {'texture': f'textures/village.png#{sprite}', 'layer': 5},
        'sindri.shadow2d': shadow((block[0] if block else 0.8) * 2.0, (block[1] if block else 0.3) * 1.2, cast=True),
        **more}
    if block:
        rx, ry, centre = block
        parts['sindri.physics2d.rigid_body'] = static_body()
        parts['sindri.physics2d.collider'] = {
            'shape': {'shape': 'box', 'half_extents': [rx / CELL, ry / CELL]}, 'offset': [0, centre / CELL],
            'rotation': 0, 'sensor': False, 'layers': {'memberships': WALL, 'filter': ALL},
            'friction': 0, 'restitution': 0}
        if occlude:
            parts['sindri.light2d.occluder'] = {
                'shape': 'polygon', 'points': [[px / CELL, py / CELL] for px, py in footprint(rx, ry, centre, 12)]}
    return entity(ident, ident, at(x, y, depth(y), sx, CELL), **parts)


def smoke(ident, x, y):
    """Chimney smoke, started when its house opens."""
    return entity(ident, ident, at(x, y, 0.2), **{'sindri.particles': scenes.motes(
        5.0, 3.0, 0.5, [[0.7, 0.7, 0.78, 0.0], [0.7, 0.7, 0.78, 0.5], [0.5, 0.5, 0.6, 0.0]], 0.3, 1.1,
        playing=False, spread=25.0, shape_size=[0.2, 0.1], gravity=[0.25, 0.7], drag=0.3, layer=6)})


def moths(ident, x, y):
    """A few pale moths that flutter round a lamp, started when its house opens."""
    return entity(ident, ident, at(x, y + 2.4, 0.2), **{'sindri.particles': scenes.motes(
        3.0, 2.4, 0.9, [[1.0, 0.95, 0.8, 0.0], [1.0, 0.95, 0.8, 0.9], [1.0, 0.95, 0.8, 0.0]], 0.09, 0.07,
        playing=False, spread=360.0, shape='circle', shape_size=[1.6, 1.0], gravity=[0.0, 0.0], drag=0.8, layer=7)})


def house(name, sprite, x, y, lamp, villager, place):
    """A house in its two states, and the warm light inside it that comes on with its lamp."""
    ident = f'house-{name}'
    clips = {'unlit': {'frames': [f'{sprite}-unlit'], 'seconds_per_frame': 0.1, 'looping': False},
             'lit': {'frames': [f'{sprite}-lit'], 'seconds_per_frame': 0.1, 'looping': False}}
    body = standing(ident, f'{sprite}-unlit', x, y, (2.5, 1.1, 1.2), **{
        'sindri.animation.sprite': {'clips': clips, 'playing': 'unlit', 'speed': 1},
        'sindri.script': {'source': 'scripts/house.decay', 'script': 'House',
                          'properties': {'lamp_name': lamp, 'glow_name': ident + '-glow',
                                         'smoke_name': ident + '-smoke', 'moths_name': ident + '-moths'}}})
    glow = entity(ident + '-glow', ident + '-glow', at(x, y + 1.0, 0.1), **{
        'sindri.light2d': {'enabled': True, 'color': [1.0, 0.72, 0.38, 1.0], 'intensity': 0.0, 'radius': 5.5,
                           'falloff': 1.5, 'height': 1.0, 'shadows': False}})
    dx, dy = CHIMNEYS[name]
    lamp_at = LAMPS[int(lamp.split('-')[1]) - 1]
    return [body, glow, smoke(ident + '-smoke', x + dx, y + dy), moths(ident + '-moths', *lamp_at)]


def villager(person, lamp, home, place):
    """Someone who stays inside until their lamp is lit, then steps out and faces Wick."""
    clips = {d: {'frames': [f'{person}-{d}'], 'seconds_per_frame': 0.1, 'looping': False} for d in DIRECTIONS}
    x, y = place
    return entity(f'villager-{person}', person, at(x, y, depth(y), 3.0, 2.75), **{
        'sindri.sprite': {'texture': f'textures/villagers.png#{person}-south', 'layer': 5,
                          'tint': [1.0, 1.0, 1.0, 0.0]},
        'sindri.shadow2d': shadow(0.9, 0.35),
        'sindri.animation.sprite': {'clips': clips, 'playing': 'south', 'speed': 1},
        'sindri.script': {'source': 'scripts/villager.decay', 'script': 'Villager',
                          'properties': {'lamp_name': lamp}}})


def ground():
    return [
        entity('ground', 'Ground', at(LEFT, TOP, -1.0), **{
            'sindri.tilemap': {
                'texture': 'textures/village-ground.png', 'palette': [f'village-ground-{i}' for i in range(16)],
                'columns': COLUMNS, 'rows': ROWS, 'tiles': [0] * (COLUMNS * ROWS), 'layer': -10},
            'sindri.script': {'source': 'scripts/village_ground.decay', 'script': 'VillageGround',
                              'properties': {'left': float(LEFT), 'top': float(TOP)}}}),
        *[entity(name, name, at(LEFT, TOP, z), **{'sindri.tilemap': {
            'texture': f'textures/village-{name}.png', 'palette': [], 'columns': COLUMNS, 'rows': ROWS,
            'tiles': [None] * (COLUMNS * ROWS), 'layer': order}})
          for name, z, order in (('soil', -0.95, -9), ('cobble', -0.9, -8), ('flag', -0.85, -7))],
    ]


def clear_of_roads(x, y, margin):
    return not any(x0 - margin <= x <= x1 + margin and y0 - margin <= y <= y1 + margin for x0, x1, y0, y1 in ROADS)


def scatter():
    """Trees and bushes, thicker towards the edge of the map, clear of the roads, the houses, what
    is placed by hand, Wick's way in and where the gloom start; and small things on the grass."""
    rng = random.Random(7)
    taken = ([(x, y, 4.2) for _, _, x, y, *_ in HOUSES] + [(x, y, 2.0) for _, _, x, y, _ in BY_HAND]
             + [(x, y, 3.5) for x, y in GLOOM_AT] + [(-20.0, 0.0, 3.0)] + [(x, y, 2.0) for x, y in LAMPS])
    kinds = [('oak-1', (0.45, 0.3, 0.3)), ('oak-2', (0.4, 0.25, 0.25)), ('tree-1', (0.3, 0.2, 0.2)),
             ('tree-2', (0.3, 0.2, 0.2)), ('bush-1', (0.55, 0.35, 0.35)), ('bush-2', (0.55, 0.35, 0.35)),
             ('bush-3', (0.55, 0.35, 0.35)), ('bush-4', (0.55, 0.35, 0.35))]
    placed = []
    out = []
    tries = 0
    while len(placed) < 110 and tries < 6000:
        tries += 1
        x, y = rng.uniform(-22.4, 22.4), rng.uniform(-18.4, 18.0)
        edge = max(abs(x) / 23.0, abs(y) / 19.0)
        if rng.random() > edge ** 2.2 + 0.04:
            continue
        if not clear_of_roads(x, y, 1.1) or any(math.hypot(x - tx, y - ty) < r for tx, ty, r in taken):
            continue
        if any(math.hypot(x - px, y - py) < 1.5 for px, py in placed):
            continue
        name, block = rng.choice(kinds)
        placed.append((x, y))
        n = len(placed)
        out.append(standing(f'plant-{n}', name, round(x, 2), round(y, 2), block, flip=rng.random() < 0.5))
    return out


def decor():
    """Tufts, night flowers, pebbles and glowing mushrooms over the grass, as in the fields."""
    rng = random.Random(13)
    taken = ([(x, y, 4.0) for _, _, x, y, *_ in HOUSES] + [(x, y, 1.6) for _, _, x, y, _ in BY_HAND]
             + [(x, y, 1.6) for x, y in LAMPS])
    kinds = ['tufts'] * 9 + ['flowers'] * 5 + ['pebbles'] * 3 + ['mushrooms'] * 2
    out = []
    while len(out) < 170:
        x, y = round(rng.uniform(-22.5, 22.5), 2), round(rng.uniform(-18.5, 18.0), 2)
        kind = rng.choice(kinds)
        if not clear_of_roads(x, y, 0.5) or any(math.hypot(x - tx, y - ty) < r for tx, ty, r in taken):
            continue
        components = {'sindri.sprite': {'texture': f'textures/decor.png#{kind}', 'layer': 3}}
        if kind == 'mushrooms':
            components['sindri.light2d'] = {'enabled': True, 'color': [0.45, 0.85, 1.0, 1.0], 'intensity': 0.7,
                                            'radius': 2.8, 'falloff': 1.6, 'height': 0.6, 'shadows': False}
        flip = -1 if rng.random() < 0.5 else 1
        ident = f'cover-{len(out) + 1}'
        out.append(entity(ident, ident, at(x, y, depth(y) - 0.2, 1.5 * flip, 1.5), **components))
        if kind == 'mushrooms':
            out.append(entity(ident + '-spores', ident + '-spores', at(0, 0.3, 0.001), ident, **{
                'sindri.particles': scenes.motes(3.5, 3.2, 0.25, [[0.5, 0.95, 1.0, 0.0], [0.5, 0.9, 1.0, 0.85],
                                                                  [0.4, 0.8, 1.0, 0.0]], 0.17, 0.03,
                                                 spread=100.0, shape_size=[0.4, 0.2], gravity=[0.0, 0.3],
                                                 drag=0.5, layer=6)}))
    return out


def village():
    stone = [0.02, 0.02, 0.04, 1.0]
    half_w, half_h = COLUMNS / 2, ROWS / 2
    ents = [
        camera(-18.0, 0.0, 'wick'),
        environment([0.05, 0.06, 0.12]),
        entity('physics', 'Physics', **{
            'sindri.physics2d.world': {'gravity': [0, 0], 'layers': ['wall', 'wick', 'lantern', 'target']},
            'sindri.input.actions': {'actions': input_actions()}}),
        *ground(),
        wall('edge-north', 0, half_h + 0.5, COLUMNS + 2, 1, stone),
        wall('edge-south', 0, -half_h - 0.5, COLUMNS + 2, 1, stone),
        wall('edge-west', -half_w - 0.5, 0, 1, ROWS, stone),
        wall('edge-east', half_w + 0.5, 0, 1, ROWS, stone),
    ]
    for args in HOUSES:
        ents += house(*args)
    for ident, sprite, x, y, block in BY_HAND:
        ents.append(standing(ident, sprite, x, y, block))
    for n, (x, y) in enumerate(LAMPS, 1):
        ents += lantern(f'lantern-{n}', x, y, sheet='textures/village.png', unlit='lamp-unlit', lit='lamp-lit',
                        size=CELL, flame=2.4)
    for args in HOUSES:
        ents.append(villager(args[5], args[4], args[0], args[6]))
    ents += scatter() + decor()
    ents += [*wick(), *effects()]
    # Wick arrives on the road from the fields, at the west edge.
    for e in ents:
        if e.get('id') == 'wick':
            e['transform_3d']['position'][:2] = [-20.0, 0.0]
    for n, (x, y) in enumerate(GLOOM_AT, 1):
        ents += [gloom(f'gloom-{n}', x, y, 1 if n == 1 else 3), *wisps(f'gloom-{n}')]
    ents += hud('title.scene')
    return ents


def main():
    scenes.write('village.scene', 'The Lamplit Square', village())


if __name__ == '__main__':
    main()
