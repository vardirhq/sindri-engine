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
        'blocks': 'blocks.tileset', 'view': 'map', 'variant_seed': 5, 'layer': -10, 'edits': [], 'soft_edges': True, 'map_cliffs': True,
        # Water that moves: pale in the shallows, dark in the deep, swaying,
        # glinting where a light catches it and foaming at the shore.
        'map_water': {'blocks': ['water'], 'shallow': [0.1, 0.34, 0.38], 'deep': [0.015, 0.05, 0.14], 'depth': 6,
                      'waves': 0.7, 'glints': 0.6, 'foam': 0.7, 'speed': 1.0,
                      'wet': 0.3, 'reflections': 0.9}},
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
SUNFALLS = [(6, 3)]   # one in sight of the start, so the first thing Wick sees is cracklight
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
        # What lives in the air, last so it never keeps a thing off its square.
        rule(['grass'], ['fireflies'], 6, 0.55),
        rule(['moss', 'mud'], ['mist'], 5, 0.6),
        rule(['deadgrass'], ['leaves'], 7, 0.45),
        # Now and then a crack in the Lid lets the sun through.
        rule(['grass', 'deadgrass', 'earth', 'moss', 'stone', 'sand'], ['sunfall'], 26, 0.35),
    ], 'sites': [hamlet()]}


# A hamlet's houses and what stands round them: the sprite, the footprint that
# stops Wick (two radii and the height of its middle, in world units, as the
# village's), and whether a lit one glows.
HAMLET_THINGS = {
    'well': ('well', (1.0, 0.55, 0.55)), 'woodpile': ('woodpile', (1.0, 0.5, 0.5)),
    'barrels': ('barrels', (0.7, 0.4, 0.4)), 'crates': ('crates', (0.9, 0.5, 0.5)),
    'cart': ('cart', (1.0, 0.5, 0.5)), 'laundry': ('laundry', None), 'notice': ('notice', (0.8, 0.3, 0.3)),
    'scarecrow': ('scarecrow', (0.4, 0.3, 0.3)), 'garden': ('garden-cabbage', None),
    'garden-2': ('garden-cabbage-2', None), 'bench': ('bench', (0.9, 0.4, 0.4)),
    'graves': ('graves', (1.0, 0.4, 0.4)),
}
HAMLET_HOUSES = ['bakery', 'child-house', 'workshop', 'keeper']
# How much bigger a hamlet's houses are than the village's: beside Wick standing
# up, the village's came only to his shoulders at the door.
HOUSE_SCALE = 1.6
HOUSE_SIZE = village_scene.CELL * HOUSE_SCALE


