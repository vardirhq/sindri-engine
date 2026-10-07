"""The contraptions, one function each. Coordinates are the room's: x from
-30 to 30 and y from -14 (the floor) to 14 (the ceiling)."""

from __future__ import annotations

from common import bit, body, box, circle, material, script, shape
from scene import PARTS, Scene, hinge, slider, spring


def loose_pile(scene: Scene) -> None:
    """Something to throw around while the contraptions are being built."""
    colors = ["#ff8a3d", "#ffd166", "#2ec4b6", "#ff5d8f", "#9b5de5"]
    for i in range(10):
        scene.crate(f"pile-crate-{i}", -4 + (i % 5) * 1.0, -13.5 + (i // 5) * 1.0, 0.9, 0.9,
                    0, colors[i % 5], "#ffffff")
    for i in range(12):
        scene.ball(f"pile-ball-{i}", 4 + (i % 6) * 0.8, -10 + (i // 6) * 0.8, 0.35, 0,
                   colors[(i + 2) % 5], "#ffffff")


TOYS = "scripts/toys.decay"
WOOD = "materials/wood.profile"
ICE = "materials/ice.profile"
RUBBER = "materials/rubber.profile"
STEEL = "materials/steel.profile"
CLAY = "materials/clay.profile"


def anchor(scene: Scene, entity_id, x, y, size=0.3, fill="#8fa3b8", name=None):
    """A static pin a joint can hang from, drawn as a bolt."""
    return scene.add(entity_id, name, x, y, 0.4, size, size, components={
        "sindri.shape": shape("ellipse", fill, "#e6edf5", sw=0.2, layer=9),
        "sindri.physics2d.collider": {"pieces": [
            circle(size / 2, "static", filter_mask=0)]},
    })


def wrecking_ball(scene: Scene) -> None:
    """A steel ball on a rigid arm, a hinge with a position motor at the top,
    and a castle of wood, ice and rubber to knock down."""
    pivot = (-21.5, 12.4)
    length = 7.0
    scene.wall("wreck-shelf", -15.8, 2.8, 7.6, 0.4, fill="#26384d")
    scene.wall("wreck-gantry", -21.5, 13.25, 2.4, 0.5, fill="#2b3f56")
    anchor(scene, "wreck-pivot", *pivot, 0.45)
    scene.add("wreck-arm", "wreck-arm", pivot[0], pivot[1] - length, 0.3, components={
        "sindri.physics2d.collider": {"pieces": [
            circle(1.1, "props", 0.4, 0.1),
            box(0.12, 2.85, "props", 0.4, 0.0, offset=(0.0, 3.85)),
        ]},
        "sindri.physics2d.rigid_body": body(damping=0.02, angular_damping=0.05),
        "sindri.physics2d.material": material(STEEL),
        "sindri.script": script(PARTS, "Part", toy=1.0),
    }, tags=("body", "cargo"))
    scene.deco("wreck-rod", 0, 3.85, 0.24, 5.7, fill="#6c7a8c", edge="#c3ccd8", sw=0.15,
               layer=7, parent="wreck-arm", z=-0.05)
    scene.deco("wreck-ball", 0, 0, 2.2, 2.2, kind="ellipse", fill="#2f3542",
               edge="#a4b0be", sw=0.06, layer=8, parent="wreck-arm")
    scene.deco("wreck-shine", -0.35, 0.4, 0.5, 0.35, kind="ellipse", fill="#ffffff",
               alpha=0.25, layer=9, parent="wreck-arm", z=0.05)
    scene.joint("wreck-hinge", "hinge", hinge(
        "wreck-pivot", "wreck-arm", (0.0, 0.0), (0.0, length)), 1)
    scene.add("wreck-control", "wreck-control", components={
        "sindri.script": script(TOYS, "WreckingBall")})

    # The castle: pillars, lintels and a crown, in three materials.
    base = 3.0

    y = base
    for row, (material_key, color) in enumerate([
        (WOOD, "#c98b4f"), (ICE, "#a8e6ff"), (WOOD, "#d9a066"),
    ]):
        for side, x in enumerate((-17.4, -14.2)):
            for k in range(2):
                scene.crate(f"castle-{row}-{side}-{k}", x, y + 0.4 + k * 0.8, 0.8, 0.8, 1,
                            color, mat=material(material_key),
                            tags=("body", "cargo"))
        y += 1.6
        scene.crate(f"castle-lintel-{row}", -15.8, y + 0.15, 4.0, 0.3, 1, "#8d5a2b",
                    mat=material(WOOD), tags=("body", "cargo"))
        y += 0.3

    scene.ball("castle-crown", -15.8, y + 0.55, 0.55, 1, "#ff4f79", "#ffd1dc",
               mat=material(RUBBER), tags=("body", "cargo"))


def gantry_crane(scene: Scene) -> None:
    """A trolley on a slider along the ceiling girder, a hook on a spring, and
    a grip that ties the hook to the cargo under it."""
    rail_y = 13.3
    scene.add("crane-rail", "crane-rail", -16.0, rail_y, 0.35, 26.0, 0.35, components={
        "sindri.shape": shape("rect", "#3a536e", "#8fb0cf", sw=0.25, layer=10),
        "sindri.physics2d.collider": {"pieces": [box(13.0, 0.17, "static", filter_mask=0)]},
    })
    scene.add("crane-trolley", "crane-trolley", -20.0, rail_y - 0.35, 0.45, 1.2, 0.5,
              components={
                  "sindri.shape": shape("rect", "#ffb703", "#fff1c1", sw=0.12, layer=11),
                  "sindri.physics2d.collider": {"pieces": [
                      box(0.6, 0.25, "props", filter_mask=0)]},
                  "sindri.physics2d.rigid_body": body(gravity=0.0, damping=0.5),
                  "sindri.script": script(PARTS, "Part", toy=2.0),
              })
    scene.joint("crane-slider", "slider", slider(
        "crane-rail", "crane-trolley", (1.0, 0.0), (0.0, 0.0), (0.0, 0.35),
        limits=(-12.0, 12.0)), 2)
    scene.add("crane-hook", "crane-hook", -20.0, rail_y - 2.3, 0.45, components={
        "sindri.physics2d.collider": {"pieces": [
            box(0.45, 0.12, "props", 0.8, 0.0),
            box(0.08, 0.3, "props", 0.8, 0.0, offset=(-0.4, -0.25)),
            box(0.08, 0.3, "props", 0.8, 0.0, offset=(0.4, -0.25)),
        ]},
        "sindri.physics2d.rigid_body": body(damping=0.3, angular_damping=2.0),
        "sindri.script": script(PARTS, "Part", toy=2.0),
    }, tags=("body",))
    scene.deco("crane-hook-bar", 0, 0, 0.9, 0.24, fill="#ffb703", edge="#fff1c1", sw=0.1,
               layer=12, parent="crane-hook")
    scene.deco("crane-hook-left", -0.4, -0.25, 0.16, 0.6, fill="#ffb703", layer=12,
               parent="crane-hook")
    scene.deco("crane-hook-right", 0.4, -0.25, 0.16, 0.6, fill="#ffb703", layer=12,
               parent="crane-hook")
    scene.joint("crane-cable", "spring", spring(
        "crane-trolley", "crane-hook", 1.4, 90.0, 6.0, (0.0, -0.25), (0.0, 0.12)), 2)
    scene.deco("crane-cable-line", 0, 0, 1, 0.06, fill="#d7e3f0", layer=10)
    scene.add("crane-grip", "crane-grip")
    scene.deco("crane-grip-line", 0, 0, 1, 0.08, fill="#ffb703", layer=10)
    scene.add("crane-control", "crane-control", components={
        "sindri.script": script(TOYS, "Crane")})



def cannon_gallery(scene: Scene) -> None:
    """A cannon on a motorised hinge, firing shells fast enough to pass
    straight through a sheet of glass in one step unless continuous
    collision is on, at a gallery of hanging panes that drop when hit."""
    base = (13.6, -13.3)
    scene.wall("cannon-base", base[0], base[1] + 0.2, 1.6, 0.8, fill="#34495e",
               edge="#7f9bb8")
    anchor(scene, "cannon-pivot", base[0], base[1] + 1.0, 0.5, "#5d6d7e")
    pivot = (base[0], base[1] + 1.0)
    scene.add("cannon-barrel", "cannon-barrel", pivot[0] + 1.0, pivot[1], 0.35, components={
        "sindri.physics2d.collider": {"pieces": [
            box(1.2, 0.3, "props", 0.5, 0.0, filter_mask=bit("static", "props", "balls"))]},
        "sindri.physics2d.rigid_body": body(damping=0.5, angular_damping=4.0),
        "sindri.script": script(PARTS, "Part", toy=3.0),
    })
    scene.deco("cannon-tube", 0, 0, 2.4, 0.6, fill="#2c3e50", edge="#95a5a6", sw=0.1,
               layer=12, parent="cannon-barrel")
    scene.deco("cannon-band", 0.95, 0, 0.18, 0.72, fill="#f39c12", layer=13,
               parent="cannon-barrel")
    scene.joint("cannon-hinge", "hinge", hinge(
        "cannon-pivot", "cannon-barrel", (0.0, 0.0), (-1.0, 0.0), limits=(-0.15, 1.2),
        motor={"motor_mode": "position", "motor_target_angle": 0.25,
               "motor_stiffness": 400.0, "motor_damping": 60.0, "motor_max_torque": 3000.0}),
        3)
    scene.add("cannon-control", "cannon-control", components={
        "sindri.script": script(TOYS, "Cannon", shell="prefabs/shell.prefab")})

    # The gallery: panes of glass hanging from a beam, and a padded back stop
    # that knows which shells arrived without touching any glass.
    beam_y = -2.8
    scene.add("gallery-beam", "gallery-beam", 19.0, beam_y, 0.3, 7.0, 0.3, components={
        "sindri.shape": shape("rect", "#3a536e", "#8fb0cf", sw=0.2, layer=10),
        "sindri.physics2d.collider": {"pieces": [box(3.5, 0.15, "static", filter_mask=0)]},
    })
    for index, x in enumerate((16.6, 18.2, 19.8, 21.4)):
        scene.add(f"pane-{index}", f"pane-{index}", x, beam_y - 4.6, 0.25, 0.1, 8.8,
                  components={
                      "sindri.shape": shape("rect", "#a8e6ff", "#e8f8ff", sw=0.25, layer=8,
                                            alpha=0.45),
                      "sindri.physics2d.collider": {"pieces": [
                          box(0.05, 4.4, "glass", 0.2, 0.1)]},
                      "sindri.physics2d.rigid_body": body(damping=0.1, angular_damping=0.4),
                      "sindri.script": script(TOYS, "GlassPane", index=float(index)),
                  }, tags=("body", "cargo", "glass"))
        scene.joint(f"pane-hinge-{index}", "hinge", hinge(
            "gallery-beam", f"pane-{index}", (x - 19.0, 0.0), (0.0, 4.4)), 3)
    scene.wall("gallery-backstop", 22.8, -7.6, 0.4, 12.6, fill="#2b2f3a", edge="#c0392b")
    scene.add("gallery-catch", "gallery-catch", 22.3, -7.6, 0.2, 0.4, 12.6, components={
        "sindri.shape": shape("rect", "#c0392b", None, layer=4, alpha=0.18),
        "sindri.physics2d.collider": {"pieces": [
            box(0.2, 6.3, "static", sensor=True, filter_mask=bit("shells"))]},
        "sindri.script": script(TOYS, "BackStop"),
    })


def build(scene: Scene) -> None:
    loose_pile(scene)
    wrecking_ball(scene)
    gantry_crane(scene)
    cannon_gallery(scene)
