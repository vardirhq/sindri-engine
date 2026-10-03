#!/usr/bin/env python3
"""Writes Low Tide's scene: the Basin, the crawler, the wrecks and the HUD.

Run from anywhere; it writes ../assets/low-tide.scene. Deterministic. The
scene is generated rather than painted because its two big maps are layouts
best read as text: the crawler's floor plan below, and the Basin's dunes and
cracks scattered from a fixed seed. The editor opens the result like any
other scene.
"""

import json
import math
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent / "assets"

# The crawler, bow at the top. Read as a floor plan:
#   W wall   w window   . deck   - door   h hatch   R ramp   T tread
#   H helm   G table    b bunk   E engine (space) nothing
#
# The hatch and ramp on the starboard side are the only way on or off.
# A fixed construction canvas keeps the deck's origin stable as it grows.
# Only the starter's seven-by-nine footprint is populated at first.
CRAWLER = [
    "           ",
    "           ",
    "           ",
    "   WWwWW   ",
    "  TW.H.WT  ",
    "  TW...WT  ",
    "  TWB..WT  ",
    "  TW...hR  ",
    "  TW...WT  ",
    "  TW..EWT  ",
    "  TW...WT  ",
    "   WWWWW   ",
    "           ",
    "           ",
    "           ",
    "           ",
    "           ",
    "           ",
]

SHIP_TILES = [
    "deck", "wall", "window", "door", "hatch", "helm", "table",
    "bunk", "engine", "tread", "ramp", "crate", "mast",
    "floor-home", "floor-bridge", "floor-hold",
    "crate-wood", "crate-stone", "crate-ore", "crate-fibre", "crate-salt", "workbench", "engine-upgraded",
]
SYMBOL = {
    ".": "deck", "W": "wall", "w": "window", "-": "door", "h": "hatch",
    "H": "helm", "G": "table", "b": "bunk", "E": "engine", "T": "tread",
    "R": "ramp", "B": "workbench",
}

# The Basin: an old sea floor, generated rather than painted, by the engine's
# natural terrain viewed as a map. The same settings with the engine's own
# blocks and land biomes make an ordinary overland world; this is the
# drained one.
#
# The sea is set far below the old floor, so brine is left only in the
# deepest trenches, ringed with salt crust. Warmth falls with height, so the
# biomes sort themselves by it: the cold high ground is the old islands,
# grassed and wooded, and the warm basin floor is salt where it is dry, mud
# where it is wet, dead reef between, and dried kelp where it is cool.
BASIN = {
    "kind": "natural_terrain",
    "seed": 11,
    "sea_level": -18,
    "relief": 40,
    "feature_size": 64,
    "tree_line": 400,
    "snow_line": 500,
    "caves": False,
    "rivers": False,
    "stone_voxel": "rock",
    "water_voxel": "brine",
    "beach_voxel": "crust",
    "sea_bed_voxel": "mud",
    "cliff_voxel": "rock",
    "snow_voxel": None,
    "ice_voxel": None,
    "trunk_voxel": "trunk",
    "leaves_voxel": "leaves",
    "biomes": [
        {"name": "Salt flats", "temperature": 0.85, "moisture": 0.15, "surface_voxel": "salt",
         "subsurface_voxel": "salt", "subsurface_depth": 3, "trees": 0.0, "relief": 0.25, "terraces": 0},
        {"name": "Dunes", "temperature": 0.95, "moisture": 0.4, "surface_voxel": "sand",
         "subsurface_voxel": "sand", "subsurface_depth": 4, "trees": 0.0, "relief": 0.9, "terraces": 0},
        {"name": "Mud flats", "temperature": 0.7, "moisture": 0.9, "surface_voxel": "mud",
         "subsurface_voxel": "mud", "subsurface_depth": 3, "trees": 0.0, "relief": 0.2, "terraces": 0},
        {"name": "Dead reef", "temperature": 0.65, "moisture": 0.55, "surface_voxel": "reef",
         "subsurface_voxel": "rock", "subsurface_depth": 2, "trees": 0.0, "relief": 1.4, "terraces": 2},
        {"name": "Kelp beds", "temperature": 0.45, "moisture": 0.75, "surface_voxel": "kelp",
         "subsurface_voxel": "mud", "subsurface_depth": 2, "trees": 0.0, "relief": 0.5, "terraces": 0},
        {"name": "Old islands", "temperature": 0.0, "moisture": 0.5, "surface_voxel": "grass",
         "subsurface_voxel": "dirt", "subsurface_depth": 3, "trees": 0.35, "relief": 1.2, "terraces": 0},
    ],
}
# Where the crawler starts, in the scene's plane: on a salt-crust shore with
# a brine lake, kelp beds and an island in reach. Column X is scene X, and row
# Z runs down the map, so the column at (-129, -53) is centred at
# (-128.5, 52.5). Found by searching the generated world for flat dry ground.
START = (-128.5, 52.5)
# One introductory wreck; the rest are streamed by Decay across the Basin.
WRECKS = [(-116.5, 72.5, 0.4)]


