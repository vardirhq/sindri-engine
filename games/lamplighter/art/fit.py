"""Colliders fitted to the pictures they stand under.

A thing standing up in the wilds is blocked where it meets the ground, not
where its picture is: a tree by its trunk, so Wick can walk behind its crown; a
bush by its base; a boulder by its foot. `fitted` reads that from the
picture itself. It takes the lowest band of the drawn pixels, the columns
solidly drawn across it, and lays a footprint on the ground that wide: a
capsule on its side, or a circle when it is as deep as it is wide, shallower
than it is wide since the ground is seen from above at a slant. The light
occluder is the same footprint, so what stops Wick also casts the shadow.

Used by `wilds_scene.py` on every scattered thing that has a collider, so a
new picture gets its footprint by being drawn.
"""
import json
import math
from pathlib import Path

from PIL import Image

ASSETS = Path(__file__).resolve().parent.parent / 'assets'
# How much of the drawn height, from the bottom, is the base, and the fewest
# rows that makes; how much of the band a column must be drawn in to count, so
# a stray blade of grass does not widen it; how deep the footprint is for its
# width; and the smallest it may be, in world units.
BASE = 0.14
FEWEST_ROWS = 3
SOLID = 0.5
DEPTH = 0.45
SMALLEST = 0.12


def frame(reference):
    """The picture a sprite draws, its sheet's anchor, and its pixel size."""
    path, _, name = reference.partition('#')
    image = Image.open(ASSETS / path).convert('RGBA')
    sheet_file = (ASSETS / path).with_suffix('.sheet')
    sheet = json.loads(sheet_file.read_text()) if sheet_file.exists() else {}
    grid = sheet.get('grid', {'columns': 1, 'rows': 1, 'names': ['']})
    width, height = image.width // grid['columns'], image.height // grid['rows']
    index = grid['names'].index(name) if name else 0
    x, y = (index % grid['columns']) * width, (index // grid['columns']) * height
    return image.crop((x, y, x + width, y + height)), sheet.get('anchor', 'center')


def base_of(picture):
    """The base of what is drawn, in pixels: the middle of its solid columns
    from the picture's middle, their half width, how high the band's middle
    is above the drawn bottom, and that bottom's gap from the picture's own."""
    alpha = picture.getchannel('A')
    width, height = picture.size
    box = alpha.getbbox()
    if box is None:
        return None
    top, bottom = box[1], box[3] - 1
    rows = max(FEWEST_ROWS, round((bottom - top + 1) * BASE))
    band = range(max(top, bottom - rows + 1), bottom + 1)
    solid = [x for x in range(width)
             if sum(1 for y in band if alpha.getpixel((x, y))) >= SOLID * len(band)]
    if not solid:
        solid = [x for x in range(width) if any(alpha.getpixel((x, y)) for y in band)]
    left, right = min(solid), max(solid) + 1
    return ((left + right) / 2 - width / 2, (right - left) / 2, len(band) / 2, height - 1 - bottom)


def fitted(thing):
    """Refits `thing`'s collider and occluder to its sprite, in place. A thing
    without both, or whose sprite draws nothing, is left as it is."""
    parts = thing['components']
    sprite, collider = parts.get('sindri.sprite'), parts.get('sindri.physics2d.collider')
    if sprite is None or collider is None or 'shape' not in collider:
        return thing
    # Only what stands still: a creature's body is its own business.
    if parts.get('sindri.physics2d.rigid_body', {}).get('kind', 'static') != 'static':
        return thing
    picture, anchor = frame(sprite['texture'])
    found = base_of(picture)
    if found is None:
        return thing
    middle, half_wide, band_up, gap = found
    width_px, height_px = picture.size
    size = sprite.get('size', [1.0, 1.0])
    scale = thing.get('transform_3d', {}).get('scale', [1.0, 1.0, 1.0])
    local = [size[0] / width_px, size[1] / height_px]   # a pixel, in the entity's own units
    world = abs(scale[0]) * local[0]                   # a pixel across, in world units
    # Where the drawn bottom is, in the entity's own units: the picture's bottom
    # edge (by its anchor), moved by the sprite's offset, up by the gap.
    bottom = (0.0 if anchor == 'bottom' else -size[1] / 2) + sprite.get('offset', [0.0, 0.0])[1]
    foot = bottom + gap * local[1]
    half_wide_world = max(SMALLEST, half_wide * world)
    half_deep_world = max(SMALLEST * DEPTH, half_wide_world * DEPTH)
    centre = [middle * local[0], foot + band_up * local[1]]
    to_local = 1.0 / abs(scale[0])
    radius = half_deep_world * to_local
    reach = half_wide_world * to_local
    if reach - radius < 0.01 * to_local:
        collider['shape'] = {'shape': 'circle', 'radius': round(reach, 5)}
        collider['rotation'] = 0
    else:
        # A capsule stands along its own y; laid on its side, it runs across.
        collider['shape'] = {'shape': 'capsule', 'half_height': round(reach - radius, 5), 'radius': round(radius, 5)}
        collider['rotation'] = round(math.pi / 2, 6)
    collider['offset'] = [round(centre[0], 5), round(centre[1], 5)]
    if 'sindri.light2d.occluder' in parts:
        parts['sindri.light2d.occluder'] = {'shape': 'polygon', 'points': [
            [round(centre[0] + reach * math.cos(math.tau * n / 12), 5),
             round(centre[1] + radius * math.sin(math.tau * n / 12), 5)] for n in range(12)]}
    return thing
