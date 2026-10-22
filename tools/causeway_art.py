"""Builds every model and texture Causeway draws.

    python3 tools/causeway_art.py

Writes `game/assets/models/*.glb` and `game/assets/textures/*.png`. Run it again
after changing anything here or an island in `islands.decay`; the outputs are
checked in like any other art. `the_art_is_built_from_the_islands_the_rules_read`
checks that every island has a model.

One script rather than a file per asset because the look is a set of decisions
that have to agree: one palette, one bevel, one scale. The wardens are warm --
teal, ochre, navy and brass. The Hollow are the cold slate of tidal mud with
eyes that glow, so a glance at the board says whose side anything is on. The
islands are read from `islands.decay`, the same text the rules read, so the
ground the game is played on and the ground that is drawn cannot disagree.

Conventions are glTF's: Y is up, a metre is a cell. Figures are authored facing
+X and are turned by their entity's yaw. Islands are written in world
coordinates, with cell (0, 0) of the board at (24, 24).
"""
import math
import pathlib
import random
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from glb import Material, Mesh  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[1] / "game"
MODELS = ROOT / "assets" / "models"
TEXTURES = ROOT / "assets" / "textures"
ISLANDS = ROOT / "assets" / "scripts" / "islands.decay"

ORIGIN = 24.0
SIZE = 8
BEVEL = 0.035


# --- Palettes -----------------------------------------------------------------

def mat(color, roughness=0.85, metallic=0.0, glow=None, strength=1.0):
    return Material(color, roughness, metallic, glow, strength)


WARDENS = {
    "steel": mat("#a7b0bb", 0.38, 0.75),
    "steel-d": mat("#6b7581", 0.45, 0.7),
    "skin": mat("#eab48a", 0.75),
    "boot": mat("#3d2f27", 0.9),
    "cloth-d": mat("#38324a", 0.9),
    "brass": mat("#e8b64e", 0.32, 0.85),
    "oak": mat("#93603a", 0.8),
    "teal": mat("#2a9a90", 0.8),
    "red": mat("#d0483b", 0.8),
    "ochre": mat("#d39a44", 0.85),
    "apron": mat("#6f4b2f", 0.9),
    "beard": mat("#b4572e", 0.9),
    "cap": mat("#5f6670", 0.85),
    "stone": mat("#a4a39c", 0.95),
    "navy": mat("#2f4f86", 0.8),
    "navy-d": mat("#223a63", 0.8),
    "scarf": mat("#e2ab3f", 0.8),
    "hat": mat("#2b2533", 0.8),
    "lamp": mat("#fff1b0", 0.6, 0.0, "#ffc94d", 4.0),
    "eye": mat("#1d1a22", 0.6),
}

HOLLOW = {
    "mud": mat("#5d5670", 0.95),
    "mud-d": mat("#3e384c", 0.95),
    "mud-l": mat("#7c7390", 0.95),
    "weed": mat("#4f7550", 0.9),
    "bone": mat("#ddd2b6", 0.8),
    "shell": mat("#76617a", 0.7),
    "eye": mat("#9ffff2", 0.5, 0.0, "#5ff4e6", 6.0),
    "maw": mat("#140f19", 0.9),
    "bile": mat("#c4ff7a", 0.5, 0.0, "#a8ff4d", 4.0),
    "ember": mat("#ffb36b", 0.5, 0.0, "#ff8a3a", 5.0),
}

STEAD = {
    "wall": mat("#efe6d2", 0.9),
    "wall-d": mat("#c9bb9e", 0.9),
    "timber": mat("#6b4529", 0.85),
    "thatch": mat("#d4a04e", 0.95),
    "thatch-d": mat("#a97a35", 0.95),
    "door": mat("#7a4a2a", 0.85),
    "window": mat("#ffe3a0", 0.5, 0.0, "#ffbe55", 3.0),
    "stone": mat("#a29f97", 0.95),
    "stone-d": mat("#77756f", 0.95),
    "moss": mat("#6f9a45", 0.9),
    "iron": mat("#3b3a40", 0.5, 0.6),
    "flame": mat("#ffd06a", 0.5, 0.0, "#ff9a2e", 6.0),
    "flame-hot": mat("#fff6c8", 0.5, 0.0, "#ffe7a0", 8.0),
}

GROUND = {
    "grass": mat("#7fae45", 0.95),
    "grass-b": mat("#8cba50", 0.95),
    "moss": mat("#6d9b4a", 0.95),
    "moss-b": mat("#78a652", 0.95),
    "earth": mat("#a06f45", 0.95),
    "earth-b": mat("#8a5c39", 0.95),
    "stone": mat("#9b978e", 0.95),
    "stone-b": mat("#8a877f", 0.95),
    "rock": mat("#7f7b74", 0.95),
    "rock-b": mat("#716d67", 0.95),
    "sand": mat("#e6d5a2", 0.95),
    "sand-b": mat("#dccb96", 0.95),
    "mud": mat("#8b7d5c", 0.95),
    "mud-b": mat("#807352", 0.95),
    "gravel": mat("#b3ab98", 0.95),
    "gravel-b": mat("#a69e8b", 0.95),
    "seabed": mat("#c8b27c", 1.0),
    "seabed-2": mat("#9e9a76", 1.0),
    "seabed-3": mat("#5f7a72", 1.0),
    "seabed-4": mat("#355a62", 1.0),
    "tuft": mat("#5f8f36", 0.95),
    "tuft-l": mat("#9cc75a", 0.95),
    "reed": mat("#b3a56a", 0.95),
    "flower-y": mat("#f4d35e", 0.8),
    "flower-w": mat("#f6f1e4", 0.8),
    "flower-p": mat("#d78ac2", 0.8),
    "pebble": mat("#b8b2a4", 0.95),
    "leaf": mat("#4e8a3a", 0.95),
    "leaf-d": mat("#3c7230", 0.95),
    "pine": mat("#2f6a45", 0.95),
    "trunk": mat("#6a4a32", 0.95),
    "road": mat("#b9b1a0", 0.95),
    "road-d": mat("#8f8878", 0.95),
    "post": mat("#5c4632", 0.9),
    "rope": mat("#c9b37e", 0.9),
}


