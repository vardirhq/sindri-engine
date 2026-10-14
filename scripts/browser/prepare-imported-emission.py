"""Derive a GLB-authored emission control; no scene material overrides."""
import json
from pathlib import Path
import struct
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
subprocess.run([sys.executable, str(root / "scripts/browser/prepare-standard-materials.py"), str(out), "--legacy"], check=True)
path = out / "assets/models/prop.glb"
data = path.read_bytes()
json_length, json_kind = struct.unpack_from("<II", data, 12)
assert json_kind == 0x4E4F534A
model = json.loads(data[20:20 + json_length])
for material in model["materials"]:
    material["emissiveFactor"] = [0.05, 0.35, 1.0] if "--no-emission" not in sys.argv else [0, 0, 0]
encoded = json.dumps(model, separators=(",", ":")).encode()
encoded += b" " * (-len(encoded) % 4)
remaining = data[20 + json_length:]
path.write_bytes(struct.pack("<IIIII", 0x46546C67, 2, 20 + len(encoded) + len(remaining), len(encoded), json_kind) + encoded + remaining)
