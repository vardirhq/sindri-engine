"""Autotiled ground: the 47 tiles of a `blob` terrain, drawn from their masks.

A blob tile is chosen by which of its eight neighbors are more ground (north 1,
north-east 2, east 4, south-east 8, south 16, south-west 32, west 64, north-west
128, a corner counting only when both sides beside it do). Rather than hand-draw
47 cells, each is worked out from the shape the mask describes: the cell's open
sides are cut straight, two open sides meeting round their corner, and two
present sides with the corner between them missing leave a notch. Every pixel
knows its distance from the nearest edge and which way that edge faces, which is
all the shading needs: grass on what faces up, a lit rim on what faces left, a
dark one on what faces down or right, rock beyond.
"""

import math

from lib import CELL, CLEAR, blank, noise
from palette import ramp

ROCK = ramp("rock")
SOIL = ramp("soil")
GRASS = ramp("grass")

OUTER = 5.0  # radius of a rounded outer corner, in pixels
INNER = 0.0  # radius of the notch at an inside corner

N, NE, E, SE, S, SW, W, NW = 1, 2, 4, 8, 16, 32, 64, 128


def blob_masks():
    """The 47 masks a blob terrain has a sprite for, in ascending order."""
    seen = set()
    for mask in range(256):
        for side_a, corner, side_b in ((N, NE, E), (E, SE, S), (S, SW, W), (W, NW, N)):
            if mask & corner and not (mask & side_a and mask & side_b):
                mask &= ~corner
        seen.add(mask)
    return sorted(seen)


def edges_of(mask):
    """Distances to the nearest boundary of the shape `mask` describes.

    Returns, per pixel, `(distance, nx, ny)`: how far inside the ground the
    pixel is (negative outside), and the outward direction of that boundary.
    """
    open_n, open_e, open_s, open_w = (not mask & bit for bit in (N, E, S, W))
    notches = []
    for side_a, corner, side_b, cx, cy in (
        (N, NE, E, CELL, 0),
        (E, SE, S, CELL, CELL),
        (S, SW, W, 0, CELL),
        (W, NW, N, 0, 0),
    ):
        if mask & side_a and mask & side_b and not mask & corner:
            notches.append((cx, cy))
    rounds = []
    for open_a, open_b, cx, cy in (
        (open_n, open_e, CELL - OUTER, OUTER),
        (open_e, open_s, CELL - OUTER, CELL - OUTER),
        (open_s, open_w, OUTER, CELL - OUTER),
        (open_w, open_n, OUTER, OUTER),
    ):
        if open_a and open_b:
            rounds.append((cx, cy))

    def at(x, y):
        px, py = x + 0.5, y + 0.5
        best = (1e9, 0.0, 0.0)

        def consider(distance, nx, ny):
            nonlocal best
            if distance < best[0]:
                best = (distance, nx, ny)

        if open_n:
            consider(py, 0.0, -1.0)
        if open_s:
            consider(CELL - py, 0.0, 1.0)
        if open_w:
            consider(px, -1.0, 0.0)
        if open_e:
            consider(CELL - px, 1.0, 0.0)
        for cx, cy in rounds:
            # Only the corner square is rounded.
            if (px - cx) * (1 if cx > CELL / 2 else -1) > 0 and (py - cy) * (1 if cy > CELL / 2 else -1) > 0:
                length = math.hypot(px - cx, py - cy) or 1.0
                consider(OUTER - length, (px - cx) / length, (py - cy) / length)
        for cx, cy in notches:
            length = math.hypot(px - cx, py - cy) or 1.0
            consider(length - INNER, (cx - px) / length, (cy - py) / length)
        return best

    return at


def grass_depth(x, salt):
    """How deep the grass is at column `x`: four to six pixels, ragged."""
    n = noise(x, 0, 7 + salt)
    return 4 + (n > 0.4) + (n > 0.8)


# Stones set into the rock: where, how wide and how tall. They stay clear of the
# tile's edge so a neighbor never cuts one in half.
STONES = ((3, 4, 5, 3), (10, 9, 4, 3), (5, 12, 4, 2))
# Strata: rows where a seam may run, and where along the row it is broken.
STRATA = ((7, 0), (14, 5))


def stone(x, y):
    """The shade of a set stone at this pixel, or None."""
    for sx, sy, width, height in STONES:
        if sx <= x < sx + width and sy <= y < sy + height:
            top, left = y == sy, x == sx
            bottom, right = y == sy + height - 1, x == sx + width - 1
            if top or left:
                return ROCK[4]
            if bottom or right:
                return ROCK[1]
            return ROCK[3]
    return None


def rock_pixel(x, y, mask, lit):
    """Interior rock: strata, set stones and sparse specks, the same wherever the tile is.

    `lit` is how close this pixel is to an upward edge, from 0 to 1: the sun
    that gilds a top also warms the rock just under it.
    """
    found = stone(x, y)
    if found is not None:
        return found
    for row, shift in STRATA:
        if y == row and (x + shift) % 9 < 6:
            return ROCK[1]
        if y == row - 1 and (x + shift) % 9 < 6 and noise(x, y, 5) > 0.7:
            return ROCK[3]
    n = noise(x, y, 11)
    if n > 0.96:
        return ROCK[4]
    if n < 0.04:
        return ROCK[1]
    if lit > 0.5 and n > 0.72:
        return ROCK[3]
    return ROCK[2]


def ground_cell(mask):
    shape = edges_of(mask)
    cell = blank(CELL, CELL)
    for y in range(CELL):
        for x in range(CELL):
            distance, nx, ny = shape(x, y)
            if distance < 0:
                continue
            cell[y][x] = shade(x, y, distance, nx, ny, mask)
    return cell


def shade(x, y, distance, nx, ny, mask):
    up = -ny
    # Grass wraps round a top and a little way down what leans toward it.
    if up > 0.25:
        depth = grass_depth(x, mask) * min(1.0, 0.4 + up * 0.8)
        if distance < depth:
            if distance < 1:
                return GRASS[4] if noise(x, y, 3) > 0.4 else GRASS[3]
            if distance < 2:
                return GRASS[3] if (x + y) % 3 else GRASS[2]
            if distance < depth - 1:
                return GRASS[2]
            return GRASS[0] if noise(x, y, 9) > 0.45 else GRASS[1]
        # Soil shows under a flat top only, not where the band turns up a wall.
        if up < 0.6:
            pass
        elif distance < depth + 2:
            return SOIL[3] if (distance < depth + 1 and noise(x, y, 4) > 0.3) else SOIL[2]
        elif distance < depth + 3:
            return SOIL[1]
    if distance < 1:
        if ny > 0.5:
            return ROCK[0]
        return ROCK[5] if nx < 0 else ROCK[1]
    if distance < 2:
        if ny > 0.5:
            return ROCK[1]
        return ROCK[4] if nx < 0 else ROCK[2]
    return rock_pixel(x, y, mask, max(0.0, 1.0 - distance / 8.0) * max(0.0, up))


def draw_ground(columns=8):
    """All 47 tiles, `columns` to a row, and the masks in the order they are drawn."""
    masks = blob_masks()
    rows = -(-len(masks) // columns)
    image = blank(columns * CELL, rows * CELL)
    for index, mask in enumerate(masks):
        cell = ground_cell(mask)
        ox, oy = (index % columns) * CELL, (index // columns) * CELL
        for y in range(CELL):
            for x in range(CELL):
                image[oy + y][ox + x] = cell[y][x]
    return image, masks, rows