# --- Figures --------------------------------------------------------------------

def bulwark(m, pose):
    lean = 0.04 if pose == "strike" else 0.0
    shield = 0.3 if pose != "strike" else 0.42
    m.box("boot", (0, 0.11, -0.12), (0.22, 0.22, 0.16), bevel=0.03)
    m.box("boot", (0, 0.11, 0.12), (0.22, 0.22, 0.16), bevel=0.03)
    m.box("steel-d", (0, 0.3, -0.11), (0.17, 0.18, 0.15), bevel=0.02)
    m.box("steel-d", (0, 0.3, 0.11), (0.17, 0.18, 0.15), bevel=0.02)
    m.box("teal", (lean, 0.55, 0), (0.34, 0.38, 0.46), bevel=0.04)
    m.box("steel", (lean, 0.7, 0), (0.37, 0.16, 0.5), bevel=0.04)
    m.box("steel", (lean, 0.74, -0.3), (0.24, 0.17, 0.17), bevel=0.04)
    m.box("steel", (lean, 0.74, 0.3), (0.24, 0.17, 0.17), bevel=0.04)
    m.box("steel", (lean + 0.02, 0.98, 0), (0.32, 0.3, 0.3), bevel=0.06)
    m.box("eye", (lean + 0.18, 0.99, 0), (0.02, 0.05, 0.22))
    m.box("red", (lean - 0.02, 1.17, 0), (0.26, 0.09, 0.06), bevel=0.02)
    m.box("red", (lean - 0.13, 1.1, 0), (0.08, 0.14, 0.06), bevel=0.02)
    m.box("oak", (shield, 0.56, 0.08), (0.08, 0.7, 0.52), bevel=0.03)
    m.box("brass", (shield + 0.045, 0.56, 0.08), (0.02, 0.7, 0.07))
    m.box("brass", (shield + 0.045, 0.6, 0.08), (0.02, 0.07, 0.52))
    m.box("brass", (shield + 0.05, 0.6, 0.08), (0.03, 0.16, 0.16), bevel=0.02)


def mason(m, pose):
    m.box("boot", (0, 0.09, -0.14), (0.24, 0.18, 0.18), bevel=0.03)
    m.box("boot", (0, 0.09, 0.14), (0.24, 0.18, 0.18), bevel=0.03)
    m.box("cloth-d", (0, 0.27, -0.13), (0.2, 0.2, 0.17), bevel=0.02)
    m.box("cloth-d", (0, 0.27, 0.13), (0.2, 0.2, 0.17), bevel=0.02)
    m.box("ochre", (0, 0.56, 0), (0.38, 0.4, 0.54), bevel=0.05)
    m.box("apron", (0.2, 0.46, 0), (0.03, 0.42, 0.38), bevel=0.01)
    if pose == "strike":
        m.box("ochre", (0.1, 0.9, -0.16), (0.13, 0.36, 0.13), (0, 0, -40), bevel=0.03)
        m.box("ochre", (0.1, 0.9, 0.16), (0.13, 0.36, 0.13), (0, 0, -40), bevel=0.03)
        m.sphere("stone", (0.24, 1.28, 0), 0.2, rings=4, segments=7)
    else:
        m.box("ochre", (0, 0.6, -0.3), (0.17, 0.34, 0.13), bevel=0.03)
        m.box("ochre", (0, 0.6, 0.3), (0.17, 0.34, 0.13), bevel=0.03)
        m.sphere("stone", (-0.16, 1.02, 0.18), 0.19, rings=4, segments=7)
        m.box("oak", (0.06, 0.42, -0.36), (0.05, 0.42, 0.05), (0, 0, -12))
        m.box("steel-d", (0.1, 0.22, -0.36), (0.16, 0.1, 0.1), bevel=0.02)
    m.box("skin", (0.02, 0.9, 0), (0.28, 0.26, 0.28), bevel=0.05)
    m.box("eye", (0.165, 0.94, -0.06), (0.01, 0.04, 0.04))
    m.box("eye", (0.165, 0.94, 0.06), (0.01, 0.04, 0.04))
    m.box("beard", (0.14, 0.82, 0), (0.08, 0.16, 0.24), bevel=0.03)
    m.box("cap", (0, 1.06, 0), (0.32, 0.08, 0.32), bevel=0.03)
    m.box("cap", (0.12, 1.04, 0), (0.14, 0.03, 0.3))


