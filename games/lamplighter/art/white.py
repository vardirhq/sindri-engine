#!/usr/bin/env python3
"""A plain white square, which the HUD's bars and the hit flash are tinted from. Writes
assets/textures/white.png."""
from pathlib import Path
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / 'assets' / 'textures' / 'white.png'


def main():
    Image.new('RGBA', (4, 4), (255, 255, 255, 255)).save(OUT)


if __name__ == '__main__':
    main()
