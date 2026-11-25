#!/usr/bin/env python3
"""Builds DRIFT's parts: the ship's thruster, flame and seat, the spare oxygen
canister, the diver's forearm and the target marker, under `assets/models/`.

Functional parts of a block-built ship are models snapped to its grid rather
than blocks: a thruster, the flame it shows while it fires, and the pilot's
seat. They are made at one metre to the block, half-metre blocks' parts are
placed at half scale by their ship. A thruster's nozzle points along its own
-Z: it pushes the ship the other way. Everything is drawn from fixed numbers,
so re-running this changes nothing.
"""
import pathlib
import sys

TOOLS = pathlib.Path(__file__).resolve().parents[3] / "tools"
sys.path.insert(0, str(TOOLS))

from glb import Material, Mesh  # noqa: E402

MODELS = pathlib.Path(__file__).resolve().parent.parent / "assets" / "models"

MATERIALS = {
    "housing": Material("#5e636b", roughness=0.55, metallic=0.7),
    "dark": Material("#25282d", roughness=0.7, metallic=0.5),
    "nozzle": Material("#3a3632", roughness=0.4, metallic=0.85),
    "inner": Material("#120c08", roughness=0.9),
    "band": Material("#d9a21e", roughness=0.6),
    "flame": Material("#2a4cff", emissive="#9fc6ff", strength=1.0),
    "flame-core": Material("#ffffff", emissive="#ffffff", strength=1.0),
    "cushion": Material("#3b3f47", roughness=0.95),
    "frame": Material("#6d727a", roughness=0.5, metallic=0.6),
    "screen": Material("#06141a", emissive="#38d0ff", strength=1.0),
    "tank": Material("#d8dde2", roughness=0.35, metallic=0.6),
    "o2": Material("#0b3a2a", emissive="#3dffb0", strength=1.0),
    "suit": Material("#c9c3b4", roughness=0.85),
    "glove": Material("#3d3f44", roughness=0.9),
    "wrist-screen": Material("#02080a", emissive="#0c3a40", strength=1.0),
    "marker": Material("#3a2a00", emissive="#ffc23d", strength=1.0),
}


def thruster():
    """A housing on a mounting plate, with a bell nozzle pointing along -Z."""
    mesh = Mesh()
    mesh.box("dark", (0, 0, 0.42), (0.9, 0.9, 0.16), bevel=0.04)
    mesh.cylinder("housing", (0, 0, 0.18), 0.32, 0.36, segments=12, rotation=(90, 0, 0))
    mesh.cylinder("band", (0, 0, 0.02), 0.33, 0.06, segments=12, rotation=(90, 0, 0))
    mesh.cylinder("nozzle", (0, 0, -0.2), 0.16, 0.4, top=0.34, segments=12, rotation=(90, 0, 0))
    mesh.cylinder("inner", (0, 0, -0.39), 0.3, 0.02, segments=12, rotation=(90, 0, 0))
    mesh.write(MODELS / "thruster.glb", MATERIALS, name="thruster")


def flame():
    """The exhaust: a glowing cone beyond the nozzle, shown while firing."""
    mesh = Mesh()
    mesh.cylinder("flame", (0, 0, -0.75), 0.3, 0.7, top=0.04, segments=10, rotation=(90, 0, 0))
    mesh.cylinder("flame-core", (0, 0, -0.6), 0.16, 0.4, top=0.02, segments=8, rotation=(90, 0, 0))
    mesh.write(MODELS / "flame.glb", MATERIALS, name="flame")


def seat():
    """The pilot's seat, facing -Z, with a small lit console before it."""
    mesh = Mesh()
    mesh.box("frame", (0, 0.2, 0), (0.6, 0.08, 0.6), bevel=0.02)
    mesh.box("cushion", (0, 0.29, 0), (0.54, 0.1, 0.54), bevel=0.03)
    mesh.box("cushion", (0, 0.75, 0.26), (0.54, 0.8, 0.1), bevel=0.03)
    mesh.box("frame", (0, 0.08, 0), (0.12, 0.16, 0.12))
    mesh.box("dark", (0, 0.7, -0.62), (0.8, 0.5, 0.12), rotation=(-25, 0, 0), bevel=0.02)
    mesh.box("screen", (0, 0.73, -0.71), (0.6, 0.32, 0.02), rotation=(-25, 0, 0))
    mesh.write(MODELS / "seat.glb", MATERIALS, name="seat")


def canister():
    """A spare oxygen canister: a white bottle with a glowing green band."""
    mesh = Mesh()
    mesh.cylinder("tank", (0, 0, 0), 0.11, 0.42, segments=14)
    mesh.cylinder("o2", (0, 0.04, 0), 0.115, 0.07, segments=14)
    mesh.cylinder("dark", (0, 0.25, 0), 0.04, 0.08, segments=8)
    mesh.box("dark", (0, 0.3, 0), (0.1, 0.03, 0.03))
    mesh.write(MODELS / "oxygen-canister.glb", MATERIALS, name="oxygen-canister")


def forearm():
    """The diver's left forearm, held up in view, with a wrist display facing
    the eye. Its own -Z runs from elbow to glove."""
    mesh = Mesh()
    mesh.cylinder("suit", (0, 0, 0.1), 0.075, 0.42, top=0.06, segments=12, rotation=(90, 0, 0))
    mesh.box("glove", (0, 0, -0.16), (0.12, 0.08, 0.14), bevel=0.02)
    mesh.box("dark", (0, 0.07, 0.02), (0.13, 0.03, 0.17), bevel=0.01)
    mesh.box("wrist-screen", (0, 0.087, 0.02), (0.11, 0.004, 0.14))
    mesh.write(MODELS / "forearm.glb", MATERIALS, name="forearm")


def marker():
    """Four glowing corner brackets around a point: what ACTION will use."""
    mesh = Mesh()
    for x in (-1, 1):
        for y in (-1, 1):
            mesh.box("marker", (x * 0.14, y * 0.18, 0), (0.08, 0.015, 0.015))
            mesh.box("marker", (x * 0.18, y * 0.14, 0), (0.015, 0.08, 0.015))
    mesh.write(MODELS / "marker.glb", MATERIALS, name="marker")


def main():
    MODELS.mkdir(parents=True, exist_ok=True)
    thruster()
    flame()
    seat()
    canister()
    forearm()
    marker()


if __name__ == "__main__":
    main()