def lamplighter(m, pose):
    m.box("boot", (0, 0.09, -0.09), (0.22, 0.18, 0.13), bevel=0.03)
    m.box("boot", (0, 0.09, 0.09), (0.22, 0.18, 0.13), bevel=0.03)
    m.box("navy", (0, 0.46, 0), (0.29, 0.6, 0.38), bevel=0.04)
    m.box("navy-d", (0.15, 0.4, 0), (0.01, 0.46, 0.07))
    m.box("brass", (0.155, 0.55, 0), (0.02, 0.035, 0.035))
    m.box("brass", (0.155, 0.43, 0), (0.02, 0.035, 0.035))
    m.box("navy", (0, 0.66, -0.22), (0.14, 0.32, 0.11), bevel=0.03)
    m.box("scarf", (0, 0.78, 0), (0.28, 0.08, 0.34), bevel=0.02)
    m.box("skin", (0.02, 0.93, 0), (0.25, 0.25, 0.25), bevel=0.05)
    m.box("eye", (0.15, 0.96, -0.06), (0.01, 0.04, 0.04))
    m.box("eye", (0.15, 0.96, 0.06), (0.01, 0.04, 0.04))
    m.cylinder("hat", (0, 1.08, 0), 0.22, 0.035, segments=10)
    m.cylinder("hat", (0, 1.2, 0), 0.13, 0.22, segments=10)
    m.box("scarf", (0, 1.12, 0), (0.27, 0.03, 0.27))
    if pose == "strike":
        m.box("navy", (0.12, 0.8, 0.2), (0.3, 0.11, 0.11), bevel=0.03)
        m.box("oak", (0.2, 0.86, 0.2), (0.04, 1.0, 0.04), (0, 0, -40))
        m.box("brass", (0.5, 1.26, 0.2), (0.16, 0.16, 0.14), bevel=0.02)
        m.box("lamp", (0.5, 1.26, 0.2), (0.12, 0.11, 0.16))
    else:
        m.box("navy", (0.05, 0.66, 0.22), (0.14, 0.32, 0.11), bevel=0.03)
        m.box("oak", (0.06, 0.8, 0.26), (0.045, 1.4, 0.045))
        m.box("brass", (0.12, 1.47, 0.26), (0.16, 0.03, 0.03))
        m.box("brass", (0.18, 1.36, 0.26), (0.15, 0.17, 0.15), bevel=0.02)
        m.box("lamp", (0.18, 1.36, 0.26), (0.11, 0.12, 0.17))


def husk(m, pose):
    reach = 0.12 if pose == "strike" else 0.0
    m.box("mud-d", (-0.04, 0.13, -0.14), (0.2, 0.26, 0.13), bevel=0.04)
    m.box("mud-d", (-0.04, 0.13, 0.14), (0.2, 0.26, 0.13), bevel=0.04)
    m.sphere("mud", (-0.04, 0.47, 0), 0.28, rings=4, segments=8)
    m.sphere("mud-l", (0.14, 0.62, 0), 0.19, rings=4, segments=7)
    m.box("eye", (0.3, 0.66, -0.08), (0.04, 0.06, 0.06))
    m.box("eye", (0.3, 0.66, 0.08), (0.04, 0.06, 0.06))
    m.box("mud-d", (0.12 + reach, 0.38, -0.27), (0.34, 0.1, 0.1), (0, 0, -18), bevel=0.03)
    m.box("mud-d", (0.12 + reach, 0.38, 0.27), (0.34, 0.1, 0.1), (0, 0, -18), bevel=0.03)
    m.cone("bone", (0.32 + reach, 0.3, -0.27), 0.05, 0.16, rotation=(0, 0, -100), segments=5)
    m.cone("bone", (0.32 + reach, 0.3, 0.27), 0.05, 0.16, rotation=(0, 0, -100), segments=5)
    m.box("weed", (-0.14, 0.72, 0.06), (0.14, 0.1, 0.22), (0, 20, 0), bevel=0.03)


def brute(m, pose):
    fist = 0.32 if pose != "strike" else 0.4
    fist_y = 0.44 if pose != "strike" else 0.72
    m.box("mud-d", (0, 0.18, -0.18), (0.24, 0.36, 0.2), bevel=0.05)
    m.box("mud-d", (0, 0.18, 0.18), (0.24, 0.36, 0.2), bevel=0.05)
    m.box("mud", (0, 0.64, 0), (0.48, 0.56, 0.64), bevel=0.08)
    m.box("mud-l", (0.04, 0.95, -0.32), (0.32, 0.26, 0.28), bevel=0.06)
    m.box("mud-l", (0.04, 0.95, 0.32), (0.32, 0.26, 0.28), bevel=0.06)
    m.box("weed", (-0.04, 1.08, -0.32), (0.24, 0.06, 0.22), bevel=0.02)
    m.box("mud", (0.16, 1.0, 0), (0.26, 0.22, 0.24), bevel=0.05)
    m.box("eye", (0.295, 1.02, -0.07), (0.02, 0.05, 0.06))
    m.box("eye", (0.295, 1.02, 0.07), (0.02, 0.05, 0.06))
    m.box("mud", (fist - 0.1, 0.62, -0.38), (0.2, 0.38, 0.16), bevel=0.04)
    m.box("mud", (fist - 0.1, 0.62, 0.38), (0.2, 0.38, 0.16), bevel=0.04)
    m.box("shell", (fist, fist_y, -0.38), (0.24, 0.24, 0.22), bevel=0.05)
    m.box("shell", (fist, fist_y, 0.38), (0.24, 0.24, 0.22), bevel=0.05)


def spitter(m, pose):
    mouth = 0.08 if pose != "strike" else 0.14
    m.sphere("mud-d", (-0.26, 0.17, 0), 0.18, rings=4, segments=7, squash=0.9)
    m.sphere("mud", (-0.06, 0.24, 0), 0.23, rings=4, segments=8, squash=0.9)
    m.sphere("mud-l", (0.14, 0.4, 0), 0.2, rings=4, segments=8)
    m.box("mud", (0.25, 0.6, 0), (0.26, 0.26, 0.28), bevel=0.06)
    m.box("maw", (0.38, 0.56, 0), (0.04, mouth * 2, 0.18))
    m.box("bile", (0.395, 0.56, 0), (0.02, mouth, 0.1))
    m.box("eye", (0.32, 0.72, -0.1), (0.05, 0.06, 0.05))
    m.box("eye", (0.32, 0.72, 0.1), (0.05, 0.06, 0.05))
    m.box("weed", (-0.12, 0.44, 0), (0.14, 0.06, 0.26), bevel=0.02)


