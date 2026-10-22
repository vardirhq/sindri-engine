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
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from causeway_art import island_slug, islands  # noqa: E402

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
    "tidecaller": ("tidecaller", True, 1.0),
    "imp": ("imp", True, 1.0),
    "burster": ("burster", True, 1.0),
    "kiln": ("kiln", True, 1.0),
    "house": ("house", False, 1.0),
    "beacon": ("beacon", False, 1.0),
    "stone": ("stone", False, 1.0),
    "vent": ("vent", False, 1.0),
    # Ground cooled out of lava and water, raised and stretched by the battle.
    "basalt": ("basalt", False, 1.0),
}
WARDEN_KINDS = ("bulwark", "mason", "lamplighter", "tidecaller")

MARKS = ["move", "aim", "danger", "spawn", "tide", "selected", "cursor", "arrow", "push"]


def prefab(name, entities):
    return {"format_version": 1, "metadata": {"name": name}, "entities": entities}


def write_prefabs():
    folder = ASSETS / "prefabs"
    folder.mkdir(parents=True, exist_ok=True)
    for kind, (model, alive, size) in FIGURES.items():
        title = kind.capitalize()
        root = entity(kind, title, {
            "sindri.model": {"asset": f"models/{model}.glb", "layer": 0,
                             **({"occluded_color": [0.85, 0.67, 0.32, 0.32] if kind in
                                WARDEN_KINDS else [0.65, 0.24, 0.18, 0.3]}
                                if alive else {})},
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
    # The Tidecaller's water in flight.
    droplet = entity("droplet", "Droplet", {"sindri.model": {"asset": "models/droplet.glb", "layer": 0},
                                            **script("Missile", "scripts/effects.decay")})
    write(folder / "droplet.prefab", prefab("Droplet", [droplet]))
    pointer = entity("pointer", "Pointer", {"sindri.model": {"asset": "models/pointer.glb", "layer": 0}})
    write(folder / "pointer.prefab", prefab("Pointer", [pointer]))
    # A pool on a cell: water drawn as the sea is, so the mood colors it with
    # the sea, and lava drawn as the lava lake is. Both are a cell across, a
    # little above the cell's top.
    water = entity("water-pool", "Water Pool", {
        "sindri.sprite": {"texture": "textures/white.png", "tint": [0.02, 0.15, 0.22, 0.88], "layer": 4,
                          "unlit": True, "shader": "shaders/sea.wgsl", "shader_params": [0.8, 0.0, 0.6, 0.0]},
        "sindri.tags": {"tags": ["sea"]},
    }, rotation=FLAT, scale=(0.94, 0.94, 1.0))
    write(folder / "water-pool.prefab", prefab("Water Pool", [water]))
    lava = entity("lava-pool", "Lava Pool", {
        "sindri.sprite": {"texture": "textures/white.png", "tint": [0.09, 0.045, 0.04, 1.0], "layer": 4,
                          "unlit": True, "shader": "shaders/lava.wgsl", "shader_params": [1.3, 0.6, 0.35, 1.0]},
        "sindri.tags": {"tags": ["lava"]},
    }, rotation=FLAT, scale=(0.96, 0.96, 1.0))
    write(folder / "lava-pool.prefab", prefab("Lava Pool", [lava]))
    # A Hollow on the causeway behind the title: the husk's model and nothing
    # else, walked by `title.decay`.
    # A speck in the air: pollen, dust, rain, a firefly or an ember, as
    # `Atmosphere` tints and scales it.
    fleck = entity("fleck", "Fleck", {"sindri.sprite": {
        "texture": "textures/mote.png", "billboard": True, "layer": 32, "unlit": True,
        "tint": [1.0, 1.0, 1.0, 0.0]}}, scale=(0.05, 0.05, 0.05))
    write(folder / "fleck.prefab", prefab("Fleck", [fleck]))
    walker = entity("walker", "Walker", {"sindri.model": {"asset": "models/husk.glb", "layer": 0}})
    write(folder / "walker.prefab", prefab("Walker", [walker]))
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
    # `Atmosphere` sets the light, fog, grade and sea for each island and the
    # title; what is authored here is only where it starts.
    e.append(entity("environment", "Environment", {
        **script("Atmosphere", "scripts/atmosphere.decay", fleck="prefabs/fleck.prefab"),
        "sindri.environment": {
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
    # Every island the rules know, read from the same text they read.
    for i, island in enumerate(islands()):
        e.append(entity(f"island-{i}", f"Island {i}", {"sindri.model": {
            "asset": f"models/island-{island_slug(island['name'])}.glb", "layer": 0}},
            disabled=i != 0))
    e.append(entity("sea", "Sea", {}, position=(CENTRE[0], 0.08, CENTRE[2])))
    # One quad, drawn by `shaders/sea.wgsl`, which works from world position
    # so needs no tiling: the tint is the deep water's color, the params
    # swell, foam, glints and rain on the water.
    e.append(entity("sea-surface", "Sea Surface", {
        "sindri.sprite": {
            "texture": "textures/white.png", "tint": [0.02, 0.15, 0.22, 0.88], "layer": -10,
            "unlit": True, "shader": "shaders/sea.wgsl", "shader_params": [0.8, 0.8, 0.6, 0.0],
        },
        "sindri.tags": {"tags": ["sea"]},
    }, parent="sea", rotation=FLAT, scale=(120.0, 120.0, 1.0)))
    # The lake round a burning island, at the same level and in the same place:
    # `Atmosphere` fades one out and the other in with the world.
    e.append(entity("lava-surface", "Lava Surface", {
        "sindri.sprite": {
            "texture": "textures/white.png", "tint": [0.09, 0.045, 0.04, 1.0], "layer": -10,
            "unlit": True, "shader": "shaders/lava.wgsl", "shader_params": [1.2, 0.5, 0.3, 0.0],
        },
        "sindri.tags": {"tags": ["lava"]},
    }, parent="sea", rotation=FLAT, scale=(120.0, 120.0, 1.0), disabled=True))
    e.append(entity("lantern-light", "Lantern Light", {"sindri.light": {
        "kind": "point", "color": [1.0, 0.76, 0.42], "intensity": 2.6, "range": 3.2}},
        position=(CENTRE[0], 3, CENTRE[2])))
    # Windows lit in each house, and a violet glow where the Hollow climb out;
    # the battle places them and `Atmosphere` sets how bright they are.
    for i in range(3):
        e.append(entity(f"house-light-{i}", f"House Light {i}", {"sindri.light": {
            "kind": "point", "color": [1.0, 0.68, 0.36], "intensity": 0.0, "range": 2.4}},
            position=(CENTRE[0], 2, CENTRE[2]), disabled=True))
    for i in range(2):
        e.append(entity(f"rift-light-{i}", f"Rift Light {i}", {"sindri.light": {
            "kind": "point", "color": [0.62, 0.32, 1.0], "intensity": 0.0, "range": 2.2}},
            position=(CENTRE[0], 2, CENTRE[2]), disabled=True))
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
        missile="prefabs/missile.prefab", spark="prefabs/spark.prefab", droplet="prefabs/droplet.prefab",
        pointer="prefabs/pointer.prefab",
        water_pool="prefabs/water-pool.prefab", lava_pool="prefabs/lava-pool.prefab", **prefabs),
        spatial=False))
    # A song for each world, both always playing, each through its own bus:
    # `Atmosphere` crossfades the buses with the world's mood.
    e.append(entity("music", "Music", {"sindri.audio.source": {
        "autoplay": True, "bus": "music-shallows", "clip": "audio/tide-song.ogg", "looping": True,
        "volume": 0.45}}, spatial=False))
    e.append(entity("music-cinderfast", "Music Cinderfast", {"sindri.audio.source": {
        "autoplay": True, "bus": "music-cinderfast", "clip": "audio/cinder-song.ogg", "looping": True,
        "volume": 0.45}}, spatial=False))
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
    e.append(entity("view-controls", "ViewControls", {
        **styled("view-controls"), "sindri.ui.layout": {"direction": "row", "spacing": 0.01},
        "sindri.ui.shape": {"kind": "rect", "anchor": "center", "layer": 100,
                            "fill": [0, 0, 0, 0], "stroke": [0, 0, 0, 0], "stroke_width": 0.0}},
        parent="hud", spatial=False))
    button(e, "rotate-left", "RotateLeft", "<", "view-controls", ("rotate",))
    # Straight down from above and back; `CameraRig` says which it will do.
    button(e, "view-top", "ViewTop", "Top", "view-controls", ("rotate", "top"))
    button(e, "rotate-right", "RotateRight", ">", "view-controls", ("rotate",))
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
    # The banner across the top for what just happened: words over a soft
    # dark cloud rather than a box, faded in and out by the battle.
    e.append(entity("banner", "Banner", {**styled("banner"), **clear_shape(140),
                                         "sindri.ui.layout": column(0.004, "center", "center")},
                    disabled=True, spatial=False))
    # Out of the column's flow, so it lies under the words at their middle.
    e.append(entity("banner-halo", "BannerHalo", {**styled("banner-halo"), **image("textures/halo.png", 141,
                                                                                (1, 1, 1, 0))},
                    parent="banner", spatial=False))
    e.append(entity("banner-title", "BannerTitle", {**styled("banner-title"), **text("", TITLE_FONT, 0.06, 142,
                                                                                    rich=True)},
                    parent="banner", spatial=False))
    e.append(entity("banner-text", "BannerText", {**styled("banner-text"), **text("", BODY_FONT, 0.028, 142,
                                                                                  wrap="word", rich=True)},
                    parent="banner", spatial=False))
    return e


def image(texture, layer, tint=(1, 1, 1, 1)):
    return {"sindri.ui.image": {"texture": texture, "tint": list(tint), "anchor": "center", "layer": layer}}


def clear_shape(layer):
    """A shape nobody sees, so a box has something to anchor and be hit by."""
    return {"sindri.ui.shape": {"kind": "rect", "anchor": "center", "layer": layer, "fill": [0, 0, 0, 0],
                                "stroke": [0, 0, 0, 0], "stroke_width": 0.0, "corner_radius": 0.0}}


def plate_button(e, id, name, label, parent, plate, classes, layer):
    """A button drawn as a plate image the title script can fade and light,
    over a box the pointer hits."""
    e.append(entity(id, name, {**styled("plate-button", *classes), "sindri.ui.button": {"label": label},
                               **clear_shape(layer)}, parent=parent, spatial=False))
    e.append(entity(id + "-plate", name + " Plate", {**styled("plate"), **image(plate, layer + 1, (1, 1, 1, 0))},
                    parent=id, spatial=False))
    e.append(entity(id + "-label", name + " Label", {**styled("plate-label", *classes), **text(
        "", BOLD_FONT, 0.034, layer + 3, rich=True)}, parent=id, spatial=False))


def title_screen(e):
    """The title: the Lighthouse at dusk behind the logo, a menu below, and
    a page on how to play. `title.decay` brings it in and takes it away; it
    starts transparent so nothing shows before the script does."""
    # No shape on the root, as on the HUD's: its children then anchor to the
    # screen's edges rather than to a box's middle.
    e.append(entity("title", "Title", {**styled("title-root")}, spatial=False))
    e.append(entity("title-shade-top", "TitleShadeTop", {**styled("title-shade-top"),
                                                         **image("textures/shade-down.png", 182, (1, 1, 1, 0))},
                    parent="title", spatial=False))
    e.append(entity("title-shade-side", "TitleShadeSide", {**styled("title-shade-side"),
                                                           **image("textures/shade-side.png", 182, (1, 1, 1, 0))},
                    parent="title", spatial=False))
    e.append(entity("title-shade-bottom", "TitleShadeBottom", {**styled("title-shade-bottom"),
                                                               **image("textures/shade-up.png", 182,
                                                                       (1, 1, 1, 0))},
                    parent="title", spatial=False))
    e.append(entity("title-halo", "TitleHalo", {**styled("title-halo"), **image("textures/halo.png", 184,
                                                                                (1, 1, 1, 0))},
                    parent="title", spatial=False))
    e.append(entity("title-logo", "TitleLogo", {**styled("title-logo"), **image("textures/logo.png", 186,
                                                                                (1, 1, 1, 0))},
                    parent="title", spatial=False))
    e.append(entity("title-tagline", "TitleTagline", {**styled("title-tagline"), **text(
        "", TITLE_FONT, 0.03, 188, rich=True)}, parent="title", spatial=False))
    e.append(entity("title-menu", "TitleMenu", {**styled("title-menu"), **clear_shape(190),
                                                "sindri.ui.layout": column(0.02, "center", "center")},
                    parent="title", spatial=False))
    ordered_menu = []
    plate_button(ordered_menu, "begin-button", "BeginButton", "Begin a crossing", "title-menu",
                 "textures/plate-ember.png", ("begin",), 192)
    plate_button(ordered_menu, "howto-button", "HowToButton", "How to play", "title-menu",
                 "textures/plate-ink.png", ("guide",), 192)
    ordered_menu.append(entity("title-record", "TitleRecord", {**styled("title-record"), **text(
        "", BODY_FONT, 0.026, 196, rich=True)}, parent="title-menu", spatial=False))
    e.extend(ordered_menu)
    e.append(entity("title-prompt", "TitlePrompt", {**styled("title-prompt"), **text(
        "", BODY_FONT, 0.024, 196, rich=True)}, parent="title", spatial=False))

    # How to play: a page over the title, opened from its menu.
    e.append(entity("howto", "HowTo", {**styled("howto-root")}, parent="title", disabled=True, spatial=False))
    e.append(entity("howto-dim", "HowToDim", {**styled("howto-dim"), **image("textures/white.png", 296,
                                                                           (0.01, 0.03, 0.05, 0.62))},
                    parent="howto", spatial=False))
    e.append(entity("howto-card", "HowToCard", {**styled("howto", "panel"),
                                                **panel(300, layout=column(0.02, "center", "start"))},
                    parent="howto", spatial=False))
    e.append(entity("howto-title", "HowToTitle", {**styled("howto-title"), **text("How to play", TITLE_FONT,
                                                                                 0.05, 302)},
                    parent="howto-card", spatial=False))
    pages = [
        ("The Hollow show their hand.",
         "Every Hollow marks the cell it will strike before it strikes. Read the marks, then move."),
        ("Each warden moves, then acts.",
         "Choose a warden, then where it walks, then its skill: the Bulwark shoves, the Mason raises stone, "
         "the Lamplighter burns at range."),
        ("The tide is on your side.",
         "Each round the sea rises. Shove the Hollow into it, and hold until high tide closes the causeway."),
        ("Guard the Hearth.",
         "Houses and the beacon feed it. Lose them all and the light goes out."),
    ]
    for i, (head, body) in enumerate(pages):
        e.append(entity(f"howto-head-{i}", f"HowToHead{i}", {**styled("howto-head"), **text(
            head, BOLD_FONT, 0.03, 302)}, parent="howto-card", spatial=False))
        e.append(entity(f"howto-body-{i}", f"HowToBody{i}", {**styled("howto-body"), **text(
            body, BODY_FONT, 0.026, 302, wrap="word")}, parent="howto-card", spatial=False))
    e.append(entity("howto-keys", "HowToKeys", {**styled("howto-body", "dim"), **text(
        "Click or tap to play.  Keys: arrows and Enter, Q aims, Tab next warden, Z undoes a move, "
        "E ends the turn, R rewinds. J/L or < > rotate, T or Top looks straight down; swipe across empty sea on touch.", BODY_FONT, 0.022, 302, wrap="word")},
                    parent="howto-card", spatial=False))
    plate_button(e, "howto-close", "HowToClose", "Back", "howto-card", "textures/plate-ink.png", ("close",), 304)

    # Over everything and outside the title, so it can cover the change from
    # the title's island to the first battle's after the title is put away.
    e.append(entity("title-fade", "TitleFade", {**styled("title-fade"), **image("textures/white.png", 400,
                                                                                (0.01, 0.02, 0.04, 0.55))},
                    spatial=False))
    e.append(entity("title-director", "TitleDirector", script(
        "TitleShow", "scripts/title.decay", walker="prefabs/walker.prefab"), spatial=False))


def between_screen(e, id, name, button_label):
    """A screen between islands: the board dimmed and shaded, a column of
    words over it and a plate to go on. `screens.decay` fills the words and
    brings it all in; it starts transparent."""
    e.append(entity(id, name, {**styled("screen-root")}, disabled=True, spatial=False))
    e.append(entity(id + "-dim", name + "Dim", {**styled("screen-dim"), **image("textures/white.png", 194,
                                                                              (0.01, 0.03, 0.05, 0))},
                    parent=id, spatial=False))
    for part, texture, cls in (("ShadeTop", "shade-down", "screen-shade-top"),
                               ("ShadeBottom", "shade-up", "screen-shade-bottom")):
        e.append(entity(f"{id}-{cls}", name + part, {**styled(cls), **image(f"textures/{texture}.png", 195,
                                                                            (1, 1, 1, 0))},
                        parent=id, spatial=False))
    e.append(entity(id + "-halo", name + "Halo", {**styled("screen-halo"), **image("textures/halo.png", 195,
                                                                                (1, 1, 1, 0))},
                    parent=id, spatial=False))
    column_id = id + "-column"
    e.append(entity(column_id, name + "Column", {**styled("screen-column"), **clear_shape(196),
                                                 "sindri.ui.layout": column(0.016, "center", "center")},
                    parent=id, spatial=False))
    for part, cls, font, size in (("Kicker", "screen-kicker", TITLE_FONT, 0.026),
                                  ("Title", "screen-title", TITLE_FONT, 0.09),
                                  ("Ornament", "screen-ornament", BODY_FONT, 0.03),
                                  ("Text", "screen-text", BODY_FONT, 0.03),
                                  ("Stats", "screen-stats", BODY_FONT, 0.024)):
        wrap = "word" if part == "Text" else "none"
        e.append(entity(f"{id}-{part.lower()}", name + part, {**styled(cls), **text(
            "", font, size, 198, rich=True, wrap=wrap)}, parent=column_id, spatial=False))
    if id == "upgrade":
        e.append(entity("boons", "Boons", {**styled("boons"), **clear_shape(198), "sindri.ui.layout": {
            "direction": "row", "spacing": 0.02, "align": "center", "justify": "center"}},
            parent=column_id, spatial=False))
        for i in range(4):
            card = f"boon-{i}"
            e.append(entity(card, f"Boon{i}", {**styled("boon-card"), "sindri.ui.button": {"label": f"Boon {i + 1}"},
                                               **clear_shape(200)}, parent="boons", spatial=False))
            art = image("textures/card.png", 201, (1, 1, 1, 0))
            art["sindri.ui.image"]["draw"] = "sliced"
            art["sindri.ui.image"]["border_scale"] = 0.0022
            e.append(entity(card + "-art", f"Boon{i} Card", {**styled("boon-art"), **art}, parent=card,
                            spatial=False))
            inner = card + "-words"
            e.append(entity(inner, f"Boon{i}Words", {**styled("boon-words"), **clear_shape(202),
                                                     "sindri.ui.layout": column(0.01, "center", "center")},
                            parent=card, spatial=False))
            for part, cls, font, size in (("Mark", "boon-mark", BODY_FONT, 0.05),
                                          ("Title", "boon-title", TITLE_FONT, 0.04),
                                          ("Text", "boon-text", BODY_FONT, 0.026),
                                          ("Key", "boon-key", BOLD_FONT, 0.024)):
                wrap = "word" if part == "Text" else "none"
                e.append(entity(f"{card}-{part.lower()}", f"Boon{i}{part}", {**styled(cls), **text(
                    "", font, size, 204, rich=True, wrap=wrap)}, parent=inner, spatial=False))
    else:
        plate_button(e, id + "-button", name + "Button", button_label, column_id, "textures/plate-ember.png",
                     ("begin",), 200)


def screens():
    e = []
    title_screen(e)
    between_screen(e, "held", "Held", "Onward")
    between_screen(e, "upgrade", "Upgrade", "")
    between_screen(e, "fallen", "Fallen", "Back to the shore")
    between_screen(e, "crossed", "Crossed", "Back to the shore")
    # Fills and fades the screens between islands as the battle opens them.
    e.append(entity("screens", "Screens", script("Screens", "scripts/screens.decay"), spatial=False))
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
