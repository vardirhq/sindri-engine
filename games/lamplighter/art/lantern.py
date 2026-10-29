#!/usr/bin/env python3
"""Wick's lantern: a small brass lantern with a lit glass, drawn on its own so it can float
about him rather than being part of his sprite. Twelve by sixteen pixels, with its handle
ring on top; writes assets/textures/lantern.png. Needs Pillow; deterministic."""
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / 'assets' / 'textures' / 'lantern.png'
PALETTE = {
    '.': (0, 0, 0, 0),
    'o': (36, 22, 18, 255),      # the dark outline
    'b': (176, 120, 48, 255),    # brass
    'B': (118, 76, 32, 255),     # brass in shade
    'g': (255, 190, 80, 255),    # the glass, warm
    'f': (255, 232, 150, 255),   # the flame's heart
    'w': (255, 250, 214, 255),   # its brightest point
}
ART = [
    '....oooo....',
    '...ob..bo...',
    '...o....o...',
    '..oooooooo..',
    '.obbbbbbbbo.',
    '.oBoooooooo.',
    '.ogggffggBo.',
    '.ogfffwfggo.',
    '.ogfffwfgBo.',
    '.ogggffggBo.',
    '.oBoooooooo.',
    '.obbbbbbbBo.',
    '..oBBBBBBo..',
    '...oooooo...',
    '............',
    '............',
]


def main():
    image = Image.new('RGBA', (12, 16))
    for y, row in enumerate(ART):
        assert len(row) == 12, row
        for x, key in enumerate(row):
            image.putpixel((x, y), PALETTE[key])
    image.save(OUT)


if __name__ == '__main__':
    main()
