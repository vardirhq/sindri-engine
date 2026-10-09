"""The test track and the domino run: the middle of the room.

The robot is a Character 2D: steps it climbs, a slope down into a dip, a lift
that carries it up, one-way planks it jumps through and drops through, and
crates to shove and kick. Above it, a domino run ends on a spring-loaded red
button that sets off DROP EVERYTHING.
"""

from __future__ import annotations

from common import body, box, capsule, script, shape
from scene import Scene, slider, spring
from toys import slope

TRACK = "scripts/track.decay"
WOOD = "materials/wood.profile"

DECK_Y = -7.0
DIP_Y = -7.6
LOW_PLANK_Y = -4.3
HIGH_Y = -2.0
LIFT_X = 8.5


def ledge(scene: Scene, entity_id, left, right, top, thickness=0.3, fill="#26384d",
          edge="#7fa7c9", one_way=False):
    """A static shelf whose walking surface is at `top`."""
    extra = {}
    tags = None
    if one_way:
        extra["sindri.physics2d.one_way"] = {"normal": [0.0, 1.0], "angle": 0.7853982}
        tags = ("one-way",)
    entity = scene.wall(entity_id, (left + right) / 2, top - thickness / 2, right - left,
                        thickness, fill=fill, edge=edge, extra=extra)
    if tags:
        entity["components"]["sindri.tags"] = {"tags": list(tags)}
    return entity


def test_track(scene: Scene) -> None:
    ledge(scene, "track-deck", -5.6, 3.0, DECK_Y)
    # Three risers a quarter unit each: the robot steps up them without jumping.
    for i in range(3):
        left = -0.6 + i * 1.1
        rise = 0.25 * (i + 1)
        scene.wall(f"track-step-{i}", (left + 3.0) / 2, DECK_Y + rise / 2, 3.0 - left, rise,
                   fill="#2b4058", edge="#7fa7c9")
    # Down the slope into the dip where the lift waits.
    top = (3.0, DECK_Y + 0.75)
    foot = (6.3, DIP_Y)
    slope(scene, "track-slope", top, foot, 0.3, fill="#2b4058", edge="#7fa7c9")
    ledge(scene, "track-dip", 6.1, LIFT_X - 0.9, DIP_Y)

    # The lift: a kinematic platform the controller rides, carried by its
    # solved motion rather than by anything the robot's script adds.
    scene.add("track-lift", "track-lift", LIFT_X, DIP_Y, 0.3, 1.8, 0.3, components={
        "sindri.shape": shape("rect", "#ffd166", "#fff1c1", sw=0.05, layer=6),
        # A lip on the far end rides with the platform, so a rider pressed
        # against it is never dragged along a wall that stands still.
        "sindri.physics2d.collider": {"pieces": [
            box(0.9, 0.15, "static", 0.8, 0.0),
            box(0.06, 0.3, "static", 0.8, 0.0, offset=(0.84, 0.4)),
        ]},
        "sindri.physics2d.rigid_body": body("kinematic_velocity", lock=True, gravity=0.0),
        "sindri.script": script(TRACK, "TrackLift", bottom=DIP_Y, top=HIGH_Y - 0.15),
    }, tags=("platform",))
    scene.deco("track-lift-lip", 0.84 / 1.8, 0.4 / 0.3, 0.12 / 1.8, 0.6 / 0.3, fill="#ffd166",
               layer=6, parent="track-lift", z=0.01)
    scene.deco("track-lift-shaft", LIFT_X, (DIP_Y + HIGH_Y) / 2, 0.08, HIGH_Y - DIP_Y,
               fill="#3a5068", layer=-5, z=-0.5)

    ledge(scene, "track-summit", 9.4, 11.8, HIGH_Y)
    ledge(scene, "track-plank-high", 2.6, 7.5, HIGH_Y, 0.2, "#7a5230", "#d9a066", True)
    ledge(scene, "track-plank-low", -2.6, 2.0, LOW_PLANK_Y, 0.2, "#7a5230", "#d9a066", True)

    # Things to shove and kick.
    for i, (x, y) in enumerate([(-3.7, DECK_Y + 0.45), (-3.7, DECK_Y + 1.35),
                                (10.3, HIGH_Y + 0.45), (11.2, HIGH_Y + 0.45),
                                (10.75, HIGH_Y + 1.35)]):
        scene.crate(f"track-crate-{i}", x, y, 0.9, 0.9, 7, "#ef476f", "#ffd6e0",
                    friction=0.6, tags=("body", "kickable"))
    scene.ball("track-ball", 0.0, LOW_PLANK_Y + 0.45, 0.4, 7, "#06d6a0", "#c9fff0",
               friction=0.5, restitution=0.5, tags=("body", "kickable"))

    robot(scene, -2.4, DECK_Y + 0.65)


