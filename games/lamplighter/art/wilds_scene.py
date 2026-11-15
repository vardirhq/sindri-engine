#!/usr/bin/env python3
"""Builds `assets/wilds.scene`: the wilds, Lamplighter as 2D in 3D space.

Stage 1 of `docs/lamplighter-plan.md`. The ground is a generated voxel world drawn
as a map, on the plane the game already plays on, in the game's own blocks
(`blocks.py`): meadow, deadwood, heath, bog and barrens, with rivers and water
from one seed. A perspective camera looks down at it from behind Wick, and
everything that stands (Wick, the gloom, the lamps, trees and boulders) is a pixel
sprite billboarded to face the camera, standing on its feet. The 2D lights,
physics and scripts are the field's, unchanged.

What stands is placed by hand around the start for now; scattering it by seed
and biome across the land is the next step of stage 1."""
import math

import scenes
from scenes import (at, effects, entity, environment, gloom, input_actions, lantern, prop, wick)

TILT = math.radians(38.0)   # how far the camera leans from straight down


def tilted_camera():
    """A perspective camera behind and above Wick, leaning towards where he goes."""
    half = TILT / 2.0
    rotation = [math.sin(half), 0.0, 0.0, math.cos(half)]
    distance = 22.0
    offset = [0.0, -distance * math.sin(TILT), distance * math.cos(TILT)]
    return entity('camera', 'Camera', {'position': offset, 'rotation': rotation, 'scale': [1, 1, 1]}, **{
        'sindri.camera': {'projection': 'perspective', 'vertical_fov_degrees': 40.0, 'near': 0.5, 'far': 200.0},
        'sindri.camera.behavior': {'follow': {'target': 'wick', 'offset': offset, 'dead_zone': [0.3, 0.3],
                                              'smoothing': 8, 'max_speed': 30}}})


def ground():
    """A world of biomes from one seed, drawn flat, its top a little below where things stand."""
    return entity('ground', 'Ground', at(-60.0, 60.0, -0.05), **{'sindri.voxel_world': {
        'generator': {
            'kind': 'natural_terrain', 'seed': 3, 'sea_level': 0, 'relief': 12, 'feature_size': 48,
            'tree_line': 400, 'snow_line': 500, 'caves': False, 'rivers': True,
            'stone_voxel': 'stone', 'water_voxel': 'water', 'beach_voxel': 'sand', 'sea_bed_voxel': 'mud',
            'cliff_voxel': 'rock', 'snow_voxel': None, 'ice_voxel': None, 'trunk_voxel': None,
            'leaves_voxel': None,
            'biomes': [
                biome('Meadow', 0.55, 0.5, 'grass', 'dirt', 0.6),
                biome('Deadwood', 0.35, 0.45, 'deadgrass', 'dirt', 0.8),
                biome('Heath', 0.6, 0.15, 'earth', 'dirt', 1.0, terraces=2),
                biome('Bog', 0.6, 0.95, 'moss', 'mud', 0.2),
                biome('Barrens', 0.1, 0.2, 'stone', 'rock', 1.4, terraces=2),
            ]},
        'blocks': 'blocks.tileset', 'view': 'map', 'variant_seed': 5, 'layer': -10, 'edits': []}})


def biome(name, temperature, moisture, surface, under, relief, terraces=0):
    return {'name': name, 'temperature': temperature, 'moisture': moisture, 'surface_voxel': surface,
            'subsurface_voxel': under, 'subsurface_depth': 3, 'trees': 0.0, 'relief': relief,
            'terraces': terraces}


def standing_up(entities):
    """Every sprite that stands in the world faces the camera, a picture on its feet."""
    for e in entities:
        sprite = e.get('components', {}).get('sindri.sprite')
        if sprite is not None and sprite.get('layer', 0) >= 4:
            sprite['billboard'] = True
    return entities


def wilds():
    ents = [
        tilted_camera(),
        environment([0.05, 0.06, 0.12]),
        entity('physics', 'Physics', **{
            'sindri.physics2d.world': {'gravity': [0, 0], 'layers': ['wall', 'wick', 'lantern', 'target']},
            'sindri.input.actions': {'actions': input_actions()}}),
        ground(),
    ]
    trees = [(-6, 4), (-3, 7), (5, 6), (9, 3), (-8, -3), (4, -6), (11, -4), (-2, -8), (14, 8), (-12, 6)]
    ents += [prop(f'tree-{n}', f'dead-tree-{n % 5}', x, y, 0.6, 0.5, (0.3, 0.18, 0.2), flip=n % 2 == 0)
             for n, (x, y) in enumerate(trees, 1)]
    ents += [prop(f'boulder-{n}', f'boulder-{n % 4}', x, y, 0.9, 0.7, (0.85, 0.42, 0.45))
             for n, (x, y) in enumerate([(2, 3), (-5, -6), (8, -2)], 1)]
    ents += [*lantern('lantern-1', 6, 1), *lantern('lantern-2', -7, 2)]
    ents += [*wick(), *effects(), gloom('gloom-1', 10, 5, 1), gloom('gloom-2', -9, -5, 3)]
    return standing_up(ents)


def main():
    scenes.write('wilds.scene', 'The Wilds', wilds())


if __name__ == '__main__':
    main()
