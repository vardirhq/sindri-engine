#!/usr/bin/env python3
"""Builds `assets/wilds.scene`: the wilds, Lamplighter as 2D in 3D space.

Stage 1 of `docs/lamplighter-plan.md`. The ground is a generated voxel world drawn
as a map, on the plane the game already plays on, in the game's own blocks
(`blocks.py`): meadow, deadwood, heath, bog and barrens, with rivers and water
from one seed. A perspective camera looks down at it from behind Wick, and
everything that stands (Wick, the gloom, the lamps, trees and boulders) is a pixel
sprite billboarded to face the camera, standing on its feet. The 2D lights,
physics and scripts are the field's, unchanged.

What grows is scattered by seed (`sindri.scatter` on the ground): dead trees
in the deadwood, oaks and bushes in the meadow, boulders in the barrens, and
ground cover everywhere, each from a prefab this script writes to
`assets/prefabs/wilds/`. The engine makes them near Wick and unmakes them far
from him, so the land goes on. The lamp posts and the gloom are placed by hand,
in clearings the scatter leaves."""
import json
import math
from pathlib import Path

from PIL import Image

import scenes
import village_scene
from scenes import (at, effects, entity, environment, gloom, input_actions, lantern, prop, wick)

TILT = math.radians(38.0)   # how far the camera leans from straight down


def tilted_camera():
    """A perspective camera behind and above Wick, leaning towards where he goes."""
    half = TILT / 2.0
    rotation = [math.sin(half), 0.0, 0.0, math.cos(half)]
    distance = 22.0
    offset = [0.0, -distance * math.sin(TILT), distance * math.cos(TILT)]
    at = [START[0] + offset[0], START[1] + offset[1], offset[2]]
    return entity('camera', 'Camera', {'position': at, 'rotation': rotation, 'scale': [1, 1, 1]}, **{
        'sindri.camera': {'projection': 'perspective', 'vertical_fov_degrees': 40.0, 'near': 0.5, 'far': 200.0},
        'sindri.camera.behavior': {'follow': {'target': 'wick', 'offset': offset, 'dead_zone': [0.3, 0.3],
                                              'smoothing': 8, 'max_speed': 30}}})


