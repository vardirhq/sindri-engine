"""The shell's scenes: title, level select, pause and settings.

Each is a backdrop of the dusk art under the same 2D lighting as a level, with a
column of Weave-free UI over it. The scenes are generated so the four share one
look and one set of sizes, and so a button is named in one place and read in the
scripts by the name it is written here.
"""

import json

WIDTH_UNITS, HEIGHT_UNITS = 20.0, 11.25  # world units across and down the screen
FONT = "fonts/Inter.ttf"
INK = [0.96, 0.93, 0.84, 1.0]
AMBER = [1.0, 0.76, 0.36, 1.0]
DIM = [0.62, 0.6, 0.78, 1.0]


def transform(x=0.0, y=0.0, z=0.0, sx=1.0, sy=1.0):
    return {"position": [x, y, z], "rotation": [0, 0, 0, 1], "scale": [sx, sy, 1]}


def backdrop():
    """Sky, far and near hills, under the level's dusk ambient light."""
    cx, cy = WIDTH_UNITS / 2, HEIGHT_UNITS / 2
    layers = (("sky", -30, -3.0), ("hills-far", -20, -2.0), ("hills-near", -10, -1.0))
    entities = [
        {"id": "camera", "name": "Camera", "transform_3d": transform(cx, cy, 10.0), "components": {"sindri.camera": {"projection": "orthographic", "vertical_size": HEIGHT_UNITS, "near": 0.1, "far": 100.0, "fit": "height"}}},
        {
            "id": "environment",
            "name": "Environment",
            "components": {
                "sindri.environment": {
                    "background": [0.08, 0.07, 0.17, 1.0],
                    "shadows": {"enabled": False, "map_size": 1024, "distance": 48.0, "bias": 0.002},
                    "lighting_2d": {"enabled": True, "ambient": [0.36, 0.34, 0.55]},
                }
            },
        },
    ]
    for name, layer, z in layers:
        entities.append({"id": name, "name": name, "transform_3d": transform(cx, cy, z, WIDTH_UNITS, HEIGHT_UNITS), "components": {"sindri.sprite": {"texture": f"textures/{name}.png", "layer": layer, "unlit": True}}})
    return entities


def text(id, words, size, color=INK, anchor="center", layer=100, bold=True, **extra):
    body = {
        "anchor": anchor,
        "color": color,
        "font": FONT,
        "font_size": size,
        "line_height": size * 1.2,
        "layer": layer,
        "text": words,
        "shadow": {"offset": [0.0, -0.004], "color": [0.0, 0.0, 0.0, 0.55], "softness": 0.003},
        "bold": bold,
        "line_align": "center",
        **extra,
    }
    return {"id": id, "name": id, "transform_3d": transform(sx=1.4, sy=size * 1.3), "components": {"sindri.ui.text": body}}


def button(id, label, parent, width=0.62, height=0.095, disabled=False):
    """A button and the words on it: the button draws its box, a child text draws its label."""
    box = {
        "id": id,
        "name": id,
        "parent": parent,
        "transform_3d": transform(sx=width, sy=height),
        "components": {
            "sindri.ui.shape": {"kind": "rect", "fill": [0.09, 0.08, 0.2, 0.92], "anchor": "center", "layer": 90},
            "sindri.ui.button": {"label": label, "autofocus": False, "disabled": disabled},
            "sindri.ui.layout": {"direction": "row", "spacing": 0.02, "justify": "center", "align": "center"},
        },
    }
    words = {**text(f"{id}-label", label, 0.05, DIM if disabled else INK, layer=95), "parent": id}
    words["transform_3d"] = transform(sx=width * 0.9, sy=height * 0.8)
    return [box, words]


def column(id, parent=None, spacing=0.035, width=1.0, height=1.6):
    entity = {
        "id": id,
        "name": id,
        "transform_3d": transform(sx=width, sy=height),
        "components": {"sindri.ui.layout": {"direction": "column", "spacing": spacing, "justify": "center", "align": "center"}},
    }
    if parent:
        entity["parent"] = parent
    return entity


def scene(title, entities):
    return {"format_version": 10, "metadata": {"name": title}, "entities": entities}


def title_scene():
    entities = backdrop()
    entities.append(column("menu"))
    entities.append({
        "id": "logo",
        "name": "logo",
        "parent": "menu",
        "transform_3d": transform(sx=0.82, sy=0.273),
        "components": {
            "sindri.ui.image": {
                "anchor": "center",
                "layer": 100,
                "texture": "textures/lamplighter-logo.png",
                "tint": [1.0, 1.0, 1.0, 1.0],
            }
        },
    })
    entities.append({**text("tagline", "Keep the way home lit.", 0.045, DIM, bold=False), "parent": "menu"})
    for id, label in (("play", "Play"), ("levels", "Levels"), ("settings", "Settings")):
        entities.extend(button(id, label, "menu"))
    entities.append(script_entity("Title"))
    return scene("Lamplighter title", entities)


LEVELS = [
    "Dusk Road",
    "The Ferry Landing",
    "Windmill Ridge",
    "The Cable Car",
    "The Hanging Lamps",
    "The Lantern Caves",
    "Storm Stair",
    "The Dawn Beacon",
]
BUILT = 1  # how many of them can be played so far


def script_entity(name):
    return {"id": "shell", "name": "Shell", "components": {"sindri.script": {"source": "scripts/shell.decay", "script": name, "properties": {}}}}


def levels_scene():
    entities = backdrop()
    entities.append(column("menu", spacing=0.02, height=1.8))
    entities.append({**text("heading", "Levels", 0.08, AMBER), "parent": "menu"})
    for number, name in enumerate(LEVELS, start=1):
        label = f"{number}  {name}" if number <= BUILT else f"{number}  {name}  -  soon"
        entities.extend(button(f"level-{number}", label, "menu", width=1.1, height=0.085, disabled=number > BUILT))
    entities.extend(button("back", "Back", "menu", width=0.5, height=0.085))
    entities.append(script_entity("Levels"))
    return scene("Lamplighter levels", entities)


def settings_scene():
    entities = backdrop()
    entities.append(column("menu", spacing=0.03, height=1.6))
    entities.append({**text("heading", "Settings", 0.08, AMBER), "parent": "menu"})
    for bus, title in (("music", "Music"), ("effects", "Effects")):
        entities.append({**text(f"{bus}-value", f"{title} 100", 0.05), "parent": "menu"})
        entities.extend(button(f"{bus}-down", f"{title} down", "menu", width=0.62))
        entities.extend(button(f"{bus}-up", f"{title} up", "menu", width=0.62))
    entities.extend(button("motion", "Reduced motion: off", "menu", width=0.8))
    entities.extend(button("back", "Back", "menu", width=0.5))
    entities.append(script_entity("Settings"))
    return scene("Lamplighter settings", entities)


def pause_scene():
    entities = backdrop()
    entities.append(column("menu"))
    entities.append({**text("heading", "Paused", 0.1, AMBER), "parent": "menu"})
    for id, label in (("resume", "Resume"), ("levels", "Levels"), ("settings", "Settings"), ("title", "Title")):
        entities.extend(button(id, label, "menu"))
    entities.append(script_entity("Pause"))
    return scene("Lamplighter pause", entities)


def write_all(assets):
    built = (("title.scene", title_scene()), ("levels.scene", levels_scene()), ("settings.scene", settings_scene()), ("pause.scene", pause_scene()))
    for name, document in built:
        (assets / name).write_text(json.dumps(document, indent=2) + "\n")
