"""Shared pieces of Lamplighter's art scripts: PNGs, repeatable noise, colors.

Pure Python, no imaging library, deterministic: running a script again changes
nothing unless the script did.
"""

import math
import pathlib
import struct
import zlib

CELL = 16
CLEAR = (0, 0, 0, 0)


def png(path, width, height, pixels):
    """Writes RGBA `pixels` (rows of (r, g, b, a) tuples) as a PNG."""
    raw = b"".join(b"\x00" + b"".join(bytes(pixel) for pixel in row) for row in pixels)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    pathlib.Path(path).write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def noise(x, y, salt=0):
    """A repeatable scatter in [0, 1)."""
    value = (x * 374761393 + y * 668265263 + salt * 2246822519) & 0xFFFFFFFF
    value = ((value ^ (value >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((value ^ (value >> 16)) & 0xFFFF) / 65536.0


def hex_color(text, alpha=255):
    text = text.lstrip("#")
    return tuple(int(text[i : i + 2], 16) for i in (0, 2, 4)) + (alpha,)


def blank(width, height):
    return [[CLEAR] * width for _ in range(height)]


def paste(image, cell, ox, oy):
    for y, row in enumerate(cell):
        for x, pixel in enumerate(row):
            if pixel[3]:
                image[oy + y][ox + x] = pixel