def transform(x, y, z=0.0, rotation=0.0, scale=(1.0, 1.0)):
    half = rotation / 2
    return {
        "position": [round(x, 4), round(y, 4), z],
        "rotation": [0, 0, round(math.sin(half), 6), round(math.cos(half), 6)],
        "scale": [scale[0], scale[1], 1],
    }


def shape(fill, layer, corner=0.0):
    return {
        "kind": "rect", "count": 4.0, "fill": fill, "stroke": [0, 0, 0, 0],
        "stroke_width": 0.0, "corner_radius": corner, "dashes": 0.0,
        "dash_duty": 0.5, "sweep_start": 0.0, "sweep_turns": 1.0,
        "blend": "over", "layer": layer,
    }


def text(anchor, size, words, colour=(1.0, 0.96, 0.88, 1.0)):
    return {
        "anchor": anchor, "color": list(colour), "font": "fonts/Inter.ttf",
        "font_size": size, "line_height": size * 1.25, "layer": 100,
        "text": words, "bold": True,
        # Light text reads against the instrument and hint panels.
        "outline": {"width": 0.0, "color": [0.12, 0.08, 0.06, 0.95]},
        "shadow": {"offset": [0.0, -0.003], "color": [0.0, 0.0, 0.0, 0.35],
                   "softness": 0.004},
    }


def script(source, name, properties=None):
    return {"source": source, "script": name, "properties": properties or {}}


def crawler_tiles():
    tiles = []
    for line in CRAWLER:
        for symbol in line:
            tiles.append(None if symbol == " " else SHIP_TILES.index(SYMBOL[symbol]))
    return tiles


