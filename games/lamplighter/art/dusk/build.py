"""Writes Lamplighter's art into the project and a scene that shows it.

    python3 games/lamplighter/art/dusk/build.py

Deterministic: running it again changes nothing unless a script did. It writes
`assets/textures/` (the ground sheet, the sky and hill layers, props, and Wick,
baked by the Sprite Baker's command line) and `assets/art-test.scene`, which
puts them together under the 2D lighting the game will use.
"""

import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from ground import blob_masks, ground_cell  # noqa: E402
from lib import CELL, blank, noise, paste, png  # noqa: E402
from palette import ramp  # noqa: E402
from preview import mask_at  # noqa: E402
import props  # noqa: E402
import sky as S  # noqa: E402
import wick  # noqa: E402

ASSETS = HERE.parents[1] / "assets"
TEXTURES = ASSETS / "textures"
BAKER = HERE.parents[3] / "tools" / "isometric-baker" / "src" / "cli.ts"
WIDTH, HEIGHT = 320, 180  # the logical screen, in art pixels; 16 to a world unit
UNITS_HIGH = HEIGHT / CELL  # 11.25 world units from top to bottom of the screen


def write_sheet(path, columns, names, cell_h=CELL, extra=None):
    sheet = {"format_version": 1, "anchor": "center", "grid": {"columns": columns, "rows": -(-len(names) // columns), "names": names}}
    if extra:
        sheet.update(extra)
    path.write_text(json.dumps(sheet, indent=2) + "\n")


def draw_ground():
    masks = blob_masks()
    columns = 8
    rows = -(-len(masks) // columns)
    image = blank(columns * CELL, rows * CELL)
    for index, mask in enumerate(masks):
        paste(image, ground_cell(mask), (index % columns) * CELL, (index // columns) * CELL)
    png(TEXTURES / "ground.png", columns * CELL, rows * CELL, image)
    names = [f"ground-{m}" for m in masks]
    names += [""] * (columns * rows - len(names))
    write_sheet(
        TEXTURES / "ground.sheet",
        columns,
        names,
        extra={"terrains": {"ground": {"kind": "blob", "sprites": [f"ground-{m}" for m in masks]}}},
    )
    return masks


def draw_decor():
    columns = 8
    image = blank(columns * CELL, CELL)
    for i, cell in enumerate(props.DECOR):
        paste(image, cell, i * CELL, 0)
    png(TEXTURES / "decor.png", columns * CELL, CELL, image)
    write_sheet(TEXTURES / "decor.sheet", columns, props.DECOR_NAMES + [""] * (columns - len(props.DECOR_NAMES)))


def draw_props():
    cells = []
    for lit in (False, True):
        cell = blank(CELL, 24)
        paste(cell, props.lantern(lit), 1, 7)
        cells.append(cell)
    image = blank(2 * CELL, 24)
    for i, cell in enumerate(cells):
        paste(image, cell, i * CELL, 0)
    png(TEXTURES / "props.png", 2 * CELL, 24, image)
    write_sheet(TEXTURES / "props.sheet", 2, ["lantern-unlit", "lantern-lit"])


GLOW = 56  # pixels across the halo sprite


def draw_glow():
    """Concentric amber bands, each edge broken by a checkerboard, so it reads as pixels."""
    image = blank(GLOW, GLOW)
    bands = ((26, 0.10), (19, 0.17), (12, 0.26), (6, 0.38))
    mid = (GLOW - 1) / 2
    for y in range(GLOW):
        for x in range(GLOW):
            d = ((x - mid) ** 2 + (y - mid) ** 2) ** 0.5
            alpha = 0.0
            for radius, a in bands:
                if d < radius - 1.5 or (d < radius and (x + y) % 2 == 0):
                    alpha = a
            if alpha:
                image[y][x] = (255, 166, 62, round(alpha * 255))
    png(TEXTURES / "glow.png", GLOW, GLOW, image)


def draw_backdrop():
    sky = S.sky_layer(WIDTH, HEIGHT)
    S.stars(sky)
    S.moon(sky, 262, 30)
    png(TEXTURES / "sky.png", WIDTH, HEIGHT, sky)
    far = S.hills(WIDTH, HEIGHT, base=0.66, amplitude=22, color=ramp("sky")[3], seed=0.3)
    near = S.hills(WIDTH, HEIGHT, base=0.74, amplitude=18, color=ramp("sky")[2], seed=1.9, trees=True)
    png(TEXTURES / "hills-far.png", WIDTH, HEIGHT, far)
    png(TEXTURES / "hills-near.png", WIDTH, HEIGHT, near)


def bake_wick():
    recipe = HERE / "wick.isobake"
    recipe.write_text(json.dumps(wick.recipe(), indent=2) + "\n")
    subprocess.run(["node", str(BAKER), str(recipe), "--out", str(ASSETS), "--quiet"], check=True)
    recipe.unlink()
    grid = json.loads((TEXTURES / "wick.sheet").read_text())["grid"]
    width, height = grid["size"]
    (mx, my), (gx, _) = grid["margin"], grid["spacing"]
    columns = grid["columns"]
    # One frame, in pixels: the image less its margins and the gutters between frames.
    return {"width": (width - 2 * mx - (columns - 1) * gx) // columns, "height": height - 2 * my, "columns": columns}


# --- The scene -------------------------------------------------------------

LEVEL = [
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....##..............",
    "..#######.......###.",
    "####################",
    "####################",
]
ROWS, COLUMNS = len(LEVEL), len(LEVEL[0])


def transform(x, y, z=0.0, sx=1.0, sy=1.0):
    return {"position": [x, y, z], "rotation": [0, 0, 0, 1], "scale": [sx, sy, 1]}


def tile_scene(masks):
    palette = [f"ground-{m}" for m in masks]
    tiles = []
    for r, line in enumerate(LEVEL):
        for c, ch in enumerate(line):
            tiles.append(palette.index(f"ground-{mask_at(LEVEL, r, c)}") if ch == "#" else None)
    decor_palette = props.DECOR_NAMES
    decor = [None] * (ROWS * COLUMNS)
    for r, line in enumerate(LEVEL):
        for c, ch in enumerate(line):
            above_open = r > 0 and LEVEL[r - 1][c] != "#"
            if ch == "#" and above_open and noise(c, r, 41) > 0.45:
                # Decoration sits in the open cell above the grass.
                pick = int(noise(c, r, 42) * len(decor_palette))
                decor[(r - 1) * COLUMNS + c] = pick
    return palette, tiles, decor_palette, decor


def build_scene(masks, sheet):
    palette, tiles, decor_palette, decor = tile_scene(masks)
    floor_y = ROWS - 12  # the top of the first full row of ground, in world units
    frame_w, frame_h = sheet["width"], sheet["height"]
    entities = [
        {
            "id": "camera",
            "name": "Camera",
            "transform_3d": transform(COLUMNS / 2, UNITS_HIGH / 2, 10.0),
            "components": {"sindri.camera": {"projection": "orthographic", "vertical_size": UNITS_HIGH, "near": 0.1, "far": 100.0, "fit": "height"}},
        },
        {
            "id": "environment",
            "name": "Environment",
            "components": {
                "sindri.environment": {
                    "background": [0.08, 0.07, 0.17, 1.0],
                    "shadows": {"enabled": False, "map_size": 1024, "distance": 48.0, "bias": 0.002},
                    "lighting_2d": {"enabled": True, "ambient": [0.36, 0.34, 0.55]},
                }
            },
        },
        {"id": "sky", "name": "Sky", "transform_3d": transform(COLUMNS / 2, UNITS_HIGH / 2, -3.0, 20.0, UNITS_HIGH), "components": {"sindri.sprite": {"texture": "textures/sky.png", "layer": -30, "unlit": True}}},
        {"id": "hills-far", "name": "Far hills", "transform_3d": transform(COLUMNS / 2, UNITS_HIGH / 2, -2.0, 20.0, UNITS_HIGH), "components": {"sindri.sprite": {"texture": "textures/hills-far.png", "layer": -20, "unlit": True}}},
        {"id": "hills-near", "name": "Near hills", "transform_3d": transform(COLUMNS / 2, UNITS_HIGH / 2, -1.0, 20.0, UNITS_HIGH), "components": {"sindri.sprite": {"texture": "textures/hills-near.png", "layer": -10, "unlit": True}}},
        {
            "id": "level",
            "name": "Level",
            "transform_3d": transform(0, ROWS),
            "components": {
                "sindri.tilemap": {"texture": "textures/ground.png", "palette": palette, "columns": COLUMNS, "rows": ROWS, "tiles": tiles, "layer": 0},
                "sindri.light2d.occluder": {"enabled": True, "shape": "tilemap"},
            },
        },
        {
            "id": "decor",
            "name": "Decoration",
            "transform_3d": transform(0, ROWS),
            "components": {"sindri.tilemap": {"texture": "textures/decor.png", "palette": decor_palette, "columns": COLUMNS, "rows": ROWS, "tiles": decor, "layer": 1}},
        },
        {
            "id": "wick",
            "name": "Wick",
            "transform_3d": transform(12.5, float(floor_y), 0.5, frame_w / 16.0, frame_h / 16.0),
            "components": {
                "sindri.sprite": {"texture": "textures/wick.png#idle-0", "layer": 5},
                "sindri.animation.sprite": {
                    "clips": {
                        "idle": {"frames": ["idle-0", "idle-1"], "seconds_per_frame": 0.45, "looping": True},
                        "run": {"frames": ["run-0", "run-1", "run-2", "run-3"], "seconds_per_frame": 0.09, "looping": True},
                    },
                    "playing": "run",
                    "speed": 1.0,
                },
            },
        },
    ]
    for i, (x, lit) in enumerate(((9.0, True), (14.5, False))):
        entities.append(
            {
                "id": f"lantern-{i}",
                "name": f"Lantern {i}",
                "transform_3d": transform(x, float(floor_y) + 0.75, 0.2, 1.0, 1.5),
                "components": {"sindri.sprite": {"texture": "textures/props.png#" + ("lantern-lit" if lit else "lantern-unlit"), "layer": 3, "unlit": lit}},
            }
        )
        if lit:
            # A halo in the air: a light can only brighten what is drawn, so the glow is
            # a stepped sprite of amber bands with a checker dither between them.
            entities.append(
                {
                    "id": "lantern-glow",
                    "name": "Lantern glow",
                    "transform_3d": transform(x, float(floor_y) + 1.05, 0.3, GLOW / 16, GLOW / 16),
                    "components": {"sindri.sprite": {"texture": "textures/glow.png", "layer": 4, "unlit": True}},
                }
            )
            entities.append(
                {
                    "id": "lantern-light",
                    "name": "Lantern light",
                    "transform_3d": transform(x, float(floor_y) + 1.05, 0.0),
                    "components": {"sindri.light2d": {"enabled": True, "color": [1.0, 0.7, 0.34, 1.0], "intensity": 2.6, "radius": 8.0, "falloff": 1.3, "height": 1.0, "shadows": True, "shadow_reach": 0.3}},
                }
            )
    return {"format_version": 10, "metadata": {"name": "Lamplighter art test"}, "entities": entities}


def main():
    TEXTURES.mkdir(parents=True, exist_ok=True)
    masks = draw_ground()
    draw_decor()
    draw_props()
    draw_glow()
    draw_backdrop()
    scene = build_scene(masks, bake_wick())
    (ASSETS / "art-test.scene").write_text(json.dumps(scene, indent=2) + "\n")
    print(f"wrote {len(masks)} ground tiles, {len(props.DECOR)} decorations, the backdrop, Wick and art-test.scene")


if __name__ == "__main__":
    main()
