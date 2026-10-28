#!/usr/bin/env python3
"""The soft round mote the gloom's smoke and surge are made of: a white disc that
fades to nothing at its edge, tinted and faded by the emitter. Writes
assets/textures/mote.png."""
from pathlib import Path
from PIL import Image

SIZE = 16
OUT = Path(__file__).resolve().parent.parent / 'assets' / 'textures' / 'mote.png'


def main():
    image = Image.new('RGBA', (SIZE, SIZE))
    centre = (SIZE - 1) / 2
    for y in range(SIZE):
        for x in range(SIZE):
            distance = ((x - centre) ** 2 + (y - centre) ** 2) ** 0.5 / (SIZE / 2)
            alpha = max(0.0, 1.0 - distance) ** 1.6
            image.putpixel((x, y), (255, 255, 255, round(255 * alpha)))
    image.save(OUT)


if __name__ == '__main__':
    main()
