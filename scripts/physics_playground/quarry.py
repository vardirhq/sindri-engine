"""The Voxel Quarry: the playground's 3D annex.

A flat voxel quarry with stepped stone walls, crates stacked on it, and a
laser that sweeps across firing a 3D ray straight down. The floor is a
`sindri.voxel_world` with a voxel collider, so digging under a crate drops it
into the hole. Written here like the playground: scene, block set, prefab and
textures, all deterministic.
"""

from __future__ import annotations

import math
import random
import struct
import zlib

from common import rgba, script
from hud import button, container, label
from scene import Scene

QUARRY = "scripts/quarry.decay"
GROUND = 6  # the flat floor's top layer of voxels
TOP = GROUND + 1  # where its surface is: the top of that layer
ALL = 4294967295
# The `cube` mesh primitive spans -1 to 1: two units wide at scale 1. A crate
# whose collider is one unit across is drawn at half scale to match it.
CUBE = 0.5


def look_at(eye, target):
    """A quaternion turning a camera's -Z forward towards the target."""
    dx, dy, dz = (target[i] - eye[i] for i in range(3))
    yaw = math.atan2(-dx, -dz)
    pitch = math.asin(dy / math.sqrt(dx * dx + dy * dy + dz * dz))
    cy, sy = math.cos(yaw / 2), math.sin(yaw / 2)
    cp, sp = math.cos(pitch / 2), math.sin(pitch / 2)
    # yaw about Y, then pitch about the turned X.
    return [round(cy * sp, 6), round(sy * cp, 6), round(-sy * sp, 6), round(cy * cp, 6)]


def at(x, y, z, scale=1.0, rotation=None):
    return {
        "position": [x, y, z],
        "rotation": rotation or [0.0, 0.0, 0.0, 1.0],
        "scale": [scale, scale, scale] if not isinstance(scale, list) else scale,
    }


def crate_components(texture="textures/crate.png", tags=("crate",), home=True):
    components = {
        "sindri.mesh": {"layer": 0, "primitive": "cube", "texture": texture},
        "sindri.physics3d.rigid_body": {
            "kind": "dynamic", "position": [0, 0, 0], "rotation": [0, 0, 0, 1],
            "linear_velocity": [0, 0, 0], "angular_velocity": [0, 0, 0],
            "gravity_scale": 1.0, "linear_damping": 0.05, "angular_damping": 0.1,
            "lock_rotation": False,
        },
        "sindri.physics3d.collider": {"pieces": [{
            "shape": {"shape": "box", "half_extents": [0.5, 0.5, 0.5]},
            "offset": [0, 0, 0], "rotation": [0, 0, 0, 1], "sensor": False,
            "layers": {"memberships": ALL, "filter": ALL},
            "friction": 0.7, "restitution": 0.1,
        }]},
        "sindri.tags": {"tags": list(tags)},
    }
    if home:
        components["sindri.script"] = script(QUARRY, "QuarryCrate")
    return components


# Stepped stone walls along two sides, raised on the flat floor: edits that
# fill cells above the generated surface.
def walls() -> list[dict]:
    edits = []
    for x in range(-9, 10):
        for z in range(-9, -5):
            for y in range(TOP, TOP + (-5 - z)):
                edits.append({"at": [x, y, z], "block": "stone"})
    for z in range(-5, 10):
        for x in range(6, 10):
            for y in range(TOP, TOP + (x - 5)):
                edits.append({"at": [x, y, z], "block": "stone"})
    # A sand bank in the corner, to dig into.
    for x in range(-8, -4):
        for z in range(4, 8):
            edits.append({"at": [x, GROUND, z], "block": "sand"})
    return edits


