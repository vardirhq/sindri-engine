#!/usr/bin/env python3
"""Writes Lamplighter's scenes: the title, the pause screen and the dark field.

Deterministic, so CI can run it again and fail on any drift. Stage 1 draws with
shapes; stage 2 replaces them with the Pixel Lab tileset and Wick's sprites.
"""
import json
import pathlib

ASSETS = pathlib.Path(__file__).resolve().parent.parent / 'assets'
Q = [0, 0, 0, 1]
WALL, WICK, LANTERN, TARGET = 1, 2, 4, 8
ALL = 4294967295


def at(x, y, z=0.0, sx=1.0, sy=1.0):
    return {'position': [x, y, z], 'rotation': Q, 'scale': [sx, sy, 1]}


def entity(ident, name, transform=None, parent=None, **components):
    out = {'id': ident, 'name': name}
    if transform:
        out['transform_3d'] = transform
    if parent:
        out['parent'] = parent
    out['components'] = components
    return out


def write(name, title, entities):
    doc = {'format_version': 10, 'metadata': {'name': title}, 'entities': entities}
    (ASSETS / name).write_text(json.dumps(doc, indent=2) + '\n')


def camera(x=0.0, y=0.0, follow=None):
    components = {'sindri.camera': {
        'projection': 'orthographic', 'vertical_size': 16.875, 'near': 0.1, 'far': 100.0, 'fit': 'height'}}
    if follow:
        components['sindri.camera.behavior'] = {
            'follow': {'target': follow, 'offset': [0, 1.2, 0], 'dead_zone': [0.5, 0.5],
                       'smoothing': 8, 'max_speed': 30},
            'confine': {'min': [-24, -16], 'max': [24, 16], 'contain_view': True}}
    return entity('camera', 'Camera', at(x, y, 10.0), **components)


def environment(ambient):
    return entity('environment', 'Environment', **{'sindri.environment': {
        'background': [0.02, 0.02, 0.05, 1.0],
        'shadows': {'enabled': False, 'map_size': 1024, 'distance': 48.0, 'bias': 0.002},
        'lighting_2d': {'enabled': True, 'ambient': ambient}}})


def text(ident, words, size, color, parent=None, bold=False):
    return entity(ident, ident, at(0, 0, 0, 1.4, size * 1.3), parent, **{'sindri.ui.text': {
        'anchor': 'center', 'color': color, 'font': 'fonts/Inter.ttf', 'font_size': size,
        'line_height': size * 1.2, 'layer': 100, 'text': words, 'bold': bold, 'line_align': 'center',
        'shadow': {'offset': [0.0, -0.004], 'color': [0, 0, 0, 0.55], 'softness': 0.003}}})


def button(ident, label):
    return [
        entity(ident, ident, at(0, 0, 0, 0.62, 0.095), 'menu', **{
            'sindri.ui.shape': {'kind': 'rect', 'fill': [0.09, 0.08, 0.2, 0.92], 'anchor': 'center', 'layer': 90},
            'sindri.ui.button': {'label': label, 'autofocus': False, 'disabled': False},
            'sindri.ui.layout': {'direction': 'row', 'spacing': 0.02, 'justify': 'center', 'align': 'center'}}),
        entity(ident + '-label', ident + '-label', at(0, 0, 0, 0.558, 0.076), ident, **{
            'sindri.ui.text': {'anchor': 'center', 'color': [0.96, 0.93, 0.84, 1.0], 'font': 'fonts/Inter.ttf',
                               'font_size': 0.05, 'line_height': 0.06, 'layer': 95, 'text': label, 'bold': True,
                               'line_align': 'center',
                               'shadow': {'offset': [0.0, -0.004], 'color': [0, 0, 0, 0.55], 'softness': 0.003}}}),
    ]


def menu(script, heading, subtitle, buttons):
    layout = entity('menu', 'menu', at(0, 0, 0, 1.0, 1.6), **{'sindri.ui.layout': {
        'direction': 'column', 'spacing': 0.035, 'justify': 'center', 'align': 'center'}})
    out = [camera(), environment([0.36, 0.34, 0.55]), layout,
           text('heading', heading, 0.11, [1.0, 0.82, 0.45, 1.0], 'menu', True),
           text('tagline', subtitle, 0.045, [0.62, 0.6, 0.78, 1.0], 'menu')]
    for ident, label in buttons:
        out += button(ident, label)
    out.append(entity('shell', 'Shell', **{'sindri.script': {
        'source': 'scripts/shell.decay', 'script': script, 'properties': {}}}))
    return out


