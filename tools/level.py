"""Generates Causeway's scene, prefabs, stylesheet and manifest.

    python3 tools/level.py

Written as code rather than edited by hand because most of a scene is the same
decision made many times -- every figure's prefab, every water tile, every
screen's buttons -- and those drift apart when they are typed out one by one.
The editor still opens and saves what this writes; this is the way to change
many things at once.

What the game does is in `game/assets/scripts/`. What it looks like comes from
`tools/causeway_art.py`. This is only where things are and what they are made
of.
"""
import json
import math
import pathlib

GAME = pathlib.Path(__file__).resolve().parents[1] / "game"
ASSETS = GAME / "assets"

ORIGIN = 24.0
CENTRE = (ORIGIN + 4.0, 1.4, ORIGIN + 4.0)
TITLE_FONT = "fonts/AlegreyaSC-Bold.ttf"
BODY_FONT = "fonts/AlegreyaSans-Medium.ttf"
BOLD_FONT = "fonts/AlegreyaSans-Bold.ttf"
FLAT = [-0.7071068, 0.0, 0.0, 0.7071068]


def transform(position=(0.0, 0.0, 0.0), rotation=(0.0, 0.0, 0.0, 1.0), scale=(1.0, 1.0, 1.0)):
    return {"position": [round(v, 5) for v in position],
            "rotation": [round(v, 6) for v in rotation],
            "scale": [round(v, 5) for v in scale]}


def look(yaw, pitch):
    """A rotation that faces `yaw` round and `pitch` up, as a camera or light does."""
    cy, sy = math.cos(yaw / 2), math.sin(yaw / 2)
    cp, sp = math.cos(pitch / 2), math.sin(pitch / 2)
    return (cy * sp, sy * cp, -sy * sp, cy * cp)


def towards(dx, dy, dz):
    length = math.sqrt(dx * dx + dy * dy + dz * dz)
    return look(math.atan2(-dx, -dz), math.asin(dy / length))


def entity(id, name, components, parent=None, position=None, rotation=None, scale=None,
           disabled=False, spatial=True):
    e = {"id": id, "name": name}
    if parent:
        e["parent"] = parent
    if spatial:
        e["transform_3d"] = transform(position or (0, 0, 0), rotation or (0, 0, 0, 1), scale or (1, 1, 1))
    e["components"] = components
    if disabled:
        e["disabled"] = True
    return e


def script(name, source, **properties):
    payload = {"script": name, "source": source}
    if properties:
        payload["properties"] = properties
    return {"sindri.script": payload}


def styled(*classes):
    return {"weave.style": {"classes": list(classes)}}


def text(value, font=BODY_FONT, size=0.04, layer=120, anchor="center", rich=False, color=(1, 1, 1, 1),
         bounds=(0.0, 0.0), wrap="none", bold=False):
    payload = {"text": value, "font": font, "font_size": size, "line_height": size * 1.22,
               "anchor": anchor, "layer": layer, "color": list(color),
               "shadow": {"offset": [0.0, -0.003], "color": [0.0, 0.02, 0.05, 0.6], "softness": 0.003}}
    if rich:
        payload["rich"] = True
    # Always present, so the stylesheet's width becomes the box the words
    # wrap and align in.
    payload["bounds"] = list(bounds)
    if wrap != "none":
        payload["wrap"] = wrap
    if bold:
        payload["bold"] = True
    return {"sindri.ui.text": payload}


def panel(layer=100, fill=(0.04, 0.08, 0.11, 0.86), stroke=(0.95, 0.8, 0.5, 0.35), layout=None):
    components = {"sindri.ui.shape": {"kind": "rect", "anchor": "center", "layer": layer,
                                      "fill": list(fill), "stroke": list(stroke), "stroke_width": 0.004,
                                      "corner_radius": 0.012}}
    if layout:
        components["sindri.ui.layout"] = layout
    return components


def column(spacing=0.01, align="start", justify="start"):
    return {"direction": "column", "spacing": spacing, "align": align, "justify": justify}


def button(entities, id, name, label, parent, classes, layer=130, label_size=0.034):
    entities.append(entity(id, name, {**styled("button", *classes), "sindri.ui.button": {"label": label},
                                      "sindri.ui.shape": {"kind": "rect", "anchor": "center", "layer": layer,
                                                          "fill": [0.1, 0.16, 0.2, 1.0],
                                                          "stroke": [0.95, 0.8, 0.5, 0.6],
                                                          "stroke_width": 0.004, "corner_radius": 0.01}},
                           parent=parent, spatial=False))
    entities.append(entity(id + "-label", name + " Label",
                           {**styled("button-label"), **text(label, BOLD_FONT, label_size, layer + 2)},
                           parent=id, spatial=False))


