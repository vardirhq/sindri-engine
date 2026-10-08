"""The playground's instruments: the probe's drawn geometry and the debug
overlay's owner. The overlay's marks are spawned by Decay as it needs them,
from the debug prefabs, under one parent that DEBUG switches on and off."""

from __future__ import annotations

from common import script, shape
from scene import Scene

LAB = "scripts/lab.decay"
PROBE = "#7df9ff"


def build(scene: Scene) -> None:
    scene.add("probe", "Probe", components={"sindri.script": script(
        LAB, "Probe", ring="prefabs/debug-ring.prefab")})
    for entity_id, kind, fill, edge, size, layer in [
        ("probe-line", "rect", PROBE, None, (1.0, 0.06), 62),
        ("probe-normal", "rect", "#ff8fab", None, (1.0, 0.07), 63),
        ("probe-hit", "ellipse", "#ffffff", PROBE, (0.3, 0.3), 64),
        ("probe-ghost-circle", "ellipse", PROBE, PROBE, (1.0, 1.0), 61),
        ("probe-ghost-box", "rect", PROBE, PROBE, (1.2, 0.7), 61),
        ("probe-area", "ellipse", PROBE, PROBE, (1.0, 1.0), 60),
    ]:
        ghostly = entity_id.startswith(("probe-ghost", "probe-area"))
        entity = scene.add(entity_id, entity_id, 0, -40, 1.9, size[0], size[1], components={
            "sindri.shape": shape(kind, fill, edge, sw=0.05 if edge else 0.0, layer=layer,
                                  alpha=0.12 if ghostly else 1.0,
                                  dashes=16 if ghostly else 0),
        })
        entity["disabled"] = True
    scene.add("debug-layer", "debug-layer")["disabled"] = True


def overlay(scene: Scene) -> None:
    """Added last, once every joint is known."""
    scene.finish()
    scene.add("overlay", "Debug overlay", components={"sindri.script": script(
        LAB, "DebugOverlay",
        outline="prefabs/debug-rect.prefab", ring="prefabs/debug-ring.prefab",
        bar="prefabs/debug-bar.prefab", dot="prefabs/debug-dot.prefab",
        links=scene.links,
    )})


def prefabs(prefab) -> dict[str, dict]:
    def mark(name, kind, fill, stroke, sw):
        return prefab(name, name, {"sindri.shape": shape(
            kind, fill, stroke, sw=sw, layer=56, alpha=0.0 if stroke else 1.0)})
    return {
        "debug-rect.prefab": mark("debug-rect", "rect", "#000000", "#ffffff", 0.06),
        "debug-ring.prefab": mark("debug-ring", "ellipse", "#000000", "#ffffff", 0.08),
        "debug-bar.prefab": mark("debug-bar", "rect", "#ffffff", None, 0.0),
        "debug-dot.prefab": mark("debug-dot", "ellipse", "#ffffff", None, 0.0),
    }
