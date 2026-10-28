import sys
from ground import ground_cell, blob_masks
from lib import CELL, blank, paste, png
from palette import ramp
from preview import mask_at
from props import lantern
import sky as S

W, H = 320, 180
frame = S.sky_layer(W, H)
S.stars(frame)
S.moon(frame, 262, 30)
for layer in (
    S.hills(W, H, base=0.66, amplitude=22, color=ramp("sky")[3], seed=0.3),
    S.hills(W, H, base=0.74, amplitude=18, color=ramp("sky")[2], seed=1.9, trees=True),
):
    paste(frame, layer, 0, 0)

GRID = [
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....................",
    "....##..............",
    "..#######.......###.",
    "####################",
    "####################",
]
cells = {m: ground_cell(m) for m in blob_masks()}
oy = H - len(GRID) * CELL
for r, line in enumerate(GRID):
    for c, ch in enumerate(line):
        if ch == "#":
            paste(frame, cells[mask_at(GRID, r, c)], c * CELL, oy + r * CELL)
# Two lanterns: one lit, one waiting.
paste(frame, lantern(True), 120, oy + 12 * CELL - 17)
paste(frame, lantern(False), 232, oy + 12 * CELL - 17)
scale = 3
big = [[frame[y // scale][x // scale] for x in range(W * scale)] for y in range(H * scale)]
png(sys.argv[1], W * scale, H * scale, big)
