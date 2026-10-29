#!/usr/bin/env python3
"""Builds the village's art from a ChatGPT sprite atlas: `textures/village.png` and `.sheet` (houses,
trees, bushes, flowers, fences, lamps, signs, props) and `textures/village-ground.png` and `.sheet`
(the grass, earth and cobblestone tiles the ground is laid from).

`art/pixellab/village/atlas.png` is one picture of a village's parts, in one style, drawn at about
three times the size of its pixel art and with some stains and soft edges, which this tidies:

- each part is cut out by its rectangle and only the piece the rectangle is about is kept (a
  neighbour's edge that reaches into it is dropped);
- it is shrunk by `SCALE` so a world unit is 16 pixels like the rest of the game, with the colours
  blended in premultiplied form so the edges do not darken;
- its edge is made hard (a pixel is there or it is not) and its colours are snapped to one palette
  shared by every part, so what was made apart looks like one thing.

The rectangles are in atlas pixels. Needs Pillow and numpy; deterministic, so CI can run it again
and fail on any drift."""
import json
import pathlib

import numpy as np
from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
ATLAS = HERE / 'pixellab' / 'village' / 'atlas.png'
TEXTURES = HERE.parent / 'assets' / 'textures'
SCALE = 0.55
CELL = 136
COLUMNS = 12
TILE = 28
PALETTE_COLORS = 96

# name: (left, top, right, bottom) in the atlas.
SPRITES = {
    # houses
    'house-teal': (1106, 38, 1295, 259), 'house-workshop': (1315, 36, 1462, 259), 'house-keeper': (1462, 84, 1642, 259),
    # trees, stump, log
    'tree-oak': (665, 298, 792, 458), 'tree-broad': (770, 298, 858, 432), 'tree-tall': (856, 286, 962, 454),
    'tree-small-1': (665, 454, 730, 545), 'tree-small-2': (743, 426, 799, 498), 'tree-small-3': (816, 423, 882, 498),
    'tree-small-4': (895, 448, 967, 547), 'stump': (736, 501, 788, 547), 'log': (806, 513, 890, 547),
    # bushes
    'bush-1': (457, 312, 511, 371), 'bush-2': (524, 309, 584, 363), 'bush-3': (596, 317, 635, 360),
    'bush-flower': (455, 382, 520, 447), 'bush-4': (532, 375, 578, 445), 'bush-5': (589, 371, 636, 444),
    'bush-6': (454, 458, 511, 515), 'bush-7': (522, 457, 565, 510), 'bush-8': (576, 459, 638, 508),
    # flowers and plants
    'flowers-1': (251, 309, 281, 354), 'flowers-2': (296, 319, 321, 353), 'flowers-3': (334, 314, 363, 353),
    'flowers-4': (248, 370, 289, 400), 'flowers-5': (340, 371, 367, 397), 'flowers-6': (385, 370, 418, 401),
    'flowers-7': (248, 420, 276, 450), 'flowers-8': (291, 413, 319, 449), 'flowers-9': (332, 414, 368, 452),
    'flowers-10': (249, 463, 291, 494), 'flowers-11': (314, 466, 355, 494), 'reeds': (383, 463, 419, 543),
    # fences and walls
    'fence-1': (994, 309, 1061, 345), 'fence-2': (1076, 311, 1123, 345), 'fence-3': (1138, 309, 1197, 344),
    'fence-garden-1': (994, 361, 1061, 404), 'wall-low': (1076, 358, 1162, 404), 'wall-post': (1177, 355, 1198, 463),
    'fence-garden-2': (994, 419, 1092, 462), 'gate-wood': (994, 477, 1061, 545), 'gate-stone': (1106, 414, 1165, 463),
    'wall-pier-1': (1106, 478, 1143, 544), 'wall-pier-2': (1155, 477, 1198, 544),
    # wells and lamps
    'well-roofed': (1514, 312, 1588, 426), 'well-small': (1632, 312, 1678, 377), 'lantern-hang': (1600, 373, 1619, 416),
    'lantern-small': (1704, 335, 1721, 368), 'lamp-short-1': (1513, 446, 1542, 547), 'lamp-short-2': (1559, 444, 1589, 547),
    'lamp-pole': (1604, 451, 1633, 547), 'lamp-tall': (1644, 403, 1677, 545), 'lamp-tall-2': (1694, 389, 1727, 545),
    # signs
    'sign-herbs': (1764, 315, 1869, 383), 'sign-ring': (1893, 308, 1965, 385), 'sign-tools': (1842, 395, 1905, 482),
    'banner': (1919, 398, 1967, 476), 'sign-arrows': (1764, 401, 1822, 482), 'noticeboard': (1781, 492, 1857, 546),
    # props
    'barrel-1': (1236, 309, 1274, 363), 'barrel-2': (1292, 312, 1330, 363), 'crate': (1336, 312, 1364, 362),
    'woodpile': (1298, 358, 1364, 410), 'table-big': (1377, 301, 1474, 358), 'barrel-3': (1237, 372, 1275, 407),
    'cart': (1381, 364, 1474, 421), 'sacks-1': (1235, 419, 1274, 443), 'sack-pot': (1286, 419, 1310, 483),
    'bench': (1332, 428, 1403, 469), 'pots': (1419, 425, 1472, 471), 'sacks-2': (1236, 454, 1274, 482),
    'pot': (1335, 478, 1360, 510), 'table-goods': (1234, 494, 1317, 547), 'sack': (1335, 518, 1359, 547),
    'workbench': (1375, 486, 1473, 544), 'cat': (1016, 659, 1077, 694),
}
# Tiles: name: (left, top, right, bottom) of a patch of the surface, cut from inside a tile.
TILES = {
    'grass-0': (23, 315, 59, 352), 'grass-1': (87, 316, 123, 351), 'grass-2': (156, 316, 194, 352),
    'grass-3': (25, 380, 59, 415), 'grass-4': (89, 381, 124, 415), 'grass-5': (153, 380, 198, 416),
    'dirt-0': (412, 45, 446, 82), 'dirt-1': (487, 45, 518, 82),
    'stone-0': (703, 43, 750, 90), 'stone-1': (781, 44, 823, 89),
}


