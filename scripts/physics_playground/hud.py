"""The screen overlay: entities Weave lays out and styles, and Decay fills."""

from __future__ import annotations

from common import layout, style, text, ui_shape


def container(scene, entity_id, parent, classes, direction="row", justify="center",
              align="center", sx=1.0, sy=0.1, panel=False):
    components = {
        "weave.style": style(*classes),
        "sindri.ui.layout": layout(direction, justify, align),
    }
    if panel:
        components["sindri.ui.shape"] = ui_shape("#0b1522", 20)
    return scene.add(entity_id, parent=parent, sx=sx, sy=sy, components=components)


def label(scene, entity_id, parent, words, classes, px=13, bounds=(0.6, 0.06)):
    return scene.add(entity_id, parent=parent, sx=bounds[0], sy=bounds[1], components={
        "weave.style": style(*classes),
        "sindri.ui.text": text(words, px, bounds),
    })


def button(scene, entity_id, parent, words, classes, width=0.3):
    scene.add(entity_id, parent=parent, sx=width, sy=0.09, components={
        "weave.style": style("button", *classes),
        "sindri.ui.shape": ui_shape("#122336", 31),
        "sindri.ui.button": {"label": words, "autofocus": False, "disabled": False},
        "sindri.ui.layout": layout("row", "center", "center"),
    })
    label(scene, f"{entity_id}-label", entity_id, words, ["button-label"],
          bounds=(width, 0.05))


def build(scene) -> None:
    container(scene, "hud", None, ["hud"], "column", "space_between", "center", 3.4, 2.0)

    container(scene, "hud-top", "hud", ["top"], "row", "space_between", "start",
                    3.3, 0.2)
    container(scene, "brand", "hud-top", ["brand"], "column", "start", "start",
                      1.0, 0.2)
    label(scene, "title", "brand", "PHYSICS PLAYGROUND", ["title"], 24, (1.0, 0.08))
    label(scene, "stats", "brand", "0 bodies", ["stats"], 11, (1.2, 0.05))

    container(scene, "globals", "hud-top", ["globals"], "row", "end", "center", 2.2, 0.1)
    button(scene, "btn-drop", "globals", "DROP EVERYTHING", ["danger"], 0.5)
    button(scene, "btn-balls", "globals", "100 BALLS", ["loud"], 0.3)
    button(scene, "btn-gravity", "globals", "GRAVITY: EARTH", [], 0.4)
    button(scene, "btn-debug", "globals", "DEBUG OFF", [], 0.3)
    button(scene, "btn-reset", "globals", "RESET", [], 0.22)

    container(scene, "hud-bottom", "hud", ["bottom"], "column", "end", "center", 3.3, 0.4)
    container(scene, "toy-row", "hud-bottom", ["toy-row"], "row", "center", "center",
              3.3, 0.1)
    button(scene, "toy-prev", "toy-row", "<", ["arrow"], 0.1)
    label(scene, "toy-label", "toy-row", "THE WHOLE ROOM", ["toy-label"], 16, (0.9, 0.06))
    button(scene, "toy-next", "toy-row", ">", ["arrow"], 0.1)

    container(scene, "dock", "hud-bottom", ["dock"], "row", "space_between", "center",
              3.3, 0.11)
    container(scene, "tools", "dock", ["tools"], "row", "start", "center", 1.5, 0.1)
    for index, (entity_id, words) in enumerate([
        ("tool-grab", "GRAB"), ("tool-blast", "BLAST"), ("tool-spawn", "SPAWN BALL"),
        ("tool-probe", "PROBE"),
    ]):
        button(scene, entity_id, "tools", words, ["tool"], 0.3)
        scene.add(f"tool-mark-{index}", parent=entity_id, sx=0.25, sy=0.01, components={
            "weave.style": style("tool-mark"),
            "sindri.ui.shape": ui_shape("#ffb347", 33),
        })
    container(scene, "actions", "dock", ["actions"], "row", "end", "center", 1.6, 0.1)
    for action in range(1, 5):
        button(scene, f"act-{action}", "actions", f"ACTION {action}", ["action"], 0.36)

    label(scene, "hint", "hud-bottom", "", ["hint"], 11, (2.6, 0.05))
