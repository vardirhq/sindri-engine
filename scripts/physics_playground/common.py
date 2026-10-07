"""Shared helpers for the Physics Playground generator: entities, shapes,
bodies, colliders, joints and screen UI, written the way the scene format
stores them."""

from __future__ import annotations

import math

FONT = "fonts/ChakraPetch-Regular.ttf"

# Collision layers, in the bit order the scene's Physics 2D World names them.
LAYERS = ["static", "props", "balls", "robot", "shells", "glass", "hand"]
ALL = 0xFFFFFFFF


def bit(*names: str) -> int:
    value = 0
    for name in names:
        value |= 1 << LAYERS.index(name)
    return value


# Who collides with whom. The robot is a character, so props it should not
# shove are kept off its filter; the hand collides with nothing.
FILTERS = {
    "static": ALL,
    "props": bit("static", "props", "balls", "robot", "shells", "glass"),
    "balls": bit("static", "props", "balls", "robot", "shells", "glass"),
    "robot": bit("static", "props", "balls"),
    "shells": bit("static", "props", "balls", "glass"),
    "glass": bit("static", "props", "balls", "shells", "glass"),
    "hand": 0,
}


def linear(channel: float) -> float:
    """World colors are linear; authored hex is sRGB, as a designer picks it."""
    if channel <= 0.04045:
        return channel / 12.92
    return ((channel + 0.055) / 1.055) ** 2.4


def rgba(hex_color: str, alpha: float = 1.0) -> list[float]:
    hex_color = hex_color.lstrip("#")
    r, g, b = (linear(int(hex_color[i:i + 2], 16) / 255.0) for i in (0, 2, 4))
    return [round(r, 5), round(g, 5), round(b, 5), alpha]


def transform(x=0.0, y=0.0, z=0.0, sx=1.0, sy=None, rot=0.0):
    if sy is None:
        sy = sx
    half = rot / 2.0
    return {
        "position": [round(x, 4), round(y, 4), round(z, 4)],
        "rotation": [0.0, 0.0, round(math.sin(half), 6), round(math.cos(half), 6)],
        "scale": [round(sx, 4), round(sy, 4), 1.0],
    }


def shape(kind="rect", fill="#ffffff", stroke=None, sw=0.0, layer=0, count=6,
          alpha=1.0, stroke_alpha=1.0, corner=0.0, dashes=0, duty=0.5, blend="over"):
    return {
        "kind": kind,
        "count": float(count),
        "fill": rgba(fill, alpha) if isinstance(fill, str) else list(fill),
        "stroke": (rgba(stroke, stroke_alpha) if isinstance(stroke, str)
                   else list(stroke) if stroke else [0.0, 0.0, 0.0, 0.0]),
        "stroke_width": float(sw),
        "corner_radius": float(corner),
        "dashes": float(dashes),
        "dash_duty": float(duty),
        "sweep_start": 0.0,
        "sweep_turns": 1.0,
        "blend": blend,
        "layer": int(layer),
    }


def box(hx, hy, layer="props", friction=0.5, restitution=0.0, offset=(0.0, 0.0),
        rotation=0.0, sensor=False, filter_mask=None):
    return piece({"shape": "box", "half_extents": [round(hx, 4), round(hy, 4)]},
                 layer, friction, restitution, offset, rotation, sensor, filter_mask)


def circle(radius, layer="props", friction=0.5, restitution=0.0, offset=(0.0, 0.0),
           sensor=False, filter_mask=None):
    return piece({"shape": "circle", "radius": round(radius, 4)},
                 layer, friction, restitution, offset, 0.0, sensor, filter_mask)


def capsule(half_height, radius, layer="props", friction=0.5, restitution=0.0,
            offset=(0.0, 0.0), rotation=0.0, sensor=False, filter_mask=None):
    return piece({"shape": "capsule", "half_height": half_height, "radius": radius},
                 layer, friction, restitution, offset, rotation, sensor, filter_mask)


def piece(geometry, layer, friction, restitution, offset, rotation, sensor, filter_mask):
    return {
        "shape": geometry,
        "offset": [round(offset[0], 4), round(offset[1], 4)],
        "rotation": round(rotation, 6),
        "sensor": sensor,
        "layers": {
            "memberships": bit(layer),
            "filter": FILTERS[layer] if filter_mask is None else filter_mask,
        },
        "friction": friction,
        "restitution": restitution,
    }


def body(kind="dynamic", damping=0.0, angular_damping=0.0, lock=False, ccd=False,
         gravity=1.0):
    return {
        "kind": kind,
        "pose": {"position": [0, 0], "rotation": 0},
        "linear_velocity": [0, 0],
        "angular_velocity": 0,
        "gravity_scale": gravity,
        "linear_damping": damping,
        "angular_damping": angular_damping,
        "lock_rotation": lock,
        "continuous_collision": ccd,
    }


def script(source, name, **properties):
    return {"source": source, "script": name, "properties": properties}


def material(profile, override_restitution=None):
    payload = {
        "profile": profile,
        "override_friction": False,
        "friction": 0.5,
        "override_restitution": override_restitution is not None,
        "restitution": override_restitution or 0.0,
    }
    return payload


def font_px(px: float) -> float:
    """A rough overlay font size for an authored pixel size; Weave overrides it."""
    return round(px / 360.0, 4)


def text(words, px=14, bounds=(1.0, 0.08), layer=40, align="center"):
    return {
        "text": words,
        "font": FONT,
        "font_size": font_px(px),
        "bounds": list(bounds),
        "anchor": "center",
        "layer": layer,
        "line_align": align,
    }


def ui_shape(fill="#0e1a28", layer=30):
    return {"kind": "rect", "fill": rgba(fill), "anchor": "center", "layer": layer}


def layout(direction="row", justify="center", align="center", spacing=0.02):
    return {"direction": direction, "spacing": spacing, "justify": justify, "align": align}


def style(*classes):
    return {"classes": list(classes)}
