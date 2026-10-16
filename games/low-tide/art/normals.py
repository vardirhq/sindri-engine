#!/usr/bin/env python3
"""Writes a normal map beside each lit picture, from the picture itself.

A 2D light rakes across a sprite only where the sprite says which way its
surface faces. Drawing a second picture of every board, plate, rivet and crate
by hand would double `draw.py`, so the relief is read back from the finished
art instead: dark lines are grooves and seams, light marks stand proud, and the
edge of anything with a transparent outline is bevelled down to the ground.

The result is laid out exactly like its picture, cell for cell, so a sheet and
its normal map are drawn as one (`docs/2d-lights.md`). Red is right, green is
up the world, blue is out of the picture. A height is never read across a cell
boundary, so one cell's relief does not leak into its neighbour.

Run by `draw.py`; deterministic and pure Python like it.
"""

import pathlib
import struct
import zlib

from draw import OUT, png

# Picture, cell size, how strongly brightness becomes height, how far the
# outline bevel reaches in pixels.
LIT = [
    ("ship", 32, 2.6, 3),
    ("treads", 32, 2.4, 2),
    ("crew", 32, 1.4, 3),
    ("wreck", None, 2.8, 4),
    ("dive", 32, 2.0, 3),
    ("terrain", 32, 3.0, 0),
    ("pipe", None, 2.0, 3),
]


def read_png(path):
    """Decodes the 8-bit RGBA PNGs `png()` writes, every filter included."""
    data = pathlib.Path(path).read_bytes()
    at, idat, width, height = 8, b"", 0, 0
    while at < len(data):
        (length,) = struct.unpack(">I", data[at : at + 4])
        kind = data[at + 4 : at + 8]
        body = data[at + 8 : at + 8 + length]
        if kind == b"IHDR":
            width, height = struct.unpack(">II", body[:8])
            if body[8] != 8 or body[9] != 6:
                raise ValueError(f"{path}: only 8-bit RGBA is read")
        elif kind == b"IDAT":
            idat += body
        at += 12 + length
    raw = zlib.decompress(idat)
    stride = width * 4
    rows, previous = [], bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1 : start + 1 + stride])
        for i in range(stride):
            left = line[i - 4] if i >= 4 else 0
            up = previous[i]
            corner = previous[i - 4] if i >= 4 else 0
            if kind == 1:
                line[i] = (line[i] + left) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + up) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + (left + up) // 2) & 0xFF
            elif kind == 4:
                guess = left + up - corner
                pa, pb, pc = abs(guess - left), abs(guess - up), abs(guess - corner)
                nearest = left if pa <= pb and pa <= pc else up if pb <= pc else corner
                line[i] = (line[i] + nearest) & 0xFF
        rows.append([tuple(line[x * 4 : x * 4 + 4]) for x in range(width)])
        previous = line
    return width, height, rows


def heights(width, height, rows, cell, bevel):
    """A height for every pixel: brightness, lowered toward a clear outline."""
    def lum(pixel):
        return (0.3 * pixel[0] + 0.59 * pixel[1] + 0.11 * pixel[2]) / 255.0

    field = [[lum(p) for p in row] for row in rows]
    if bevel <= 0:
        return field
    cell_w = cell or width
    cell_h = cell or height
    out = []
    for y in range(height):
        line = []
        for x in range(width):
            if rows[y][x][3] < 8:
                line.append(0.0)
                continue
            nearest = bevel
            x0, y0 = x - x % cell_w, y - y % cell_h
            for dy in range(-bevel, bevel + 1):
                for dx in range(-bevel, bevel + 1):
                    sx, sy = x + dx, y + dy
                    inside = x0 <= sx < x0 + cell_w and y0 <= sy < y0 + cell_h
                    # A cell's border is not an outline: tiles meet there.
                    if inside and rows[sy][sx][3] < 128:
                        nearest = min(nearest, max(abs(dx), abs(dy)))
            line.append(field[y][x] * 0.5 + nearest / bevel)
        out.append(line)
    return out


def normals(width, height, field, cell, strength):
    cell_w = cell or width
    cell_h = cell or height

    def at(x, y, ox, oy):
        x = min(max(x, ox), ox + cell_w - 1)
        y = min(max(y, oy), oy + cell_h - 1)
        return field[y][x]

    rows = []
    for y in range(height):
        row = []
        oy = y - y % cell_h
        for x in range(width):
            ox = x - x % cell_w
            # A Sobel pair, so one-pixel lines still read as grooves.
            across = (
                at(x + 1, y - 1, ox, oy) + 2 * at(x + 1, y, ox, oy) + at(x + 1, y + 1, ox, oy)
                - at(x - 1, y - 1, ox, oy) - 2 * at(x - 1, y, ox, oy) - at(x - 1, y + 1, ox, oy)
            ) * strength / 4
            # Rows run down the picture, and green means up the world.
            down = (
                at(x - 1, y + 1, ox, oy) + 2 * at(x, y + 1, ox, oy) + at(x + 1, y + 1, ox, oy)
                - at(x - 1, y - 1, ox, oy) - 2 * at(x, y - 1, ox, oy) - at(x + 1, y - 1, ox, oy)
            ) * strength / 4
            length = (across * across + down * down + 1.0) ** 0.5
            normal = (-across / length, down / length, 1.0 / length)
            row.append(tuple(round((n * 0.5 + 0.5) * 255) for n in normal) + (255,))
        rows.append(row)
    return rows


def main():
    for name, cell, strength, bevel in LIT:
        width, height, rows = read_png(OUT / f"{name}.png")
        field = heights(width, height, rows, cell, bevel)
        png(OUT / f"{name}_n.png", width, height, normals(width, height, field, cell, strength))


if __name__ == "__main__":
    main()