def mortar(m, pose):
    glow = 0.11 if pose != "strike" else 0.15
    for x in (-0.18, 0.16):
        for z in (-1, 1):
            m.box("mud-d", (x, 0.11, z * 0.3), (0.09, 0.22, 0.17), (z * 25, 0, 0), bevel=0.02)
    m.box("shell", (0, 0.32, 0), (0.54, 0.22, 0.52), bevel=0.07)
    m.box("mud", (0, 0.45, 0), (0.44, 0.1, 0.42), bevel=0.03)
    m.cylinder("mud-l", (-0.06, 0.64, 0), 0.16, 0.3, top=0.1, segments=8)
    m.cylinder("ember", (-0.06, 0.8, 0), glow, 0.04, segments=8)
    m.box("eye", (0.28, 0.38, -0.1), (0.03, 0.06, 0.06))
    m.box("eye", (0.28, 0.38, 0.1), (0.03, 0.06, 0.06))
    m.box("bone", (0.33, 0.25, -0.2), (0.15, 0.05, 0.08), bevel=0.01)
    m.box("bone", (0.33, 0.25, 0.2), (0.15, 0.05, 0.08), bevel=0.01)


def colossus(m, pose):
    arm_y = 0.98 if pose != "strike" else 1.34
    arm_x = 0.08 if pose != "strike" else 0.26
    m.box("mud-d", (0, 0.32, -0.21), (0.32, 0.64, 0.24), bevel=0.06)
    m.box("mud-d", (0, 0.32, 0.21), (0.32, 0.64, 0.24), bevel=0.06)
    m.box("mud", (0, 1.04, 0), (0.6, 0.86, 0.66), bevel=0.1)
    m.box("shell", (0, 1.46, -0.34), (0.52, 0.38, 0.36), bevel=0.08)
    m.box("shell", (0, 1.46, 0.34), (0.52, 0.38, 0.36), bevel=0.08)
    m.cone("bone", (0.08, 1.72, -0.38), 0.07, 0.2, segments=5)
    m.cone("bone", (-0.1, 1.74, 0.36), 0.07, 0.22, segments=5)
    m.box("mud-l", (0.12, 1.72, 0), (0.38, 0.34, 0.34), bevel=0.07)
    m.box("eye", (0.32, 1.76, -0.09), (0.02, 0.07, 0.08))
    m.box("eye", (0.32, 1.76, 0.09), (0.02, 0.07, 0.08))
    m.box("eye", (0.305, 1.12, 0), (0.02, 0.44, 0.05))
    m.box("weed", (-0.1, 1.93, 0), (0.32, 0.08, 0.32), (0, 30, 0), bevel=0.02)
    m.box("mud", (arm_x, arm_y, -0.41), (0.26, 0.7, 0.21), bevel=0.05)
    m.box("mud", (arm_x, arm_y, 0.41), (0.26, 0.7, 0.21), bevel=0.05)
    m.box("shell", (arm_x + 0.06, arm_y - 0.44, -0.4), (0.32, 0.28, 0.26), bevel=0.06)
    m.box("shell", (arm_x + 0.06, arm_y - 0.44, 0.4), (0.32, 0.28, 0.26), bevel=0.06)


def house(m, pose):
    m.box("stone-d", (0, 0.05, 0), (0.78, 0.1, 0.84), bevel=0.02)
    m.box("wall", (0, 0.32, 0), (0.7, 0.5, 0.76), bevel=0.03)
    for z in (-0.38, 0.38):
        m.box("timber", (0, 0.32, z), (0.72, 0.05, 0.03))
    for x in (-0.35, 0.35):
        m.box("timber", (x, 0.32, 0), (0.03, 0.5, 0.78))
    m.wedge("thatch", (0, 0.78, 0), (0.86, 0.46, 0.92), rotation=(0, 90, 0))
    m.box("thatch-d", (0, 1.0, 0), (0.94, 0.05, 0.12), bevel=0.02)
    m.box("door", (0.36, 0.22, 0.14), (0.03, 0.32, 0.18))
    m.box("window", (0.36, 0.38, -0.18), (0.03, 0.14, 0.14))
    m.box("window", (0, 0.38, 0.385), (0.16, 0.14, 0.03))
    m.box("stone-d", (-0.22, 0.98, 0.2), (0.12, 0.36, 0.12), bevel=0.02)


def beacon(m, pose):
    m.cylinder("stone-d", (0, 0.14, 0), 0.36, 0.28, top=0.34, segments=8)
    m.cylinder("stone", (0, 0.68, 0), 0.31, 0.8, top=0.26, segments=8)
    m.cylinder("stone-d", (0, 1.1, 0), 0.32, 0.08, segments=8)
    m.box("door", (0.29, 0.22, 0), (0.04, 0.3, 0.15))
    m.box("window", (0.25, 0.74, 0), (0.04, 0.12, 0.07))
    m.cylinder("iron", (0, 1.2, 0), 0.2, 0.12, top=0.27, segments=8)
    m.cone("flame", (0, 1.44, 0), 0.19, 0.38, segments=7)
    m.cone("flame-hot", (0.02, 1.38, 0.03), 0.1, 0.26, segments=6)
    m.box("moss", (0.2, 0.08, -0.22), (0.16, 0.08, 0.14), bevel=0.02)


def stone(m, pose):
    m.box("stone", (0, 0.24, 0), (0.54, 0.46, 0.5), (6, 18, 4), bevel=0.07)
    m.box("stone-d", (-0.08, 0.54, 0.04), (0.34, 0.22, 0.32), (10, 40, -8), bevel=0.05)
    m.box("stone", (0.2, 0.12, 0.18), (0.22, 0.22, 0.22), (0, 30, 12), bevel=0.04)
    m.box("moss", (-0.04, 0.66, 0), (0.24, 0.04, 0.22), (0, 40, 0), bevel=0.01)


