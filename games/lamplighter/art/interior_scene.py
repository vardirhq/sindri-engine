#!/usr/bin/env python3
"""Builds `assets/interior-<house>.scene`, the room behind each of the village's four doors.

A room is `interior.py`'s shell (a wall face along the top, planks below, a doorway at the bottom)
with that house's furniture set against the walls and a fire or two for light. Wick comes in at the
doorway; the way out is the same door, and the village is as he left it, since a scene that is left
is switched off, not unloaded. What he carries (the run's count of lamps) is not reset, so his
`fresh` is off."""
import scenes
from scenes import at, camera, effects, entity, environment, input_actions, wall, wick
from village_scene import glow, standing

FURNITURE = 'textures/furniture.png'
CELL = 4.0           # a 64 pixel furniture cell is four world units across
ROOM = (14.0, 11.0)  # the shell, in world units: its wall face is the top three
WALLS = [  # name, x, y, width, height of what Wick cannot cross
    ('wall-back', 0.0, 4.0, 14.0, 3.0), ('wall-left', -6.5, 0.0, 1.0, 11.0), ('wall-right', 6.5, 0.0, 1.0, 11.0),
    ('wall-front-left', -4.0, -5.0, 6.0, 1.0), ('wall-front-right', 4.0, -5.0, 6.0, 1.0),
    ('doorstep', 0.0, -5.7, 2.0, 0.4)]
# Per house: the room's floor, its furniture (sprite, x, y, what blocks: rx, ry, height of the middle,
# or None) and its fires (x, y, radius, intensity), the first of which is the fireplace the room is lit by.
ROOMS = {
    'bakery': {'pieces': [('oven', 3.2, 2.1, (1.9, 0.7, 0.7)), ('cupboard', -3.8, 2.1, (0.9, 0.5, 0.5)),
                          ('table', -1.2, -0.5, (1.5, 0.9, 1.0)), ('chest', 4.4, -1.8, (1.6, 0.6, 0.6))],
               'fires': [(3.2, 1.0, 8.0, 2.4)]},
    'child-house': {'pieces': [('hearth', 0.0, 2.2, (1.6, 0.7, 0.8)), ('bed', -4.6, 1.6, (1.0, 1.3, 1.4)),
                               ('cradle', -2.2, -1.0, (1.6, 0.5, 0.5)), ('chest', 4.4, 0.6, (1.6, 0.6, 0.6)),
                               ('table', 2.4, -2.2, (1.5, 0.9, 1.0))],
                    'fires': [(0.0, 1.1, 8.0, 2.4)]},
    'workshop': {'pieces': [('anvil', 3.6, 2.0, (1.5, 0.8, 0.8)), ('shelf', -4.6, 2.1, (1.0, 0.5, 0.5)),
                            ('cupboard', -2.0, 2.1, (0.9, 0.5, 0.5)), ('table', -2.4, -1.4, (1.5, 0.9, 1.0)),
                            ('chest', 4.4, -1.8, (1.6, 0.6, 0.6))],
                 'fires': [(3.0, 1.0, 8.0, 2.4)]},
    'keeper': {'pieces': [('hearth', 0.0, 2.2, (1.6, 0.7, 0.8)), ('armchair', -3.4, 0.4, (1.5, 0.7, 0.7)),
                          ('shelf', -5.2, 2.1, (1.0, 0.5, 0.5)), ('shelf-2', 4.6, 2.1, (1.2, 0.5, 0.5)),
                          ('table', 3.0, -1.8, (1.5, 0.9, 1.0))],
               'fires': [(0.0, 1.1, 8.0, 2.4)]},
}
COLUMNS = {'bakery': 0, 'child-house': 1, 'workshop': 2, 'keeper': 3}


def room(name):
    spec = ROOMS[name]
    ents = [
        camera(0.0, -1.0, 'wick', size=10.5, confine=((-6.0, -4.0), (6.0, 3.0))),
        environment([0.15, 0.11, 0.11]),
        entity('physics', 'Physics', **{
            'sindri.physics2d.world': {'gravity': [0, 0], 'layers': ['wall', 'wick', 'lantern', 'target']},
            'sindri.input.actions': {'actions': input_actions()}}),
        entity('room', 'room', at(0.0, 0.0, -1.0, 14.0, 11.0), **{
            'sindri.sprite': {'texture': f'textures/room.png#{name}', 'layer': -5}}),
        entity('door-prompt', 'door-prompt', at(0, 0.12, 0, 1.0, 1.0), **{'sindri.ui.text': {
            'anchor': 'bottom', 'color': [1, 0.95, 0.6, 1], 'font': 'fonts/Inter.ttf', 'font_size': 0.04,
            'line_height': 0.048, 'layer': 100, 'text': '', 'bold': True, 'line_align': 'center',
            'shadow': {'offset': [0, -0.003], 'color': [0, 0, 0, 0.45], 'softness': 0.002}}}),
        entity('door-out', 'door-out', at(0.0, -4.9, 0.1), **{
            'sindri.script': {'source': 'scripts/door.decay', 'script': 'Door',
                              'properties': {'to': 'village.scene', 'words': 'E   Leave', 'radius': 1.1}}}),
        # The windows give the room a little of the night.
        glow('window-left', -3.5, 2.0, 6.0, 0.5, 1.0, (0.55, 0.7, 0.95)),
        glow('window-right', 3.5, 2.0, 6.0, 0.5, 1.0, (0.55, 0.7, 0.95)),
    ]
    for ident, x, y, w, h in WALLS:
        ents.append(wall(ident, x, y, w, h, [0.0, 0.0, 0.0, 0.0]))
    ents.append(entity('rug', 'rug', at(-0.2, -1.8, -0.5, CELL, CELL), **{
        'sindri.sprite': {'texture': f'{FURNITURE}#rug', 'layer': -3}}))
    for n, (sprite, x, y, block) in enumerate(spec['pieces'], 1):
        ents.append(standing(f'{sprite}-{n}', sprite, x, y, block, size=CELL, sheet=FURNITURE))
    for n, (x, y, radius, intensity) in enumerate(spec['fires'], 1):
        ents.append(glow(f'fire-{n}', x, y, radius, intensity, 0.8, (1.0, 0.62, 0.3)))
    ents += [*wick(fresh=False), *effects()]
    for e in ents:
        if e.get('id') == 'wick':
            e['transform_3d']['position'][:2] = [0.0, -3.6]
    return ents


def main():
    for name in ROOMS:
        scenes.write(f'interior-{name}.scene', f'The {name} room', room(name))


if __name__ == '__main__':
    main()
