#!/usr/bin/env python3
"""Builds `assets/hold.scene` and its textures: the *Tern*, cabin, airlock and hold.

Stage 3 adds the cabin and the airlock chamber aft of the hold; the hold
itself stays as stage 1 left it, and is now the outside.


Stage 1 of `docs/drift-plan.md`, "Floating". A long box of a room in zero
gravity, lit by a few work lamps and one red emergency light, with yellow
handholds along its walls and a few loose crates drifting in it. The diver
starts holding the handhold at the aft end, looking down the hold at the
handhold on the forward bulkhead. The movement is all Decay
(`scripts/diver.decay`); this script only places things.

Run from anywhere: `python3 games/drift/art/hold.py`. Re-running it changes
nothing: the textures are drawn from fixed numbers and the scene is written
with sorted, fixed formatting."""
import json
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw

import ship as ships
import touch

HERE = Path(__file__).resolve().parent
ASSETS = HERE.parent / "assets"

# The hold, inside the walls: X runs aft (-) to forward (+).
LENGTH = 14.0
HEIGHT = 5.0
WIDTH = 5.0
WALL = 0.4
STAR_ROTATION = (-0.25, 0.35, 0.0, 0.9)
STAR_DISTANCE = 150.0
# Open space between the refuge's airlock and the damaged cargo section.
HOLD_OFFSET = 5.0

SOLID = {"memberships": 1, "filter": 4294967295}
DIVER = {"memberships": 2, "filter": 4294967295}
# The tether's links touch the hold and what is in it, never each other or the diver.
LINK = {"memberships": 4, "filter": 1}

# The tether: links between the aft handhold and the diver, as many segments
# as links plus one, each a rope that may slacken but never stretch.
TETHER_LENGTH = 10.0
TETHER_LINKS = 16
LINK_RADIUS = 0.08


# --- textures ---------------------------------------------------------------

def panel_texture(path: Path) -> None:
    """Grey hull plating: a grid of panels with seams, bolts and wear."""
    size = 256
    rng = random.Random(7)
    image = Image.new("RGB", (size, size), (92, 96, 102))
    draw = ImageDraw.Draw(image)
    for _ in range(1800):
        x, y = rng.randrange(size), rng.randrange(size)
        shade = rng.randint(-10, 10)
        draw.point((x, y), fill=(92 + shade, 96 + shade, 102 + shade))
    step = size // 2
    for i in range(0, size + 1, step):
        draw.line([(i, 0), (i, size)], fill=(48, 50, 54), width=3)
        draw.line([(0, i), (size, i)], fill=(48, 50, 54), width=3)
    for px in range(0, size, step):
        for py in range(0, size, step):
            for bx, by in ((10, 10), (step - 10, 10), (10, step - 10), (step - 10, step - 10)):
                draw.ellipse([px + bx - 3, py + by - 3, px + bx + 3, py + by + 3], fill=(130, 134, 140))
    image.save(path, optimize=True)


def grip_texture(path: Path) -> None:
    """Safety yellow with black chevrons, so a handhold reads from across the hold."""
    size = 64
    image = Image.new("RGB", (size, size), (230, 182, 32))
    draw = ImageDraw.Draw(image)
    for offset in range(-size, size * 2, 16):
        draw.polygon([(offset, 0), (offset + 8, 0), (offset + 8 - size, size), (offset - size, size)], fill=(26, 24, 20))
    image.save(path, optimize=True)


def line_texture(path: Path) -> None:
    """High-visibility orange webbing, so the tether reads against the dark."""
    size = 16
    image = Image.new("RGB", (size, size), (240, 110, 30))
    draw = ImageDraw.Draw(image)
    for y in range(0, size, 4):
        draw.line([(0, y), (size, y)], fill=(200, 80, 20))
    image.save(path, optimize=True)


def door_texture(path: Path) -> None:
    """A pressure door: dark plate with a yellow-and-black edge and a window slot."""
    size = 128
    image = Image.new("RGB", (size, size), (70, 76, 84))
    draw = ImageDraw.Draw(image)
    for offset in range(-size, size * 2, 20):
        draw.polygon([(offset, 0), (offset + 10, 0), (offset + 10 - 12, 12), (offset - 12, 12)], fill=(220, 170, 30))
        draw.polygon([(offset, size - 12), (offset + 10, size - 12), (offset - 2, size), (offset - 12, size)], fill=(220, 170, 30))
    draw.rectangle([44, 30, 84, 46], fill=(20, 26, 34))
    image.save(path, optimize=True)


def crate_texture(path: Path) -> None:
    """A stencilled orange cargo crate."""
    size = 128
    image = Image.new("RGB", (size, size), (176, 92, 40))
    draw = ImageDraw.Draw(image)
    draw.rectangle([0, 0, size - 1, size - 1], outline=(70, 40, 20), width=6)
    draw.line([(0, 0), (size, size)], fill=(120, 62, 28), width=8)
    draw.line([(size, 0), (0, size)], fill=(120, 62, 28), width=8)
    draw.rectangle([40, 52, 88, 76], fill=(232, 220, 200))
    image.save(path, optimize=True)


# --- entities ---------------------------------------------------------------

def box_piece(layers=SOLID, friction=0.6, restitution=0.1):
    return {
        "shape": {"shape": "box", "half_extents": [0.5, 0.5, 0.5]},
        "offset": [0, 0, 0],
        "rotation": [0, 0, 0, 1],
        "sensor": False,
        "layers": layers,
        "friction": friction,
        "restitution": restitution,
    }


