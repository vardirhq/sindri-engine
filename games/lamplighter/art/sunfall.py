#!/usr/bin/env python3
"""The shaft of a sunfall: cracklight falling through the Lid, as a picture that
stands up from where it lands. A soft beam, narrow at its foot and wider as it
rises, leaning the way the light comes in, faint streaks down its length, and
fading to nothing towards its top so it reaches into the dark rather than ending.
White: the sprite tints it gold and `shaders/sunfall.wgsl` moves light down it.
Writes assets/textures/sunfall.png and its sheet, which stands it on its foot.
Deterministic."""
import json
import math
from pathlib import Path

from PIL import Image

WIDTH, HEIGHT = 48, 144
FOOT = (WIDTH / 2, 6.0)   # where it lands, from the bottom: its middle and half its width
TOP = (WIDTH / 2 + 9, 15.0)   # the same at its top: leaning right and wider
STRONGEST = 0.6
OUT = Path(__file__).resolve().parent.parent / 'assets' / 'textures' / 'sunfall.png'


def smooth(edge0, edge1, x):
    t = min(1.0, max(0.0, (x - edge0) / (edge1 - edge0)))
    return t * t * (3 - 2 * t)


def main():
    image = Image.new('RGBA', (WIDTH, HEIGHT))
    for row in range(HEIGHT):
        up = (HEIGHT - 1 - row) / (HEIGHT - 1)   # 0 at its foot, 1 at its top
        middle = FOOT[0] + (TOP[0] - FOOT[0]) * up
        half = FOOT[1] + (TOP[1] - FOOT[1]) * up
        # Soft where it lands, and gone well before its top.
        along = smooth(0.0, 0.08, up) * (1.0 - smooth(0.35, 1.0, up))
        for column in range(WIDTH):
            across = (column + 0.5 - middle) / half   # -1..1 inside the beam
            if abs(across) >= 1.0:
                continue
            body = math.cos(across * math.pi / 2) ** 1.5
            # Streaks along the beam: light through the uneven edge of a crack.
            streaks = 0.72 + 0.16 * math.sin(across * 7.1 + 0.6) + 0.12 * math.sin(across * 15.3 + 2.0)
            alpha = STRONGEST * body * along * streaks
            image.putpixel((column, row), (255, 255, 255, round(255 * alpha)))
    image.save(OUT, optimize=True)
    OUT.with_suffix('.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom', 'grid': {'columns': 1, 'rows': 1, 'names': ['shaft']}},
        indent=2) + '\n')


if __name__ == '__main__':
    main()
