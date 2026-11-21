#!/usr/bin/env python3
"""Builds `assets/hold.scene` and its textures: the *Tern*'s cargo hold.

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

HERE = Path(__file__).resolve().parent
ASSETS = HERE.parent / "assets"

# The hold, inside the walls: X runs aft (-) to forward (+).
LENGTH = 14.0
HEIGHT = 5.0
WIDTH = 5.0
WALL = 0.4

SOLID = {"memberships": 1, "filter": 4294967295}
DIVER = {"memberships": 2, "filter": 4294967295}


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


def handhold(id_, position, along):
    """A grab bar standing a little proud of a surface, long along `along`."""
    scale = [0.12, 0.12, 0.12]
    scale["xyz".index(along)] = 0.7
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
            "ambient_intensity": 0.18,
            "bloom": {"enabled": True, "intensity": 0.6, "knee": 0.2, "passes": 4, "threshold": 0.8},
        }}),
        entity("sun", "Starlight", rotation=(-0.25, 0.35, 0.0, 0.9),
               **{"sindri.light": {"kind": "directional", "color": [0.7, 0.78, 1.0], "intensity": 0.15}}),
    ]
    # The hold's six walls.
    entities += wall("aft-bulkhead", (-half_l - t, 0, 0), (WALL, outer_h, outer_w))
    entities += wall("forward-bulkhead", (half_l + t, 0, 0), (WALL, outer_h, outer_w))
    entities += wall("deck", (0, -half_h - t, 0), (outer_l, WALL, outer_w))
    entities += wall("overhead", (0, half_h + t, 0), (outer_l, WALL, outer_w))
    entities += wall("port-wall", (0, 0, -half_w - t), (outer_l, outer_h, WALL))
    entities += wall("starboard-wall", (0, 0, half_w + t), (outer_l, outer_h, WALL))
    entities += [
        # Handholds: one on each bulkhead at the diver's height, and a run down each side.
        handhold("start-hold", (-half_l + 0.12, 0, 0), "z"),
        handhold("far-hold", (half_l - 0.12, 0, 0), "z"),
    ]
    entities[-2]["name"] = "StartHold"
    entities[-1]["name"] = "FarHold"
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
    # The diver: a dynamic sphere that physics never turns, with the eye under it
    # carrying the camera and the helmet lamp. Turning is the eye's.
    diver = entity(
        "diver", "Diver", position=(-half_l + 0.7, 0, 0),
        **{
            "sindri.physics3d.collider": {"pieces": [{
                "shape": {"shape": "sphere", "radius": 0.35},
                "offset": [0, 0, 0], "rotation": [0, 0, 0, 1], "sensor": False,
                "layers": DIVER, "friction": 0.4, "restitution": 0.15,
            }]},
            "sindri.physics3d.rigid_body": body("dynamic", lock_rotation=True),
            "sindri.script": {"source": "scripts/diver.decay", "script": "Diver"},
        },
    )
    eye = entity(
        "eye", "Eye", parent="diver", rotation=yaw_quat(-math.pi / 2),
        **{"sindri.camera": {"projection": "perspective", "vertical_fov_degrees": 72, "near": 0.05, "far": 120}},
    )
    helmet = entity(
        "helmet-lamp", "HelmetLamp", parent="eye", position=(0.15, 0.1, 0),
        **{"sindri.light": {"kind": "spot", "color": [0.9, 0.95, 1.0], "intensity": 1.6, "range": 14.0,
                            "inner_angle": 0.25, "outer_angle": 0.5}},
    )
    entities += [diver, eye, helmet]
    # The HUD: a reticle, the propellant gauge and a prompt for what can be grabbed.
    entities += [
        text("hud-reticle", (0, 0, 0), "+", 0.05, (0.85, 0.92, 1.0, 0.6)),
        text("hud-propellant", (-1.25, -0.88, 0), "PROPELLANT {}%", 0.04),
        text("hud-velocity", (1.25, -0.88, 0), "{} M/S", 0.04),
        text("hud-prompt", (0, -0.2, 0), "", 0.035, (1.0, 0.85, 0.3, 1.0)),
    ]
    return {"format_version": 10, "metadata": {"name": "The Hold"}, "entities": entities}


def main():
    (ASSETS / "textures").mkdir(parents=True, exist_ok=True)
    panel_texture(ASSETS / "textures" / "panel.png")
    grip_texture(ASSETS / "textures" / "grip.png")
    crate_texture(ASSETS / "textures" / "crate.png")
    (ASSETS / "hold.scene").write_text(json.dumps(scene(), indent=2) + "\n")


if __name__ == "__main__":
    main()