def build() -> dict:
    scene = Scene()
    eye, target = (9.0, 13.5, 10.5), (-0.5, TOP, 0.5)
    camera = scene.add("q-camera", "Quarry Camera")
    camera["transform_3d"] = at(*eye, rotation=look_at(eye, target))
    camera["components"]["sindri.camera"] = {
        "projection": "perspective", "vertical_fov_degrees": 45.0, "near": 0.1, "far": 200.0,
    }
    sun = scene.add("q-sun", "Quarry Sun")
    sun["transform_3d"] = at(0, 30, 0, rotation=[-0.416852, 0.401068, 0.212271, 0.787604])
    sun["components"]["sindri.light"] = {"kind": "directional", "color": [1.0, 0.93, 0.8],
                                         "intensity": 1.1}
    scene.add("q-environment", "Quarry Environment", components={"sindri.environment": {
        "background": rgba("#1d3348"), "ambient_intensity": 0.45,
    }})
    scene.add("q-physics", "Quarry Physics", components={"sindri.physics3d.world": {
        "gravity": [0.0, -16.0, 0.0], "layers": ["solid"],
    }})
    floor = scene.add("q-floor", "Quarry Floor")
    floor["components"] = {
        "sindri.voxel_world": {
            "blocks": "quarry.tileset",
            "generator": {"kind": "layered_terrain", "base_height": GROUND,
                          "height_variation": 0, "surface_voxel": "grass",
                          "subsurface_voxel": "dirt", "deep_voxel": "stone",
                          "subsurface_depth": 3},
            "render_radius": 2, "vertical_radius": 1, "focus": [0, 0, 0],
            "edits": walls(),
        },
        "sindri.physics3d.voxel_collider": {
            "friction": 0.8, "layers": {"filter": ALL, "memberships": ALL},
            "margin": 2.0, "restitution": 0.0,
        },
    }

    # A tower, a pyramid and a row of crates to dig out from under.
    crates = []
    for level in range(5):
        crates.append((-2.5, TOP + 0.5 + level * 1.0, 0.5))
    for level, count in enumerate([3, 2, 1]):
        for i in range(count):
            crates.append((1.0 + i * 1.05 + level * 0.52, TOP + 0.5 + level, 3.5))
    for i in range(4):
        crates.append((-6.0 + i * 1.6, TOP + 0.5, -3.0))
    for index, (x, y, z) in enumerate(crates):
        texture = "textures/crate.png" if index % 4 else "textures/steel.png"
        entity = scene.add(f"q-crate-{index}", f"q-crate-{index}")
        entity["transform_3d"] = at(x, y, z, CUBE)
        entity["components"] = crate_components(texture)

    # The laser: a thin red column swept across the quarry, firing a ray down.
    laser = scene.add("q-laser", "q-laser")
    laser["transform_3d"] = at(0, 14, 0.5, [0.03, 0.5, 0.03])
    laser["components"]["sindri.mesh"] = {"layer": 0, "primitive": "cube",
                                          "texture": "textures/laser.png"}
    spot = scene.add("q-laser-hit", "q-laser-hit")
    spot["transform_3d"] = at(0, TOP, 0.5, 0.15)
    spot["components"]["sindri.mesh"] = {"layer": 0, "primitive": "cube",
                                         "texture": "textures/laser.png"}

    scene.add("q-director", "Quarry Director", components={"sindri.script": script(
        QUARRY, "Quarry", crate_prefab="prefabs/crate3d.prefab")})
    hud(scene)
    return {"format_version": 10, "metadata": {"name": "Voxel Quarry"},
            "entities": scene.entities}


