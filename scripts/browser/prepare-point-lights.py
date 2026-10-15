"""Derive local-light controls from the checked 3D Benchmark and ready script."""
import json
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
subprocess.run([sys.executable, str(root / "scripts/browser/prepare-surface-normals.py"), str(out)], check=True)
path = out / "assets/benchmark.scene"
scene = json.loads(path.read_text())
for entity in scene["entities"]:
    components = entity["components"]
    if "sindri.environment" in components:
        components["sindri.environment"]["ambient_intensity"] = 0.025
    if "sindri.light" in components:
        components["sindri.light"]["intensity"] = 0
    if "sindri.material" in components:
        components["sindri.material"].update(base_color=[0.7, 0.7, 0.7, 1], emissive=[0, 0, 0])
# A scaled, turned carrier proves that the light uses the composed position;
# its range remains a distance in world units. The controls differ only in
# the carrier's active state, leaving geometry, materials and ambient identical.
scene["entities"].append({"id": "light-carrier", "name": "Light carrier", "disabled": "--off" in sys.argv,
    "transform_3d": {"position": [0, 0, 0], "rotation": [0, 0.70710677, 0, 0.70710677], "scale": [2, 2, 2]}})
for name, position, color in [("Warm point", [0.5, 1.5, -1.25], [1, 0.12, 0.025]),
                               ("Blue point", [-0.5, 1.5, 1.25], [0.025, 0.15, 1])]:
    scene["entities"].append({"id": name.replace(" ", "-"), "name": name, "parent": "light-carrier",
        "transform_3d": {"position": position},
        "components": {"sindri.light": {"kind": "point", "color": color, "intensity": 24, "range": 6}}})
path.write_text(json.dumps(scene, indent=2) + "\n")
