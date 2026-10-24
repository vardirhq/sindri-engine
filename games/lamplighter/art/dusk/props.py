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
