#!/usr/bin/env python3
"""DRIFT's block-built ships, as `sindri.voxel_world` edits.

A ship is a built voxel world (nothing generated, only its placed blocks),
scaled to half-metre blocks, and a `sindri.physics3d.voxel_body`, so the whole
ship is one rigid body. These functions return the edits; `hold.py` places the
ships in the scene.
"""


def shuttle():
    """A small shuttle, 24 x 8 x 10 blocks (12 x 4 x 5 m), nose towards -X.

    A shell of hull plate with a stripe along each flank, a tread-plate deck,
    dark frame ribs every four blocks, a hazard band round the stern, light
    panels down the ceiling, an open cockpit at the nose and a hatch at the
    stern."""
    length, height, width = 24, 8, 10
    blocks = {}

    def put(x, y, z, block):
        blocks[(x, y, z)] = block

    for x in range(length):
        # The nose narrows over its first three blocks.
        inset = max(0, 3 - x)
        y_top = height - 1 - inset // 2
        z_lo, z_hi = inset, width - 1 - inset
        for y in range(0, y_top + 1):
            for z in range(z_lo, z_hi + 1):
                shell = y in (0, y_top) or z in (z_lo, z_hi) or x in (0, length - 1)
                if not shell:
                    continue
                block = "hull"
                if y == 0:
                    block = "deck" if 0 < z < width - 1 and x > 0 else "hull"
                elif z in (z_lo, z_hi) and y == 3:
                    block = "hull-stripe"
                if x % 4 == 0 and x not in (0, length - 1) and (y in (0, y_top) or z in (z_lo, z_hi)):
                    block = "frame"
                if x >= length - 3 and (y == y_top or z in (z_lo, z_hi)):
                    block = "hazard"
                if y == y_top and x % 4 == 2 and 3 < z < width - 4 and x < length - 3:
                    block = "light"
                put(x, y, z, block)
    # The cockpit: the nose's upper front left open.
    for y in range(3, height - 1):
        for z in range(3, width - 3):
            blocks.pop((0, y, z), None)
    # The stern hatch: a doorway at deck level.
    for y in range(1, 5):
        for z in range(3, width - 3):
            blocks.pop((length - 1, y, z), None)
    return [{"at": [x, y, z], "block": block} for (x, y, z), block in sorted(blocks.items())]