def hamlet():
    """A few houses round a well, now and then, on open ground: some still
    lived in and lit, some long dark. The seed chooses which houses stand, which
    of them they are, and whether the whole is mirrored."""
    def piece(names, x, y, chance=1.0):
        return {'prefabs': [f'prefabs/wilds/hamlet/{name}.prefab' for name in names], 'at': [x, y],
                'chance': chance}
    houses = [f'{name}{state}' for name in HAMLET_HOUSES for state in ('', '-lit', '-lit')]
    return {'name': 'hamlet', 'on': ['grass', 'deadgrass', 'earth'], 'spacing': 80, 'chance': 0.7,
            'radius': 14.0, 'pieces': [
                piece(['well'], 0.0, 0.0),
                piece(houses, -7.5, 4.0), piece(houses, 7.5, 4.5, 0.9),
                piece(houses, -6.5, -6.5, 0.7), piece(houses, 7.0, -7.0, 0.6),
                piece(['woodpile', 'barrels', 'crates'], 2.4, -1.4, 0.7),
                piece(['cart', 'bench', 'notice'], -2.8, -1.6, 0.6),
                piece(['laundry', 'scarecrow'], 12.0, -1.0, 0.6),
                piece(['garden', 'garden-2'], -12.0, -1.0, 0.6),
                piece(['graves'], 0.4, 10.5, 0.4),
                piece(['barrels', 'crates'], -1.4, -10.0, 0.5),
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
    made.update(air())
    made['sunfall'] = sunfall('sunfall', 0, 0)
    hamlet_things = {}
    for name, (sprite, block) in HAMLET_THINGS.items():
        hamlet_things[f'hamlet/{name}'] = village_scene.standing('thing', sprite, 0, 0, block)
    for name in HAMLET_HOUSES:
        hamlet_things[f'hamlet/{name}'] = village_scene.standing('house', f'{name}-unlit', 0, 0, (2.5, 1.1, 1.2),
                                                                 size=HOUSE_SIZE)
        lit = village_scene.standing('house', f'{name}-lit', 0, 0, (2.5, 1.1, 1.2), size=HOUSE_SIZE)
        # Someone is home: their windows light the ground in front of the door.
        lit['components']['sindri.light2d'] = {'enabled': True, 'color': [1.0, 0.72, 0.38, 1.0],
                                               'intensity': 1.8, 'radius': 9.0, 'falloff': 1.5, 'height': 1.0,
                                               'offset': [0.0, -0.2], 'flicker': 0.3,
                                               'shadows': False}
        hamlet_things[f'hamlet/{name}-lit'] = lit
    made.update(hamlet_things)
    # What grows moves in the wind: grass and flowers most, bushes less, a
    # tree's crown a little, a dead tree's bare branches least.
    sway = {'dead-tree': 0.025, 'oak': 0.035, 'tree': 0.035, 'bush': 0.06, 'tufts': 0.14, 'flowers': 0.14,
            'mushrooms': 0.05, 'hamlet/laundry': 0.08, 'hamlet/garden': 0.06, 'hamlet/scarecrow': 0.03}
    for name, thing in made.items():
        kind = next((k for k in sway if name == k or name.startswith(k + '-')), None)
        if kind is not None:
            thing['components']['sindri.sprite']['sway'] = sway[kind]
    (PREFABS / 'hamlet').mkdir(parents=True, exist_ok=True)
    for name, thing in made.items():
        thing['id'] = name.replace('/', '-')
        thing['name'] = name.rsplit('/', 1)[-1]
        (one,) = standing_up([thing])
        one['transform_3d']['position'] = [0.0, 0.0, 0.0]
        doc = {'format_version': 1, 'metadata': {'name': thing['name']}, 'entities': [one]}
        (PREFABS / f'{name}.prefab').write_text(json.dumps(doc, indent=2) + '\n')


def air():
    """What lives in the air over the land, each an emitter of its own: fireflies
    that glow over the meadow, mist that lies on the bog, leaves blowing across
    the deadwood."""
    def drift(name, **particles):
        return entity(name, name, at(0, 0, 0), **{'sindri.particles': scenes.motes(**particles)})
    fade = lambda rgb, peak: [[*rgb, 0.0], [*rgb, peak], [*rgb, 0.0]]
    return {
        'fireflies': drift('fireflies', rate=2.0, lifetime=4.5, speed=0.35, colors=fade([1.6, 2.0, 0.7], 1.0),
                           size=0.4, size_end=0.25, spread=360.0, shape_size=[2.5, 2.5], gravity=[0.0, 0.0],
                           drag=0.3, layer=7, glow=True, max=12),
        'mist': drift('mist', rate=0.7, lifetime=9.0, speed=0.18, colors=fade([0.62, 0.68, 0.78], 0.09),
                      size=2.2, size_end=4.0, direction=10.0, spread=30.0, shape_size=[2.0, 1.5],
                      gravity=[0.0, 0.0], drag=0.0, layer=3, glow=True, max=10),
        'leaves': drift('leaves', rate=0.8, lifetime=5.0, speed=0.7, colors=fade([0.55, 0.36, 0.18], 1.0),
                        size=0.32, size_end=0.3, direction=200.0, spread=40.0, shape_size=[2.0, 2.0],
                        gravity=[-0.05, -0.08], drag=0.1, layer=5, spin=140.0, max=6,
                        texture='textures/leaf.png'),
    }


def sunfall(name, x, y, phase=0.0):
    """Cracklight falling through the Lid: a shaft of gold standing up from where
    it lands (`art/sunfall.py`, moved by `shaders/sunfall.wgsl`), the pool it
    lights on the ground, and dust glinting in it. The shaft is unlit, being light
    itself; the pool is a wide, even light that does not flicker as a flame does."""
    gold = [1.0, 0.74, 0.36]
    return entity(name, name, at(x, y, 0), **{
        'sindri.sprite': {'texture': 'textures/sunfall.png#shaft', 'layer': 5, 'size': [3.0, 9.0],
                          'tint': [1.35, 0.88, 0.34, 1.0], 'unlit': True,
                          'shader': 'shaders/sunfall.wgsl', 'shader_params': [1.0, phase, 0.0, 0.0]},
        'sindri.light2d': {'enabled': True, 'color': [*gold, 1.0], 'intensity': 2.0, 'radius': 4.5,
                           'falloff': 0.9, 'height': 3.0, 'shadows': False, 'flicker': 0.0},
        'sindri.particles': scenes.motes(1.6, 5.0, 0.12, [[*gold, 0.0], [1.6, 1.4, 0.9, 0.9], [*gold, 0.0]],
                                         0.14, 0.1, spread=360.0, shape_size=[1.4, 1.4],
                                         gravity=[0.0, 0.03], drag=0.4, layer=7, glow=True, max=10)})


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
    # Clouds drift over the land; by night their shadows barely show, by day
    # they will.
    env['components']['sindri.environment']['lighting_2d']['clouds'] = {
        'cover': 0.45, 'size': 28.0, 'darkness': 0.4, 'wind': [1.1, 0.35]}
    return env


TEXTURES = Path(__file__).resolve().parent.parent / 'assets' / 'textures'
HAND = 0.95     # how high Wick carries his lantern
ROUNDER = 0.6   # a contact shadow's depth for its width: the camera foreshortens it already
GROUNDED = 0.7  # how dark a contact shadow is: what stands has to be seen to touch the ground
SPREAD = 1.5    # how much wider than the field's: its far half is hidden behind what stands on it
FLICKER = 0.6   # how restless a flame's light is


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
            # The grass parts round him, and his steps stay a while behind him.
            e['components']['sindri.wake'] = {'radius': 1.3, 'strength': 0.35}
            e['components']['sindri.particles#steps'] = scenes.motes(
                0.0, 7.0, 0.0, [[0.12, 0.09, 0.07, 0.45], [0.12, 0.09, 0.07, 0.3], [0.12, 0.09, 0.07, 0.0]],
                0.42, 0.42, texture='textures/footprints.png', per_distance=1.4, max=24, spread=0.0,
                shape='point', shape_size=[0.0, 0.0], gravity=[0.0, 0.0], drag=0.0, random_rotation=False,
                lifetime_spread=0.1, speed_spread=0.0, layer=-9)
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
            for name, part in components.items():
                if name.startswith('sindri.light2d') and not name.startswith('sindri.light2d.'):
                    part['flicker'] = FLICKER
            place['position'] = [place['position'][0], 0.0, HAND]
            continue
        if e.get('parent') is None:
            place['position'][2] = 0.0
        light = components.get('sindri.light2d')
        # A flame is never quite still.
        for name, part in components.items():
            if name.startswith('sindri.light2d') and not name.startswith('sindri.light2d.') \
                    and part.get('color', [0, 0, 0])[2] < 0.6 and 'flicker' not in part:
                part['flicker'] = FLICKER
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
    # The first one breathes 6.66 seconds out of step: it came in with pull request 666,
    # and something below the Lid should remember that.
    ents += [sunfall(f'sunfall-{n}', x, y, phase=6.66 * n) for n, (x, y) in enumerate(SUNFALLS, 1)]
    ents += [*placed_at_start(wick()), *effects(), *[gloom(f'gloom-{n}', x, y, size) for n, ((x, y), size) in enumerate(zip(GLOOM, [1, 3]), 1)]]
    return standing_up(ents)


def main():
    prefabs()
    scenes.write('wilds.scene', 'The Wilds', wilds())


if __name__ == '__main__':
    main()
