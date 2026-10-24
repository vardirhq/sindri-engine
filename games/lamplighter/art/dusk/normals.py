"""Normal maps read back from finished tiles, so a lantern rakes across the rock.

Brightness is height: seams and shadowed pixels are grooves, lit stones stand
proud, and a tile's open edge is bevelled down to the ground. Heights are never
read across a cell boundary, so one tile's relief does not leak into its
neighbor. Laid out cell for cell like the picture (`docs/2d-lights.md`): red is
right, green is up the world, blue is out of the picture.
"""

import math

from lib import CLEAR


def height_map(image, x0, y0, cell, bevel):
    """Heights for one cell: luminance, lowered within `bevel` pixels of a transparent one."""
    heights = [[0.0] * cell for _ in range(cell)]
    for y in range(cell):
        for x in range(cell):
            r, g, b, a = image[y0 + y][x0 + x]
            if a == 0:
                heights[y][x] = -1.0
                continue
            h = (0.3 * r + 0.59 * g + 0.11 * b) / 255.0
            if bevel:
                near = bevel
                for dy in range(-bevel, bevel + 1):
                    for dx in range(-bevel, bevel + 1):
                        nx, ny = x + dx, y + dy
                        if 0 <= nx < cell and 0 <= ny < cell and image[y0 + ny][x0 + nx][3] == 0:
                            near = min(near, max(abs(dx), abs(dy)) - 1)
                h -= 0.25 * (1 - near / bevel) if near < bevel else 0.0
            heights[y][x] = h
    return heights


def normal_map(image, cell, strength=3.0, bevel=0):
    """The normal map of a whole sheet whose cells are `cell` pixels square."""
    height, width = len(image), len(image[0])
    out = [[CLEAR] * width for _ in range(height)]
    for cy in range(0, height, cell):
        for cx in range(0, width, cell):
            h = height_map(image, cx, cy, cell, bevel)

            def at(x, y):
                return h[min(max(y, 0), cell - 1)][min(max(x, 0), cell - 1)]

            for y in range(cell):
                for x in range(cell):
                    if image[cy + y][cx + x][3] == 0:
                        continue
                    dx = (at(x + 1, y) - at(x - 1, y)) * strength
                    dy = (at(x, y - 1) - at(x, y + 1)) * strength  # up the world is -y in the picture
                    length = math.sqrt(dx * dx + dy * dy + 1.0)
                    nx, ny, nz = -dx / length, -dy / length, 1.0 / length
                    out[cy + y][cx + x] = (round((nx * 0.5 + 0.5) * 255), round((ny * 0.5 + 0.5) * 255), round((nz * 0.5 + 0.5) * 255), 255)
    return out
