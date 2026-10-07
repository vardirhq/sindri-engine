#!/usr/bin/env python3
"""Generate the Physics Playground: `examples/physics/assets`.

The scene, the spawnable prefabs, the physics material profiles and the one
particle texture are written here, deterministically, so geometry is edited
as numbers in one place and the authored files stay reviewable. Behavior is
the Decay in `assets/scripts`, and styling is `assets/ui/playground.weave`;
neither is generated.

    python3 scripts/physics_playground/generate.py
"""

from __future__ import annotations

import json
import math
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import hud  # noqa: E402
import toys  # noqa: E402
from common import (  # noqa: E402
    LAYERS, body, box, circle, rgba, script, shape, transform,
)
from scene import Scene  # noqa: E402

ROOT = Path(__file__).resolve().parents[2] / "examples/physics/assets"
DIRECTOR = "scripts/playground.decay"

# The room's inside: x from -30 to 30, y from -14 to 14.
LEFT, RIGHT, FLOOR, CEILING = -30.0, 30.0, -14.0, 14.0

GRAVITIES = [
    ("Earth", [0.0, -16.0]),
    ("Moon", [0.0, -3.0]),
    ("Zero-G", [0.0, 0.0]),
    ("Upside down", [0.0, 16.0]),
    ("Sideways", [16.0, 0.0]),
]


def room(scene: Scene) -> None:
    scene.add("camera", "Camera", 0.0, 0.0, 20.0, components={"sindri.camera": {
        "projection": "orthographic", "vertical_size": 40.0, "near": 0.1, "far": 100.0,
        "fit": "height",
    }, "sindri.camera.behavior": {
        "confine": None, "follow": None,
        "shake": {"decay": 2.4, "frequency": 40.0, "phase": 0.0, "strength": 0.6,
                  "trauma": 0.0},
    }})
    scene.add("environment", "Environment", components={"sindri.environment": {
        "background": rgba("#070c16"),
    }})
    for index, (name, gravity) in enumerate(GRAVITIES):
        entity = scene.add(f"gravity-{index}", f"gravity-{index}", components={
            "sindri.physics2d.world": {"gravity": gravity, "layers": LAYERS},
        })
        entity["name"] = f"gravity-{index}"
        if index:
            entity["disabled"] = True
        entity["components"]["sindri.tags"] = {"tags": [f"gravity {name.lower()}"]}

    # The back wall: a panel and two grids, the ancestor's graph paper.
    scene.deco("backdrop", 0, 0, 60, 28, fill="#0c1424", layer=-30, z=-2)
    scene.deco("grid-major", 0, 0, 60, 28, kind="grid", fill="#00000000", alpha=0.0,
               edge="#1c2d44", sw=0.0015, count=15, layer=-29, z=-1.9)
    scene.deco("grid-minor", 0, 0, 60, 28, kind="grid", fill="#00000000", alpha=0.0,
               edge="#14223a", sw=0.0008, count=60, layer=-29, z=-1.95)

    scene.wall("floor", 0, FLOOR - 0.5, 62, 1, fill="#223246")
    scene.wall("ceiling", 0, CEILING + 0.5, 62, 1, fill="#223246")
    scene.wall("left-wall", LEFT - 0.5, 0, 1, 30, fill="#223246")
    scene.wall("right-wall", RIGHT + 0.5, 0, 1, 30, fill="#223246")


def director(scene: Scene) -> None:
    scene.add("director", "Director", components={"sindri.script": script(
        DIRECTOR, "Director",
        ball="prefabs/ball.prefab", crate_box="prefabs/crate.prefab",
        plank="prefabs/plank.prefab", anvil="prefabs/anvil.prefab",
        bowling="prefabs/bowling-ball.prefab", hand_prefab="prefabs/hand.prefab",
    )})
    scene.deco("cursor", 0, 0, 0.5, 0.5, kind="ellipse", fill="#00000000", alpha=0.0,
               edge="#ffd166", sw=0.12, layer=60, z=2.0)
    scene.deco("grab-string", 0, 0, 1, 0.06, fill="#ffd166", layer=59, z=1.9)["disabled"] = True
    scene.add("blast-ring", "blast-ring", 0, -40, 1.5, 1, 1, components={
        "sindri.shape": shape("ellipse", "#ff8c42", "#ffd166", sw=0.04, layer=58, alpha=0.0),
        "sindri.script": script(DIRECTOR, "BlastRing"),
    })
    scene.add("blast-sparks", "blast-sparks", 0, -40, 1.5, components={
        "sindri.effect.burst": {
            "count": 36, "drag": 1.6, "fade": True, "layer": 57, "lifetime": 0.6,
            "size": 0.28, "speed": 14.0, "spread": 3.1416, "texture": "textures/spark.png",
            "tint": [1.0, 0.75, 0.35, 1.0],
        },
    })


def bursts(scene: Scene) -> None:
    """Particle bursts the scripts play at a point; never drawn on their own."""
    for entity_id, tint, count, speed, size in [
        ("glass-shards", [0.75, 0.93, 1.0, 1.0], 28, 7.0, 0.16),
        ("muzzle-flash", [1.0, 0.85, 0.4, 1.0], 14, 5.0, 0.22),
        ("bumper-sparks", [1.0, 0.45, 0.85, 1.0], 18, 6.0, 0.18),
        ("confetti", [1.0, 0.85, 0.3, 1.0], 80, 16.0, 0.3),
    ]:
        scene.add(entity_id, entity_id, 0, -40, 1.5, components={"sindri.effect.burst": {
            "count": count, "drag": 1.2, "fade": True, "layer": 57, "lifetime": 0.8,
            "size": size, "speed": speed, "spread": 3.1416, "texture": "textures/spark.png",
            "tint": tint,
        }})


