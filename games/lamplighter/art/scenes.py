#!/usr/bin/env python3
"""Writes Lamplighter's scenes: the title, the pause screen and the dark field.

Deterministic, so CI can run it again and fail on any drift. Stage 1 draws with
shapes; stage 2 replaces them with the Pixel Lab tileset and Wick's sprites.
"""
import json
import pathlib

ASSETS = pathlib.Path(__file__).resolve().parent.parent / 'assets'
Q = [0, 0, 0, 1]
WALL, WICK, LANTERN = 1, 2, 4
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


def lantern(ident, x, y, goal=False):
    tags = ['lantern', 'goal'] if goal else ['lantern']
    return entity(ident, ident, at(x, y, 0.3, 0.5, 0.7), **{
        'sindri.shape': {'kind': 'rect', 'fill': [0.32, 0.28, 0.22, 1.0], 'layer': 3},
        'sindri.tags': {'tags': tags},
        'sindri.physics2d.collider': {'shape': {'shape': 'circle', 'radius': 1.0}, 'offset': [0, 0],
                                      'rotation': 0, 'sensor': True,
                                      'layers': {'memberships': LANTERN, 'filter': ALL},
                                      'friction': 0, 'restitution': 0},
        'sindri.light2d': {'enabled': True, 'color': [1.0, 0.72, 0.38, 1.0], 'intensity': 0.0, 'radius': 6.5,
                           'falloff': 1.4, 'height': 1.0, 'shadows': True, 'shadow_reach': 0.35,
                           'shadow_source_radius': 0.3},
        'sindri.script': {'source': 'scripts/lantern.decay', 'script': 'Lantern', 'properties': {}}})


def clips():
    """One idle and one walk clip for each direction Wick faces."""
    out = {}
    for direction in ('south', 'west', 'north', 'east'):
        out[f'idle-{direction}'] = {'frames': [f'idle-{direction}'], 'seconds_per_frame': 0.4, 'looping': True}
        out[f'walk-{direction}'] = {'frames': [f'walk-{direction}-{n}' for n in range(8)],
                                    'seconds_per_frame': 0.09, 'looping': True}
    return out


def wick():
    """Wick stands where the entity is: the sprite grows up from the feet."""
    body = {'kind': 'dynamic', 'pose': {'position': [0, 0], 'rotation': 0}, 'linear_velocity': [0, 0],
            'angular_velocity': 0, 'gravity_scale': 0, 'linear_damping': 0, 'angular_damping': 0,
            'lock_rotation': True, 'continuous_collision': True}
    piece = lambda radius, sensor, mask: {
        'shape': {'shape': 'circle', 'radius': radius}, 'offset': [0, 0.45], 'rotation': 0, 'sensor': sensor,
        'layers': {'memberships': WICK, 'filter': mask}, 'friction': 0, 'restitution': 0}
    return [
        entity('wick', 'Wick', at(0, 0, 0.5), **{
            'sindri.tags': {'tags': ['wick']},
            'sindri.physics2d.rigid_body': body,
            'sindri.physics2d.collider': {'pieces': [piece(0.4, False, WALL), piece(0.7, True, ALL)]},
            'sindri.script': {'source': 'scripts/wick.decay', 'script': 'Wick', 'properties': {}}}),
        # One pixel of the art is a sixteenth of a world unit.
        entity('wick-sprite', 'wick-sprite', at(0, 0, 0, 2.5, 2.75), 'wick', **{
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
        {'name': 'pause', 'kind': 'button', 'bindings': ['key.Escape', 'gamepad.start']}]
    stone = [0.16, 0.15, 0.2, 1.0]
    ents = [
        camera(0, 0, 'wick'),
        environment([0.02, 0.02, 0.06]),
        entity('physics', 'Physics', **{
            'sindri.physics2d.world': {'gravity': [0, 0], 'layers': ['wall', 'wick', 'lantern']},
            'sindri.input.actions': {'actions': actions}}),
        ground(),
        wall('edge-north', 0, 18.5, 54, 1, stone),
        wall('edge-south', 0, -18.5, 54, 1, stone),
        wall('edge-west', -26.5, 0, 1, 38, stone),
        wall('edge-east', 26.5, 0, 1, 38, stone),
        # A few stones to walk around, to give the carried light something to shadow.
        wall('stone-1', 5, 3, 2, 2, stone), wall('stone-2', 9, -4, 3, 1.5, stone),
        wall('stone-3', -6, -5, 1.5, 3, stone), wall('stone-4', 14, 5, 1.5, 1.5, stone),
        wall('stone-5', -10, 6, 2.5, 2, stone),
        lantern('lantern-1', 8, 0), lantern('lantern-2', 17, -3), lantern('lantern-3', 22, 8, goal=True),
        *wick(),
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