def body(kind, gravity_scale=0.0, lock_rotation=False, linear=(0, 0, 0), angular=(0, 0, 0)):
    return {
        "kind": kind,
        "position": [0, 0, 0],
        "rotation": [0, 0, 0, 1],
        "linear_velocity": list(linear),
        "angular_velocity": list(angular),
        "gravity_scale": gravity_scale,
        "linear_damping": 0.0,
        "angular_damping": 0.0,
        "lock_rotation": lock_rotation,
    }


def entity(id_, name=None, position=(0, 0, 0), scale=(1, 1, 1), rotation=(0, 0, 0, 1), parent=None, **components):
    out = {
        "id": id_,
        "name": name or id_,
        "transform_3d": {
            "position": [round(v, 4) for v in position],
            "rotation": [round(v, 6) for v in rotation],
            "scale": [round(v, 4) for v in scale],
        },
        "components": components,
    }
    if parent:
        out["parent"] = parent
    return out


PANEL = 2.5   # roughly how big one plate of hull is, in metres


def wall(id_, position, scale):
    """A wall laid as plates about PANEL across, so the plating keeps its size.

    The texture stretches over a whole cube face, so a long wall made of one
    cube would show four plates the size of a room. Each plate is its own solid
    cube; together they close the wall."""
    thin = min(range(3), key=lambda i: scale[i])
    spans = [i for i in range(3) if i != thin]
    counts = [max(1, round(scale[i] / PANEL)) for i in spans]
    plates = []
    for a in range(counts[0]):
        for b in range(counts[1]):
            centre = list(position)
            size = list(scale)
            for axis, count, index in ((spans[0], counts[0], a), (spans[1], counts[1], b)):
                size[axis] = scale[axis] / count
                centre[axis] = position[axis] - scale[axis] / 2 + size[axis] * (index + 0.5)
            plates.append(entity(
                f"{id_}-{a}-{b}",
                position=centre,
                scale=size,
                **{
                    "sindri.mesh": {"primitive": "cube", "texture": "textures/panel.png"},
                    "sindri.material": {"base_color": [1, 1, 1, 1], "metallic": 0.3, "roughness": 0.7, "override_factors": True},
                    "sindri.physics3d.collider": {"pieces": [box_piece()]},
                },
            ))
    return plates


def handhold(id_, position, along, length=0.7):
    """A grab bar standing a little proud of a surface, long along `along`."""
    scale = [0.12, 0.12, 0.12]
    scale["xyz".index(along)] = length
    return entity(
        id_,
        position=position,
        scale=scale,
        **{
            "sindri.mesh": {"primitive": "cube", "texture": "textures/grip.png"},
            "sindri.material": {"base_color": [1, 1, 1, 1], "metallic": 0.1, "roughness": 0.5,
                                "emissive": [0.08, 0.06, 0.0], "override_factors": True},
            "sindri.physics3d.collider": {"pieces": [box_piece()]},
            "sindri.tags": {"tags": ["handhold"]},
        },
    )


def crate(id_, position, size, spin):
    return entity(
        id_,
        position=position,
        scale=(size, size, size),
        **{
            "sindri.mesh": {"primitive": "cube", "texture": "textures/crate.png"},
            "sindri.material": {"base_color": [1, 1, 1, 1], "metallic": 0.0, "roughness": 0.8, "override_factors": True},
            "sindri.physics3d.collider": {"pieces": [box_piece(restitution=0.3)]},
            "sindri.physics3d.rigid_body": body("dynamic", angular=spin),
            "sindri.tags": {"tags": ["cargo"]},
        },
    )


def lamp(id_, position, color, intensity, range_):
    return entity(
        id_,
        position=position,
        scale=(0.5, 0.08, 0.3),
        **{
            "sindri.mesh": {"primitive": "cube", "texture": "textures/panel.png"},
            "sindri.material": {"base_color": [0.2, 0.2, 0.2, 1], "emissive": [c * 2.0 for c in color], "override_factors": True},
            "sindri.light": {"kind": "point", "color": list(color), "intensity": intensity, "range": range_},
        },
    )


def text(id_, position, words, size, color=(0.85, 0.92, 1.0, 1.0)):
    return entity(
        id_,
        position=position,
        **{
            "sindri.ui.text": {
                "anchor": "center",
                "color": list(color),
                "font": "fonts/ChakraPetch-Regular.ttf",
                "font_size": size,
                "line_height": size * 1.2,
                "layer": 200,
                "text": words,
                "letter_spacing": 0.004,
            }
        },
    )