def rect(ident, name, x, y, z, sx, sy, fill, layer, **extra):
    return entity(ident, name, at(x, y, z, sx, sy), **{
        'sindri.shape': {'kind': 'rect', 'fill': fill, 'layer': layer}, **extra})


def wall(ident, x, y, w, h, fill):
    """A solid block: it stops Wick and it stops light."""
    return rect(ident, ident, x, y, 0.2, w, h, fill, 2, **{
        'sindri.physics2d.rigid_body': {'kind': 'static', 'pose': {'position': [0, 0], 'rotation': 0},
                                        'linear_velocity': [0, 0], 'angular_velocity': 0, 'gravity_scale': 0,
                                        'linear_damping': 0, 'angular_damping': 0, 'lock_rotation': True,
                                        'continuous_collision': False},
        'sindri.physics2d.collider': {'shape': {'shape': 'box', 'half_extents': [0.5, 0.5]}, 'offset': [0, 0],
                                      'rotation': 0, 'sensor': False,
                                      'layers': {'memberships': WALL, 'filter': ALL},
                                      'friction': 0, 'restitution': 0},
        'sindri.light2d.occluder': {'shape': 'box', 'size': [1, 1]}})


def depth(y):
    """Higher on the screen is further away, so it is drawn first."""
    return round(0.5 - y * 0.01, 3)


def static_body():
    return {'kind': 'static', 'pose': {'position': [0, 0], 'rotation': 0}, 'linear_velocity': [0, 0],
            'angular_velocity': 0, 'gravity_scale': 0, 'linear_damping': 0, 'angular_damping': 0,
            'lock_rotation': True, 'continuous_collision': False}


def prop(ident, sprite, x, y, radius, centre, occluder, flip=False):
    """A tree or a boulder: it stands on its position, stops Wick and stops light.

    The radius, the height of the collider's middle above the foot and the
    occluder's size are in world units; the entity is scaled up from 16 art
    pixels to a unit, so each is divided by that scale for the component."""
    return entity(ident, ident, at(x, y, depth(y), -4 if flip else 4, 4), **{
        'sindri.sprite': {'texture': f'textures/objects.png#{sprite}', 'layer': 5},
        'sindri.physics2d.rigid_body': static_body(),
        'sindri.physics2d.collider': {'shape': {'shape': 'circle', 'radius': radius / 4}, 'offset': [0, centre / 4],
                                      'rotation': 0, 'sensor': False,
                                      'layers': {'memberships': WALL, 'filter': ALL},
                                      'friction': 0, 'restitution': 0},
        'sindri.light2d.occluder': {'shape': 'box', 'size': [occluder[0] / 4, occluder[1] / 4]}})


def lantern(ident, x, y, goal=False):
    """A lamp post: dark until Wick touches it, then it swaps to its lit sprite and
    its light, up at the lantern, flares."""
    tags = ['lantern', 'goal'] if goal else ['lantern']
    post = entity(ident, ident, at(x, y, depth(y), 4, 4), **{
        'sindri.sprite': {'texture': 'textures/objects.png#lamp-post-unlit', 'layer': 5},
        'sindri.animation.sprite': {'clips': {
            'unlit': {'frames': ['lamp-post-unlit'], 'seconds_per_frame': 0.1, 'looping': False},
            'lit': {'frames': ['lamp-post-lit'], 'seconds_per_frame': 0.1, 'looping': False}},
            'playing': 'unlit', 'speed': 1},
        'sindri.tags': {'tags': tags},
        'sindri.physics2d.collider': {'shape': {'shape': 'circle', 'radius': 1.3 / 4}, 'offset': [0, 0.6 / 4],
                                      'rotation': 0, 'sensor': True,
                                      'layers': {'memberships': LANTERN, 'filter': ALL},
                                      'friction': 0, 'restitution': 0},
        'sindri.script': {'source': 'scripts/lantern.decay', 'script': 'Lantern',
                          'properties': {'light_name': ident + '-light'}}})
    flame = entity(ident + '-light', ident + '-light', at(x, y + 2.6, depth(y) + 0.01), **{
        'sindri.light2d': {'enabled': True, 'color': [1.0, 0.72, 0.38, 1.0], 'intensity': 0.0, 'radius': 7.5,
                           'falloff': 1.4, 'height': 1.0, 'shadows': True, 'shadow_reach': 0.35,
                           'shadow_source_radius': 0.3}})
    return [post, flame]


