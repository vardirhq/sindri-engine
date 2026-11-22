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
    # Adjacent ventilation inserts straddle the x=16 section boundary.
    for z in (0, width - 1):
        for x in (15, 16, 17):
            put(x, 5, z, "vent")
    return [{"at": [x, y, z], "block": block} for (x, y, z), block in sorted(blocks.items())]


# Which way a part's own -Z points, as the quaternion that turns -Z there.
_HALF = 0.7071068
FACING = {
    "+x": (0.0, -_HALF, 0.0, _HALF),
    "-x": (0.0, _HALF, 0.0, _HALF),
    "+z": (0.0, 1.0, 0.0, 0.0),
    "-z": (0.0, 0.0, 0.0, 1.0),
    "+y": (_HALF, 0.0, 0.0, _HALF),
    "-y": (-_HALF, 0.0, 0.0, _HALF),
}


def shuttle_thrusters():
    """The shuttle's thrusters: (where, in blocks; which way the nozzle points).

    A thruster pushes the ship opposite to its nozzle, through where it is
    mounted. Main engines at the stern push it forward (towards -X, its nose);
    retros by the nose push it back; pairs on each flank, the keel and the roof
    push it sideways, up and down. Paired fore and aft, so pushing sideways
    does not turn it unless one of a pair is lost."""
    return [
        ((24.9, 3.5, 3.0), "+x"), ((24.9, 3.5, 7.0), "+x"),
        ((4.0, 4.5, -0.9), "-x"), ((4.0, 4.5, 10.9), "-x"),
        ((7.0, 4.5, -0.9), "-z"), ((17.0, 4.5, -0.9), "-z"),
        ((7.0, 4.5, 10.9), "+z"), ((17.0, 4.5, 10.9), "+z"),
        ((7.0, -0.9, 5.0), "-y"), ((17.0, -0.9, 5.0), "-y"),
        ((7.0, 8.9, 5.0), "+y"), ((17.0, 8.9, 5.0), "+y"),
    ]


# The pilot's seat, in blocks, on the cockpit deck facing the nose.
SHUTTLE_SEAT = (2.8, 1.0, 5.0)