def tether(anchor, diver):
    """The safety line, coiled loosely between the handhold and the diver.

    Each link is a small body that the hold's walls and props stop; each
    segment between two of them is an owned `sindri.physics3d.distance_joint`.
    The strands are only drawn: `scripts/tether.decay` lays each one between
    its two links every frame."""
    segment = TETHER_LENGTH / (TETHER_LINKS + 1)
    out = [entity("tether", "Tether", **{
        "sindri.script": {"source": "scripts/tether.decay", "script": "Tether",
                          "properties": {"links": TETHER_LINKS, "length": TETHER_LENGTH}},
    })]
    names = ["start-hold"] + [f"tether-link-{i}" for i in range(TETHER_LINKS)] + ["diver"]
    for i in range(TETHER_LINKS):
        # A loose coil in the space between the handhold and the diver.
        turn = i * 0.9
        position = (anchor[0] + 0.25 + 0.02 * i, 0.25 * math.sin(turn), 0.25 * math.cos(turn))
        link = entity(
            names[i + 1], parent="tether", position=position,
            # Unscaled and undrawn: a collider takes its entity's scale, and the
            # strands already draw the line.
            **{
                "sindri.physics3d.collider": {"pieces": [{
                    "shape": {"shape": "sphere", "radius": LINK_RADIUS},
                    "offset": [0, 0, 0], "rotation": [0, 0, 0, 1], "sensor": False,
                    "layers": LINK, "friction": 0.3, "restitution": 0.0,
                }]},
                "sindri.physics3d.rigid_body": dict(body("dynamic"), linear_damping=1.0, angular_damping=1.0),
                "sindri.tags": {"tags": ["tether-link"]},
            },
        )
        out.append(link)
    for i in range(TETHER_LINKS + 1):
        # The diver's end starts unclipped: they clip on at the loose end.
        joint = {"first": names[i], "second": names[i + 1], "enabled": i < TETHER_LINKS,
                 "max_distance": round(segment, 4), "first_anchor": [0, 0, 0], "second_anchor": [0, 0, 0]}
        out.append({"id": f"tether-joint-{i}", "name": f"tether-joint-{i}", "parent": "tether",
                    "components": {"sindri.physics3d.distance_joint": joint,
                                   "sindri.tags": {"tags": ["tether-joint"]}}})
        out.append(entity(
            f"tether-strand-{i}", parent="tether", scale=(0.015, 0.015, segment),
            **{"sindri.mesh": {"primitive": "cube", "texture": "textures/line.png"},
               "sindri.tags": {"tags": ["tether-strand"]}},
        ))
    return out


# The ship aft of the hold, inside its walls. The hold is outside: vacuum and
# no gravity. The airlock chamber joins it to the cabin, where the ship's
# gravity holds. Every door is the same opening: across z, up from the deck.
DOOR_Z = (0.9, 2.1)
DOOR_TOP = -0.5
DECK = -2.5
CHAMBER = ((-9.6, DECK, 0.6), (-7.4, -0.1, 2.4))
CABIN = ((-16.0, DECK, -1.0), (-10.0, 0.1, 2.4))
# The diver's capsule is 1.6 m tall; the eye sits 0.6 m above its middle.
EYE_HEIGHT = 0.6
SPAWN = (-14.2, DECK + 0.82, sum(DOOR_Z) / 2)
# Where a test puts the diver to start at the aft handhold, eye level with it.
HOLD_START = (-6.3, -EYE_HEIGHT, 0.0)


# The cargo hatch through the forward bulkhead, and what lies beyond it.
HATCH_Y = (-1.5, 1.5)
HATCH_Z = (-1.5, 1.5)
# The derelict module: where it floats and how it lies.
WRECK_AT = (24.0, 0.5, 3.0)
WRECK_YAW = 14.0


def wall_with_opening(id_, lo, hi, y_range, z_range):
    """A wall across x with a rectangular opening through it."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    pieces = []
    pieces += slab(f"{id_}-port", (x0, y0, z0), (x1, y1, z_range[0]))
    pieces += slab(f"{id_}-starboard", (x0, y0, z_range[1]), (x1, y1, z1))
    pieces += slab(f"{id_}-top", (x0, y_range[1], z_range[0]), (x1, y1, z_range[1]))
    pieces += slab(f"{id_}-bottom", (x0, y0, z_range[0]), (x1, y_range[0], z_range[1]))
    return pieces


def model(id_, asset, position, rotation=(0, 0, 0, 1), parent=None, **components):
    """An imported model, by its path under `assets/`."""
    return entity(id_, position=position, rotation=rotation, parent=parent,
                  **{"sindri.model": {"asset": f"models/{asset}.glb"}}, **components)


def quat(axis, degrees):
    half = math.radians(degrees) / 2
    length = math.sqrt(sum(a * a for a in axis)) or 1.0
    x, y, z = (a / length * math.sin(half) for a in axis)
    return (x, y, z, math.cos(half))


def star_position():
    """Place the visible star opposite the directional light's -Z ray."""
    length = math.sqrt(sum(value * value for value in STAR_ROTATION))
    x, y, z, w = (value / length for value in STAR_ROTATION)
    return tuple(STAR_DISTANCE * value for value in (
        2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)))


def convex(asset, restitution=0.2):
    return {"asset": f"models/{asset}.glb", "friction": 0.5, "restitution": restitution,
            "layers": SOLID, "sensor": False}


def ruined_frame():
    """A broken open-sided extension, built from the same half-metre blocks."""
    blocks = {}
    for x in range(36):
        # Interrupted deck and long rails make the scale legible from afar.
        if x not in range(13, 19):
            for z in range(8):
                blocks[(x, 0, z)] = "deck"
        for z in (0, 7):
            blocks[(x, 1, z)] = "frame"
        # Ribs stop at different heights where the structure has broken apart.
        if x in (0, 6, 12, 21, 27, 33):
            height = 8 if x in (0, 6, 27) else 5
            for y in range(2, height):
                for z in (0, 7):
                    blocks[(x, y, z)] = "frame"
            if height == 8:
                for z in range(1, 7):
                    blocks[(x, 7, z)] = "frame"
    edits = [{"at": [x, y, z], "block": block}
             for (x, y, z), block in sorted(blocks.items())]
    return entity(
        "ruin-frame", "Broken structural frame", position=(14.0, -4.0, 7.0),
        scale=(0.5, 0.5, 0.5), **{
            "sindri.voxel_world": {"generator": {"kind": "built"}, "blocks": "blocks.tileset",
                                   "edits": edits, "focus": [0, 0, 0],
                                   "render_radius": 2, "vertical_radius": 1, "layer": 0},
            "sindri.physics3d.voxel_body": {"friction": 0.6, "restitution": 0.1, "layers": SOLID},
        })