def clips():
    """An idle, a walk and a slash clip for each direction Wick faces."""
    out = {}
    for direction in ('south', 'west', 'north', 'east'):
        out[f'idle-{direction}'] = {'frames': [f'idle-{direction}'], 'seconds_per_frame': 0.4, 'looping': True}
        out[f'walk-{direction}'] = {'frames': [f'walk-{direction}-{n}' for n in range(8)],
                                    'seconds_per_frame': 0.09, 'looping': True}
        out[f'slash-{direction}'] = {'frames': [f'slash-{direction}-{n}' for n in range(8)],
                                     'seconds_per_frame': 0.07, 'looping': False}
    return out


def gloom(ident, x, y):
    """A creature of the dark: dim, slow to notice, and cut down by the sword."""
    body = {'kind': 'dynamic', 'pose': {'position': [0, 0], 'rotation': 0}, 'linear_velocity': [0, 0],
            'angular_velocity': 0, 'gravity_scale': 0, 'linear_damping': 7, 'angular_damping': 0,
            'lock_rotation': True, 'continuous_collision': True}
    return entity(ident, ident, at(x, y, depth(y), 0.9, 0.9), **{
        'sindri.shape': {'kind': 'ellipse', 'fill': [0.12, 0.08, 0.2, 1.0], 'layer': 5},
        'sindri.tags': {'tags': ['gloom']},
        'sindri.physics2d.rigid_body': body,
        'sindri.physics2d.collider': {'shape': {'shape': 'circle', 'radius': 0.5}, 'offset': [0, 0],
                                      'rotation': 0, 'sensor': False,
                                      'layers': {'memberships': TARGET, 'filter': WALL | WICK | TARGET},
                                      'friction': 0, 'restitution': 0},
        'sindri.script': {'source': 'scripts/gloom.decay', 'script': 'Gloom', 'properties': {}}})


def wick():
    """Wick stands where the entity is: the sprite grows up from the feet."""
    body = {'kind': 'dynamic', 'pose': {'position': [0, 0], 'rotation': 0}, 'linear_velocity': [0, 0],
            'angular_velocity': 0, 'gravity_scale': 0, 'linear_damping': 0, 'angular_damping': 0,
            'lock_rotation': True, 'continuous_collision': True}
    piece = lambda radius, sensor, mask: {
        'shape': {'shape': 'circle', 'radius': radius}, 'offset': [0, 0.45], 'rotation': 0, 'sensor': sensor,
        'layers': {'memberships': WICK, 'filter': mask}, 'friction': 0, 'restitution': 0}
    return [
        entity('wick', 'Wick', at(0, 0, depth(0)), **{
            'sindri.tags': {'tags': ['wick']},
            'sindri.physics2d.rigid_body': body,
            'sindri.physics2d.collider': {'pieces': [piece(0.4, False, WALL | TARGET), piece(0.7, True, ALL)]},
            'sindri.script': {'source': 'scripts/wick.decay', 'script': 'Wick', 'properties': {}}}),
        # One pixel of the art is a sixteenth of a world unit.
        entity('wick-sprite', 'wick-sprite', at(0, 0, 0, 3.0, 2.75), 'wick', **{
            'sindri.sprite': {'texture': 'textures/wick.png#idle-south', 'layer': 5},
            'sindri.animation.sprite': {'clips': clips(), 'playing': 'idle-south', 'speed': 1}}),
        # The lantern's flame: a light a little above Wick's feet and out to one
        # side, so what it lights and the shadows it casts start at the lantern.
        entity('wick-light', 'wick-light', at(0, 1.6, 0.1), 'wick', **{
            'sindri.light2d': {'enabled': True, 'color': [1.0, 0.76, 0.42, 1.0], 'intensity': 1.3,
                               'radius': 8.0, 'falloff': 1.5, 'height': 1.0, 'shadows': True,
                               'shadow_reach': 0.3, 'shadow_source_radius': 0.3},
            'sindri.script': {'source': 'scripts/flame.decay', 'script': 'Flame', 'properties': {}}}),
    ]


COLUMNS, ROWS = 52, 36


def ground():
    """Plain grass. `ground.decay` paints the dirt path onto it when the world opens."""
    left, top = -COLUMNS // 2, ROWS // 2
    return entity('ground', 'Ground', at(left, top, -1.0), **{
        'sindri.tilemap': {
            'texture': 'textures/ground.png', 'palette': [f'ground-{i}' for i in range(16)],
            'columns': COLUMNS, 'rows': ROWS, 'tiles': [0] * (COLUMNS * ROWS), 'layer': -10},
        'sindri.script': {'source': 'scripts/ground.decay', 'script': 'Ground', 'properties': {}}})


