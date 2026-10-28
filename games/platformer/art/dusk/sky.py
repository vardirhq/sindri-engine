"""Dusk sky and the hills behind it, drawn as the layers a level scrolls.

The sky is one gradient from night at the top to ember at the horizon, dithered
between bands so it does not stripe. Hills are silhouettes in the violet of the
sky they stand against, so depth comes from value alone, and loop horizontally
so a layer can scroll for ever.
"""

import math

from lib import blank, noise
from palette import ramp

SKY = ramp("sky")
WIDTH, HEIGHT = 320, 180

BAYER = ((0, 8, 2, 10), (12, 4, 14, 6), (3, 11, 1, 9), (15, 7, 13, 5))


def sky_layer(width=WIDTH, height=HEIGHT, horizon=0.78):
    """The gradient: night at the top, ember where the hills begin."""
    image = blank(width, height)
    last = len(SKY) - 1
    for y in range(height):
        t = min(1.0, y / (height * horizon)) ** 1.35
        position = t * last
        low = int(position)
        fraction = position - low
        high = min(last, low + 1)
        for x in range(width):
            threshold = (BAYER[y % 4][x % 4] + 0.5) / 16.0
            image[y][x] = SKY[high] if fraction > threshold else SKY[low]
    return image


def stars(image, count=46, seed=1):
    """A scatter of stars, brighter and denser high up where the sky is darkest."""
    height = len(image)
    width = len(image[0])
    for i in range(count):
        x = int(noise(i, 0, seed) * width)
        y = int((noise(i, 1, seed) ** 1.8) * height * 0.55)
        bright = noise(i, 2, seed)
        color = ramp("rock")[5] if bright > 0.7 else ramp("rock")[4]
        image[y][x] = color
        if bright > 0.93:
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                if 0 <= x + dx < width and 0 <= y + dy < height:
                    image[y + dy][x + dx] = ramp("rock")[3]


def moon(image, cx, cy, radius=9):
    """A pale crescent, cool so that amber stays the only warm thing."""
    light, shade = ramp("rock")[5], ramp("rock")[3]
    for y in range(cy - radius, cy + radius + 1):
        for x in range(cx - radius, cx + radius + 1):
            inside = (x - cx) ** 2 + (y - cy) ** 2 <= radius ** 2
            bite = (x - cx - 4) ** 2 + (y - cy + 2) ** 2 <= (radius - 1) ** 2
            if inside and not bite:
                image[y][x] = light if (x - cx) + (y - cy) < 2 else shade


def hills(width=WIDTH, height=HEIGHT, base=0.62, amplitude=26, period=WIDTH, color=None, seed=0, trees=False):
    """One silhouette of hills that loops every `period` pixels, on transparency."""
    color = color or ramp("sky")[3]
    image = blank(width, height)
    heights = []
    for x in range(width):
        phase = 2 * math.pi * x / period
        h = (
            math.sin(phase + seed) * 0.55
            + math.sin(phase * 2 + seed * 1.7) * 0.3
            + math.sin(phase * 5 + seed * 0.4) * 0.15
        )
        heights.append(int(height * base - h * amplitude))
    for x in range(width):
        for y in range(heights[x], height):
            image[y][x] = color
    if trees:
        for i in range(0, width, 7):
            tx = i + int(noise(i, 3, int(seed * 10)) * 5)
            if tx >= width:
                continue
            top = heights[tx] - 6 - int(noise(i, 4, int(seed * 10)) * 8)
            for dy in range(top, heights[tx] + 1):
                spread = (dy - top) // 3
                for dx in range(-spread, spread + 1):
                    if 0 <= tx + dx < width and 0 <= dy < height:
                        image[dy][tx + dx] = color
    return image