def wreckage():
    """Outside the hatch: the derelict, a field of debris, salvage, the stars."""
    rng = random.Random(23)
    out = [model("stars", "stars", (0, 0, 0)),
           model("local-star", "local-star", star_position()), ruined_frame()]
    out.append(model("wreck", "wreck", WRECK_AT, quat((0, 1, 0), WRECK_YAW), **{
        "sindri.physics3d.model_collider": {"asset": "models/wreck.glb", "friction": 0.6, "restitution": 0.1,
                                            "layers": SOLID, "sensor": False},
    }))
    out[-1]["name"] = "Wreck"
    # A red lamp still burning in the wreck, and a work light on the hatch rim.
    out.append(entity("wreck-lamp", "WreckLamp", position=(WRECK_AT[0] + 0.5, WRECK_AT[1] - 1.6, WRECK_AT[2]),
                      **{"sindri.light": {"kind": "point", "color": [1.0, 0.2, 0.1], "intensity": 1.6, "range": 7.0}}))
    out.append(lamp("hatch-lamp", (7.0, HATCH_Y[1] + 0.3, 0.0), (0.85, 0.9, 1.0), 1.4, 9.0))
    # Debris: plates and girders drifting and turning slowly, clear of the
    # hatch and of the wreck itself.
    placed = 0
    while placed < 22:
        at = (rng.uniform(10.0, 36.0), rng.uniform(-6.0, 6.0), rng.uniform(-8.0, 9.0))
        near_wreck = all(abs(at[i] - WRECK_AT[i]) < (5.5, 3.6, 3.6)[i] for i in range(3))
        near_hatch = at[0] < 13.0 and abs(at[1]) < 3.0 and abs(at[2]) < 3.0
        if near_wreck or near_hatch:
            continue
        kind = "girder" if placed % 3 == 0 else "plate"
        spin = [round(rng.uniform(-0.25, 0.25), 3) for _ in range(3)]
        drift = [round(rng.uniform(-0.05, 0.05), 3) for _ in range(3)]
        axis = (rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-1, 1))
        out.append(model(f"debris-{placed}", kind, at, quat(axis, rng.uniform(0, 180)), **{
            "sindri.physics3d.model_convex_collider": convex(kind),
            "sindri.physics3d.rigid_body": body("dynamic", linear=drift, angular=spin),
            "sindri.tags": {"tags": ["debris"]},
        }))
        placed += 1
    # Salvage: one just outside the hatch, one by the wreck, one inside it.
    for i, at in enumerate(((11.5, -0.8, 0.6),
                            (WRECK_AT[0] - 5.2, WRECK_AT[1] + 1.8, WRECK_AT[2] - 1.0),
                            (WRECK_AT[0] + 1.2, WRECK_AT[1] - 0.6, WRECK_AT[2]))):
        out.append(model(f"salvage-{i}", "canister", at, quat((1, 0, 1), 25 * i), **{
            "sindri.physics3d.model_convex_collider": convex("canister", 0.3),
            "sindri.physics3d.rigid_body": body("dynamic", angular=(0.0, 0.15, 0.05)),
            "sindri.tags": {"tags": ["salvage"]},
        }))
    # A shuttle of half-metre blocks, parked off to port: one rigid body.
    edits = ships.shuttle()
    out.append(entity(
        "shuttle", "Shuttle", position=(13.0, -2.0, -17.0), scale=(0.5, 0.5, 0.5),
        **{
            "sindri.voxel_world": {
                "generator": {"kind": "built"}, "blocks": "blocks.tileset", "edits": edits,
                "focus": [0, 0, 0], "render_radius": 1, "vertical_radius": 1, "layer": 0,
            },
            "sindri.physics3d.voxel_body": {"friction": 0.6, "restitution": 0.1, "layers": SOLID},
            "sindri.physics3d.rigid_body": body("dynamic", angular=(0.0, 0.01, 0.0)),
            # Its gravity holds inside, measured in its own blocks; it is open
            # at both ends, so there is no air.
            "sindri.tags": {"tags": ["hull"]},
            "sindri.script": hull_script(min_x=1.0, max_x=23.0, min_y=1.0, max_y=7.0,
                                         min_z=1.0, max_z=9.0, air=False),
        },
    ))
    for index, x in enumerate((2.5, 6.5, 10.5, 14.5, 18.5)):
        out.append(entity(
            f"shuttle-fittings-light-{index}", "Shuttle work light",
            parent="shuttle", position=(x, 6.72, 5.0),
            **{"sindri.light": {"kind": "point", "color": [1.0, 0.79, 0.53],
                               "intensity": 2.1, "range": 3.8}},
        ))
    out += shuttle_parts()
    out.append(entity("salvage", "Salvage", **{
        "sindri.script": {"source": "scripts/salvage.decay", "script": "Salvage"}}))
    return out


def shuttle_parts():
    """The shuttle's thrusters, their flames and its pilot's seat and controls.

    Parts are children of the ship, placed in its blocks and scaled back up to
    a metre to the block. The pilot script lives on its own entity so the
    shuttle's own script stays its `Hull`."""
    out = []
    for i, (at, facing) in enumerate(ships.shuttle_thrusters()):
        out.append(entity(
            f"shuttle-thruster-{i}", parent="shuttle", position=at, rotation=ships.FACING[facing],
            scale=(2.0, 2.0, 2.0),
            **{"sindri.model": {"asset": "models/thruster.glb"},
               "sindri.tags": {"tags": ["shuttle-thruster"]}},
        ))
        flame = entity(
            f"shuttle-flame-{i}", parent=f"shuttle-thruster-{i}",
            **{"sindri.model": {"asset": "models/flame.glb"},
               "sindri.tags": {"tags": ["shuttle-flame"]}},
        )
        flame["disabled"] = True
        out.append(flame)
    out.append(entity(
        "shuttle-seat", "Seat", parent="shuttle", position=ships.SHUTTLE_SEAT,
        rotation=ships.FACING["-x"], scale=(2.0, 2.0, 2.0),
        **{"sindri.model": {"asset": "models/seat.glb"}},
    ))
    out.append(entity("shuttle-pilot", "Pilot", **{
        "sindri.script": {"source": "scripts/pilot.decay", "script": "Pilot",
                          "properties": {}},
    }))
    return out