def missile(m, pose):
    """What a bolt, a stone or a glob is while it flies: a small hot ember."""
    m.sphere("flame-hot", (0, 0, 0), 0.11, rings=3, segments=6)
    m.sphere("flame", (0, 0, 0), 0.16, rings=3, segments=6, squash=0.8)


FIGURES = {
    "bulwark": (bulwark, WARDENS, ["idle"]),
    "mason": (mason, WARDENS, ["idle"]),
    "lamplighter": (lamplighter, WARDENS, ["idle"]),
    "husk": (husk, HOLLOW, ["idle"]),
    "brute": (brute, HOLLOW, ["idle"]),
    "spitter": (spitter, HOLLOW, ["idle"]),
    "mortar": (mortar, HOLLOW, ["idle"]),
    "colossus": (colossus, HOLLOW, ["idle"]),
    "house": (house, STEAD, ["idle"]),
    "beacon": (beacon, STEAD, ["idle"]),
    "stone": (stone, STEAD, ["idle"]),
    "missile": (missile, STEAD, ["idle"]),
}


# --- Islands -------------------------------------------------------------------

def islands():
    """Every island's name, heights, features and top blocks, from islands.decay."""
    text = ISLANDS.read_text()
    found = []
    for block in re.split(r"IslandDef\(", text)[1:]:
        def field(name):
            match = re.search(name + r':\s*((?:"[^"]*"\s*\+?\s*)+)', block)
            return "".join(re.findall(r'"([^"]*)"', match.group(1)))
        found.append({
            "name": field("name"),
            "heights": field("heights"),
            "features": field("features"),
            "shore": field("shore"),
            "land": field("land"),
            "high": field("high"),
        })
    return found


def column_top(island, height):
    if height <= 1:
        return island["shore"]
    if height >= 4:
        return island["high"]
    return island["land"]


SIDE = {"ground": "earth", "moss": "earth", "stone": "stone", "rock": "rock", "sand": "sand",
        "mud": "mud", "gravel": "gravel"}
TOP = {"ground": "grass", "moss": "moss", "stone": "stone", "rock": "rock", "sand": "sand",
       "mud": "mud", "gravel": "gravel"}


def scatter(m, rng, top, cx, cy, cz, busy):
    """Small things on a cell's top: never in its middle, where a figure stands."""
    def spot():
        for _ in range(8):
            u, v = rng.uniform(-0.4, 0.4), rng.uniform(-0.4, 0.4)
            if max(abs(u), abs(v)) > 0.26:
                return u, v
        return 0.38, 0.38
    if top in ("grass", "moss"):
        for _ in range(rng.randint(1, 3) if not busy else 1):
            u, v = spot()
            shade = rng.choice(["tuft", "tuft-l"])
            m.cone(shade, (cx + u, cy + 0.05, cz + v), 0.045, 0.12, segments=4)
            m.cone(shade, (cx + u + 0.04, cy + 0.04, cz + v + 0.02), 0.035, 0.09, segments=4)
        if rng.random() < 0.35:
            u, v = spot()
            m.box(rng.choice(["flower-y", "flower-w", "flower-p"]), (cx + u, cy + 0.03, cz + v),
                  (0.05, 0.05, 0.05))
    elif top in ("sand", "gravel", "stone"):
        for _ in range(rng.randint(0, 2)):
            u, v = spot()
            m.box("pebble", (cx + u, cy + 0.025, cz + v), (0.08, 0.05, 0.07),
                  (0, rng.uniform(0, 90), 0), bevel=0.015)
    elif top == "mud":
        for _ in range(rng.randint(1, 3)):
            u, v = spot()
            for k in range(3):
                m.box("reed", (cx + u + k * 0.03, cy + 0.14 + k * 0.02, cz + v), (0.015, 0.28 + k * 0.04, 0.015),
                      (rng.uniform(-8, 8), 0, rng.uniform(-8, 8)))


def tree(m, rng, x, y, z, pine=False):
    if pine:
        m.box("trunk", (x, y + 0.12, z), (0.1, 0.24, 0.1))
        for i, r in enumerate((0.36, 0.28, 0.18)):
            m.cone("pine", (x, y + 0.36 + i * 0.26, z), r, 0.42, segments=7)
        return
    m.box("trunk", (x, y + 0.2, z), (0.1, 0.4, 0.1))
    m.sphere(rng.choice(["leaf", "leaf-d"]), (x, y + 0.58, z), 0.3, rings=3, segments=7)
    m.sphere("leaf", (x + 0.14, y + 0.48, z + 0.08), 0.18, rings=3, segments=6)