def entities():
    width, height = len(CRAWLER[0]), len(CRAWLER)
    found = [
        {
            "id": "camera", "name": "Camera",
            "transform_3d": transform(START[0], START[1], 10.0),
            "components": {
                "sindri.camera": {"projection": "orthographic", "vertical_size": 30.0,
                                  "near": 0.1, "far": 100.0, "fit": "height"},
                "sindri.script": script("scripts/view.decay", "View"),
            },
        },
        {
            "id": "environment", "name": "Environment",
            "components": {"sindri.environment": {
                "background": [0.8, 0.77, 0.69, 1.0],
                "shadows": {"enabled": False, "map_size": 1024, "distance": 48.0,
                            "bias": 0.002},
            }},
        },
        {
            "id": "controls", "name": "Controls",
            "components": {
                "sindri.input.actions": {"actions": [
                    {"name": "move", "kind": "vector", "bindings": [
                        {"vector": {"up": "key.W", "down": "key.S",
                                    "left": "key.A", "right": "key.D"}},
                        {"vector": {"up": "key.ArrowUp", "down": "key.ArrowDown",
                                    "left": "key.ArrowLeft", "right": "key.ArrowRight"}},
                        {"vector": {"up": "gamepad.dpad_up", "down": "gamepad.dpad_down",
                                    "left": "gamepad.dpad_left", "right": "gamepad.dpad_right"}},
                    ]},
                    {"name": "use", "kind": "button",
                     "bindings": ["key.E", "key.Space", "gamepad.south"]},
                    {"name": "drop", "kind": "button",
                     "bindings": ["key.X", "gamepad.west"]},
                    {"name": "next_plan", "kind": "button",
                     "bindings": ["key.Q", "gamepad.east"]},
                    {"name": "cancel", "kind": "button", "bindings": ["key.Escape"]},
                    {"name": "view", "kind": "button",
                     "bindings": ["key.Tab", "key.V", "gamepad.north"]},
                ]},
            },
        },
        {
            "id": "basin", "name": "Basin",
            "transform_3d": transform(0.0, 0.0, -0.05),
            "components": {"sindri.voxel_world": {
                "generator": BASIN, "blocks": "basin.tileset", "view": "map",
                "variant_seed": 5, "layer": -10, "edits": [],
            }, "sindri.script": script("scripts/tide.decay", "Tide")},
        },
    ]
    for index, (x, y, turn) in enumerate(WRECKS, start=1):
        found.append({
            "id": f"wreck-{index}", "name": f"Wreck {index}",
            "transform_3d": transform(x, y, 0.05, turn, (7.0, 4.5)),
            "components": {
                "sindri.sprite": {"texture": "textures/wreck.png", "layer": 1},
                "sindri.tags": {"tags": ["wreck"]},
                "sindri.script": script("scripts/wreck.decay", "Wreck"),
            },
        })
    found += [
        {
            "id": "wreck-field", "name": "Basin wrecks",
            "components": {"sindri.script": script("scripts/wreck-field.decay", "WreckField",
                                                  {"template": "prefabs/wreck.prefab"})},
        },
        {
            "id": "shadow", "name": "Crawler shadow",
            "transform_3d": transform(START[0] + 0.4, START[1] - 0.4, 0.08, 0.0,
                                      (6.4, 8.4)),
            "components": {"sindri.shape": shape([0.0, 0.0, 0.0, 0.22], 2, 0.06)},
        },
        {
            "id": "crawler", "name": "Crawler",
            "transform_3d": transform(START[0], START[1], 0.1),
            "components": {"sindri.script": script("scripts/crawler.decay", "Crawler")},
        },
        {
            "id": "deck", "name": "Deck", "parent": "crawler",
            "transform_3d": transform(-5.5, 7.0),
            "components": {"sindri.tilemap": {
                "texture": "textures/ship.png", "palette": SHIP_TILES,
                "columns": width, "rows": height, "tiles": crawler_tiles(), "layer": 4,
            }},
        },
        {
            "id": "floors", "name": "Room floors", "parent": "deck",
            "transform_3d": transform(0.0, 0.0),
            "components": {"sindri.tilemap": {
                "texture": "textures/ship.png", "palette": SHIP_TILES,
                "columns": width, "rows": height, "layer": 3,
                "tiles": [None if ch == " " else (14 if row < 6 else 13 if row < 8 else 15)
                          for row, line in enumerate(CRAWLER) for ch in line],
            }},
        },
        {
            "id": "crew", "name": "Crew", "parent": "deck",
            "transform_3d": transform(5.5, -7.5, 0.1, 0.0, (1.1, 1.1)),
            "components": {
                "sindri.sprite": {"texture": "textures/crew.png#down-0", "layer": 6},
                "sindri.animation.sprite": {
                    "clips": {
                        "down": {"frames": ["down-0", "down-1"], "seconds_per_frame": 0.18, "looping": True},
                        "up": {"frames": ["up-0", "up-1"], "seconds_per_frame": 0.18, "looping": True},
                        "side": {"frames": ["side-0", "side-1"], "seconds_per_frame": 0.18, "looping": True},
                        "down-still": {"frames": ["down-0"], "seconds_per_frame": 0.5, "looping": False},
                        "up-still": {"frames": ["up-0"], "seconds_per_frame": 0.5, "looping": False},
                        "side-still": {"frames": ["side-0"], "seconds_per_frame": 0.5, "looping": False},
                    },
                    "playing": "down-still", "speed": 1.0,
                },
                "sindri.script": script("scripts/crew.decay", "Crew"),
            },
        },
        {
            "id": "carried", "name": "Carried", "parent": "crew",
            "transform_3d": transform(0.0, 0.42, 0.01, 0.0, (0.5, 0.5)),
            "components": {
                "sindri.sprite": {"texture": "textures/ship.png#crate", "layer": 7},
                "sindri.animation.sprite": {
                    "clips": {kind: {"frames": ["crate" if kind == "scrap" else f"crate-{kind}"],
                                     "seconds_per_frame": 1.0, "looping": False}
                              for kind in ("scrap", "wood", "stone", "ore", "fibre", "salt")},
                    "playing": "scrap", "speed": 1.0,
                },
            },
            "disabled": True,
        },
        {
            "id": "status", "name": "Status",
            "transform_3d": transform(0.12, -0.33),
            "components": {
                "sindri.ui.text": text("top_left", 0.031, "Salt flats"),
                "sindri.script": script("scripts/hud.decay", "Hud"),
            },
        },
        {
            "id": "hint", "name": "Hint",
            "transform_3d": transform(0.0, 0.06),
            "components": {"sindri.ui.text": text("bottom", 0.029, "")},
        },
    ] + crawler_motion() + cabin_details() + instrument_panel() + touch_controls() + workbench() + dive_cutaway() + [
    ]
    return found