def robot(scene: Scene, x, y) -> None:
    scene.add("robot", "Robot", x, y, 0.6, components={
        "sindri.physics2d.collider": {"pieces": [
            capsule(0.25, 0.34, "robot", 0.0, 0.0),
        ]},
        "sindri.physics2d.character": {
            "skin": 0.01, "max_iterations": 8, "up": [0.0, 1.0],
            "max_slope_angle": 0.8, "snap_distance": 0.25, "step_height": 0.3,
            "carry_platforms": True,
        },
        "sindri.script": script(TRACK, "Robot"),
    }, tags=("robot",))
    scene.deco("robot-body", 0, 0.02, 0.72, 0.86, fill="#3a86ff", edge="#d7e7ff", sw=0.05,
               layer=8, parent="robot", z=0.01)
    scene.entities[-1]["components"]["sindri.shape"]["corner_radius"] = 0.18
    scene.deco("robot-visor", 0.12, 0.2, 0.42, 0.16, fill="#ffd166", layer=9,
               parent="robot", z=0.02)
    scene.deco("robot-antenna", 0, 0.56, 0.06, 0.24, fill="#d7e7ff", layer=9,
               parent="robot", z=0.02)
    scene.deco("robot-light", 0, 0.7, 0.16, 0.16, kind="ellipse", fill="#ff5d8f",
               layer=9, parent="robot", z=0.02)
    scene.deco("robot-wheel", 0, -0.48, 0.5, 0.18, kind="ellipse", fill="#14223a",
               edge="#7fa7c9", sw=0.04, layer=9, parent="robot", z=0.02)
    scene.deco("robot-boot", 0.4, -0.42, 0.36, 0.16, fill="#ffd166", layer=9,
               parent="robot", z=0.03)["disabled"] = True


LEDGE_TOP = 3.0
DOMINOES = 14
BUTTON_X = 9.15


def domino_run(scene: Scene) -> None:
    ledge(scene, "domino-ledge", -5.6, 10.8, LEDGE_TOP)
    colors = ["#ff595e", "#ff924c", "#ffca3a", "#8ac926", "#1982c4", "#6a4c93"]
    for i in range(DOMINOES):
        x = -4.7 + i * 0.95
        scene.crate(f"domino-{i}", x, LEDGE_TOP + 0.85, 0.24, 1.7, 8, colors[i % 6], "#ffffff",
                    friction=0.45, restitution=0.0, mat=None, tags=("body", "domino"))
    red_button(scene)


def red_button(scene: Scene) -> None:
    rest = LEDGE_TOP + 0.62
    scene.add("button-socket", "button-socket", BUTTON_X, LEDGE_TOP + 0.08, 0.3, 1.5, 0.16,
              components={
                  "sindri.shape": shape("rect", "#3d4554", "#9aa7bb", sw=0.05, layer=6),
                  "sindri.physics2d.collider": {"pieces": [
                      box(0.75, 0.08, "static", filter_mask=0)]},
              })
    scene.add("button-cap", "button-cap", BUTTON_X, rest, 0.3, 1.1, 0.36, components={
        "sindri.shape": shape("rect", "#e63946", "#ffb3b8", sw=0.06, layer=7),
        "sindri.physics2d.collider": {"pieces": [box(0.55, 0.18, "props", 0.6, 0.0)]},
        "sindri.physics2d.rigid_body": body(damping=0.2, angular_damping=0.5, lock=True),
        "sindri.script": script(TRACK, "RedButton", toy=8.0),
    }, tags=("body", "button"))
    scene.entities[-1]["components"]["sindri.shape"]["corner_radius"] = 0.12
    lift = rest - (LEDGE_TOP + 0.08)
    scene.joint("button-rail", "slider", slider(
        "button-socket", "button-cap", axis=(0, 1), first_anchor=(0, lift),
        limits=(-0.36, 0.0)), 8)
    scene.joint("button-spring", "spring", spring(
        "button-socket", "button-cap", lift + 0.4, 30.0, 1.2), 8)
