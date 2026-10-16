"""Derive a smooth/flat ellipsoid control from the checked 3D Benchmark."""
import json
import math
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
subprocess.run([sys.executable, str(root / "scripts/browser/prepare-standard-materials.py"), str(out)], check=True)
path = out / "assets/benchmark.scene"
scene = json.loads(path.read_text())
# Light the visible hemisphere so the control shows the full shading gradient.
camera, sun = scene["entities"][:2]
sun["transform_3d"]["rotation"] = camera["transform_3d"]["rotation"]
entity = next(e for e in scene["entities"] if e["id"] == "Swatch-3")
# Exercise orientation-aware color/shadow pipelines in the browser control too.
entity["transform_3d"]["scale"][0] *= -1
vertices, normals, uvs, indices = [], [], [], []
# Deliberately coarse geometry: normals visibly smooth the lighting, not edges.
rows, columns = 8, 16
for row in range(rows + 1):
    latitude = math.pi * row / rows
    for column in range(columns + 1):
        longitude = math.tau * column / columns
        normal = [math.sin(latitude) * math.cos(longitude), math.cos(latitude), math.sin(latitude) * math.sin(longitude)]
        vertices.append([0.65 * x for x in normal])
        normals.append(normal)
        uvs.append([column / columns, row / rows])
for row in range(rows):
    for column in range(columns):
        a = row * (columns + 1) + column
        b = a + columns + 1
        if row != 0:
            indices.extend([a, a + 1, b])
        if row != rows - 1:
            indices.extend([a + 1, b + 1, b])
surface = {"vertices": vertices, "uvs": uvs, "indices": indices}
if "--flat" not in sys.argv:
    surface["normals"] = normals
entity["components"]["sindri.mesh"]["surface"] = surface
entity["components"]["sindri.material"].update(metallic=0.0, roughness=0.55)
path.write_text(json.dumps(scene, indent=2) + "\n")