def ui_shape(kind, anchor, fill, stroke, stroke_width, layer, corner=0.0):
    return {
        "kind": kind, "count": 48.0, "fill": fill, "stroke": stroke,
        "stroke_width": stroke_width, "corner_radius": corner, "dashes": 0.0,
        "dash_duty": 0.5, "sweep_start": 0.0, "sweep_turns": 1.0,
        "blend": "over", "anchor": anchor, "layer": layer,
    }


def crawler_motion():
    """Two upper belts: forward carries the visible cleats toward the bow.

    Image Y runs down toward the stern, so forward decreases the art phase.
    The logical tread cells still block walking.
    """
    found = []
    for side, column, direction in (("port", 2.5, -1), ("starboard", 8.5, 1)):
        for row in range(4, 17):
            found.append({
                "id": f"{side}-tread" if row == 4 else f"{side}-tread-{row}",
                "name": f"{side.title()} tread {row}", "parent": "deck",
                "transform_3d": transform(column, -row - 0.5, 0.02, 0, (1, 1)),
                "components": {
                    "sindri.sprite": {"texture": f"textures/treads.png#{side}-0", "layer": 5},
                    "sindri.animation.sprite": {
                        "clips": {
                            "forward": {"frames": [f"{side}-{i}" for i in reversed(range(8))],
                                        "seconds_per_frame": 0.055, "looping": True},
                            "reverse": {"frames": [f"{side}-{i}" for i in range(8)],
                                        "seconds_per_frame": 0.055, "looping": True},
                        }, "playing": "forward", "speed": 0.0,
                    },
                    "sindri.script": script("scripts/tread.decay", "Tread", {"side": direction}),
                },
                "disabled": row > 10 or (side == "starboard" and row == 7),
            })
    return found


def cabin_details():
    # Fixed fittings decorate the frame; furniture still comes from upgrades.
    found = []
    for i, (x, y) in enumerate([(3.5, -4.5), (7.5, -4.5), (3.5, -10.5), (7.5, -10.5)]):
        found.append({"id": f"cabin-lamp-{i}", "name": f"Frame lamp {i}", "parent": "deck",
            "transform_3d": transform(x, y, 0.2, 0, (1.4, 1.4)),
            "components": {"sindri.sprite": {"texture": "textures/lamp.png", "layer": 9}}})
    for side, x in [("port", 3.3), ("starboard", 7.7)]:
        found.append({"id": "pipe-"+side, "name": "Frame pipe "+side, "parent": "deck",
            "transform_3d": transform(x, -7.2, 0.1, 0, (0.25, 4.0)),
            "components": {"sindri.sprite": {"texture": "textures/pipe.png", "layer": 5}}})
    return found


