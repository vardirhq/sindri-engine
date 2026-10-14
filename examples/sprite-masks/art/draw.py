#!/usr/bin/env python3
"""Draws the sprite-mask example's art: a meadow, a keyhole and a gem.

Writes into ../assets/textures. Deterministic and pure Python, like the
platformer's art script.
"""

import pathlib
import struct
import zlib

OUT = pathlib.Path(__file__).resolve().parent.parent / "assets" / "textures"


def png(path, width, height, pixels):
    raw = b"".join(b"\x00" + b"".join(bytes(pixel) for pixel in row) for row in pixels)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def meadow():
    """A sunny meadow: sky, sun, two rows of hills and flowers."""
    width, height = 64, 36
    sky_top, sky_low = (92, 160, 236), (186, 222, 250)
    near, far = (86, 170, 72), (126, 192, 98)
    pixels = []
    for y in range(height):
        row = []
        for x in range(width):
            color = mix(sky_top, sky_low, y / height)
            if (x - 48) ** 2 + (y - 9) ** 2 <= 20:
                color = (255, 226, 110)
            far_hill = 22 + round(3 * ((x * 7) % 23) / 23 - 2 * ((x % 16) - 8) ** 2 / 64)
            near_hill = 27 + round(((x - 20) ** 2) / 120)
            if y >= far_hill:
                color = far
            if y >= near_hill:
                color = near
                if (x * 13 + y * 7) % 17 == 0:
                    color = (238, 96, 132) if x % 2 else (255, 240, 120)
            row.append(color + (255,))
        pixels.append(row)
    png(OUT / "meadow.png", width, height, pixels)


def keyhole():
    """White where the hole is: a round head over a flared foot."""
    size = 32
    pixels = []
    for y in range(size):
        row = []
        for x in range(size):
            head = (x - 15.5) ** 2 + (y - 11.5) ** 2 <= 8.5 ** 2
            spread = 4 + (y - 14) * 0.35
            foot = 14 <= y <= 28 and abs(x - 15.5) <= spread
            row.append((255, 255, 255, 255) if head or foot else (0, 0, 0, 0))
        pixels.append(row)
    png(OUT / "keyhole.png", size, size, pixels)


def gem():
    """A cut gem, only ever seen through the keyhole."""
    size = 16
    pixels = []
    for y in range(size):
        row = []
        for x in range(size):
            d = abs(x - 7.5) + abs(y - 7.5) * 1.2
            if d > 7:
                row.append((0, 0, 0, 0))
            elif d > 6:
                row.append((60, 20, 90, 255))
            else:
                shine = 1.0 - d / 6
                row.append(mix((150, 60, 210), (240, 200, 255), shine * (x < 8)) + (255,))
        pixels.append(row)
    png(OUT / "gem.png", size, size, pixels)


def white():
    png(OUT / "white.png", 1, 1, [[(255, 255, 255, 255)]])


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    meadow()
    keyhole()
    gem()
    white()