# --- Prefabs -----------------------------------------------------------------

FIGURES = {
    # kind: (model, alive, size)
    "bulwark": ("bulwark", True, 1.0),
    "mason": ("mason", True, 1.0),
    "lamplighter": ("lamplighter", True, 1.0),
    "husk": ("husk", True, 1.0),
    "brute": ("brute", True, 1.0),
    "spitter": ("spitter", True, 1.0),
    "mortar": ("mortar", True, 1.0),
    "colossus": ("colossus", True, 1.0),
    "house": ("house", False, 1.0),
    "beacon": ("beacon", False, 1.0),
    "stone": ("stone", False, 1.0),
}

MARKS = ["move", "aim", "danger", "spawn", "tide", "selected", "cursor", "arrow", "push"]


def prefab(name, entities):
    return {"format_version": 1, "metadata": {"name": name}, "entities": entities}


def write_prefabs():
    folder = ASSETS / "prefabs"
    folder.mkdir(parents=True, exist_ok=True)
    for kind, (model, alive, size) in FIGURES.items():
        title = kind.capitalize()
        root = entity(kind, title, {
            "sindri.model": {"asset": f"models/{model}.glb", "layer": 0},
            **script("Piece", "scripts/piece.decay", alive=alive, size=size),
        })
        write(folder / f"{kind}.prefab", prefab(title, [root]))
    mark = entity("mark", "Mark", {
        "sindri.sprite": {"texture": "textures/marks.png#move", "tint": [1.0, 1.0, 1.0, 1.0], "layer": 5,
                          "unlit": True},
        "sindri.animation.sprite": {"clips": {
            name: {"frames": [name], "looping": True, "seconds_per_frame": 1.0} for name in MARKS
        }, "speed": 1.0},
    }, rotation=FLAT, scale=(0.94, 0.94, 1.0))
    write(folder / "mark.prefab", prefab("Mark", [mark]))
    label = entity("label", "Label", {"sindri.text": {
        "text": "", "font": BOLD_FONT, "font_size": 0.24, "line_height": 0.28, "anchor": "bottom",
        "layer": 40, "rich": True, "color": [1.0, 1.0, 1.0, 1.0],
        "outline": {"width": 0.025, "color": [0.03, 0.05, 0.08, 0.95]},
    }})
    write(folder / "label.prefab", prefab("Label", [label]))
    popup = entity("popup", "Popup", {"sindri.text": {
        "text": "", "font": BOLD_FONT, "font_size": 0.42, "line_height": 0.5, "anchor": "bottom",
        "layer": 50, "rich": True, "color": [1.0, 1.0, 1.0, 1.0],
        "outline": {"width": 0.04, "color": [0.05, 0.03, 0.02, 0.95]},
    }, **script("Popup", "scripts/effects.decay")})
    write(folder / "popup.prefab", prefab("Popup", [popup]))
    missile = entity("missile", "Missile", {"sindri.model": {"asset": "models/missile.glb", "layer": 0},
                                            **script("Missile", "scripts/effects.decay")})
    write(folder / "missile.prefab", prefab("Missile", [missile]))
    spark = entity("spark", "Spark", {
        "sindri.sprite": {"texture": "textures/spark.png#spark-0", "billboard": True, "layer": 30,
                          "unlit": True, "tint": [1.0, 0.95, 0.8, 1.0]},
        "sindri.animation.sprite": {"clips": {"burst": {
            "frames": [f"spark-{i}" for i in range(4)], "looping": False, "seconds_per_frame": 0.06}},
            "speed": 1.0},
        **script("Spark", "scripts/effects.decay"),
    }, scale=(0.9, 0.9, 1.0))
    write(folder / "spark.prefab", prefab("Spark", [spark]))


# --- The scene ---------------------------------------------------------------