def island_mesh(island, seed):
    """An island: a column per cell, the sea floor round it, and the shore beyond."""
    m = Mesh()
    rng = random.Random(seed)
    heights = [int(c) for c in island["heights"]]
    for c, h in enumerate(heights):
        x, z = c % SIZE, c // SIZE
        cx, cz = ORIGIN + x + 0.5, ORIGIN + z + 0.5
        if h <= 0:
            continue
        top_block = column_top(island, h)
        top, side = TOP[top_block], SIDE[top_block]
        # A checker between neighbouring cells, faint, so the grid reads
        # without a line drawn over the art.
        alt = "-b" if (x + z) % 2 else ""
        # Each level of a cliff a shade apart, so a step is counted at a
        # glance, and the top capped in its own block with a slight lip.
        for level in range(h):
            shade = side + ("-b" if level % 2 else "")
            bottom = -1.0 if level == 0 else float(level)
            top_y = level + 1.0 - (0.16 if level == h - 1 else 0.0)
            m.box(shade, (cx, (bottom + top_y) / 2, cz), (0.98, top_y - bottom - 0.02, 0.98), bevel=BEVEL)
        m.box(top + alt, (cx, h - 0.08, cz), (1.0, 0.16, 1.0), bevel=0.04)
        busy = island["features"][c] != "."
        scatter(m, rng, top, cx, h, cz, busy)

    # The sea floor: sand round the island fading through weed to deep water,
    # far wider than the screen so the sea has no edge.
    centre = (ORIGIN + 4, 0, ORIGIN + 4)
    for name, radius, y in (("seabed", 7.5, -0.7), ("seabed-2", 11.0, -1.0), ("seabed-3", 16.0, -1.5)):
        m.cylinder(name, (centre[0], y, centre[2]), radius, 0.5, top=radius - 0.4, segments=28)
    m.box("seabed-4", (centre[0], -2.4, centre[2]), (140, 0.5, 140))

    # Rocks in the shallows, and islets with trees, off the board.
    for _ in range(14):
        while True:
            x = rng.uniform(-5, 13)
            z = rng.uniform(-5, 13)
            if not (-0.8 < x < 8.8 and -0.8 < z < 8.8):
                break
        size = rng.uniform(0.25, 0.7)
        m.box(rng.choice(["rock", "rock-b", "stone"]), (ORIGIN + x, -0.25 + size * 0.4, ORIGIN + z),
              (size, size * 1.2, size * 0.9), (rng.uniform(-10, 10), rng.uniform(0, 90), rng.uniform(-10, 10)),
              bevel=0.06)
    for ix, iz, r in ((-3.5, 2.0, 1.6), (2.0, -3.6, 1.3), (-2.6, 10.5, 1.2)):
        cx, cz = ORIGIN + ix, ORIGIN + iz
        m.box("sand", (cx, -0.1, cz), (r * 1.6, 0.8, r * 1.4), (0, 20, 0), bevel=0.12)
        m.box("grass", (cx, 0.45, cz), (r * 1.1, 0.5, r * 1.0), (0, 20, 0), bevel=0.1)
        for t in range(3):
            tree(m, rng, cx + rng.uniform(-0.4, 0.4) * r, 0.7, cz + rng.uniform(-0.35, 0.35) * r,
                 pine=rng.random() < 0.5)

    # The causeway itself: a stone road on posts from the island's east edge
    # out to the mainland, low enough that the tide covers it.
    road_z = ORIGIN + 3.5
    for k in range(10):
        x = ORIGIN + SIZE + 0.5 + k
        m.box("road" if k % 2 else "road-d", (x, 0.55, road_z), (1.0, 0.3, 1.2), bevel=0.04)
        m.box("post", (x - 0.4, 0.15, road_z - 0.62), (0.1, 1.2, 0.1))
        m.box("post", (x - 0.4, 0.15, road_z + 0.62), (0.1, 1.2, 0.1))
        if k % 2 == 0:
            m.box("rope", (x, 0.82, road_z - 0.62), (1.0, 0.03, 0.03))
            m.box("rope", (x, 0.82, road_z + 0.62), (1.0, 0.03, 0.03))
    m.box("sand", (ORIGIN + SIZE + 14, -0.2, ORIGIN + 4), (8, 1.2, 30), bevel=0.2)
    m.box("grass", (ORIGIN + SIZE + 15.5, 0.6, ORIGIN + 4), (6, 0.8, 30), bevel=0.2)
    for t in range(9):
        tree(m, rng, ORIGIN + SIZE + 14 + rng.uniform(0, 3), 1.0, ORIGIN - 8 + t * 3.2 + rng.uniform(-1, 1),
             pine=True)
    return m


# --- Textures ------------------------------------------------------------------