def instrument_panel():
    found = []
    def panel(key, name, x, y, w, h, fill, anchor="top_left", layer=90):
        found.append({"id": key, "name": name, "transform_3d": transform(x, y, 0, 0, (w, h)),
            "components": {"sindri.ui.shape": ui_shape("rect", anchor, fill,
                [0.65, 0.55, 0.34, 0.9], 0.008, layer, 0.06)}})
    def words(key, name, x, y, size, content, anchor="top_left"):
        found.append({"id": key, "name": name, "transform_3d": transform(x, y),
            "components": {"sindri.ui.text": text(anchor, size, content)}})
    def icon(key, frame, x, y, size, anchor="top_left"):
        found.append({"id": key, "name": key, "transform_3d": transform(x, y, 0, 0, (size, size)),
            "components": {"sindri.ui.image": {"anchor": anchor, "texture": "textures/instruments.png#"+frame, "layer": 101}}})
    panel("instruments", "Instrument panel", 0.34, -0.21, 0.64, 0.38, [0.008, 0.018, 0.022, 0.98])
    icon("brand-icon", "sun", 0.07, -0.064, 0.072)
    words("instrument-title", "Instrument title", 0.12, -0.030, 0.047, "LOW TIDE")
    words("instrument-subtitle", "Instrument subtitle", 0.12, -0.092, 0.024, "CRAWLER 01")
    for key, frame, y in [("hold-icon", "hold", -0.14), ("speed-icon", "speed", -0.195), ("tide-icon", "tide", -0.25), ("terrain-icon", "pin", -0.33)]:
        icon(key, frame, 0.07, y - 0.016, 0.043)
    words("hold-readout", "Hold readout", 0.12, -0.14, 0.034, "Hold 0/4")
    words("speed-readout", "Speed readout", 0.12, -0.195, 0.034, "Speed 0.0")
    words("tide-readout", "Tide readout", 0.12, -0.25, 0.032, "LOW TIDE / 180s")
    panel("speed-track", "Speed track", 0.49, -0.195, 0.19, 0.013, [0.05, 0.065, 0.07, 1], layer=96)
    panel("speed-fill", "Speed fill", 0.49, -0.195, 0.19, 0.013, [0.61, 0.48, 0.22, 1], layer=97)
    panel("tide-track", "Tide track", 0.35, -0.288, 0.5, 0.014, [0.05, 0.065, 0.07, 1], layer=96)
    panel("tide-fill", "Tide fill", 0.35, -0.288, 0.5, 0.014, [0.05, 0.37, 0.52, 1], layer=97)
    for i in range(16):
        panel(f"cargo-empty-{i}", f"Cargo empty {i}", 0.45, -0.14, 0.04, 0.018, [0.05, 0.065, 0.07, 1], layer=96)
        panel(f"cargo-fill-{i}", f"Cargo fill {i}", 0.45, -0.14, 0.04, 0.018, [0.59, 0.45, 0.19, 1], layer=97)
        panel(f"cargo-full-{i}", f"Cargo full {i}", 0.45, -0.14, 0.04, 0.018, [0.64, 0.06, 0.065, 1], layer=98)
    panel("hint-panel", "Hint panel", 0, 0.065, 0.87, 0.095, [0.008, 0.018, 0.022, 0.98], "bottom")
    panel("context-panel", "Context panel", 0, 0.335, 0.86, 0.34, [0.008, 0.018, 0.022, 0.98], "bottom")
    icon("context-icon", "hold", -0.31, 0.40, 0.075, "bottom")
    words("context-title", "Context title", 0, 0.445, 0.035, "CARGO", "bottom")
    words("context-detail", "Context detail", 0, 0.38, 0.028, "", "bottom")
    panel("notice-panel", "Notice panel", 0, 0.55, 0.86, 0.09, [0.09, 0.027, 0.019, 0.98], "bottom")
    words("notice", "Notice", 0, 0.55, 0.027, "", "bottom")
    found.append({"id": "crate-focus", "name": "Crate focus", "parent": "deck", "disabled": True,
        "transform_3d": transform(0, 0, 0.2, 0, (1.04, 1.04)),
        "components": {"sindri.shape": shape([0,0,0,0], 8, 0.03) | {"stroke": [0.95,0.68,0.18,1], "stroke_width": 0.055}}})
    return found


