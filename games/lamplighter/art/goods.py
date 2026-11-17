#!/usr/bin/env python3
"""What Wick gathers and what he builds with it, in the wilds' palette.

`goods.png` is a row of 16 pixel cells, each lying on the ground: a bundle of
deadwood, a pale flint, a drop of resin catching the light, a sprig of berries. `campfire.png` is a
ring of stones round crossed logs, its fire left to the light and the embers
the prefab carries, so a fire that has burnt down is the same picture. Each has
a sheet that stands it on its foot. Deterministic.
"""
import json
from pathlib import Path

from PIL import Image

TEXTURES = Path(__file__).resolve().parent.parent / 'assets' / 'textures'
OUTLINE = (20, 14, 12, 255)
BARK, BARK_DARK, BARK_LIGHT = (92, 62, 40, 255), (62, 40, 26, 255), (128, 90, 58, 255)
FLINT, FLINT_DARK, FLINT_LIGHT = (128, 128, 136, 255), (84, 84, 94, 255), (176, 176, 184, 255)
RESIN, RESIN_DARK, RESIN_LIGHT = (214, 128, 30, 255), (150, 72, 20, 255), (255, 214, 120, 255)
EMBER = (196, 70, 24, 255)
BERRY, BERRY_DARK, BERRY_LIGHT = (150, 36, 78, 255), (96, 20, 54, 255), (226, 112, 150, 255)
LEAF, LEAF_DARK = (74, 112, 52, 255), (46, 74, 34, 255)


def pixels(image, ox, oy, rows, colours):
    """Paints a picture given as rows of letters, one colour a letter, '.' none."""
    for y, row in enumerate(rows):
        for x, letter in enumerate(row):
            if letter != '.':
                image.putpixel((ox + x, oy + y), colours[letter])


WOOD = [
    '................',
    '................',
    '................',
    '................',
    '................',
    '................',
    '..o.........o...',
    '.obo.......obo..',
    '.oblooooooooblo.',
    '..obbbbbbbbbbbo.',
    '.oolllllllllloo.',
    '.obddddddddddbo.',
    '..obbbbbbbbbbo..',
    '...oooooooooo...',
    '................',
    '................',
]
STONE = [
    '................',
    '................',
    '................',
    '................',
    '................',
    '................',
    '................',
    '.......ooo......',
    '.....oollfoo....',
    '....oflllffdo...',
    '...offffffddo...',
    '...offfffdddo...',
    '....odddddoo....',
    '.....ooooo......',
    '................',
    '................',
]
RESIN_DROP = [
    '................',
    '................',
    '................',
    '................',
    '................',
    '................',
    '.......o........',
    '......oro.......',
    '.....orlro......',
    '....orlrrro.....',
    '....orrrrdo.....',
    '....ordrddo.....',
    '.....oddoo......',
    '......oo........',
    '................',
    '................',
]
BERRIES = [
    '................',
    '................',
    '................',
    '................',
    '................',
    '.......oo.......',
    '......ogo.oo....',
    '.....oggooggo...',
    '....oo.ogkgo....',
    '...oblo.okoolo..',
    '..obbldoolo.bldo',
    '..obbddobldobbdo',
    '...oddoobbdoodo.',
    '....oo.obddo.o..',
    '........ooo.....',
    '................',
]
CAMPFIRE = [
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '................................',
    '.........o..........o...........',
    '........obo........obo..........',
    '.........obo......obo...........',
    '..........obooooobo.............',
    '......ooo..obeeeeob..ooo........',
    '.....offdo.oeeeeeeo.offdo.......',
    '....offfddoobbbbbbooffddo.......',
    '....ofdddo.oblllllbo.oddo.......',
    '.ooo.ooo..obbbbbbbbbo.oo..ooo...',
    'offdo...ooobbbbbbbbbooo..offdo..',
    'offddoooffdooooooooofffdooffddo.',
    '.oddo.offfdddo...offfddo.oddo...',
    '..oo...ooddoo.....ooddoo..oo....',
    '........oooo.......oooo.........',
    '................................',
]


def main():
    colours = {'o': OUTLINE, 'b': BARK, 'd': BARK_DARK, 'l': BARK_LIGHT}
    goods = Image.new('RGBA', (64, 16))
    pixels(goods, 0, 0, WOOD, colours)
    pixels(goods, 16, 0, STONE, {'o': OUTLINE, 'f': FLINT, 'd': FLINT_DARK, 'l': FLINT_LIGHT})
    pixels(goods, 32, 0, RESIN_DROP, {'o': OUTLINE, 'r': RESIN, 'd': RESIN_DARK, 'l': RESIN_LIGHT})
    pixels(goods, 48, 0, BERRIES, {'o': OUTLINE, 'b': BERRY, 'd': BERRY_DARK, 'l': BERRY_LIGHT,
                                   'g': LEAF, 'k': LEAF_DARK})
    goods.save(TEXTURES / 'goods.png', optimize=True)
    (TEXTURES / 'goods.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom', 'grid': {'columns': 4, 'rows': 1,
                                                           'names': ['wood', 'stone', 'resin', 'berries']}}, indent=2) + '\n')
    fire = Image.new('RGBA', (32, 32))
    pixels(fire, 0, 0, CAMPFIRE, {'o': OUTLINE, 'b': BARK, 'd': FLINT_DARK, 'l': BARK_LIGHT,
                                  'f': FLINT, 'e': EMBER})
    fire.save(TEXTURES / 'campfire.png', optimize=True)
    (TEXTURES / 'campfire.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom', 'grid': {'columns': 1, 'rows': 1, 'names': ['campfire']}},
        indent=2) + '\n')


if __name__ == '__main__':
    main()
