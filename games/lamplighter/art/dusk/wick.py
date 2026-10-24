"""Wick, the courier: a recipe for the Sprite Baker, one model per animation frame.

A figure of boxes, cylinders and spheres, posed by joint angles, so that "the
front leg swings 38 degrees in run-1" is one number and a walk cycle is a table.
Side view, facing +X (the right of the screen). 16 pixels to a world unit.

Each limb is a cylinder hung from a joint: it is placed at the joint plus half its
length along the limb's direction, and turned to match, so it swings about the
joint rather than about its own middle.
"""

import json
import math
import sys

PPU = 16

MATERIALS = {
    "coat": {"color": "#3f5f9c"},
    "trousers": {"color": "#2a3550"},
    "skin": {"color": "#e0a98a"},
    "scarf": {"color": "#f0912e", "unlit": False},
    "hat": {"color": "#2b2a45"},
    "boot": {"color": "#4a3640"},
    "wood": {"color": "#8a6258"},
    "iron": {"color": "#4e566f"},
    "flame": {"color": "#ffc35c", "unlit": True},
    "eye": {"color": "#1d1b3a", "unlit": True},
}

HIP = (0.0, 0.52)
SHOULDER = (0.02, 0.9)
HEAD = (0.05, 1.14)


def limb(joint, angle, length, radius, material, z=0.0):
    """A cylinder hanging from `joint`, swung `angle` degrees forward (toward +X)."""
    a = math.radians(angle)
    dx, dy = math.sin(a), -math.cos(a)
    cx, cy = joint[0] + dx * length / 2, joint[1] + dy * length / 2
    # A cylinder stands along Y, so turning it about Z by -angle swings its foot forward.
    return {
        "type": "cylinder",
        "material": material,
        "radius": radius,
        "height": length,
        "segments": 10,
        "position": [round(cx, 4), round(cy, 4), z],
        "rotation": [0, 0, round(-angle, 3)],
    }, (joint[0] + dx * length, joint[1] + dy * length)


def box(material, size, position, angle=0.0, z=0.0):
    return {
        "type": "box",
        "material": material,
        "size": list(size),
        "position": [round(position[0], 4), round(position[1], 4), z],
        "rotation": [0, 0, round(-angle, 3)],
    }


def sphere(material, radius, position, squash=(1, 1, 1), z=0.0):
    return {
        "type": "sphere",
        "material": material,
        "radius": radius,
        "segments": 12,
        "position": [round(position[0], 4), round(position[1], 4), z],
        "scale": list(squash),
    }


def pose(name, *, bob=0.0, lean=0.0, front_leg=0.0, back_leg=0.0, front_knee=0.0, back_knee=0.0,
         front_arm=20.0, back_arm=-20.0, front_elbow=0.0, staff_angle=0.0, scarf=-18.0, head_tilt=0.0):
    """One frame: every angle in degrees, positive swinging toward the front (+X)."""
    parts = []
    hip = (HIP[0], HIP[1] + bob)
    shoulder = (SHOULDER[0] + math.sin(math.radians(lean)) * 0.1, SHOULDER[1] + bob)
    head = (HEAD[0] + math.sin(math.radians(lean)) * 0.14, HEAD[1] + bob)

    # Back limbs first, so the front ones are drawn over them.
    for leg_angle, knee_bend, shade in ((back_leg, back_knee, "trousers"), (front_leg, front_knee, "trousers")):
        thigh, knee = limb(hip, leg_angle, 0.26, 0.11, shade)
        shin, foot = limb(knee, leg_angle - knee_bend, 0.26, 0.1, shade)
        parts += [thigh, shin, box("boot", (0.32, 0.17, 0.26), (foot[0] + 0.06, foot[1] + 0.04), leg_angle - knee_bend)]

    back_upper, back_elbow = limb(shoulder, back_arm, 0.2, 0.085, "coat", z=-0.05)
    back_fore, back_hand = limb(back_elbow, back_arm + 8, 0.18, 0.075, "coat", z=-0.05)
    parts += [back_upper, back_fore, sphere("skin", 0.085, back_hand)]

    parts.append(box("coat", (0.44, 0.46, 0.32), ((hip[0] + shoulder[0]) / 2, (hip[1] + shoulder[1]) / 2 + 0.02), -lean))
    parts.append(box("coat", (0.5, 0.16, 0.34), (hip[0], hip[1] - 0.02), -lean * 0.5))  # coat skirt
    # Scarf: a band at the neck and a tail that streams back.
    neck = (shoulder[0] + 0.01, shoulder[1] + 0.08)
    parts.append(box("scarf", (0.38, 0.13, 0.34), neck))
    tail, tail_end = limb((neck[0] - 0.14, neck[1] - 0.02), scarf, 0.4, 0.075, "scarf")
    parts.append(tail)

    parts.append(sphere("skin", 0.25, head, squash=(1, 1.02, 1)))
    parts.append(box("eye", (0.07, 0.1, 0.06), (head[0] + 0.1, head[1] + 0.03), z=0.2))
    parts.append(sphere("skin", 0.075, (head[0] + 0.24, head[1] - 0.03), z=0.04))
    parts.append(box("hat", (0.64, 0.07, 0.5), (head[0] + 0.01, head[1] + 0.17), head_tilt))
    brim_top = (head[0] + 0.01, head[1] + 0.2)
    parts.append({"type": "cone", "material": "hat", "radius": 0.26, "height": 0.32, "segments": 10,
                  "position": [round(brim_top[0] - 0.02, 4), round(brim_top[1] + 0.15, 4), 0.0],
                  "rotation": [0, 0, round(-head_tilt - 8, 3)]})
    
    # The front arm holds the staff: an upper arm, a forearm, a hand, and the staff through it.
    front_upper, front_elbow_pos = limb(shoulder, front_arm, 0.2, 0.09, "coat", z=0.06)
    front_fore, front_hand = limb(front_elbow_pos, front_arm + front_elbow, 0.18, 0.08, "coat", z=0.06)
    parts += [front_upper, front_fore, sphere("skin", 0.09, front_hand)]
    # The hand lantern: an iron cage swinging from the hand, with the flame in it.
    swing = math.radians(front_arm * 0.35 + staff_angle)
    hang = (front_hand[0] + math.sin(swing) * 0.22 + 0.03, front_hand[1] - math.cos(swing) * 0.22)
    parts.append(sphere("flame", 0.15, hang, z=0.02))
    parts.append(box("iron", (0.32, 0.07, 0.3), (hang[0], hang[1] + 0.15)))
    parts.append(box("iron", (0.32, 0.07, 0.3), (hang[0], hang[1] - 0.15)))
    parts.append(box("iron", (0.07, 0.12, 0.07), (hang[0], hang[1] + 0.22)))
    return {"name": name, "model": {"kind": "primitives", "materials": MATERIALS, "parts": parts}}