def textures():
    from PIL import Image, ImageDraw

    TEXTURES.mkdir(parents=True, exist_ok=True)
    # The sea: a calm swell and the odd glint, tiled across the plane.
    size = 256
    water = Image.new("RGBA", (size, size))
    px = water.load()
    for y in range(size):
        for x in range(size):
            v = 0.5 + 0.25 * math.sin((x + y) * 2 * math.pi / size * 3) + 0.25 * math.sin((x - 2 * y) * 2 * math.pi / size * 2)
            g = int(222 + 33 * v)
            px[x, y] = (g, g, g, 255)
    rng = random.Random(5)
    for _ in range(70):
        x, y, length = rng.randrange(size), rng.randrange(size), rng.randrange(5, 16)
        for d in range(length):
            px[(x + d) % size, y] = (255, 255, 255, 255)
    water.save(TEXTURES / "water.png")

    # Marks on the ground: one sheet, a frame each, drawn at 64 pixels.
    marks = [
        ("move", (110, 200, 255), "fill"),
        ("aim", (255, 196, 80), "fill"),
        ("danger", (255, 86, 70), "stripes"),
        ("spawn", (196, 120, 255), "ring"),
        ("tide", (120, 210, 255), "waves"),
        ("selected", (255, 255, 255), "corners"),
        ("cursor", (255, 246, 214), "brackets"),
        ("arrow", (255, 120, 90), "arrow"),
        ("push", (255, 255, 255), "arrow"),
    ]
    cell = 64
    sheet = Image.new("RGBA", (cell * len(marks) + 4 * (len(marks) - 1), cell), (0, 0, 0, 0))
    names = []
    for i, (name, rgb, style) in enumerate(marks):
        tile = Image.new("RGBA", (cell, cell), (0, 0, 0, 0))
        d = ImageDraw.Draw(tile)
        r, g, b = rgb
        if style == "fill":
            d.rounded_rectangle((3, 3, cell - 4, cell - 4), 8, fill=(r, g, b, 70), outline=(r, g, b, 235), width=3)
        elif style == "stripes":
            d.rounded_rectangle((3, 3, cell - 4, cell - 4), 8, fill=(r, g, b, 60))
            for k in range(-cell, cell, 12):
                d.line((k, cell, k + cell, 0), fill=(r, g, b, 150), width=4)
            d.rounded_rectangle((3, 3, cell - 4, cell - 4), 8, outline=(r, g, b, 255), width=3)
        elif style == "ring":
            d.ellipse((10, 10, cell - 11, cell - 11), outline=(r, g, b, 235), width=4)
            d.ellipse((22, 22, cell - 23, cell - 23), fill=(r, g, b, 150))
        elif style == "waves":
            for k in range(3):
                y = 16 + k * 14
                d.arc((6, y - 6, 32, y + 6), 200, 340, fill=(r, g, b, 210), width=3)
                d.arc((32, y - 6, 58, y + 6), 200, 340, fill=(r, g, b, 210), width=3)
        elif style == "corners":
            for (x0, y0, dx, dy) in ((4, 4, 1, 1), (cell - 5, 4, -1, 1), (4, cell - 5, 1, -1), (cell - 5, cell - 5, -1, -1)):
                d.line((x0, y0, x0 + dx * 16, y0), fill=(r, g, b, 255), width=4)
                d.line((x0, y0, x0, y0 + dy * 16), fill=(r, g, b, 255), width=4)
        elif style == "brackets":
            d.rounded_rectangle((2, 2, cell - 3, cell - 3), 8, outline=(r, g, b, 200), width=3)
        elif style == "arrow":
            d.polygon([(cell * 0.5, 6), (cell - 10, cell * 0.5), (cell * 0.62, cell * 0.5), (cell * 0.62, cell - 8),
                       (cell * 0.38, cell - 8), (cell * 0.38, cell * 0.5), (10, cell * 0.5)], fill=(r, g, b, 235))
        sheet.paste(tile, (i * (cell + 4), 0))
        names.append(name)
    sheet.save(TEXTURES / "marks.png")
    width = sheet.width
    (TEXTURES / "marks.sheet").write_text(
        '{\n  "format_version": 1,\n  "anchor": "center",\n  "grid": {\n'
        f'    "columns": {len(names)},\n    "rows": 1,\n    "size": [{width}, {cell}],\n'
        '    "spacing": [4, 0],\n    "names": [' + ", ".join(f'"{n}"' for n in names) + ']\n  }\n}\n')

    # A soft round shadow under every figure, which a billboard never cast and
    # a model standing on a lit tile needs less, but still reads better with.
    blob = Image.new("RGBA", (64, 64), (0, 0, 0, 0))
    bp = blob.load()
    for y in range(64):
        for x in range(64):
            d2 = ((x - 31.5) / 30) ** 2 + ((y - 31.5) / 30) ** 2
            bp[x, y] = (12, 16, 24, int(max(0.0, 1.0 - d2) ** 1.5 * 140))
    blob.save(TEXTURES / "blob.png")

    # A hit: a white star that opens and fades over four frames, drawn facing
    # the camera over whatever was struck.
    frames = 4
    spark = Image.new("RGBA", (64 * frames + 4 * (frames - 1), 64), (0, 0, 0, 0))
    for f in range(frames):
        tile = Image.new("RGBA", (64, 64), (0, 0, 0, 0))
        d = ImageDraw.Draw(tile)
        reach = 12 + f * 6
        alpha = 255 - f * 55
        for k in range(8):
            a = k * math.pi / 4 + (0.2 if k % 2 else 0.0)
            length = reach if k % 2 == 0 else reach * 0.6
            x, y = 32 + math.cos(a) * length, 32 + math.sin(a) * length
            d.line((32, 32, x, y), fill=(255, 250, 225, alpha), width=max(1, 5 - f))
        d.ellipse((32 - 7 + f, 32 - 7 + f, 32 + 7 - f, 32 + 7 - f), fill=(255, 255, 255, alpha))
        spark.paste(tile, (f * 68, 0))
    spark.save(TEXTURES / "spark.png")
    (TEXTURES / "spark.sheet").write_text(
        '{\n  "format_version": 1,\n  "anchor": "center",\n  "grid": {\n'
        f'    "columns": {frames},\n    "rows": 1,\n    "size": [{spark.width}, 64],\n'
        '    "spacing": [4, 0],\n    "names": ["spark-0", "spark-1", "spark-2", "spark-3"]\n  }\n}\n')


