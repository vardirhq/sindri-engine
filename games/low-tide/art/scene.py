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
CRAWLER = [
    "  WWwwwWW  ",
    " WW..H..WW ",
    "TW.......WT",
    "TWWWW-WWWWT",
    "TWb.....bWT",
    "Tw...G...wT",
    "TW.......hR",
    "TWb.....bWT",
    "TWWWW-WWWWT",
    "TW.......WT",
    "TW.......WT",
    "TWE.....EWT",
    "TW.......WT",
    " WWWWWWWWW ",
]
SHIP_TILES = [
    "deck", "wall", "window", "door", "hatch", "helm", "table",
    "bunk", "engine", "tread", "ramp", "crate", "mast",
]
SYMBOL = {
    ".": "deck", "W": "wall", "w": "window", "-": "door", "h": "hatch",
    "H": "helm", "G": "table", "b": "bunk", "E": "engine", "T": "tread",
    "R": "ramp",
}

GROUND_TILES = ["salt", "salt-dim", "salt-fleck", "crack", "dune", "rise"]
# The Basin is wider than the part the crawler may drive in, so a turned
# view never shows past its edge.
MARGIN = 14
COLUMNS, ROWS = 44 + 2 * MARGIN, 270
RISE_Y = 250.0
START = (MARGIN + 22.0, 14.0)

# Wrecks, each a detour from the straight line north.
WRECKS = [(MARGIN + x, y, turn) for x, y, turn in [
    (12.0, 34.0, 0.4), (33.0, 62.0, -0.7), (9.0, 98.0, 1.2),
    (32.0, 134.0, 0.2), (13.0, 170.0, -0.3), (31.0, 208.0, 0.9)]]
# Dune fields: centre x, centre y, radius x, radius y. Soft sand halves the
# crawler's speed, so these are what the route bends around.
DUNES = [(MARGIN + x, y, rx, ry) for x, y, rx, ry in [(24.0, 46.0, 7.0, 3.5), (8.0, 70.0, 6.0, 5.0), (30.0, 84.0, 9.0, 4.0),
         (18.0, 118.0, 10.0, 3.0), (36.0, 150.0, 7.0, 6.0), (10.0, 140.0, 5.0, 8.0),
         (22.0, 188.0, 12.0, 3.5), (34.0, 228.0, 6.0, 4.0), (12.0, 222.0, 7.0, 3.0)]]


def noise(x, y, salt):
    value = (x * 374761393 + y * 668265263 + salt * 2246822519) & 0xFFFFFFFF
    value = ((value ^ (value >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((value ^ (value >> 16)) & 0xFFFF) / 65536.0


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


def text(anchor, size, words, colour=(0.16, 0.1, 0.08, 1.0)):
    return {
        "anchor": anchor, "color": list(colour), "font": "fonts/Inter.ttf",
        "font_size": size, "line_height": size * 1.25, "layer": 100,
        "text": words, "bold": True,
        "shadow": {"offset": [0.0, -0.002], "color": [1.0, 0.98, 0.92, 0.7],
                   "softness": 0.003},
    }


def script(source, name, properties=None):
    return {"source": source, "script": name, "properties": properties or {}}


def ground_tiles():
    tiles = []
    for row in range(ROWS):
        y = ROWS - row - 0.5
        edge = RISE_Y + math.sin(row * 0.9) * 0.8
        for column in range(COLUMNS):
            x = column + 0.5
            ragged = (noise(column, row, 5) - 0.5) * 2.0
            if y >= edge + ragged:
                tiles.append(5)
                continue
            if any(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
                   <= 1.0 + (noise(column, row, 6) - 0.5) * 0.5
                   for cx, cy, rx, ry in DUNES):
                tiles.append(4)
                continue
            roll = noise(column, row, 7)
            if roll < 0.05:
                tiles.append(3)
            elif roll < 0.25:
                tiles.append(1)
            elif roll < 0.35:
                tiles.append(2)
            else:
                tiles.append(0)
    return tiles


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
                "sindri.camera": {"projection": "orthographic", "vertical_size": 28.0,
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
                    {"name": "view", "kind": "button",
                     "bindings": ["key.Tab", "key.V", "gamepad.north"]},
                ]},
                "sindri.script": script("scripts/voyage.decay", "Voyage"),
            },
        },
        {
            "id": "basin", "name": "Basin",
            "transform_3d": transform(0.0, float(ROWS)),
            "components": {"sindri.tilemap": {
                "texture": "textures/ground.png", "palette": GROUND_TILES,
                "columns": COLUMNS, "rows": ROWS, "tiles": ground_tiles(),
            }},
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
            "id": "shadow", "name": "Crawler shadow",
            "transform_3d": transform(START[0] + 0.4, START[1] - 0.4, 0.08, 0.0,
                                      (width - 0.6, height - 0.6)),
            "components": {"sindri.shape": shape([0.0, 0.0, 0.0, 0.22], 2, 0.06)},
        },
        {
            "id": "crawler", "name": "Crawler",
            "transform_3d": transform(START[0], START[1], 0.1),
            "components": {"sindri.script": script("scripts/crawler.decay", "Crawler")},
        },
        {
            "id": "deck", "name": "Deck", "parent": "crawler",
            "transform_3d": transform(-width / 2, height / 2),
            "components": {"sindri.tilemap": {
                "texture": "textures/ship.png", "palette": SHIP_TILES,
                "columns": width, "rows": height, "tiles": crawler_tiles(),
            }},
        },
        {
            "id": "crew", "name": "Crew", "parent": "deck",
            "transform_3d": transform(5.5, -6.5, 0.1, 0.0, (1.1, 1.1)),
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
            "components": {"sindri.sprite": {"texture": "textures/ship.png#crate", "layer": 7}},
            "disabled": True,
        },
        {
            "id": "tide", "name": "Tide",
            "transform_3d": transform(COLUMNS / 2, -4.0 - 150.0, 0.4, 0.0, (COLUMNS + 40.0, 300.0)),
            "components": {
                "sindri.shape": shape([0.05, 0.42, 0.42, 0.86], 20),
                "sindri.script": script("scripts/tide.decay", "Tide"),
            },
        },
        {
            "id": "foam", "name": "Foam",
            "transform_3d": transform(COLUMNS / 2, -4.0, 0.41, 0.0, (COLUMNS + 40.0, 0.5)),
            "components": {"sindri.shape": shape([0.75, 0.95, 0.92, 0.9], 21)},
        },
        {
            "id": "status", "name": "Status",
            "transform_3d": transform(0.03, -0.03),
            "components": {
                "sindri.ui.text": text("top_left", 0.036, "Scrap 0"),
                "sindri.script": script("scripts/hud.decay", "Hud"),
            },
        },
        {
            "id": "tide-gap", "name": "Tide gap",
            "transform_3d": transform(-0.03, -0.03),
            "components": {"sindri.ui.text": text("top_right", 0.036, "Tide 18 m behind",
                                                  (0.02, 0.3, 0.32, 1.0))},
        },
        {
            "id": "hint", "name": "Hint",
            "transform_3d": transform(0.0, 0.04),
            "components": {"sindri.ui.text": text("bottom", 0.032, "")},
        },
        {
            "id": "banner", "name": "Banner",
            "transform_3d": transform(0.0, 0.05),
            "components": {"sindri.ui.text": text("center", 0.07, "", (1.0, 0.97, 0.9, 1.0)) | {
                "shadow": {"offset": [0.0, -0.004], "color": [0.1, 0.06, 0.04, 0.8], "softness": 0.004}}},
            "disabled": True,
        },
    ]
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


if __name__ == "__main__":
    main()
