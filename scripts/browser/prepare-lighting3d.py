"""Lighting Lab controls: lights, sidedness, or reusable material delivery."""
import argparse
import json
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("out", type=Path)
modes = parser.add_mutually_exclusive_group()
modes.add_argument("--single-sided", action="store_true")
modes.add_argument("--inline-materials", action="store_true")
modes.add_argument("--green-materials", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
out = args.out
shutil.copytree(root / "examples/lighting3d", out, dirs_exist_ok=True)
path = out / "assets/lighting.scene"
scene = json.loads(path.read_text())
if args.single_sided:
    panel = next(e for e in scene["entities"] if e["id"] == "Backface-double")
    panel["components"]["sindri.material"].update(override_sidedness=True, sidedness="single")
elif args.inline_materials:
    # Only the lab's full-value profiles are inlined; retain explicit local choices.
    for entity in scene["entities"]:
        material = entity.get("components", {}).get("sindri.material", {})
        if not material.get("profile"):
            continue
        profile = json.loads((out / "assets" / material["profile"]).read_text())
        for key, value in profile["values"].items():
            override = "override_sidedness" if key == "sidedness" else "override_factors"
            if not material.get(override, False):
                material[key] = value
        for key in ["profile", "override_factors", "override_sidedness"]:
            material.pop(key, None)
elif args.green_materials:
    profile_path = out / "assets/materials/polished.profile"
    profile = json.loads(profile_path.read_text())
    profile["values"]["base_color"] = [0.1, 0.8, 0.1, 1.0]
    profile["values"]["emissive"] = [0.0, 0.8, 0.0]
    profile_path.write_text(json.dumps(profile, indent=2) + "\n")
else:
    camera = next(e for e in scene["entities"] if e["id"] == "Camera")
    camera["components"]["sindri.script"]["properties"]["lit"] = False
path.write_text(json.dumps(scene, indent=2) + "\n")