def isolate(rgba, box):
    """The part of the crop that belongs to what the rectangle is about: its largest piece and any
    piece of a fair size beside it, with the rest (a neighbour's edge) made clear."""
    a = rgba[..., 3] > 140
    h, w = a.shape
    seen = np.zeros((h, w), bool)
    pieces = []
    for y0 in range(h):
        for x0 in range(w):
            if a[y0, x0] and not seen[y0, x0]:
                stack = [(y0, x0)]
                seen[y0, x0] = True
                cells = []
                while stack:
                    y, x = stack.pop()
                    cells.append((y, x))
                    for dy in (-1, 0, 1):
                        for dx in (-1, 0, 1):
                            ny, nx = y + dy, x + dx
                            if 0 <= ny < h and 0 <= nx < w and a[ny, nx] and not seen[ny, nx]:
                                seen[ny, nx] = True
                                stack.append((ny, nx))
                pieces.append(cells)
    if not pieces:
        return rgba
    biggest = max(len(p) for p in pieces)
    keep = np.zeros((h, w), bool)
    for cells in pieces:
        if len(cells) >= max(30, biggest // 5):
            for y, x in cells:
                keep[y, x] = True
    # grow the kept area a few pixels to take back the soft edge the threshold cut
    grown = keep.copy()
    for _ in range(3):
        grown = grown | np.roll(grown, 1, 0) | np.roll(grown, -1, 0) | np.roll(grown, 1, 1) | np.roll(grown, -1, 1)
    out = rgba.copy()
    out[..., 3] = np.where(grown, rgba[..., 3], 0)
    return out


def shrink(picture, scale):
    """The picture scaled down with its colours blended in premultiplied form, its edge made hard."""
    size = (max(1, round(picture.width * scale)), max(1, round(picture.height * scale)))
    small = picture.convert('RGBa').resize(size, Image.BOX).convert('RGBA')
    pixels = np.array(small)
    pixels[..., 3] = np.where(pixels[..., 3] >= 128, 255, 0)
    return Image.fromarray(pixels, 'RGBA')


def palette_from(atlas):
    solid = np.array(atlas)
    solid = solid[solid[..., 3] > 215][:, :3]
    sample = Image.fromarray(solid[:: max(1, len(solid) // 200000)].reshape(1, -1, 3).astype(np.uint8), 'RGB')
    return sample.quantize(colors=PALETTE_COLORS, method=Image.MEDIANCUT, dither=Image.NONE)


def snap(picture, palette):
    """Every colour of the picture moved to the nearest of the shared palette; its edge kept."""
    alpha = picture.getchannel('A')
    snapped = picture.convert('RGB').quantize(palette=palette, dither=Image.NONE).convert('RGB')
    out = snapped.convert('RGBA')
    out.putalpha(alpha)
    return out


# These come twice: lit, as they were drawn, and unlit (`<name>-unlit`) for before their lamp is lit.
DIMMED = ['house-teal', 'house-workshop', 'house-keeper', 'lamp-short-1', 'lamp-short-2', 'lamp-tall', 'lamp-tall-2']


def dim(picture):
    """The picture with its light out: warm pixels dulled to the dark behind them, the rest darker."""
    out = picture.copy()
    pixels = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = pixels[x, y]
            if not a:
                continue
            if r > b + 24 and r * 10 >= g * 8:
                shade = max(14, (r + g + b) // 3 * 2 // 5)
                pixels[x, y] = (shade + 4, shade + 2, max(shade - 4, 0), a)
            else:
                pixels[x, y] = (r * 4 // 5, g * 4 // 5, b * 4 // 5, a)
    return out


def build():
    atlas = Image.open(ATLAS).convert('RGBA')
    palette = palette_from(atlas)
    TEXTURES.mkdir(parents=True, exist_ok=True)
    pictures = []
    for name, box in SPRITES.items():
        crop = np.array(atlas.crop(box))
        shown = snap(shrink(Image.fromarray(isolate(crop, box), 'RGBA'), SCALE), palette)
        bbox = shown.getbbox()
        assert bbox, name
        shown = shown.crop(bbox)
        assert shown.width <= CELL and shown.height <= CELL, (name, shown.size)
        pictures.append((name, shown))
        if name in DIMMED:
            pictures.append((f'{name}-unlit', dim(shown)))
    names = [name for name, _ in pictures]
    rows = (len(pictures) + COLUMNS - 1) // COLUMNS
    sheet = Image.new('RGBA', (CELL * COLUMNS, CELL * rows))
    for index, (name, shown) in enumerate(pictures):
        column, row = index % COLUMNS, index // COLUMNS
        sheet.alpha_composite(shown, (column * CELL + (CELL - shown.width) // 2, row * CELL + CELL - shown.height))
    (HERE / 'village_sizes.json').write_text(json.dumps({name: list(p.size) for name, p in pictures}, indent=1) + '\n')
    sheet.save(TEXTURES / 'village.png', optimize=True)
    (TEXTURES / 'village.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'bottom',
         'grid': {'columns': COLUMNS, 'rows': rows, 'names': names}}, indent=2) + '\n')

    tiles = []
    ground = Image.new('RGBA', (TILE * 4, TILE * ((len(TILES) + 3) // 4)))
    for index, (name, box) in enumerate(TILES.items()):
        patch = atlas.crop(box).convert('RGB').resize((TILE, TILE), Image.BOX)
        patch = patch.quantize(palette=palette, dither=Image.NONE).convert('RGBA')
        ground.paste(patch, ((index % 4) * TILE, (index // 4) * TILE))
        tiles.append(name)
    ground.save(TEXTURES / 'village-ground.png', optimize=True)
    (TEXTURES / 'village-ground.sheet').write_text(json.dumps(
        {'format_version': 1, 'anchor': 'center',
         'grid': {'columns': 4, 'rows': (len(TILES) + 3) // 4, 'names': tiles}}, indent=2) + '\n')


if __name__ == '__main__':
    build()
