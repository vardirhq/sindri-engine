#!/usr/bin/env python3
"""Builds `assets/village.scene`: the square Wick reaches once the fields are lit.

Four lamps stand round a well, and each of them is one house's: lighting it opens the house
(its windows light, its light comes on) and brings its villager out. The gloom keep to the
dark beyond the lamps' pools. Everything in the Dim Fields that Wick carries (his sprite, the
lantern, the bursts, the HUD) is the same here; what this adds is the place.

The art is cropped and set on one cell by `village.py`, 16 art pixels to a world unit, so a
house is 6 units wide and a door about a unit and a half tall."""
import sys

import scenes
from scenes import (ALL, WALL, at, camera, effects, entity, environment, footprint, gloom, hud, input_actions,
                    lantern, depth, shadow, static_body, wall, wick, wisps)

COLUMNS, ROWS = 46, 38
LEFT, TOP = -COLUMNS // 2, ROWS // 2
CELL = 7.0          # a 112 pixel cell is seven world units across

# Each house: where its foot stands, the lamp that opens it, and where its villager waits.
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


def standing(ident, sprite, x, y, rx, ry, centre, **more):
    """A thing of the village that stands on its position and stops Wick and light: the
    collider and the occluder are its footprint, in world units, divided by the cell scale."""
    return entity(ident, ident, at(x, y, depth(y), CELL, CELL), **{
        'sindri.sprite': {'texture': f'textures/village.png#{sprite}', 'layer': 5},
        'sindri.shadow2d': shadow(rx * 2.0, ry * 1.2, cast=True),
        'sindri.physics2d.rigid_body': static_body(),
        'sindri.physics2d.collider': {'shape': {'shape': 'box', 'half_extents': [rx / CELL, ry / CELL]},
                                      'offset': [0, centre / CELL], 'rotation': 0, 'sensor': False,
                                      'layers': {'memberships': WALL, 'filter': ALL},
                                      'friction': 0, 'restitution': 0},
        'sindri.light2d.occluder': {'shape': 'polygon',
                                    'points': [[px / CELL, py / CELL] for px, py in footprint(rx, ry, centre, 12)]},
        **more})


def house(name, sprite, x, y, lamp, villager, place):
    """A house in its two states, and the warm light inside it that comes on with its lamp."""
    ident = f'house-{name}'
    clips = {'unlit': {'frames': [f'{sprite}-unlit'], 'seconds_per_frame': 0.1, 'looping': False},
             'lit': {'frames': [f'{sprite}-lit'], 'seconds_per_frame': 0.1, 'looping': False}}
    body = standing(ident, f'{sprite}-unlit', x, y, 2.7, 1.1, 1.3, **{
        'sindri.animation.sprite': {'clips': clips, 'playing': 'unlit', 'speed': 1},
        'sindri.script': {'source': 'scripts/house.decay', 'script': 'House',
                          'properties': {'lamp_name': lamp, 'glow_name': ident + '-glow'}}})
    glow = entity(ident + '-glow', ident + '-glow', at(x, y + 1.0, 0.1), **{
        'sindri.light2d': {'enabled': True, 'color': [1.0, 0.72, 0.38, 1.0], 'intensity': 0.0, 'radius': 5.5,
                           'falloff': 1.5, 'height': 1.0, 'shadows': False}})
    return [body, glow]


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
    return entity('ground', 'Ground', at(LEFT, TOP, -1.0), **{
        'sindri.tilemap': {
            'texture': 'textures/ground.png', 'palette': [f'ground-{i}' for i in range(16)],
            'columns': COLUMNS, 'rows': ROWS, 'tiles': [0] * (COLUMNS * ROWS), 'layer': -10},
        'sindri.script': {'source': 'scripts/village_ground.decay', 'script': 'VillageGround',
                          'properties': {'left': float(LEFT), 'top': float(TOP)}}})


def road():
    return entity('road', 'road', at(LEFT, TOP, -0.9), **{'sindri.tilemap': {
        'texture': 'textures/road.png', 'palette': [], 'columns': COLUMNS, 'rows': ROWS,
        'tiles': [None] * (COLUMNS * ROWS), 'layer': -9}})


def village():
    stone = [0.02, 0.02, 0.04, 1.0]
    half_w, half_h = COLUMNS / 2, ROWS / 2
    ents = [
        camera(-18.0, 0.0, 'wick'),
        environment([0.05, 0.06, 0.12]),
        entity('physics', 'Physics', **{
            'sindri.physics2d.world': {'gravity': [0, 0], 'layers': ['wall', 'wick', 'lantern', 'target']},
            'sindri.input.actions': {'actions': input_actions()}}),
        ground(),
        road(),
        wall('edge-north', 0, half_h + 0.5, COLUMNS + 2, 1, stone),
        wall('edge-south', 0, -half_h - 0.5, COLUMNS + 2, 1, stone),
        wall('edge-west', -half_w - 0.5, 0, 1, ROWS, stone),
        wall('edge-east', half_w + 0.5, 0, 1, ROWS, stone),
    ]
    for args in HOUSES:
        ents += house(*args[:6], args[6])
    ents += [
        standing('chapel', 'chapel-lit', 0.0, 14.0, 3.0, 1.2, 1.4),
        standing('well', 'well', 0.0, 0.8, 0.9, 0.5, 0.5),
        standing('signpost', 'signpost', -7.5, 1.9, 0.3, 0.2, 0.3),
    ]
    for n, (x, y) in enumerate(LAMPS, 1):
        ents += lantern(f'lantern-{n}', x, y)
    for args in HOUSES:
        ents.append(villager(args[5], args[4], args[0], args[6]))
    ents += [*wick(), *effects()]
    # Wick arrives on the road from the fields, at the west edge.
    for e in ents:
        if e.get('id') == 'wick':
            e['transform_3d']['position'][:2] = [-20.0, 0.0]
    ents += [gloom('gloom-1', -17.0, -15.0, 1), *wisps('gloom-1'),
             gloom('gloom-2', 17.0, 15.0, 3), *wisps('gloom-2')]
    ents += hud('title.scene')
    return ents


def main():
    scenes.write('village.scene', 'The Lamplit Square', village())


if __name__ == '__main__':
    main()
