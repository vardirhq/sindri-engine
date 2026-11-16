#!/usr/bin/env python3
"""The shaft of a sunfall, and the beam a Seeker shines down: cracklight falling through the Lid, as a picture that
stands up from where it lands. A soft beam, narrow at its foot and wider as it
rises, leaning the way the light comes in, faint streaks down its length, and
fading to nothing towards its top so it reaches into the dark rather than ending.
A Seeker's beam is the other way about: narrow where it leaves the Seeker,
wide where its circle falls, and straight down. White: each sprite tints its own
(gold, red) and `shaders/sunfall.wgsl` moves light down it. Writes
assets/textures/sunfall.png and seeker-beam.png, each with a sheet that stands it
on its foot.
Deterministic."""
import json
import math
from pathlib import Path

from PIL import Image

TEXTURES = Path(__file__).resolve().parent.parent / 'assets' / 'textures'
# Each beam: its picture's size, its middle and half its width at its foot and
# at its top, where it fades (a share of its height), and how strong it is.
BEAMS = {
    'sunfall': {'size': (48, 144), 'foot': (24, 6.0), 'top': (33, 15.0), 'fade': (0.35, 1.0), 'strongest': 0.6},
    'seeker-beam': {'size': (32, 80), 'foot': (16, 14.0), 'top': (16, 2.5), 'fade': (0.9, 1.0), 'strongest': 0.7},
}


def smooth(edge0, edge1, x):
    t = min(1.0, max(0.0, (x - edge0) / (edge1 - edge0)))
    return t * t * (3 - 2 * t)


def beam(name, size, foot, top, fade, strongest):
    width, height = size
    image = Image.new('RGBA', size)
    for row in range(height):
        up = (height - 1 - row) / (height - 1)   # 0 at its foot, 1 at its top
        middle = foot[0] + (top[0] - foot[0]) * up
        half = foot[1] + (top[1] - foot[1]) * up
        # Soft where it lands, and gone by its top.
        along = smooth(0.0, 0.08, up) * (1.0 - smooth(fade[0], fade[1], up))
        for column in range(width):
            across = (column + 0.5 - middle) / half   # -1..1 inside the beam
            if abs(across) >= 1.0:
                continue
            body = math.cos(across * math.pi / 2) ** 1.5
            # Streaks along the beam: light through the uneven edge of a crack.
            streaks = 0.72 + 0.16 * math.sin(across * 7.1 + 0.6) + 0.12 * math.sin(across * 15.3 + 2.0)
            alpha = strongest * body * along * streaks
            image.putpixel((column, row), (255, 255, 255, round(255 * alpha)))
    out = TEXTURES / f'{name}.png'
    image.save(out, optimize=True)
    out.with_suffix('.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom', 'grid': {'columns': 1, 'rows': 1, 'names': ['shaft']}},
        indent=2) + '\n')


def main():
    for name, shape in BEAMS.items():
        beam(name, **shape)


if __name__ == '__main__':
    main()