def slab(id_, lo, hi):
    """A solid plated box from its two corners."""
    centre = tuple((a + b) / 2 for a, b in zip(lo, hi))
    size = tuple(b - a for a, b in zip(lo, hi))
    return wall(id_, centre, size)


def wall_with_door(id_, lo, hi):
    """A wall across x with the door opening cut out of it."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    pieces = []
    pieces += slab(f"{id_}-port", (x0, y0, z0), (x1, y1, DOOR_Z[0]))
    pieces += slab(f"{id_}-starboard", (x0, y0, DOOR_Z[1]), (x1, y1, z1))
    pieces += slab(f"{id_}-lintel", (x0, DOOR_TOP, DOOR_Z[0]), (x1, y1, DOOR_Z[1]))
    if y0 < DECK:
        pieces += slab(f"{id_}-sill", (x0, y0, DOOR_Z[0]), (x1, DECK, DOOR_Z[1]))
    return pieces


def hull_script(**properties):
    """A `Hull` (`scripts/hull.decay`): an interior the diver can be aboard."""
    return {"source": "scripts/hull.decay", "script": "Hull", "properties": properties}


def zone(id_, bounds, airlock=False):
    """An invisible box in the world that is a hull: the ship's gravity and
    air inside it. The chamber's air follows the airlock's pressure."""
    lo, hi = bounds
    centre = tuple((a + b) / 2 for a, b in zip(lo, hi))
    size = tuple(b - a for a, b in zip(lo, hi))
    return entity(id_, position=centre, scale=size, **{
        "sindri.tags": {"tags": ["zone", "hull"]},
        "sindri.script": hull_script(airlock=airlock),
    })


def door(id_, x):
    """A sliding pressure door filling the opening at `x`; scripts move it."""
    return entity(
        id_, position=(x, (DECK + DOOR_TOP) / 2, sum(DOOR_Z) / 2),
        scale=(0.15, DOOR_TOP - DECK, DOOR_Z[1] - DOOR_Z[0]),
        **{
            "sindri.mesh": {"primitive": "cube", "texture": "textures/door.png"},
            "sindri.material": {"base_color": [1, 1, 1, 1], "metallic": 0.4, "roughness": 0.5, "override_factors": True},
            "sindri.physics3d.collider": {"pieces": [box_piece()]},
        },
    )


def panel(id_, position):
    """An airlock control: a lit plate the diver presses E at."""
    return entity(
        id_, position=position, scale=(0.08, 0.3, 0.22),
        **{
            "sindri.mesh": {"primitive": "cube", "texture": "textures/panel.png"},
            "sindri.material": {"base_color": [0.15, 0.2, 0.18, 1], "emissive": [0.2, 0.9, 0.5],
                                "override_factors": True},
            "sindri.tags": {"tags": ["airlock-panel"]},
        },
    )


def furniture(id_, lo, hi, color, emissive=(0, 0, 0)):
    """A solid piece of the cabin's fittings."""
    centre = tuple((a + b) / 2 for a, b in zip(lo, hi))
    size = tuple(b - a for a, b in zip(lo, hi))
    return entity(
        id_, position=centre, scale=size,
        **{
            "sindri.mesh": {"primitive": "cube", "texture": "textures/panel.png"},
            "sindri.material": {"base_color": [*color, 1.0], "emissive": list(emissive),
                                "metallic": 0.2, "roughness": 0.7, "override_factors": True},
            "sindri.physics3d.collider": {"pieces": [box_piece()]},
        },
    )