def touch_button(key, words, x, y, width=0.3, height=0.13, anchor="bottom_right"):
    """A button for a thumb, anchored to a corner on the right."""
    fill = [0.15, 0.018, 0.025, 0.98] if key == "drop" else [0.008, 0.018, 0.022, 0.98]
    return [
        {
            "id": f"{key}-button", "name": f"{words} button",
            "transform_3d": transform(x, y, 0.0, 0.0, (width, height)),
            "components": {
                "sindri.ui.shape": ui_shape("rect", anchor, fill,
                                            [1.0, 0.86, 0.55, 0.9], 0.015, 90, 0.10),
                "sindri.ui.button": {"label": words},
            },
            "disabled": True,
        },
        {
            "id": f"{key}-label", "name": f"{words} label", "parent": f"{key}-button",
            "transform_3d": transform(0.0 if key == "view" else 0.035, -0.035 if key == "view" else 0.0, 0.0, 0.0, (width, 0.05)),
            "components": {"sindri.ui.text": text("center", 0.032 if key == "view" else 0.036, words, (1.0, 0.95, 0.85, 1.0)) | {
                "layer": 95, "shadow": {"offset": [0.0, 0.0], "color": [0, 0, 0, 0], "softness": 0.0}}},
        },
        {
            "id": f"{key}-icon", "name": f"{words} icon", "parent": f"{key}-button",
            "transform_3d": transform(0 if key == "view" else -0.10, 0.025 if key == "view" else 0, 0, 0, (0.052, 0.052)),
            "components": {"sindri.ui.image": {"anchor": "center", "layer": 96,
                "texture": "textures/instruments.png#" + ("compass" if key == "view" else "hand" if key == "drop" else "tool")}},
        },
    ]


def touch_controls():
    """The phone's controls, hidden until the first finger touches the screen.

    A thumb anywhere off the buttons is Sindri's virtual stick, drawn as a ring
    where it landed; the buttons stand in for E, X and Tab."""
    return [
        {
            "id": "touch", "name": "Touch controls",
            "components": {"sindri.script": script("scripts/touch.decay", "TouchControls")},
        },
        {
            "id": "stick-ring", "name": "Stick ring",
            "transform_3d": transform(0.0, 0.0, 0.0, 0.0, (0.3, 0.3)),
            "components": {"sindri.ui.shape": ui_shape("ellipse", "center", [0.16, 0.1, 0.08, 0.18],
                                                       [0.16, 0.1, 0.08, 0.6], 0.04, 80)},
            "disabled": True,
        },
        {
            "id": "stick-knob", "name": "Stick knob",
            "transform_3d": transform(0.0, 0.0, 0.0, 0.0, (0.12, 0.12)),
            "components": {"sindri.ui.shape": ui_shape("ellipse", "center", [0.16, 0.1, 0.08, 0.7],
                                                       [1.0, 0.86, 0.55, 0.9], 0.08, 81)},
            "disabled": True,
        },
    ] + touch_button("use", "Use", 0.24, 0.245, 0.34, 0.13, "bottom") + touch_button("drop", "Drop", 0.24, 0.245, 0.34, 0.13, "bottom") \
      + touch_button("view", "View", -0.105, -0.085, 0.15, 0.13, "top_right")