def title_textures():
    """The title's shading, the halo behind its logo, its button plates and a
    white square to fade with. Drawn at four times their size and scaled down,
    which is the antialiasing."""
    from PIL import Image, ImageDraw

    ink = (5, 13, 19)

    # Shade over the top and bottom of the screen, so the logo and the menu
    # read over a bright sea.
    for name, flip in (("shade-down", False), ("shade-up", True)):
        shade = Image.new("RGBA", (8, 256))
        for y in range(256):
            t = y / 255
            alpha = int(235 * (1 - t) ** 1.7)
            for x in range(8):
                shade.putpixel((x, 255 - y if flip else y), (*ink, alpha))
        shade.save(TEXTURES / f"{name}.png")

    # And down the left of a wide screen, where the logo and menu stand.
    side = Image.new("RGBA", (256, 8))
    for x in range(256):
        t = x / 255
        alpha = int(225 * (1 - t) ** 1.5)
        for y in range(8):
            side.putpixel((x, y), (*ink, alpha))
    side.save(TEXTURES / "shade-side.png")

    # A soft dark cloud behind the logo, wider than tall.
    halo = Image.new("RGBA", (256, 128))
    for y in range(128):
        for x in range(256):
            d2 = ((x - 127.5) / 128) ** 2 + ((y - 63.5) / 64) ** 2
            halo.putpixel((x, y), (*ink, int(max(0.0, 1.0 - d2) ** 1.1 * 240)))
    halo.save(TEXTURES / "halo.png")

    # A soft round speck for whatever drifts in the air; white, so a tint
    # makes it pollen, rain, a firefly or an ember.
    mote = Image.new("RGBA", (32, 32))
    for y in range(32):
        for x in range(32):
            d2 = ((x - 15.5) / 15.5) ** 2 + ((y - 15.5) / 15.5) ** 2
            mote.putpixel((x, y), (255, 255, 255, int(max(0.0, 1.0 - d2) ** 1.6 * 255)))
    mote.save(TEXTURES / "mote.png")

    white = Image.new("RGBA", (4, 4), (255, 255, 255, 255))
    white.save(TEXTURES / "white.png")

    # A button plate: a banner with pointed ends, a brass rim, an inner line
    # and a diamond at each point. Ember for the way in, ink for the rest.
    scale = 4
    w, h = 640, 128
    for name, top, bottom, rim in (
        ("plate-ember", (214, 124, 56), (148, 66, 28), (255, 222, 160)),
        ("plate-ink", (22, 46, 58), (10, 24, 32), (233, 201, 138)),
    ):
        big = Image.new("RGBA", (w * scale, h * scale), (0, 0, 0, 0))
        W, H = big.size
        tip = H * 0.42
        inset = H * 0.16

        def banner(margin):
            m = margin
            return [(tip + m * 0.4, m), (W - tip - m * 0.4, m), (W - m * 1.6, H / 2), (W - tip - m * 0.4, H - m),
                    (tip + m * 0.4, H - m), (m * 1.6, H / 2)]

        body = Image.new("RGBA", big.size, (0, 0, 0, 0))
        mask = Image.new("L", big.size, 0)
        ImageDraw.Draw(mask).polygon(banner(inset), fill=255)
        for y in range(H):
            t = y / (H - 1)
            c = tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
            ImageDraw.Draw(body).line((0, y, W, y), fill=(*c, 238))
        big.paste(body, (0, 0), mask)
        d = ImageDraw.Draw(big)
        d.line(banner(inset) + [banner(inset)[0]], fill=(*rim, 255), width=3 * scale, joint="curve")
        d.line(banner(inset + 7 * scale) + [banner(inset + 7 * scale)[0]], fill=(*rim, 110), width=scale,
               joint="curve")
        for cx in (inset * 0.55, W - inset * 0.55):
            r = H * 0.1
            d.polygon([(cx, H / 2 - r), (cx + r * 0.7, H / 2), (cx, H / 2 + r), (cx - r * 0.7, H / 2)],
                      fill=(*rim, 255))
        big.resize((w, h), Image.LANCZOS).save(TEXTURES / f"{name}.png")

    # A boon card: dark and near grey, with a light rim and corner diamonds,
    # so a tint colors it for the warden it is for.
    w, h = 300, 400
    big = Image.new("RGBA", (w * scale, h * scale), (0, 0, 0, 0))
    W, H = big.size
    d = ImageDraw.Draw(big)
    m = 10 * scale
    for y in range(m, H - m):
        t = (y - m) / (H - 2 * m)
        c = int(40 - 18 * t)
        d.line((m, y, W - m, y), fill=(c, c + 4, c + 8, 236))
    d.rounded_rectangle((m, m, W - m, H - m), 14 * scale, outline=(236, 226, 205, 255), width=3 * scale)
    d.rounded_rectangle((m + 9 * scale, m + 9 * scale, W - m - 9 * scale, H - m - 9 * scale), 10 * scale,
                        outline=(236, 226, 205, 110), width=scale)
    # Diamonds in the corners, which a nine-slice never stretches.
    for cx, cy in ((m + 6 * scale, m + 6 * scale), (W - m - 6 * scale, m + 6 * scale),
                   (m + 6 * scale, H - m - 6 * scale), (W - m - 6 * scale, H - m - 6 * scale)):
        r = 8 * scale
        d.polygon([(cx, cy - r), (cx + r * 0.7, cy), (cx, cy + r), (cx - r * 0.7, cy)], fill=(236, 226, 205, 255))
    big.resize((w, h), Image.LANCZOS).save(TEXTURES / "card.png")
    # Drawn sliced, so a card keeps its rim whether it stands tall on a desk or
    # lies wide on a phone.
    (TEXTURES / "card.sheet").write_text(
        '{\n  "format_version": 1,\n  "size": [300, 400],\n'
        '  "grid": { "columns": 1, "rows": 1, "names": ["card"] },\n'
        '  "borders": { "": [40, 40, 40, 40] }\n}\n')


def main():
    MODELS.mkdir(parents=True, exist_ok=True)
    for name, (build, palette, poses) in FIGURES.items():
        for pose in poses:
            m = Mesh()
            build(m, pose)
            suffix = "" if pose == "idle" else f"-{pose}"
            m.write(MODELS / f"{name}{suffix}.glb", palette, name)
    for index, island in enumerate(islands()):
        mesh = island_mesh(island, seed=index + 1)
        slug = island["name"].lower().replace(" ", "-").replace("the-", "")
        mesh.write(MODELS / f"island-{slug}.glb", GROUND, island["name"])
        print(f"island {index}: {island['name']} -> island-{slug}.glb")
    textures()
    title_textures()
    print("wrote", len(list(MODELS.glob("*.glb"))), "models")


if __name__ == "__main__":
    main()
