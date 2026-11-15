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
import json
import math
from pathlib import Path

from PIL import Image

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
        'blocks': 'blocks.tileset', 'view': 'map', 'variant_seed': 5, 'layer': -10, 'edits': [], 'soft_edges': True}})


def biome(name, temperature, moisture, surface, under, relief, terraces=0):
    return {'name': name, 'temperature': temperature, 'moisture': moisture, 'surface_voxel': surface,
            'subsurface_voxel': under, 'subsurface_depth': 3, 'trees': 0.0, 'relief': relief,
            'terraces': terraces}


def seen_in_the_dark():
    """The wilds' environment: a darker night than the field's, and an eye that
    sees it as eyes do. The band where Wick walks is in focus and the land
    above and below it softens, and what the lantern does not reach goes grey
    and soft, so the dark is somewhere it is truly hard to see into."""
    env = environment([0.025, 0.028, 0.05])
    env['components']['sindri.environment'].update({
        'background': [0.005, 0.005, 0.012, 1.0],
        'depth_of_field': {'enabled': True, 'focus': 0.56, 'range': 0.1, 'falloff': 0.28, 'blur': 0.0045},
        'night_vision': {'enabled': True, 'threshold': 0.14, 'blur': 0.007, 'grey': 0.9},
        'post_process': {'vignette': 0.45},
    })
    return env


TEXTURES = Path(__file__).resolve().parent.parent / 'assets' / 'textures'
HAND = 0.95     # how high Wick carries his lantern
ROUNDER = 0.6   # a contact shadow's depth for its width: the camera foreshortens it already
GROUNDED = 0.7  # how dark a contact shadow is: what stands has to be seen to touch the ground
SPREAD = 1.5    # how much wider than the field's: its far half is hidden behind what stands on it


def gap_below(reference):
    """How much of a picture's height is empty below what is drawn in it, so a
    picture standing on its foot can be sunk until its feet touch the ground."""
    path, _, name = reference.partition('#')
    image = Image.open(TEXTURES.parent / path).convert('RGBA')
    sheet_file = (TEXTURES.parent / path).with_suffix('.sheet')
    sheet = json.loads(sheet_file.read_text())['grid'] if sheet_file.exists() else \
        {'columns': 1, 'rows': 1, 'names': ['']}
    width, height = image.width // sheet['columns'], image.height // sheet['rows']
    index = sheet['names'].index(name) if name else 0
    x, y = (index % sheet['columns']) * width, (index // sheet['columns']) * height
    alpha = image.crop((x, y, x + width, y + height)).getchannel('A')
    bottom = max(row for row in range(height) if any(alpha.getpixel((c, row)) for c in range(width)))
    return (height - 1 - bottom) / height


def standing_up(entities):
    """The field's pictures, stood up in 3D space.

    The field is a flat picture, and says height and nearness the way one does:
    what is nearer is drawn over what is further by a depth that grows down the
    screen, and what is high (a lamp post's flame, the lantern in Wick's hand) is
    placed a little up the screen. Standing up, the camera's distance orders what
    is drawn and height is height: everything stands on the ground, a flame is
    up its post, and the lantern is at Wick's hand. Every picture faces the
    camera, sunk until its feet touch the ground, and its contact shadow is
    rounder, since the camera now foreshortens it."""
    by_id = {e['id']: e for e in entities}
    for e in entities:
        components = e.get('components', {})
        place = e.get('transform_3d')
        sprite = components.get('sindri.sprite')
        if sprite is not None and sprite.get('layer', 0) >= 4:
            sprite['billboard'] = True
            gap = gap_below(sprite['texture'])
            # Only what stands on the ground: the lantern hangs from Wick's hand.
            if gap > 0 and e.get('parent') is None:
                size = sprite.get('size', [1, 1])
                sprite['offset'] = [0.0, round(-gap * size[1], 4)]
        shadow = components.get('sindri.shadow2d')
        if shadow is not None:
            width = round(shadow['contact_size'][0] * SPREAD, 3)
            shadow['contact_size'] = [width, round(width * ROUNDER, 3)]
            shadow['contact_opacity'] = max(shadow['contact_opacity'], GROUNDED)
        script = components.get('sindri.script')
        if script is not None and script['script'] in ('Wick', 'Gloom'):
            script.setdefault('properties', {})['upright'] = True
        if place is None or e['id'] == 'camera' or e['id'] == 'ground':
            continue
        if e.get('parent') == 'wick':
            place['position'] = [place['position'][0], 0.0, HAND]
            continue
        if e.get('parent') is None:
            place['position'][2] = 0.0
        light = components.get('sindri.light2d')
        post = by_id.get(e['id'].removesuffix('-light'))
        if light is not None and post is not e and post is not None:
            # A lamp post's flame: at the post, up it, rather than up the screen.
            flame = place['position'][1] - post['transform_3d']['position'][1]
            place['position'][1] = post['transform_3d']['position'][1]
            light['height'] = round(flame, 3)
    return entities


def wilds():
    ents = [
        tilted_camera(),
        seen_in_the_dark(),
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
