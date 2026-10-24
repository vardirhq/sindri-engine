"""Lamplighter's palette.

One warm color in a cool world: everything is violet, teal and slate, and
amber is light. A lit lantern is the only warmth on screen, so it reads from
across a level. Every ramp is dark to light and every material draws from one.
"""

from lib import hex_color

RAMPS = {
    # Sky, dusk to horizon.
    "sky": ["#14122b", "#1d1b3a", "#2b2650", "#443768", "#6b4876", "#a05a78", "#d9805b", "#f2a65a"],
    # Rock and earth: cool slate, with a violet cast in the shade.
    "rock": ["#1b1e30", "#262b42", "#343b57", "#47506c", "#5f6a85", "#7d89a1"],
    # Soil under the grass.
    "soil": ["#2a2030", "#3b2b3a", "#503a46", "#6a4b52"],
    # Grass: blue-green in shadow, lit yellow-green only at the very tips.
    "grass": ["#1f4a4a", "#2a6155", "#3b7c5c", "#59995f", "#86b36a"],
    # Wood, for planks, ferries and posts.
    "wood": ["#2d1f2e", "#47303a", "#654550", "#8a6258", "#b0845f"],
    # Cloth and the courier's coat.
    "cloth": ["#1d2a45", "#2c3f66", "#41609a", "#5f86bf"],
    # Skin.
    "skin": ["#6b4540", "#a06a58", "#d29a7a", "#f0c4a0"],
    # Metal, for lanterns' frames and hinges.
    "metal": ["#202436", "#33394f", "#4e566f", "#7c869e"],
    # Light itself: the only warm color, and the only thing that glows.
    "amber": ["#8a3d2a", "#c9622c", "#f0912e", "#ffc35c", "#ffe9a8"],
    # Water at dusk.
    "water": ["#14233f", "#1c3557", "#2a5071", "#438096", "#79b8bd"],
    # The dark that creeps in: near black with a violet cast.
    "gloom": ["#0c0a17", "#15112a", "#201a3d", "#2f2552"],
}


def ramp(name):
    return [hex_color(c) for c in RAMPS[name]]