def ground():
    """A world of biomes from one seed, drawn flat, its top a little below where things stand."""
    return entity('ground', 'Ground', at(-60.0, 60.0, -0.05), **{'sindri.voxel_world': {
        'generator': {
            'kind': 'natural_terrain', 'seed': 3, 'sea_level': 0, 'relief': 32, 'feature_size': 64,
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
        'blocks': 'blocks.tileset', 'view': 'map', 'variant_seed': 5, 'layer': -10, 'edits': [], 'soft_edges': True},
        'sindri.scatter': scatter(),
        # Water stops Wick, near him, however far he goes.
        'sindri.map_walls': {'around': 'wick', 'reach': 20, 'memberships': scenes.WALL, 'filter': scenes.ALL},
        # The land goes on as far as Wick walks; the world is moved back round him
        # so it stays as sharp a hundred thousand squares out as at the start.
        'sindri.floating_origin': {'around': 'wick', 'distance': 512.0, 'step': 256.0, 'moved': [0.0, 0.0]}})


ORIGIN = (-60.0, 60.0)   # where the ground's first square is: square (60, 60) is the world's middle
PREFABS = Path(__file__).resolve().parent.parent / 'assets' / 'prefabs' / 'wilds'
START = (10, 6)   # Wick's first step: meadow, by a river, the sea to the west
LAMPS = [(15, 3), (4, 9)]
GLOOM = [(18, 12), (18, -4)]


def square_of(x, y):
    """The ground's column and row at a place in the world, as a clearing names them."""
    return [x - ORIGIN[0], ORIGIN[1] - y]


def scatter():
    """What grows on the wilds, by the ground it grows on."""
    prefab = lambda names: [f'prefabs/wilds/{name}.prefab' for name in names]
    rule = lambda on, names, spacing, chance: {'on': on, 'prefabs': prefab(names), 'spacing': spacing,
                                               'chance': chance, 'flip': True}
    trees = [f'dead-tree-{n}' for n in range(5)]
    boulders = [f'boulder-{n}' for n in range(4)]
    clear = [square_of(*START) + [4.0]] + [square_of(x, y) + [2.5] for x, y in LAMPS + GLOOM]
    return {'around': 'wick', 'reach': 22, 'seed': 17, 'clearings': clear, 'taken': [], 'rules': [
        rule(['deadgrass'], trees, 3, 0.55),
        rule(['grass'], ['oak-1', 'oak-2', 'oak-3', 'oak-4', 'tree-1', 'tree-2'], 7, 0.5),
        rule(['grass'], ['bush-1', 'bush-2', 'bush-3', 'bush-4'], 5, 0.3),
        rule(['earth'], ['bush-1', 'bush-2', 'bush-3', 'bush-4'] + boulders[:2], 5, 0.4),
        rule(['moss', 'mud'], trees[:2], 6, 0.3),
        rule(['stone', 'rock'], boulders, 4, 0.5),
        rule(['grass', 'deadgrass', 'earth'], ['tufts', 'flowers', 'pebbles'], 2, 0.3),
        rule(['moss'], ['mushrooms', 'tufts'], 2, 0.35),
        rule(['stone', 'rock', 'sand'], ['pebbles'], 3, 0.3),
    ]}


def prefabs():
    """Every prefab the scatter makes, stood up as the rest of the wilds is."""
    made = {}
    for n in range(5):
        made[f'dead-tree-{n}'] = prop('tree', f'dead-tree-{n}', 0, 0, 0.6, 0.5, (0.3, 0.18, 0.2))
    for n in range(4):
        made[f'boulder-{n}'] = prop('boulder', f'boulder-{n}', 0, 0, 0.9, 0.7, (0.85, 0.42, 0.45))
    blocks = {'oak': (0.45, 0.3, 0.3), 'tree': (0.3, 0.2, 0.2), 'bush': (0.55, 0.35, 0.35)}
    for name in ['oak-1', 'oak-2', 'oak-3', 'oak-4', 'tree-1', 'tree-2', 'bush-1', 'bush-2', 'bush-3', 'bush-4']:
        made[name] = village_scene.standing('plant', name, 0, 0, blocks[name.split('-')[0]])
    for kind in ['tufts', 'flowers', 'pebbles', 'mushrooms']:
        components = {'sindri.sprite': {'texture': f'textures/decor.png#{kind}', 'layer': 4}}
        if kind == 'mushrooms':
            components['sindri.light2d'] = {'enabled': True, 'color': [0.45, 0.85, 1.0, 1.0], 'intensity': 0.7,
                                            'radius': 2.8, 'falloff': 1.6, 'height': 0.6, 'shadows': False}
        made[kind] = entity('cover', 'cover', at(0, 0, 0, 1.5, 1.5), **components)
    PREFABS.mkdir(parents=True, exist_ok=True)
    for name, thing in made.items():
        thing['id'] = name
        thing['name'] = name
        (one,) = standing_up([thing])
        one['transform_3d']['position'] = [0.0, 0.0, 0.0]
        doc = {'format_version': 1, 'metadata': {'name': name}, 'entities': [one]}
        (PREFABS / f'{name}.prefab').write_text(json.dumps(doc, indent=2) + '\n')


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


def placed_at_start(entities):
    """Wick where the wilds begin: his own entity moved there, what he carries with him."""
    for e in entities:
        if e['id'] == 'wick':
            e['transform_3d']['position'][0] += START[0]
            e['transform_3d']['position'][1] += START[1]
    return entities


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
    ents += [e for n, (x, y) in enumerate(LAMPS, 1) for e in lantern(f'lantern-{n}', x, y)]
    ents += [*placed_at_start(wick()), *effects(), *[gloom(f'gloom-{n}', x, y, size) for n, ((x, y), size) in enumerate(zip(GLOOM, [1, 3]), 1)]]
    return standing_up(ents)


def main():
    prefabs()
    scenes.write('wilds.scene', 'The Wilds', wilds())


if __name__ == '__main__':
    main()
