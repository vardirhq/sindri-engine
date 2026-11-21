#!/usr/bin/env python3
"""Builds DRIFT's wreckage models: `assets/models/*.glb`.

Stage 4 of `docs/drift-plan.md`, "The wreckage". Everything outside the
*Tern*'s forward hatch is made here from boxes and cylinders by the repository's
own GLB writer (`tools/glb.py`), the way Causeway's art is: a derelict hull
module torn open at one end, bent hull plates and a truss girder for the debris
field, the salvage canisters worth going out for, and a starfield drawn as one
model so the dark has depth.

Everything is drawn from fixed numbers and one seed, so re-running this changes
nothing. `hold.py` places the models in the scene.
"""
import math
import pathlib
import random
import sys

TOOLS = pathlib.Path(__file__).resolve().parents[3] / "tools"
sys.path.insert(0, str(TOOLS))

from glb import Material, Mesh  # noqa: E402

MODELS = pathlib.Path(__file__).resolve().parent.parent / "assets" / "models"

MATERIALS = {
    "hull": Material("#6b7078", roughness=0.7, metallic=0.5),
    "hull-dark": Material("#3c4048", roughness=0.8, metallic=0.4),
    "rib": Material("#8a8f96", roughness=0.6, metallic=0.6),
    "burnt": Material("#2a2520", roughness=0.95),
    "stripe": Material("#d9a21e", roughness=0.6),
    "warning": Material("#3a0806", emissive="#ff2a14", strength=1.0),
    "canister": Material("#9aa0a6", roughness=0.4, metallic=0.7),
    "band": Material("#e2721c", roughness=0.6),
    "indicator": Material("#0a2a14", emissive="#2bff7a", strength=1.0),
    "star": Material("#000000", emissive="#dfe8ff", strength=1.0),
    "star-warm": Material("#000000", emissive="#ffd9a8", strength=1.0),
}

# The derelict: an octagonal hull module along X, torn open at its far end.
WRECK_LENGTH = 8.0
WRECK_RADIUS = 2.6
WRECK_SIDES = 8


def wreck():
    """A survey module like the *Tern*'s, broken off its ship long ago."""
    mesh = Mesh()
    rng = random.Random(41)
    half = WRECK_LENGTH / 2
    step = 2 * math.pi / WRECK_SIDES
    panel_width = 2 * WRECK_RADIUS * math.tan(step / 2) * 0.98
    segments = 4
    segment = WRECK_LENGTH / segments
    for side in range(WRECK_SIDES):
        angle = side * step
        y, z = WRECK_RADIUS * math.cos(angle), WRECK_RADIUS * math.sin(angle)
        for s in range(segments):
            x = -half + segment * (s + 0.5)
            # The far end is torn away: panels there go missing or hang loose.
            torn = s == segments - 1 and rng.random() < 0.55
            if torn:
                continue
            twist = rng.uniform(-6, 6) if s == segments - 1 else 0.0
            material = "hull-dark" if (side + s) % 3 == 0 else "hull"
            if s == segments - 1 and rng.random() < 0.4:
                material = "burnt"
            mesh.box(material, (x, y, z), (segment * 0.98, 0.12, panel_width),
                     rotation=(math.degrees(angle) + twist, 0, 0), bevel=0.03)
    # Ribs: a ring of short struts at each segment joint.
    for s in range(segments):
        x = -half + segment * s
        for side in range(WRECK_SIDES):
            angle = (side + 0.5) * step
            r = WRECK_RADIUS * math.cos(step / 2) - 0.05
            mesh.box("rib", (x, r * math.cos(angle), r * math.sin(angle)), (0.22, 0.22, panel_width * 1.02),
                     rotation=(math.degrees(angle), 0, 0))
    # The near end is closed by a bulkhead with a dark hatch, striped round.
    mesh.cylinder("hull-dark", (-half - 0.06, 0, 0), WRECK_RADIUS * 0.98, 0.12,
                  segments=WRECK_SIDES, rotation=(0, 0, 90))
    mesh.box("stripe", (-half - 0.14, 0, 0), (0.04, 1.6, 1.6))
    mesh.box("burnt", (-half - 0.16, 0, 0), (0.04, 1.3, 1.3))
    # A warning lamp still burning inside, on the deck.
    mesh.box("warning", (0.5, -WRECK_RADIUS + 0.35, 0.0), (0.6, 0.08, 0.2))
    mesh.write(MODELS / "wreck.glb", MATERIALS, name="wreck")


def plate():
    """A hull plate torn off and bent across its middle."""
    mesh = Mesh()
    mesh.box("hull", (-0.38, 0.05, 0), (0.8, 0.06, 1.0), rotation=(0, 0, 12), bevel=0.02)
    mesh.box("hull-dark", (0.38, 0.05, 0), (0.8, 0.06, 1.0), rotation=(0, 0, -12), bevel=0.02)
    mesh.box("stripe", (-0.38, 0.09, 0.42), (0.7, 0.02, 0.1), rotation=(0, 0, 12))
    mesh.write(MODELS / "plate.glb", MATERIALS, name="plate")


def girder():
    """A three-metre length of truss: two rails and their bracing."""
    mesh = Mesh()
    length = 3.0
    for z in (-0.3, 0.3):
        mesh.box("rib", (0, 0, z), (length, 0.12, 0.12), bevel=0.02)
    bays = 5
    bay = length / bays
    for i in range(bays + 1):
        mesh.box("rib", (-length / 2 + bay * i, 0, 0), (0.08, 0.08, 0.6))
    for i in range(bays):
        x = -length / 2 + bay * (i + 0.5)
        angle = math.degrees(math.atan2(0.6, bay)) * (1 if i % 2 else -1)
        mesh.box("hull-dark", (x, 0, 0), (math.hypot(bay, 0.6), 0.06, 0.06), rotation=(0, angle, 0))
    mesh.write(MODELS / "girder.glb", MATERIALS, name="girder")


def canister():
    """A salvage canister: a sealed cylinder banded orange, its light still green."""
    mesh = Mesh()
    mesh.cylinder("canister", (0, 0, 0), 0.2, 0.6, segments=10)
    for y in (-0.18, 0.18):
        mesh.cylinder("band", (0, y, 0), 0.21, 0.06, segments=10)
    mesh.cylinder("hull-dark", (0, 0.33, 0), 0.12, 0.06, segments=8)
    mesh.box("indicator", (0, 0.0, 0.205), (0.06, 0.1, 0.02))
    mesh.write(MODELS / "canister.glb", MATERIALS, name="canister")


STAR_DISTANCE = 150.0


def stars():
    """Points of light on a far sphere: one model, so the whole sky is one draw."""
    mesh = Mesh()
    rng = random.Random(7)
    for i in range(420):
        # Even over the sphere: a uniform height and a uniform turn.
        y = rng.uniform(-1, 1)
        turn = rng.uniform(0, 2 * math.pi)
        r = math.sqrt(1 - y * y)
        direction = (r * math.cos(turn), y, r * math.sin(turn))
        size = rng.choice((0.25, 0.3, 0.4, 0.6)) if i % 9 else 0.9
        centre = tuple(c * STAR_DISTANCE for c in direction)
        material = "star-warm" if i % 5 == 0 else "star"
        mesh.box(material, centre, (size, size, size), rotation=(rng.uniform(0, 90), rng.uniform(0, 90), 0))
    mesh.write(MODELS / "stars.glb", MATERIALS, name="stars")


def main():
    MODELS.mkdir(parents=True, exist_ok=True)
    wreck()
    plate()
    girder()
    canister()
    stars()


if __name__ == "__main__":
    main()