def ship():
    """The cabin and the airlock chamber, aft of the hold."""
    out = []
    w = WALL
    (cx0, cy0, cz0), (cx1, cy1, cz1) = CHAMBER
    # The chamber: deck, overhead and two side walls; the bulkheads close its ends.
    out += slab("chamber-deck", (cx0, cy0 - w, cz0 - w), (cx1, cy0, cz1 + w))
    out += slab("chamber-overhead", (cx0, cy1, cz0 - w), (cx1, cy1 + w, cz1 + w))
    out += slab("chamber-port", (cx0, cy0, cz0 - w), (cx1, cy1, cz0))
    out += slab("chamber-starboard", (cx0, cy0, cz1), (cx1, cy1, cz1 + w))
    # The outer bulkhead belongs to the refuge, even after the hold breaks off.
    out += wall_with_door("outer-bulkhead", (cx1, cy0 - w, cz0 - w),
                          (cx1 + w, cy1 + w, cz1 + w))
    (kx0, ky0, kz0), (kx1, ky1, kz1) = CABIN
    # The inner bulkhead between cabin and chamber, with the inner door through it.
    out += wall_with_door("inner-bulkhead", (kx1, ky0 - w, kz0 - w), (cx0, ky1 + w, kz1 + w))
    # The cabin: deck, overhead, both sides and the aft wall.
    out += slab("cabin-deck", (kx0 - w, ky0 - w, kz0 - w), (kx1, ky0, kz1 + w))
    out += slab("cabin-overhead", (kx0 - w, ky1, kz0 - w), (kx1, ky1 + w, kz1 + w))
    out += slab("cabin-port", (kx0 - w, ky0, kz0 - w), (kx1, ky1, kz0))
    out += slab("cabin-starboard", (kx0 - w, ky0, kz1), (kx1, ky1, kz1 + w))
    out += slab("cabin-aft", (kx0 - w, ky0 - w, kz0 - w), (kx0, ky1 + w, kz1 + w))
    # Fittings: a bunk along the port side, a console and the oxygen recycler.
    out += [
        furniture("bunk", (kx0, DECK, kz0), (kx0 + 2.0, DECK + 0.55, kz0 + 0.8), (0.35, 0.4, 0.5)),
        furniture("console", (kx0 + 3.5, DECK, kz0), (kx0 + 4.7, DECK + 1.0, kz0 + 0.6),
                  (0.25, 0.28, 0.3), (0.05, 0.25, 0.35)),
        furniture("recycler", (kx0, DECK, kz1 - 0.9), (kx0 + 0.9, DECK + 1.8, kz1),
                  (0.4, 0.42, 0.38), (0.4, 0.08, 0.02)),
    ]
    out[-1]["name"] = "Recycler"
    # Doors, the airlock and its controls: one in the chamber and one by each door.
    out += [
        door("inner-door", (kx1 + cx0) / 2),
        door("outer-door", cx1 + w / 2),
        entity("airlock", "Airlock", **{
            "sindri.script": {"source": "scripts/airlock.decay", "script": "Airlock"}}),
        panel("airlock-panel", ((cx0 + cx1) / 2, -1.1, cz1 - 0.05)),
        # The chamber's own display, above its panel: pressure, the doors and
        # what the airlock is doing, read on the wall rather than the screen.
        entity("airlock-display", "airlock-display", position=((cx0 + cx1) / 2, -0.55, cz1 - 0.06),
               rotation=quat((0.0, 1.0, 0.0), 180),
               **{"sindri.text": {"text": "", "anchor": "center", "color": [0.55, 1.0, 0.7, 1.0],
                                  "font": "fonts/ChakraPetch-Regular.ttf", "font_size": 0.07,
                                  "line_height": 0.085, "layer": 30, "follow_rotation": True}}),
        panel("cabin-panel", (kx1 - 0.05, -1.1, DOOR_Z[1] + 0.3)),
        panel("hold-panel", (cx1 + w + 0.05, -1.1, DOOR_Z[0] - 0.3)),
        zone("cabin-zone", CABIN),
        # The chamber's air and gravity reach through the inner doorway to the
        # cabin's, so walking between them never passes through neither.
        zone("chamber-zone", ((CABIN[1][0], CHAMBER[0][1], CHAMBER[0][2]), CHAMBER[1]), airlock=True),
        entity("spawn", "Spawn", position=SPAWN),
    ]
    out[-3]["name"] = "CabinZone"
    out[-2]["name"] = "ChamberZone"
    # Light: warm in the cabin; the chamber's lamp shows its pressure.
    out += [
        lamp("cabin-lamp-0", (kx0 + 1.5, ky1 - 0.05, (kz0 + kz1) / 2), (1.0, 0.8, 0.6), 2.5, 5.0),
        lamp("cabin-lamp-1", (kx1 - 1.5, ky1 - 0.05, (kz0 + kz1) / 2), (1.0, 0.8, 0.6), 2.5, 5.0),
        lamp("airlock-lamp", ((cx0 + cx1) / 2, cy1 - 0.05, (cz0 + cz1) / 2), (0.3, 1.0, 0.45), 2.5, 3.5),
    ]
    out[-1]["name"] = "AirlockLamp"
    return out


def yaw_quat(radians):
    return (0.0, math.sin(radians / 2), 0.0, math.cos(radians / 2))