def frames():
    out = [
        pose("idle-0"),
        pose("idle-1", bob=0.025, front_arm=17, back_arm=-17, scarf=-14),
    ]
    run = [
        dict(bob=0.0, front_leg=-30, back_leg=36, front_knee=-4, back_knee=38, front_arm=-30, back_arm=34, front_elbow=40, lean=10, scarf=-30),
        dict(bob=0.04, front_leg=8, back_leg=18, front_knee=14, back_knee=60, front_arm=-8, back_arm=10, front_elbow=40, lean=10, scarf=-34),
        dict(bob=0.0, front_leg=36, back_leg=-30, front_knee=38, back_knee=-4, front_arm=34, back_arm=-30, front_elbow=40, lean=10, scarf=-30),
        dict(bob=0.04, front_leg=18, back_leg=8, front_knee=60, back_knee=14, front_arm=10, back_arm=-8, front_elbow=40, lean=10, scarf=-34),
    ]
    out += [pose(f"run-{i}", **p) for i, p in enumerate(run)]
    out.append(pose("jump", bob=0.0, front_leg=40, back_leg=-12, front_knee=62, back_knee=20, front_arm=74, back_arm=-40, front_elbow=-10, lean=-4, scarf=-52))
    out.append(pose("fall", bob=0.02, front_leg=18, back_leg=-20, front_knee=22, back_knee=14, front_arm=96, back_arm=-70, front_elbow=-20, lean=-6, scarf=-8))
    out.append(pose("land", bob=-0.1, front_leg=26, back_leg=-24, front_knee=50, back_knee=50, front_arm=30, back_arm=-30, lean=14, scarf=-26))
    out += [
        pose("light-0", front_arm=-10, back_arm=-26, staff_angle=0, lean=6),
        pose("light-1", front_arm=64, back_arm=-30, front_elbow=-20, staff_angle=0, lean=12),
        pose("light-2", front_arm=58, back_arm=-30, front_elbow=-20, staff_angle=0, lean=14, bob=-0.02),
    ]
    return out


def recipe():
    return {
        "format_version": 1,
        "id": "wick",
        "texture": "textures/wick.png",
        "view": "side",
        "pixels_per_unit": PPU,
        "directions": 1,
        "footprint": {"width": 1, "height": 1},
        "render": {
            "supersample": 6,
            "padding": 1,
            "palette_snap": True,
            "alpha_cutoff": 140,
            "outline": {"enabled": True, "color": "#1d1b3a"},
            "shading": {"light": [-0.55, 0.6, 0.58], "thresholds": [0.3, 0.52, 0.78]},
        },
        "variants": frames(),
    }


if __name__ == "__main__":
    with open(sys.argv[1], "w") as handle:
        json.dump(recipe(), handle, indent=2)
        handle.write("\n")
