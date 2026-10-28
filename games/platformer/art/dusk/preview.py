"""Lays a small level out of the autotiled ground and writes it as a picture."""

import sys

from ground import N, NE, E, SE, S, SW, W, NW, blob_masks, ground_cell
from lib import CELL, blank, hex_color, paste, png
from palette import ramp

MAP = [
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "...........####...............",
    ".......#########..............",
    "######################.....###",
    "######################.....###",
    "######################.....###",
    "######################.....###",
    "########..###########..#######",
    "########..###########..#######",
    "##############################",
]


def mask_at(grid, row, col):
    def filled(r, c):
        # Past the map's edge counts as more ground, as the engine does.
        if c < 0 or c >= len(grid[0]) or r >= len(grid):
            return True
        if r < 0:
            return False
        return grid[r][c] == "#"

    mask = 0
    for bit, (dr, dc) in ((N, (-1, 0)), (NE, (-1, 1)), (E, (0, 1)), (SE, (1, 1)), (S, (1, 0)), (SW, (1, -1)), (W, (0, -1)), (NW, (-1, -1))):
        if filled(row + dr, col + dc):
            mask |= bit
    for side_a, corner, side_b in ((N, NE, E), (E, SE, S), (S, SW, W), (W, NW, N)):
        if mask & corner and not (mask & side_a and mask & side_b):
            mask &= ~corner
    return mask


def render(grid, scale=3):
    width, height = len(grid[0]) * CELL, len(grid) * CELL
    image = blank(width, height)
    sky = ramp("sky")
    for y in range(height):
        t = y / (height - 1)
        index = min(len(sky) - 1, int(t * (len(sky) - 1) * 1.4))
        for x in range(width):
            image[y][x] = sky[index]
    cells = {mask: ground_cell(mask) for mask in blob_masks()}
    for row, line in enumerate(grid):
        for col, ch in enumerate(line):
            if ch == "#":
                paste(image, cells[mask_at(grid, row, col)], col * CELL, row * CELL)
    big = [[image[y // scale][x // scale] for x in range(width * scale)] for y in range(height * scale)]
    return big, width * scale, height * scale


if __name__ == "__main__":
    pixels, w, h = render(MAP)
    png(sys.argv[1], w, h, pixels)