def scene():
    half_l, half_h, half_w = LENGTH / 2, HEIGHT / 2, WIDTH / 2
    t = WALL / 2
    outer_l, outer_h, outer_w = LENGTH + 2 * WALL, HEIGHT + 2 * WALL, WIDTH + 2 * WALL
    entities = [
        entity("physics", "Physics", **{"sindri.physics3d.world": {"gravity": [0, 0, 0]}}),
        entity("environment", "Environment", **{"sindri.environment": {
            "background": [0.004, 0.005, 0.009, 1.0],
            "ambient_color": [0.55, 0.65, 0.85],
            # A small fill preserves silhouettes without washing out shadows.
            "ambient_intensity": 0.035,
            "bloom": {"enabled": True, "intensity": 0.6, "knee": 0.2, "passes": 4, "threshold": 0.8},
            # A near star: hard light and hard shadow, the way sunlight is in vacuum.
            "shadows": {"enabled": True, "map_size": 2048, "distance": 40.0, "bias": 0.002},
        }}),
        entity("sun", "Starlight", rotation=STAR_ROTATION,
               **{"sindri.light": {"kind": "directional", "color": [1.0, 0.96, 0.9], "intensity": 1.3}}),
    ]
    # The hold's six walls.
    # The aft bulkhead, with the airlock's outer door through it.
    entities += wall_with_door("aft-bulkhead", (-half_l - WALL, -half_h - WALL, -half_w - WALL),
                               (-half_l, half_h + WALL, half_w + WALL))
    # The forward bulkhead is the cargo hatch, open to space.
    entities += wall_with_opening("forward-bulkhead", (half_l, -half_h - WALL, -half_w - WALL),
                                  (half_l + WALL, half_h + WALL, half_w + WALL), HATCH_Y, HATCH_Z)
    entities += wall("deck", (0, -half_h - t, 0), (outer_l, WALL, outer_w))
    entities += wall("overhead", (0, half_h + t, 0), (outer_l, WALL, outer_w))
    entities += wall("port-wall", (0, 0, -half_w - t), (outer_l, outer_h, WALL))
    entities += wall("starboard-wall", (0, 0, half_w + t), (outer_l, outer_h, WALL))
    entities += [
        # Handholds: one on each bulkhead at the diver's height, and a run down each side.
        handhold("start-hold", (-half_l + 0.12, 0, 0), "z"),
        # A safety bar across the open hatch: the last thing to hold before outside.
        handhold("far-hold", (half_l - 0.12, 0, 0), "z", HATCH_Z[1] - HATCH_Z[0]),
    ]
    entities[-2]["name"] = "StartHold"
    entities[-1]["name"] = "FarHold"
    # The bar crosses the middle of the way out, so it is not solid: a hand
    # closes on it by reach, and a diver flying out passes it rather than
    # stopping dead against it.
    del entities[-1]["components"]["sindri.physics3d.collider"]
    for i, x in enumerate((-4.0, 0.0, 4.0)):
        entities.append(handhold(f"port-hold-{i}", (x, 0.6, -half_w + 0.12), "x"))
        entities.append(handhold(f"starboard-hold-{i}", (x, -0.6, half_w - 0.12), "x"))
        entities.append(handhold(f"overhead-hold-{i}", (x + 2.0, half_h - 0.12, 0), "z"))
    # Loose cargo, drifting and turning slowly, clear of the hold's centre line.
    entities += [
        crate("crate-a", (-1.5, 1.4, 1.3), 0.8, (0.05, 0.12, 0.0)),
        crate("crate-b", (2.5, -1.5, -1.2), 1.0, (0.0, -0.06, 0.08)),
        crate("crate-c", (4.8, 1.6, -1.4), 0.6, (0.1, 0.0, -0.05)),
    ]
    # Work lamps along the overhead, and the emergency light over the forward bulkhead.
    for i, x in enumerate((-4.5, 0.0, 4.5)):
        entities.append(lamp(f"lamp-{i}", (x, half_h - 0.05, 0), (1.0, 0.86, 0.66), 1.4, 7.0))
    entities.append(lamp("emergency", (half_l - 0.05, 1.8, 0), (1.0, 0.12, 0.08), 1.6, 6.0))
    entities[-1]["transform_3d"]["scale"] = [0.08, 0.3, 0.5]
    # The old cargo hold is a separate, unpressurized piece of the ruin. Its
    # walls and fittings move together, leaving open space beyond the airlock.
    for part in entities[3:]:
        if "transform_3d" in part:
            part["transform_3d"]["position"][0] += HOLD_OFFSET
    entities += ship()
    # Severed services are visual landmarks across the gap, not a solid bridge.
    # Keep the middle of the airlock approach clear for a drifting diver.
    for index, (y, z) in enumerate(((1.8, -1.9), (2.0, -1.6), (-1.9, -2.0))):
        entities.append(entity(
            f"ruin-cable-{index}", "Severed service cable",
            position=(-4.9, y, z), scale=(5.0, 0.12, 0.12),
            **{"sindri.mesh": {"primitive": "cube", "texture": "textures/line.png"},
               "sindri.material": {"base_color": [0.25, 0.21, 0.17, 1.0],
                                   "metallic": 0.35, "roughness": 0.8, "override_factors": True}},
        ))
    # The guide line: a service cable still strung from the refuge's outer door
    # frame into the ruin's, at hand height beside the doorway. With no
    # thruster pack, it is the way across: hold it and pull hand over hand.
    # Not solid, so a diver drifting through the doorway is not stopped by it.
    door_z = DOOR_Z[0] - 0.12
    start, end = CHAMBER[1][0] + 0.1, -7.0 + HOLD_OFFSET + 0.6
    entities.append(entity(
        "guide-cable", "Guide cable", position=((start + end) / 2, DOOR_TOP - 0.3, door_z),
        scale=(end - start, 0.07, 0.07),
        **{"sindri.mesh": {"primitive": "cube", "texture": "textures/line.png"},
           "sindri.material": {"base_color": [0.9, 0.7, 0.2, 1.0], "emissive": [0.12, 0.08, 0.0],
                               "metallic": 0.3, "roughness": 0.6, "override_factors": True},
           "sindri.tags": {"tags": ["cable"]}},
    ))
    # Alarm beacons: red lights flashing through the refuge and the ruin, so the
    # opening reads as a disaster that only just happened (`scripts/alarm.decay`).
    # The renderer keeps the eight lights that light the camera's position
    # most, so the helmet lamp stays lit however many alarms flash elsewhere.
    for index, at in enumerate(((-13.0, 0.0, 0.7), (-8.5, -0.2, 1.5), (-7.25, 0.3, 2.7),
                                (0.5, 2.35, -2.3), (6.5, 2.35, 2.3), (11.0, 2.35, -2.3))):
        beacon = lamp(f"alarm-{index}", at, (1.0, 0.08, 0.04), 3.0, 7.0)
        beacon["transform_3d"]["scale"] = [0.18, 0.12, 0.18]
        beacon["components"]["sindri.tags"] = {"tags": ["alarm"]}
        entities.append(beacon)
    entities.append(entity("alarms", "Alarms",
                           **{"sindri.script": {"source": "scripts/alarm.decay", "script": "Alarms"}}))
    entities += wreckage()
    # The diver: a dynamic upright capsule that physics never turns, with the
    # eye under it carrying the camera and the helmet lamp. Turning is the
    # eye's. They wake in the cabin, standing by the bunk.
    diver = entity(
        "diver", "Diver", position=SPAWN,
        **{
            "sindri.physics3d.collider": {"pieces": [{
                "shape": {"shape": "capsule", "half_height": 0.5, "radius": 0.3},
                "offset": [0, 0, 0], "rotation": [0, 0, 0, 1], "sensor": False,
                "layers": DIVER, "friction": 0.4, "restitution": 0.15,
            }]},
            "sindri.physics3d.rigid_body": body("dynamic", lock_rotation=True),
            "sindri.script": {"source": "scripts/diver.decay", "script": "Diver"},
        },
    )
    eye = entity(
        "eye", "Eye", parent="diver", position=(0, EYE_HEIGHT, 0), rotation=yaw_quat(-math.pi / 2),
        **{"sindri.camera": {"projection": "perspective", "vertical_fov_degrees": 72, "near": 0.05, "far": 220}},
    )
    helmet = entity(
        "helmet-lamp", "HelmetLamp", parent="eye", position=(0.15, 0.1, 0),
        **{"sindri.light": {"kind": "spot", "color": [0.9, 0.95, 1.0], "intensity": 2.5, "range": 14.0,
                            "inner_angle": 0.25, "outer_angle": 0.5}},
    )
    entities += [diver, eye, helmet]
    # The diver's own left forearm, held up in view, and the wrist display on it:
    # air, suit power and the thruster pack read where a diver would read them.
    # Low in the corner of the view, the screen tipped up towards the eye; the
    # readout floats just in front of the screen so the screen never hides it.
    # The rest of the diver's body hangs from the neck, posed by the diver
    # script: upright with the heading aboard, following the view in free fall.
    entities.append(model("diver-body", "body", (0, EYE_HEIGHT - 0.1, 0), parent="diver"))
    entities[-1]["name"] = "DiverBody"
    entities.append(model("forearm", "forearm", (-0.26, -0.3, -0.5), parent="eye",
                          rotation=quat((1.0, -0.35, 0.0), 38)))
    entities[-1]["transform_3d"]["scale"] = [0.7, 0.7, 0.7]
    entities.append(entity(
        "wrist-readout", "wrist-readout", parent="eye", position=(-0.245, -0.255, -0.44),
        **{"sindri.text": {"text": "", "anchor": "center", "color": [0.55, 1.0, 0.85, 1.0],
                           "font": "fonts/ChakraPetch-Regular.ttf", "font_size": 0.016,
                           "line_height": 0.019, "layer": 30, "follow_rotation": True}},
    ))
    # What ACTION will use: brackets around it, placed by the diver script.
    # Out of sight until there is something to aim at.
    entities.append(model("aim-marker", "marker", (0, -1000, 0)))
    # Spare oxygen, floating where a diver short of air can reach it: beside the
    # guide cable, inside the ruin, and out by the wreck.
    for index, at in enumerate(((-4.4, -0.45, 1.25), (1.0, -0.3, 0.8), (7.5, 1.2, -1.2),
                                (15.0, 0.4, 1.6))):
        entities.append(model(f"oxygen-{index}", "oxygen-canister", at,
                              rotation=quat((1.0, 0.0, 0.6), 25 + index * 40),
                              **{"sindri.tags": {"tags": ["oxygen"]}}))
    entities += tether((-half_l + 0.12 + HOLD_OFFSET, 0, 0), (-half_l + 0.7 + HOLD_OFFSET, 0, 0))
    # A stanchion part way down the hold, for the tether to catch on.
    entities.append(entity(
        "stanchion", "Stanchion", position=(-2.5 + HOLD_OFFSET, 0, 1.1), scale=(0.35, HEIGHT, 0.35),
        **{
            "sindri.mesh": {"primitive": "cube", "texture": "textures/grip.png"},
            "sindri.material": {"base_color": [0.55, 0.55, 0.55, 1], "metallic": 0.4, "roughness": 0.6, "override_factors": True},
            "sindri.physics3d.collider": {"pieces": [box_piece()]},
        },
    ))
    # The HUD: a reticle, the propellant gauge and a prompt for what can be grabbed.
    entities += [
        text("hud-reticle", (0, 0, 0), "+", 0.06, (0.95, 0.98, 1.0, 0.9)),
        text("hud-prompt", (0, -0.2, 0), "", 0.035, (1.0, 0.85, 0.3, 1.0)),
        text("hud-tether", (0, -0.88, 0), "TETHER {} / {} M", 0.04),
        text("hud-tether-state", (0, -0.78, 0), "", 0.035, (1.0, 0.55, 0.25, 1.0)),
        text("hud-alert", (0, 0.3, 0), "", 0.05, (1.0, 0.3, 0.2, 1.0)),
        text("hud-breach", (0, 0.82, 0), "", 0.05, (1.0, 0.2, 0.12, 1.0)),
        text("hud-salvage", (0.8, -0.68, 0), "", 0.04, (1.0, 0.75, 0.4, 1.0)),
    ]
    entities += touch.controls()
    return {"format_version": 10, "metadata": {"name": "The Tern"}, "entities": entities}


def main():
    (ASSETS / "textures").mkdir(parents=True, exist_ok=True)
    panel_texture(ASSETS / "textures" / "panel.png")
    grip_texture(ASSETS / "textures" / "grip.png")
    crate_texture(ASSETS / "textures" / "crate.png")
    line_texture(ASSETS / "textures" / "line.png")
    door_texture(ASSETS / "textures" / "door.png")
    (ASSETS / "hold.scene").write_text(json.dumps(scene(), indent=2) + "\n")


if __name__ == "__main__":
    main()
