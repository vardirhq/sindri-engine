#!/usr/bin/env python3
"""DRIFT's on-screen controls, for a phone held sideways. `hold.py` adds them.

Always shown. The left thumb steers with the engine's stick from wherever it
lands on the left half; the right half is a look pad, an invisible button a
thumb drags across; and buttons on both edges stand in for the keys that are
held or tapped. `scripts/touch.decay` reads them into the `Controls` state.
"""

# What each button does, where it sits from its corner, and its words.
BUTTONS = [
    # The right thumb's cluster, in from the bottom right corner.
    ("up", "bottom_right", (-0.22, 0.22), "UP"),
    ("down", "bottom_right", (-0.58, 0.22), "DOWN"),
    ("grab", "bottom_right", (-0.22, 0.58), "GRAB"),
    ("brake", "bottom_right", (-0.58, 0.58), "BRAKE"),
    ("use", "bottom_right", (-0.22, 0.94), "USE"),
    ("reel", "bottom_right", (-0.58, 0.94), "REEL"),
    # Rarer things, along the top left, out of the steering thumb's way.
    ("roll-left", "top_left", (0.22, -0.22), "ROLL <"),
    ("roll-right", "top_left", (0.58, -0.22), "ROLL >"),
    ("clip", "top_left", (0.94, -0.22), "CLIP"),
]

SIZE = 0.3
FILL = [0.06, 0.09, 0.12, 0.55]
RING = [0.75, 0.85, 1.0, 0.35]
FONT = "fonts/ChakraPetch-Regular.ttf"


def _ui(id_, position, scale, parent=None, **components):
    out = {
        "id": id_,
        "name": id_,
        "transform_3d": {
            "position": [round(v, 4) for v in position] + [0],
            "rotation": [0, 0, 0, 1],
            "scale": [scale[0], scale[1], 1],
        },
        "components": components,
    }
    if parent:
        out["parent"] = parent
    return out


def _label(parent, words):
    return _ui(f"{parent}-label", (0, 0), (1, 1), parent=parent, **{
        "sindri.ui.text": {
            "anchor": "center", "color": [0.85, 0.92, 1.0, 0.9], "font": FONT,
            "font_size": 0.042, "line_height": 0.05, "layer": 152, "text": words,
            "letter_spacing": 0.004,
        },
    })


def controls():
    """The touch controls' entities: the look pad, the stick and the buttons."""
    entities = [
        # Placed and sized by the script, to fill the right half at any aspect.
        _ui("touch-look", (1.0, 0), (2.0, 2.0), **{
            "sindri.ui.button": {"label": "", "autofocus": False, "disabled": False},
        }),
        _ui("touch-stick-ring", (-1.4, -0.4), (0.55, 0.55), **{
            "sindri.ui.shape": {"kind": "ellipse", "fill": [0, 0, 0, 0], "stroke": RING,
                                "stroke_width": 0.03, "anchor": "center", "layer": 150},
        }),
        _ui("touch-stick-knob", (-1.4, -0.4), (0.2, 0.2), **{
            "sindri.ui.shape": {"kind": "ellipse", "fill": [0.75, 0.85, 1.0, 0.4],
                                "anchor": "center", "layer": 151},
        }),
    ]
    for name, anchor, position, words in BUTTONS:
        id_ = f"touch-{name}"
        entities.append(_ui(id_, position, (SIZE, SIZE), **{
            "sindri.ui.shape": {"kind": "rect", "fill": FILL, "stroke": RING,
                                "stroke_width": 0.02, "anchor": anchor, "layer": 150},
            "sindri.ui.button": {"label": words, "autofocus": False, "disabled": False},
        }))
        entities.append(_label(id_, words))
    entities.append(_ui("touch-controls", (0, 0), (1, 1), **{
        "sindri.script": {"source": "scripts/touch.decay", "script": "TouchControls"},
    }))
    return entities