def world_entities():
    e = []
    e.append(entity("environment", "Environment", {"sindri.environment": {
        "background": [0.06, 0.15, 0.19, 1.0],
        "ambient_color": [0.56, 0.68, 0.88],
        "ambient_intensity": 0.42,
        "ambient_occlusion": {"enabled": True, "strength": 0.6},
        "shadows": {"enabled": True, "distance": 14.0, "map_size": 2048, "bias": 0.0015},
        "post_process": {"exposure": 0.0, "contrast": 1.04, "saturation": 0.95, "tone_mapping": "aces",
                         "vignette": 0.3},
        "bloom": {"enabled": True, "threshold": 0.85, "knee": 0.25, "intensity": 0.55, "passes": 4},
    }}, spatial=False))
    e.append(entity("sun", "Sun", {"sindri.light": {"kind": "directional", "color": [1.0, 0.85, 0.66],
                                                   "intensity": 2.4}},
                    position=(0, 10, 0), rotation=towards(0.25, -1.0, -0.6)))
    names = ["Saltmarsh", "The Fold", "Quarry Rise", "The Lighthouse"]
    files = ["saltmarsh", "fold", "quarry-rise", "lighthouse"]
    for i, (name, file) in enumerate(zip(names, files)):
        e.append(entity(f"island-{i}", f"Island {i}", {"sindri.model": {"asset": f"models/island-{file}.glb",
                                                                        "layer": 0}},
                        disabled=i != 0))
    e.append(entity("sea", "Sea", {}, position=(CENTRE[0], 0.08, CENTRE[2])))
    for gx in range(-2, 3):
        for gz in range(-2, 3):
            e.append(entity(f"sea-{gx + 2}-{gz + 2}", "Sea Tile", {"sindri.sprite": {
                "texture": "textures/water.png", "tint": [0.07, 0.32, 0.4, 0.82], "layer": -10, "unlit": True,
            }}, parent="sea", position=(gx * 24.0, 0.0, gz * 24.0), rotation=FLAT, scale=(24.0, 24.0, 1.0)))
    e.append(entity("lantern-light", "Lantern Light", {"sindri.light": {
        "kind": "point", "color": [1.0, 0.76, 0.42], "intensity": 2.6, "range": 3.2}},
        position=(CENTRE[0], 3, CENTRE[2])))
    e.append(entity("beacon-light", "Beacon Light", {"sindri.light": {
        "kind": "point", "color": [1.0, 0.6, 0.28], "intensity": 4.0, "range": 4.5}},
        position=(CENTRE[0], 4, CENTRE[2])))
    distance = 28.46
    offset = (distance, distance * math.sqrt(2) * math.tan(math.radians(30)), distance)
    e.append(entity("world-camera", "World Camera", {
        "sindri.camera": {"projection": "orthographic", "vertical_size": 10.0, "near": 0.1, "far": 92.0},
        "sindri.camera.behavior": {"shake": {"strength": 0.09, "frequency": 38.0, "decay": 3.2, "trauma": 0.0,
                                             "phase": 0.0}},
        **script("CameraRig", "scripts/camera.decay"),
    }, position=(CENTRE[0] + offset[0], CENTRE[1] + offset[1], CENTRE[2] + offset[2]),
        rotation=look(math.radians(45), math.radians(-30))))
    cursor = entity("cursor", "Cursor", {
        "sindri.sprite": {"texture": "textures/marks.png#cursor", "tint": [1.0, 1.0, 1.0, 1.0], "layer": 8,
                          "unlit": True},
    }, rotation=FLAT, scale=(0.98, 0.98, 1.0))
    e.append(cursor)
    prefabs = {kind: f"prefabs/{kind}.prefab" for kind in FIGURES}
    e.append(entity("battle", "Battle", script(
        "Battle", "scripts/battle.decay",
        mark="prefabs/mark.prefab", label="prefabs/label.prefab", popup="prefabs/popup.prefab",
        missile="prefabs/missile.prefab", spark="prefabs/spark.prefab", **prefabs), spatial=False))
    e.append(entity("music", "Music", {"sindri.audio.source": {
        "autoplay": True, "bus": "music", "clip": "audio/tide-song.ogg", "looping": True, "volume": 0.45}},
        spatial=False))
    return e


