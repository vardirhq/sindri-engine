"""Fails when Causeway's checked-in art and scene differ from what
`causeway_art.py` and `level.py` make.

Run after both scripts. Models, scenes and prefabs must match byte for byte.
PNGs are compared by pixel, because the same image compresses to different
bytes under different zlib builds and that is not drift.
"""

import subprocess
import sys
from io import BytesIO

from PIL import Image

ASSETS = "game/assets"


def changed():
    out = subprocess.run(
        ["git", "diff", "--name-only", "--", ASSETS],
        check=True, capture_output=True, text=True,
    ).stdout
    return [line for line in out.splitlines() if line]


def pixels(data):
    image = Image.open(BytesIO(data))
    return image.mode, image.size, image.tobytes()


def main():
    drifted = []
    for path in changed():
        if not path.endswith(".png"):
            drifted.append(path)
            continue
        before = subprocess.run(
            ["git", "show", f"HEAD:{path}"], check=True, capture_output=True
        ).stdout
        with open(path, "rb") as now:
            if pixels(before) != pixels(now.read()):
                drifted.append(path)
    for path in drifted:
        print(f"{path} is not what its script makes; run tools/causeway_art.py and tools/level.py")
    return 1 if drifted else 0


if __name__ == "__main__":
    sys.exit(main())
