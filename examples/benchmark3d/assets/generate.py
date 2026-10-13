"""Reproduce the fixed 300-prop workload, or verify it with --check."""
import json
import math
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def entity(name, position, scale, components):
    return {"id": name, "name": name,
            "transform_3d": {"position": position, "scale": scale},
            "components": components}


def scene():
    camera = entity("Camera", [34, 36, 46], [1, 1, 1], {
        "sindri.camera": {"projection": "perspective", "vertical_fov_degrees": 35,
                          "near": 0.1, "far": 200},
        "sindri.script": {"source": "benchmark.decay", "script": "Benchmark"}})
    sun = entity("Sun", [0, 20, 0], [1, 1, 1], {
        "sindri.light": {"kind": "directional", "intensity": 1.0}})
    sy, cy = math.sin(math.radians(15)), math.cos(math.radians(15))
    sp, cp = math.sin(math.radians(-25)), math.cos(math.radians(-25))
    sun["transform_3d"]["rotation"] = [sp * cy, cp * sy, -sp * sy, cp * cy]
    world = entity("World", [0, 0, 0], [1, 1, 1], {
        "sindri.environment": {"ambient_intensity": 0.35,
                               "shadows": {"enabled": True, "distance": 100,
                                           "map_size": 1024, "bias": 0.002}}})
    floor = entity("Floor", [0, -0.1, 0], [54, 0.2, 20], {
        "sindri.mesh": {"primitive": "cube", "texture": "textures/floor.png"}})
    entities = [camera, sun, world, floor]
    pyramid = {"vertices": [[-0.5, -0.5, -0.5], [0.5, -0.5, -0.5],
                            [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5], [0, 0.5, 0]],
               "uvs": [[0, 0], [1, 0], [1, 1], [0, 1], [0.5, 0.5]],
               "indices": [0, 1, 2, 0, 2, 3, 0, 4, 1, 1, 4, 2,
                           2, 4, 3, 3, 4, 0]}
    for kind, offset in [("cube", -16), ("surface", 0), ("model", 16)]:
        for row in range(10):
            for column in range(10):
                if kind == "model":
                    components = {"sindri.model": {"asset": "models/prop.glb"}}
                else:
                    mesh = {"primitive": kind, "texture": f"textures/{kind}.png"}
                    if kind == "surface":
                        mesh["surface"] = pyramid
                    components = {"sindri.mesh": mesh}
                entities.append(entity(f"{kind}-{row}-{column}",
                                       [offset + (column - 4.5) * 1.6, 0.6,
                                        (row - 4.5) * 1.6], [0.9, 1.2, 0.9], components))
    return {"format_version": 10, "metadata": {"name": "3D Benchmark"},
            "entities": entities}


files = {HERE / "benchmark.scene": (json.dumps(scene(), indent=2) + "\n").encode()}
for destination, source in {
    "models/prop.glb": "examples/model-collision/assets/models/prop.glb",
    "textures/cube.png": "examples/transform/assets/textures/walker.png",
    "textures/surface.png": "examples/transform/assets/textures/goal.png",
    "textures/floor.png": "examples/transform/assets/textures/floor.png",
}.items():
    files[HERE / destination] = (ROOT / source).read_bytes()
for path, content in files.items():
    if "--check" in sys.argv:
        if not path.exists() or path.read_bytes() != content:
            raise SystemExit(f"stale fixture: {path.relative_to(ROOT)}")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
print("3D benchmark fixture: 100 cubes, 100 surfaces, 100 shared models")