def hud_entities():
    e = []
    # Top left: where and when.
    e.append(entity("hud", "Hud", {**styled("hud")}, spatial=False))
    e.append(entity("status", "Status", {**styled("status", "panel"), **panel(layout=column(0.004))},
                    parent="hud", spatial=False))
    e.append(entity("island-name", "IslandName", {**styled("island-name"), **text("Saltmarsh", TITLE_FONT, 0.05,
                                                                                   122)},
                    parent="status", spatial=False))
    e.append(entity("round-text", "RoundText", {**styled("status-line"), **text("Round 1 of 5", BOLD_FONT, 0.03,
                                                                                122)},
                    parent="status", spatial=False))
    e.append(entity("tide-text", "TideText", {**styled("status-line", "dim"), **text("Tide 0", BODY_FONT, 0.026,
                                                                                    122)},
                    parent="status", spatial=False))
    # Top right: the Hearth.
    e.append(entity("hearth", "Hearth", {**styled("hearth", "panel"), **panel(layout=column(0.002, "end"))},
                    parent="hud", spatial=False))
    e.append(entity("hearth-title", "HearthTitle", {**styled("hearth-title"), **text("HEARTH", TITLE_FONT, 0.03,
                                                                                    122)},
                    parent="hearth", spatial=False))
    e.append(entity("hearth-pips", "HearthPips", {**styled("hearth-pips"), **text("", BOLD_FONT, 0.04, 122,
                                                                                  rich=True)},
                    parent="hearth", spatial=False))
    # Bottom left: the warden in hand.
    e.append(entity("card", "Card", {**styled("card", "panel"), **panel(layout=column(0.006))},
                    parent="hud", spatial=False))
    for id, name, font, size, classes in (
            ("unit-name", "UnitName", TITLE_FONT, 0.044, ("unit-name",)),
            ("unit-stats", "UnitStats", BOLD_FONT, 0.03, ("unit-stats",)),
            ("skill-name", "SkillName", BOLD_FONT, 0.03, ("skill-name",)),
            ("skill-text", "SkillText", BODY_FONT, 0.026, ("skill-text",)),
            ("unit-hint", "UnitHint", BODY_FONT, 0.024, ("unit-hint",)),
            ("preview", "Preview", BOLD_FONT, 0.026, ("preview",))):
        e.append(entity(id, name, {**styled(*classes), **text("", font, size, 122, rich=True, wrap="word")},
                        parent="card", spatial=False))
    # Bottom right: the turn's controls.
    # A shape that draws nothing, because an anchor belongs to what is drawn:
    # without one the row would be placed by its middle from the screen's.
    e.append(entity("actions", "Actions", {**styled("actions"), "sindri.ui.layout": {
        "direction": "row", "spacing": 0.015, "align": "center", "justify": "end"},
        "sindri.ui.shape": {"kind": "rect", "anchor": "center", "layer": 100, "fill": [0, 0, 0, 0],
                            "stroke": [0, 0, 0, 0], "stroke_width": 0.0}}, parent="hud", spatial=False))
    button(e, "undo-move", "UndoMove", "Undo move", "actions", ("small",), label_size=0.028)
    button(e, "rewind", "Rewind", "Rewind (1)", "actions", ("small",), label_size=0.028)
    e[-1] = entity("rewind-label", "RewindLabel", {**styled("button-label"), **text("Rewind (1)", BOLD_FONT,
                                                                                    0.028, 132)},
                   parent="rewind", spatial=False)
    button(e, "end-turn", "EndTurn", "End turn", "actions", ("primary",), label_size=0.034)
    # The banner across the top for what just happened.
    e.append(entity("banner", "Banner", {**styled("banner"), **panel(140, layout=column(0.004, "center", "center"))},
                    disabled=True, spatial=False))
    e.append(entity("banner-title", "BannerTitle", {**styled("banner-title"), **text("", TITLE_FONT, 0.06, 142)},
                    parent="banner", spatial=False))
    e.append(entity("banner-text", "BannerText", {**styled("banner-text"), **text("", BODY_FONT, 0.028, 142,
                                                                                  wrap="word")},
                    parent="banner", spatial=False))
    return e


def screen(e, id, name, title, body, button_label, title_size=0.09, disabled=True):
    e.append(entity(id, name, {**styled("screen"), **panel(200, fill=(0.02, 0.05, 0.07, 0.55), stroke=(0, 0, 0, 0),
                                                           layout=column(0.02, "center", "center"))},
                    disabled=disabled, spatial=False))
    e.append(entity(id + "-card", name + " Card", {**styled("screen-card", "panel"),
                                                   **panel(205, layout=column(0.02, "center", "center"))},
                    parent=id, spatial=False))
    e.append(entity(id + "-title", name + "Title", {**styled("screen-title"), **text(title, TITLE_FONT, title_size,
                                                                                    210)},
                    parent=id + "-card", spatial=False))
    e.append(entity(id + "-text", name + "Text", {**styled("screen-text"), **text(body, BODY_FONT, 0.032, 210,
                                                                                  wrap="word")},
                    parent=id + "-card", spatial=False))
    if button_label:
        button(e, id + "-button", name + "Button", button_label, id + "-card", ("primary", "wide"), layer=212)


