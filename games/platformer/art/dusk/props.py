"""Props drawn pixel by pixel: the lantern, a signpost, tufts and flowers.

Each is a small picture in rows of characters, so the drawing is the source and
a change to it is a change to the art. `.` is clear.
"""

from lib import CLEAR
from palette import ramp

WOOD, METAL, AMBER, ROCK = ramp("wood"), ramp("metal"), ramp("amber"), ramp("rock")


def art(rows, key):
    return [[key[c] if c != "." else CLEAR for c in row] for row in rows]


# A lamp post: iron frame, a glass lantern on a hook, and a wooden post.
# `u` is the unlit glass, `a`-`e` the flame from outer glow to white core.
LANTERN_UNLIT = [
    "......ff......",
    ".....fggf.....",
    "....fggggf....",
    "....fuuuuf....",
    "....fuuuuf....",
    "....fuuuuf....",
    "....fuuuuf....",
    "....fggggf....",
    ".....ffff.....",
    "......pp......",
    "......pp......",
    "......pp......",
    "......pp......",
    "......pp......",
    "......pp......",
    "......pp......",
    ".....pppp.....",
]


def lantern(lit):
    key = {
        "f": METAL[1],
        "g": METAL[2],
        "p": WOOD[1],
        "u": METAL[0],
    }
    rows = LANTERN_UNLIT
    if lit:
        key.update({"u": AMBER[3]})
        rows = [r.replace("fuuuuf", "fuebuf") if i == 4 else r for i, r in enumerate(rows)]
        key.update({"e": AMBER[4], "b": AMBER[3]})
    cell = art(rows, key)
    if lit:
        cell[3][5:9] = [AMBER[2], AMBER[3], AMBER[3], AMBER[2]]
        cell[4][5:9] = [AMBER[3], AMBER[4], AMBER[4], AMBER[3]]
        cell[5][5:9] = [AMBER[3], AMBER[4], AMBER[4], AMBER[3]]
        cell[6][5:9] = [AMBER[2], AMBER[3], AMBER[3], AMBER[2]]
    return cell


# --- Small decoration, placed over the ground by a level's script -----------
#
# The ground is autotiled and has no variants, so on its own it repeats. These
# are laid over it at hashed positions, one to a cell, and break that up.

GRASS = ramp("grass")
SKY = ramp("sky")


def tuft(shape):
    """Blades of grass, tall and short, leaning a little: `shape` picks the cluster."""
    cell = [[CLEAR] * 16 for _ in range(16)]
    clusters = (
        ((3, 4), (5, 6), (7, 3), (9, 5), (11, 4)),
        ((2, 3), (4, 5), (6, 7), (8, 4), (10, 6), (12, 3)),
        ((5, 5), (7, 8), (9, 5)),
    )[shape % 3]
    for x, height in clusters:
        for y in range(16 - height, 16):
            lean = 1 if (16 - y) > height - 2 and x % 2 else 0
            cell[y][min(15, x + lean)] = GRASS[3] if y > 16 - height + 1 else GRASS[4]
        cell[16 - height][min(15, x + 1)] = GRASS[4]
    return cell


def flower(kind):
    """A tiny flower on a stalk. Cool colors only: amber is for light."""
    cell = [[CLEAR] * 16 for _ in range(16)]
    petal = (SKY[5], ROCK[5])[kind % 2]
    for y in range(11, 16):
        cell[y][8] = GRASS[2]
    cell[10][8] = petal
    for dx, dy in ((-1, 10), (1, 10), (0, 9)):
        cell[dy][8 + dx] = petal
    cell[10][8] = ROCK[4] if kind % 2 else SKY[6]
    cell[14][7] = GRASS[3]
    cell[13][9] = GRASS[3]
    return cell


def pebbles(kind):
    """A couple of stones sitting on the ground."""
    cell = [[CLEAR] * 16 for _ in range(16)]
    spots = ((3, 13, 4, 3), (9, 14, 3, 2)) if kind % 2 == 0 else ((6, 13, 5, 3), (2, 14, 3, 2), (12, 14, 2, 2))
    for sx, sy, w, h in spots:
        for y in range(sy, min(16, sy + h)):
            for x in range(sx, min(16, sx + w)):
                top, left = y == sy, x == sx
                bottom, right = y == sy + h - 1, x == sx + w - 1
                cell[y][x] = ROCK[4] if (top or left) else ROCK[1] if (bottom or right) else ROCK[3]
    return cell


DECOR = [tuft(0), tuft(1), tuft(2), flower(0), flower(1), pebbles(0), pebbles(1)]
DECOR_NAMES = ["tuft-0", "tuft-1", "tuft-2", "flower-0", "flower-1", "pebbles-0", "pebbles-1"]