BOULDERS = [(5, 3), (9, -4), (-6, -5), (16, 3), (-10, 6)]
TREES = [(-8, 9), (-14, 3), (-18, -7), (4, 10), (10, 11), (20, -9), (24, 3), (-3, -10), (0, -14),
         (14, -12), (-21, 11), (-22, -13), (26, 12), (-12, -3), (12, 5)]


def scenery():
    """Boulders to walk around and a dead wood: what the carried light shadows."""
    # Which of the five trees and four boulders stands where is fixed by its
    # number, and every other one is turned about, so a wood is not one tree.
    out = [prop(f'boulder-{n}', f'boulder-{(n * 3) % 4}', x, y, 0.9, 0.7, (1.8, 1.2), flip=n % 2 == 0)
           for n, (x, y) in enumerate(BOULDERS, 1)]
    out += [prop(f'tree-{n}', f'dead-tree-{(n * 2) % 5}', x, y, 0.6, 0.5, (1.2, 0.9), flip=n % 3 == 0)
            for n, (x, y) in enumerate(TREES, 1)]
    return out


def field():
    actions = [
        {'name': 'move_x', 'kind': 'axis', 'bindings': [
            {'axis': {'negative': 'key.A', 'positive': 'key.D'}},
            {'axis': {'negative': 'key.ArrowLeft', 'positive': 'key.ArrowRight'}},
            'gamepad.axis.left_x',
            {'axis': {'negative': 'gamepad.dpad_left', 'positive': 'gamepad.dpad_right'}}]},
        {'name': 'move_y', 'kind': 'axis', 'bindings': [
            {'axis': {'negative': 'key.S', 'positive': 'key.W'}},
            {'axis': {'negative': 'key.ArrowDown', 'positive': 'key.ArrowUp'}},
            'gamepad.axis.left_y',
            {'axis': {'negative': 'gamepad.dpad_down', 'positive': 'gamepad.dpad_up'}}]},
        {'name': 'attack', 'kind': 'button', 'bindings': ['key.J', 'key.Space', 'gamepad.west']},
        {'name': 'pause', 'kind': 'button', 'bindings': ['key.Escape', 'gamepad.start']}]
    stone = [0.02, 0.02, 0.04, 1.0]
    ents = [
        camera(0, 0, 'wick'),
        environment([0.02, 0.02, 0.06]),
        entity('physics', 'Physics', **{
            'sindri.physics2d.world': {'gravity': [0, 0], 'layers': ['wall', 'wick', 'lantern', 'target']},
            'sindri.input.actions': {'actions': actions}}),
        ground(),
        wall('edge-north', 0, 18.5, 54, 1, stone),
        wall('edge-south', 0, -18.5, 54, 1, stone),
        wall('edge-west', -26.5, 0, 1, 38, stone),
        wall('edge-east', 26.5, 0, 1, 38, stone),
        *scenery(),
        *lantern('lantern-1', 8, 0), *lantern('lantern-2', 17, -3), *lantern('lantern-3', 22, 8, goal=True),
        *wick(),
        gloom('gloom-1', 6.5, 0.45), gloom('gloom-2', -5, 0.45), gloom('gloom-3', 14, 6),
        entity('hud', 'HUD', at(0.03, -0.03, 0), **{
            'sindri.ui.text': {'anchor': 'top_left', 'color': [1, 0.95, 0.6, 1], 'font': 'fonts/Inter.ttf',
                               'font_size': 0.04, 'line_height': 0.048, 'layer': 100,
                               'text': 'Lanterns {}/{}', 'bold': True,
                               'shadow': {'offset': [0, -0.003], 'color': [0, 0, 0, 0.45], 'softness': 0.002}},
            'sindri.script': {'source': 'scripts/hud.decay', 'script': 'Hud', 'properties': {}}}),
    ]
    return ents


def main():
    write('title.scene', 'Lamplighter title', menu(
        'Title', 'Lamplighter', 'Keep the way home lit.', [('play', 'Play')]))
    write('pause.scene', 'Lamplighter pause', menu(
        'Pause', 'Paused', 'The light waits for you.', [('resume', 'Resume'), ('title', 'Title')]))
    write('world.scene', 'The Dim Fields', field())


if __name__ == '__main__':
    main()