def screens():
    e = []
    screen(e, "title", "Title", "Causeway",
           "The Hollow cross at low tide. Hold each island until the sea comes in and closes the causeway "
           "behind them.", "Begin a crossing", title_size=0.16, disabled=False)
    e.append(entity("title-record", "TitleRecord", {**styled("screen-text", "dim"), **text("", BODY_FONT, 0.026, 210)},
                    parent="title-card", spatial=False))
    e.append(entity("title-controls", "TitleControls", {**styled("screen-text", "dim"), **text(
        "Click a warden, then where to move, then what to strike.  Arrows and Enter work too.  "
        "E ends the turn, Z undoes a move, R rewinds the turn once per island.", BODY_FONT, 0.024, 210,
        wrap="word")}, parent="title-card", spatial=False))
    screen(e, "held", "Held", "Saltmarsh holds", "", "Onward")
    screen(e, "fallen", "Fallen", "The light goes out", "", "Back to the shore")
    screen(e, "crossed", "Crossed", "The lamp is lit", "", "Back to the shore")
    # Boons between islands, one card each.
    e.append(entity("upgrade", "Upgrade", {**styled("screen"), **panel(200, fill=(0.02, 0.05, 0.07, 0.55),
                                                                       stroke=(0, 0, 0, 0),
                                                                       layout=column(0.03, "center", "center"))},
                    disabled=True, spatial=False))
    e.append(entity("upgrade-title", "UpgradeTitle", {**styled("screen-title"), **text("Choose a boon", TITLE_FONT,
                                                                                      0.08, 210)},
                    parent="upgrade", spatial=False))
    e.append(entity("boons", "Boons", {**styled("boons"), "sindri.ui.layout": {
        "direction": "row", "spacing": 0.02, "align": "center", "justify": "center"}}, parent="upgrade",
        spatial=False))
    boons = [("Thicker plate", "The Bulwark gains a point of health."),
             ("Heavier stone", "The Mason's stone hits 1 harder."),
             ("Longer legs", "The Lamplighter moves a cell further."),
             ("Bank the fire", "The Hearth regains two points.")]
    for i, (title, body) in enumerate(boons):
        e.append(entity(f"boon-{i}", f"Boon{i}", {**styled("boon", "button"), "sindri.ui.button": {"label": title},
                                                  **panel(205, layout=column(0.012, "center", "center"))},
                        parent="boons", spatial=False))
        e.append(entity(f"boon-{i}-title", f"Boon{i}Title", {**styled("boon-title"), **text(title, TITLE_FONT, 0.04,
                                                                                            210)},
                        parent=f"boon-{i}", spatial=False))
        e.append(entity(f"boon-{i}-text", f"Boon{i}Text", {**styled("boon-text"), **text(body, BODY_FONT, 0.026, 210,
                                                                                          wrap="word")},
                        parent=f"boon-{i}", spatial=False))
        e.append(entity(f"boon-{i}-key", f"Boon{i}Key", {**styled("boon-key", "dim"), **text(str(i + 1), BOLD_FONT,
                                                                                             0.024, 210)},
                        parent=f"boon-{i}", spatial=False))
    return e


def write(path, document):
    path.write_text(json.dumps(document, indent=2) + "\n")


def ordered(entities):
    """Gives every screen element its place among its siblings.

    A layout lays its children out in scene order, and the editor saves a
    scene sorted by id, so the order they were written in here would not
    survive the first save. Stated, it does.
    """
    seen = {}
    for e in entities:
        parent = e.get("parent")
        if parent is None or not any(key.startswith("sindri.ui.") for key in e["components"]):
            continue
        index = seen.get(parent, 0)
        seen[parent] = index + 1
        e["components"].setdefault("sindri.ui.box", {})["order"] = index
    return entities


def main():
    write_prefabs()
    entities = ordered(world_entities() + hud_entities() + screens())
    entities.sort(key=lambda item: item["id"])
    write(ASSETS / "causeway.scene", {"format_version": 10, "metadata": {"name": "Causeway"}, "entities": entities})
    print(f"wrote causeway.scene with {len(entities)} entities")


if __name__ == "__main__":
    main()
