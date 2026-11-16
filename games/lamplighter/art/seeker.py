#!/usr/bin/env python3
"""A Seeker, as it hangs over the land: a dark shell like a small piece of the
Lid itself, plated, its rim catching a little red, with one cold red eye
underneath that its beam shines down from. Writes assets/textures/seeker.png
and a sheet that hangs it from the middle of its picture. Deterministic.
"""
import json
from pathlib import Path

from PIL import Image

OUT = Path(__file__).resolve().parent.parent / 'assets' / 'textures' / 'seeker.png'
WIDTH, HEIGHT = 32, 20
SHELL, PLATE, SEAM = (22, 18, 30, 255), (38, 32, 50, 255), (12, 10, 18, 255)
RIM, EYE, GLARE = (120, 30, 38, 255), (255, 60, 60, 255), (255, 200, 190, 255)


def main():
    image = Image.new('RGBA', (WIDTH, HEIGHT))
    middle = (WIDTH - 1) / 2
    for y in range(HEIGHT):
        for x in range(WIDTH):
            across = (x - middle) / 15.0
            # A shell, domed on top and flatter underneath.
            if y < 11:
                inside = across * across + ((y - 11) / 9.0) ** 2 <= 1.0
            else:
                inside = across * across + ((y - 11) / 5.0) ** 2 <= 1.0 and y < 16
            if not inside:
                continue
            colour = SHELL
            # Plates on the dome, split by seams.
            if y < 10 and (x - 2) % 7 == 0:
                colour = SEAM
            elif y < 7:
                colour = PLATE
            # The rim, where the dome meets its underside, catches the eye's light.
            if y in (10, 11):
                colour = RIM
            image.putpixel((x, y), colour)
    # The eye underneath, and the glare at its heart.
    for y in range(12, 17):
        for x in range(13, 19):
            if (x - middle) ** 2 + ((y - 14) * 1.4) ** 2 <= 7.5:
                image.putpixel((x, y), EYE)
    for x in (15, 16):
        image.putpixel((x, 14), GLARE)
    image.save(OUT, optimize=True)
    OUT.with_suffix('.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'center', 'grid': {'columns': 1, 'rows': 1, 'names': ['seeker']}},
        indent=2) + '\n')


if __name__ == '__main__':
    main()