def workbench():
    found = [{
        "id": "workbench", "name": "Workbench", "parent": "deck",
        "transform_3d": transform(4.5, -6.5),
        "components": {"sindri.script": script("scripts/workbench.decay", "Workbench")},
    }]
    def panel(key, name, y, width, height, fill, button=False):
        components = {"sindri.ui.shape": ui_shape("rect", "center", fill,
            [0.55, 0.48, 0.33, 0.9], 0.006, 120, 0.025)}
        if button: components["sindri.ui.button"] = {"label": name}
        found.append({"id": key, "name": name, "disabled": True,
            "transform_3d": transform(0, y, 0, 0, (width, height)), "components": components})
    def label(key, name, y, size, words):
        found.append({"id": key, "name": name, "disabled": True,
            "transform_3d": transform(0, y),
            "components": {"sindri.ui.text": text("center", size, words) | {"layer": 130}}})
    panel("build-shade", "Build shade", 0, 8, 4, [0.01, 0.02, 0.02, 0.8])
    panel("build-panel", "Build panel", 0, 1.9, 1.66, [0.008, 0.018, 0.022, 1])
    label("build-title", "Build title", 0.68, 0.055, "CRAWLER WORKBENCH")
    label("build-stock", "Build stock", 0.57, 0.031, "Cargo aboard")
    for key, name, y in [("extension", "Extension", 0.40), ("bunk", "Bunk", 0.18), ("engine", "Engine", -0.04)]:
        panel("plan-"+key, "Plan "+name, y, 1.74, 0.19, [0.018, 0.031, 0.035, 1], True)
        panel("plan-"+key+"-selected", "Plan "+name+" selected", y, 1.74, 0.19, [0.10, 0.23, 0.20, 1])
        found[-1]["components"]["sindri.ui.shape"]["layer"] = 121
        found[-1]["components"]["sindri.ui.shape"]["stroke"] = [0.9, 0.76, 0.42, 1]
        label("plan-"+key+"-words", "Plan "+name+" words", y, 0.043, name)
    label("build-words", "Build words", -0.29, 0.043, "Select a blueprint")
    label("build-status", "Build status", -0.48, 0.038, "Materials needed")
    for key, name, x, width in [("next", "Next", -0.55, 0.38),
                               ("build-confirm", "Build confirm", 0, 0.58),
                               ("build-close", "Build close", 0.55, 0.38)]:
        panel(key+"-button", name+" button", -0.65, width, 0.13, [0.07, 0.06, 0.035, 1], True)
        found[-1]["transform_3d"]["position"][0] = x
        label(key+"-label", name+" label", -0.65, 0.043, name)
        found[-1]["transform_3d"]["position"][0] = x
    label("build-help", "Build help", -0.77, 0.026, "Q next  /  E build  /  Esc close")
    return found