def hud(scene: Scene) -> None:
    container(scene, "q-hud", None, ["hud"], "column", "space_between", "center", 3.4, 2.0)
    container(scene, "q-top", "q-hud", ["top"], "row", "space_between", "start", 3.3, 0.2)
    container(scene, "q-brand", "q-top", ["brand"], "column", "start", "start", 1.0, 0.2)
    label(scene, "q-title", "q-brand", "VOXEL QUARRY", ["title"], 24, (1.0, 0.08))
    label(scene, "q-stats", "q-brand", "", ["stats"], 11, (1.2, 0.05))
    label(scene, "q-readout", "q-brand", "", ["readout"], 11, (1.6, 0.08))
    container(scene, "q-globals", "q-top", ["globals"], "row", "end", "center", 2.2, 0.1)
    button(scene, "q-back", "q-globals", "< THE ROOM", ["loud"], 0.3)
    button(scene, "q-reset", "q-globals", "RESET", [], 0.22)
    container(scene, "q-bottom", "q-hud", ["bottom"], "column", "end", "center", 3.3, 0.4)
    container(scene, "q-dock", "q-bottom", ["dock"], "row", "center", "center", 3.3, 0.11)
    container(scene, "q-actions", "q-dock", ["actions"], "row", "center", "center", 3.0, 0.1)
    for action, words in enumerate(["DROP CRATES", "DIG UNDER", "SHOVE", "STOP LASER"], 1):
        button(scene, f"q-act-{action}", "q-actions", words, ["action"], 0.36)
    label(scene, "q-hint", "q-bottom",
          "Tap or click the ground to dig a block. 1-4 work the quarry; K goes back to the room.",
          ["hint"], 11, (2.6, 0.05))


TILESET = {
    "format_version": 1,
    "tiles": {
        name: {"faces": {face: {"sprite": f"textures/{sprite}.png", "size": [1.0, 1.0]}
                         for face, sprite in faces.items()}}
        for name, faces in {
            "grass": {"top": "grass", "bottom": "dirt", "north": "grass-side",
                      "south": "grass-side", "east": "grass-side", "west": "grass-side"},
            "dirt": {face: "dirt" for face in ("top", "bottom", "north", "south", "east", "west")},
            "stone": {face: "stone" for face in ("top", "bottom", "north", "south", "east",
                                                 "west")},
            "sand": {face: "sand" for face in ("top", "bottom", "north", "south", "east", "west")},
        }.items()
    },
}


def crate_prefab() -> dict:
    return {
        "format_version": 1,
        "metadata": {"name": "Crate 3D"},
        "entities": [{
            "id": "crate3d", "name": "Crate 3D",
            "transform_3d": at(0, 0, 0, CUBE),
            "components": crate_components(tags=("crate", "dropped"), home=False),
        }],
    }


def png(size, pixel) -> bytes:
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            row += bytes(pixel(x, y))
        rows.append(bytes(row))
    raw = zlib.compress(b"".join(rows), 9)

    def chunk(kind: bytes, data: bytes) -> bytes:
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF))

    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", raw)
            + chunk(b"IEND", b""))


def speckled(base, spread, seed):
    rng = random.Random(seed)
    noise = [[rng.uniform(-spread, spread) for _ in range(16)] for _ in range(16)]

    def pixel(x, y):
        shade = noise[y][x]
        return [max(0, min(255, int(c + shade))) for c in base] + [255]
    return pixel


def textures() -> dict[str, bytes]:
    grass = speckled((92, 158, 72), 18, 1)
    dirt = speckled((128, 92, 62), 16, 2)
    stone = speckled((128, 132, 140), 20, 3)
    sand = speckled((216, 196, 140), 12, 4)

    def grass_side(x, y):
        return grass(x, y) if y < 4 + (x * 7 % 3) else dirt(x, y)

    def planks(base, edge):
        wood = speckled(base, 14, 5)

        def pixel(x, y):
            if x in (0, 15) or y in (0, 15) or x == y or x == 15 - y:
                return list(edge) + [255]
            if y % 5 == 0:
                return [int(c * 0.8) for c in base] + [255]
            return wood(x, y)
        return pixel

    def laser(x, y):
        return [255, 60 + (x + y) % 4 * 10, 70, 255]

    return {
        "grass.png": png(16, grass),
        "grass-side.png": png(16, grass_side),
        "dirt.png": png(16, dirt),
        "stone.png": png(16, stone),
        "sand.png": png(16, sand),
        "crate.png": png(16, planks((182, 128, 74), (96, 62, 34))),
        "steel.png": png(16, planks((120, 132, 150), (200, 210, 225))),
        "laser.png": png(4, laser),
    }