def prefab(name, entity_id, components, sx=1.0, sy=None):
    return {
        "format_version": 1,
        "metadata": {"name": name},
        "entities": [{
            "id": entity_id, "name": name,
            "transform_3d": transform(0, 0, 0.3, sx, sy),
            "components": components,
        }],
    }


def loose(kind_shape, fill, edge, piece, mass_body, sx, sy=None, tags=("loose",)):
    return {
        "sindri.shape": shape(kind_shape, fill, edge, sw=0.07, layer=7),
        "sindri.physics2d.collider": {"pieces": [piece]},
        "sindri.physics2d.rigid_body": mass_body,
        "sindri.tags": {"tags": list(tags)},
    }


def prefabs() -> dict[str, dict]:
    return {
        "ball.prefab": prefab("Ball", "ball", loose(
            "ellipse", "#4cc9f0", "#bdf3ff", circle(0.3, "balls", 0.3, 0.6),
            body(damping=0.02, angular_damping=0.05), 0.6)),
        "crate.prefab": prefab("Crate", "crate", loose(
            "rect", "#c98b4f", "#f2c38b", box(0.45, 0.45, "props", 0.6, 0.05),
            body(damping=0.05, angular_damping=0.05), 0.9)),
        "plank.prefab": prefab("Plank", "plank", loose(
            "rect", "#d9a066", "#f6d2a0", box(1.4, 0.14, "props", 0.6, 0.05),
            body(damping=0.05, angular_damping=0.05), 2.8, 0.28)),
        "anvil.prefab": prefab("Anvil", "anvil", {
            **loose("rect", "#3d4554", "#9aa7bb", box(0.55, 0.4, "props", 0.7, 0.0),
                    body(damping=0.0, angular_damping=0.1, ccd=True), 1.1, 0.8),
            # Steel, and dense: the anvil's mass comes from its material's
            # density being the same as everything else's, so it is drawn
            # small and heavy-looking and weighs what its box weighs.
        }, 1.1, 0.8),
        "bowling-ball.prefab": prefab("Bowling ball", "bowling-ball", loose(
            "ellipse", "#5b2a86", "#c77dff", circle(0.55, "balls", 0.2, 0.15),
            body(damping=0.0, angular_damping=0.02, ccd=True), 1.1)),
        "shell.prefab": prefab("Shell", "shell", {
            "sindri.shape": shape("ellipse", "#ffdd57", "#ffffff", sw=0.25, layer=9),
            "sindri.physics2d.collider": {"pieces": [circle(0.12, "shells", 0.3, 0.2)]},
            "sindri.physics2d.rigid_body": body(damping=0.0, angular_damping=0.0, ccd=True),
            "sindri.script": script("scripts/toys.decay", "Shell"),
            "sindri.tags": {"tags": ["loose", "shell"]},
        }, 0.24),
        "hand.prefab": prefab("Hand", "hand", {
            "sindri.shape": shape("ellipse", "#ffd166", "#ffffff", sw=0.18, layer=61,
                                  alpha=0.9),
            "sindri.physics2d.collider": {"pieces": [circle(0.05, "hand", 0.0, 0.0)]},
            "sindri.physics2d.rigid_body": body("kinematic_velocity"),
        }, 0.35),
    }


MATERIALS = {
    "ice": ("Ice", 0.01, 0.05),
    "wood": ("Wood", 0.45, 0.2),
    "rubber": ("Rubber", 1.2, 0.92),
    "steel": ("Steel", 0.3, 0.45),
    "clay": ("Clay", 0.9, 0.0),
}


def profiles() -> dict[str, dict]:
    return {
        f"{key}.profile": {
            "format_version": 1, "name": name, "type": "physics_material",
            "values": {"friction": friction, "restitution": restitution},
        }
        for key, (name, friction, restitution) in MATERIALS.items()
    }


def spark_png(size: int = 32) -> bytes:
    """A soft white dot: the one texture the particle bursts need."""
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            dx = (x + 0.5) / size * 2 - 1
            dy = (y + 0.5) / size * 2 - 1
            alpha = max(0.0, 1.0 - math.sqrt(dx * dx + dy * dy)) ** 1.6
            row += bytes([255, 255, 255, int(alpha * 255)])
        rows.append(bytes(row))
    raw = zlib.compress(b"".join(rows), 9)

    def chunk(kind: bytes, data: bytes) -> bytes:
        return (struct.pack(">I", len(data)) + kind + data
                + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF))

    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", raw)
            + chunk(b"IEND", b""))


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1) + "\n")


def main() -> None:
    scene = Scene()
    room(scene)
    director(scene)
    bursts(scene)
    toys.build(scene)
    hud.build(scene)
    document = {
        "format_version": 10,
        "metadata": {"name": "Physics Playground"},
        "entities": scene.entities,
    }
    write_json(ROOT / "playground.scene", document)
    for name, value in prefabs().items():
        write_json(ROOT / "prefabs" / name, value)
    for name, value in profiles().items():
        write_json(ROOT / "materials" / name, value)
    (ROOT / "textures").mkdir(parents=True, exist_ok=True)
    (ROOT / "textures/spark.png").write_bytes(spark_png())
    print(f"wrote {len(scene.entities)} entities")


if __name__ == "__main__":
    main()