def dive_cutaway():
    # One authored wreck interior, selected from a nearby flooded map wreck.
    # This is a cutaway activity, not a slice of the voxel volume.
    root = {"id": "dive-stage", "name": "Dive cutaway", "disabled": True,
            "transform_3d": transform(10000, 10000)}
    found = [root, {"id": "dive-camera", "name": "Dive camera", "disabled": True,
             "transform_3d": transform(10000, 9996, 10),
             "components": {"sindri.camera": {"projection": "orthographic",
                 "vertical_size": 20.0, "near": 0.1, "far": 100.0, "fit": "height"}}},
             {"id": "dive-controller", "name": "Dive controller",
             "components": {"sindri.script": script("scripts/dive.decay", "Dive")}}]
    def rect(key, x, y, w, h, colour, layer, corner=0):
        found.append({"id": key, "name": key, "parent": "dive-stage",
            "transform_3d": transform(x, y, 0.1, 0, (w, h)),
            "components": {"sindri.shape": shape(colour, layer, corner)}})
    rect("deep-water", 0, -5, 80, 60, [0.025, 0.105, 0.14, 1], 20)
    for i in range(12):
        shade = 0.18 - i * 0.01
        rect(f"water-band-{i}", 0, 1-i*1.4, 70, 1.42,
             [0.035, shade, shade+0.05, 1], 21)
    rect("water-surface", 0, 0.35, 70, 0.13, [0.47, 0.71, 0.65, 1], 23)
    rect("sea-bed", 0, -11, 70, 2.5, [0.105, 0.16, 0.16, 1], 23)
    for i in range(9):
        rect(f"sea-bed-stone-{i}", -6+i*1.5, -9.8+(i%3)*0.12, 1.1, 0.4,
             [0.18, 0.23, 0.21, 1], 24, 0.15)
    # A side silhouette of the crawler and its boarding line stay visible above.
    rect("dive-crawler-tread", -1.5, 0.85, 6.2, 0.65, [0.13, 0.15, 0.13, 1], 25, 0.1)
    rect("dive-crawler-home", -1.5, 1.7, 5.6, 1.25, [0.40, 0.34, 0.25, 1], 26, 0.1)
    for i in range(3):
        rect(f"dive-crawler-window-{i}", -3+i*1.5, 1.8, 0.65, 0.4,
             [0.80, 0.70, 0.45, 1], 27, 0.02)
    rect("boarding-line", -1.5, -4.5, 0.045, 9, [0.80, 0.68, 0.36, 0.55], 29)
    rect("wreck-interior", 0, -6.5, 8, 5, [0.06, 0.10, 0.11, 1], 24)
    tiles = [None] * (14 * 12)
    for row in range(5, 11):
        for column in range(3, 11):
            wall = (row in (5, 10) or column in (3, 10)
                    or (column == 6 and row in (7, 9)))
            if row == 5 and column in (5, 6): wall = False
            if wall: tiles[row * 14 + column] = 1
    found.append({"id": "dive-hull", "name": "Sunken hull", "parent": "dive-stage",
        "transform_3d": transform(-7, 1),
        "components": {"sindri.tilemap": {"texture": "textures/dive.png",
            "palette": ["water", "hull"], "columns": 14, "rows": 12,
            "tiles": tiles, "layer": 26}}})
    found.append({"id": "dive-cache", "name": "Sealed salvage", "parent": "dive-stage",
        "transform_3d": transform(2.5, -7.5, 0.1, 0, (0.8, 0.7)),
        "components": {"sindri.sprite": {"texture": "textures/dive.png#salvage", "layer": 28}}})
    found.append({"id": "diver", "name": "Diver", "parent": "dive-stage",
        "transform_3d": transform(-1.5, 0, 0.2, 0, (0.9, 0.9)),
        "components": {"sindri.sprite": {"texture": "textures/dive.png#diver", "layer": 30}}})
    return found


def main():
    document = {
        "format_version": 10,
        "metadata": {"name": "Low Tide"},
        "entities": entities(),
    }
    written = json.dumps(document, indent=1)
    # A map's cells on one line per map, rather than one line per cell.
    written = re.sub(
        r'"tiles": \[[^\]]*\]',
        lambda found: '"tiles": ' + json.dumps(json.loads(found.group(0)[9:]), separators=(",", ":")),
        written,
    )
    (ROOT / "low-tide.scene").write_text(written + "\n")
    prefab = {
        "format_version": 1, "metadata": {"name": "Basin wreck"},
        "entities": [{
            "id": "wreck", "name": "Basin wreck",
            "transform_3d": transform(0.0, 0.0, 0.05, 0.0, (7.0, 4.5)),
            "components": {
                "sindri.sprite": {"texture": "textures/wreck.png", "layer": 1},
                "sindri.tags": {"tags": ["wreck"]},
                "sindri.script": script("scripts/wreck.decay", "Wreck"),
            },
        }],
    }
    (ROOT / "prefabs").mkdir(exist_ok=True)
    (ROOT / "prefabs/wreck.prefab").write_text(json.dumps(prefab, indent=1) + "\n")


if __name__ == "__main__":
    main()
