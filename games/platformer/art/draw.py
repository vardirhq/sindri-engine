#!/usr/bin/env python3
"""Draws the platformer's pixel art: tiles, the hero, a coin and the flag.

Run from anywhere; it writes into ../assets/textures. Deterministic, so running
it again changes nothing unless this file changed. Pure Python, no imaging
library: the art is small enough to be written a pixel at a time.
"""

import pathlib
import struct
import zlib

OUT = pathlib.Path(__file__).resolve().parent.parent / "assets" / "textures"
CELL = 16


def png(path, width, height, pixels):
    """Writes RGBA `pixels` (rows of (r, g, b, a) tuples) as a PNG."""
    raw = b"".join(
        b"\x00" + b"".join(bytes(pixel) for pixel in row) for row in pixels
    )

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def noise(x, y, salt):
    """A repeatable scatter in [0, 1), so textured tiles are the same every run."""
    value = (x * 374761393 + y * 668265263 + salt * 2246822519) & 0xFFFFFFFF
    value = ((value ^ (value >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((value ^ (value >> 16)) & 0xFFFF) / 65536.0


def hex_color(text):
    text = text.lstrip("#")
    return tuple(int(text[i : i + 2], 16) for i in (0, 2, 4)) + (255,)


CLEAR = (0, 0, 0, 0)


def sheet(cells, rows=1, cell_h=CELL):
    """Lays cells (each a list of rows) side by side into one image."""
    columns = len(cells) // rows
    image = [[CLEAR] * (columns * CELL) for _ in range(rows * cell_h)]
    for index, cell in enumerate(cells):
        ox, oy = (index % columns) * CELL, (index // columns) * cell_h
        for y, row in enumerate(cell):
            for x, pixel in enumerate(row):
                image[oy + y][ox + x] = pixel
    return image


def from_art(art, palette):
    """A cell from rows of characters; `.` is clear."""
    return [[palette[c] if c != "." else CLEAR for c in row] for row in art]


# --- Tiles ------------------------------------------------------------------

EARTH = [hex_color(c) for c in ("#8a5a36", "#7a4e2f", "#9c6a42", "#6b4329")]
GRASS = [hex_color(c) for c in ("#5fbf4a", "#4ea83c", "#79d25c")]
STONE = [hex_color(c) for c in ("#8d8f99", "#7a7c86", "#a2a4ad", "#63656e")]
PLANK = [hex_color(c) for c in ("#c08a4f", "#a8733d", "#d59e60", "#7d5328")]


def dirt_cell(salt):
    return [
        [EARTH[int(noise(x, y, salt) * 4) % 4] for x in range(CELL)] for y in range(CELL)
    ]


def grass_cell():
    cell = dirt_cell(1)
    for x in range(CELL):
        depth = 4 + (1 if noise(x, 0, 7) > 0.6 else 0)
        for y in range(depth):
            cell[y][x] = GRASS[int(noise(x, y, 3) * 3) % 3]
        cell[0][x] = GRASS[2]
    return cell


def stone_cell():
    cell = []
    for y in range(CELL):
        row = []
        for x in range(CELL):
            # Two courses of bricks, offset by half a brick.
            course = y // 8
            seam_x = (x + (4 if course else 0)) % 8 == 0
            seam_y = y % 8 == 0
            if seam_x or seam_y:
                row.append(STONE[3])
            else:
                row.append(STONE[int(noise(x, y, 11) * 3) % 3])
        cell.append(row)
    return cell


def plank_cell():
    cell = [[CLEAR] * CELL for _ in range(CELL)]
    for y in range(6):
        for x in range(CELL):
            shade = PLANK[3] if y in (0, 5) else PLANK[int(noise(x, y, 5) * 3) % 3]
            if x in (0, 15) or (x == 8 and y not in (0, 5)):
                shade = PLANK[3]
            cell[y][x] = shade
    return cell


def tuft_cell():
    cell = [[CLEAR] * CELL for _ in range(CELL)]
    blades = [(3, 5), (5, 8), (7, 4), (9, 7), (11, 5), (13, 6)]
    for x, height in blades:
        for y in range(CELL - height, CELL):
            cell[y][x] = GRASS[1 if y > CELL - 3 else 2]
    return cell


# The ground terrain: sixteen cells, one per combination of which sides are
# more ground (north 1, east 2, south 4, west 8). Grass grows on an open top,
# and an open side or bottom shows a darker rim of earth.
RIM = hex_color("#4a2d18")
COLUMNS = 8
BLANK = [[CLEAR] * CELL for _ in range(CELL)]


def ground_sides(mask):
    return tuple(mask & bit == 0 for bit in (1, 2, 4, 8))


def ground_cell(mask):
    open_n, open_e, open_s, open_w = ground_sides(mask)
    cell = dirt_cell(20 + mask)
    for y in range(CELL):
        for x in range(CELL):
            edge = min(
                x if open_w else CELL,
                CELL - 1 - x if open_e else CELL,
                CELL - 1 - y if open_s else CELL,
            )
            if edge == 0:
                cell[y][x] = RIM
            elif edge == 1:
                cell[y][x] = EARTH[3]
    if open_n:
        for x in range(CELL):
            depth = 4 + (1 if noise(x, 0, 7) > 0.6 else 0)
            # Grass hangs a little down an open side.
            if (open_w and x < 2) or (open_e and x > CELL - 3):
                depth += 2
            for y in range(depth):
                cell[y][x] = GRASS[int(noise(x, y, 3) * 3) % 3]
            cell[0][x] = GRASS[2]
    return cell


def ground_height(mask):
    open_n, open_e, open_s, open_w = ground_sides(mask)

    def height(x, y):
        edge = min(
            x if open_w else CELL,
            CELL - 1 - x if open_e else CELL,
            CELL - 1 - y if open_s else CELL,
            y if open_n else CELL,
        )
        return min(edge, 2) / 2.0 * 0.6 + noise(x, y, 20 + mask) * 0.4

    return height


def tile_cells(first_row, ground):
    """The five loose tiles, then the sixteen ground cells, eight a row."""
    cells = first_row + [BLANK] * (COLUMNS - len(first_row)) + ground
    return cells, len(cells) // COLUMNS


def draw_tiles():
    loose = [grass_cell(), dirt_cell(2), stone_cell(), plank_cell(), tuft_cell()]
    cells, rows = tile_cells(loose, [ground_cell(mask) for mask in range(16)])
    png(OUT / "tiles.png", CELL * COLUMNS, CELL * rows, sheet(cells, rows))


# --- Tile normals -----------------------------------------------------------
#
# A height for every texel of every tile, turned into the direction its surface
# faces: red is right, green is up, blue is out of the picture. Laid out exactly
# like tiles.png, so the same sprite rectangle reads both.


def seam_height(x, y, offset):
    """How far a brick texel is from the mortar round it, capped: a bevel."""
    course_x = (x + offset) % 8
    edge = min(course_x, 8 - course_x, y % 8, 8 - y % 8)
    return min(edge, 2) / 2.0


def stone_height(x, y):
    return seam_height(x, y, 4 if (y % CELL) // 8 else 0) + noise(x, y, 11) * 0.15


def plank_height(x, y):
    if y >= 6:
        return 0.0
    return min(y, 5 - y, 1) * 0.6 + (0.0 if x in (0, 8, 15) else 0.4)


def rough_height(salt):
    return lambda x, y: noise(x, y, salt) * 0.5


HEIGHTS = [rough_height(3), rough_height(2), stone_height, plank_height, lambda x, y: 0.0]


def normal_cell(height, strength=2.0):
    def at(x, y):
        return height(min(max(x, 0), CELL - 1), min(max(y, 0), CELL - 1))

    cell = []
    for y in range(CELL):
        row = []
        for x in range(CELL):
            across = (at(x + 1, y) - at(x - 1, y)) * strength
            # Rows run down the picture, and green means up the world.
            down = (at(x, y + 1) - at(x, y - 1)) * strength
            length = (across * across + down * down + 1.0) ** 0.5
            normal = (-across / length, down / length, 1.0 / length)
            row.append(tuple(round((n * 0.5 + 0.5) * 255) for n in normal) + (255,))
        cell.append(row)
    return cell


def draw_tile_normals():
    loose = [normal_cell(height) for height in HEIGHTS]
    ground = [normal_cell(ground_height(mask)) for mask in range(16)]
    flat = [[(128, 128, 255, 255)] * CELL for _ in range(CELL)]
    cells, rows = tile_cells(loose, ground)
    cells = [flat if cell is BLANK else cell for cell in cells]
    png(OUT / "tiles_n.png", CELL * COLUMNS, CELL * rows, sheet(cells, rows))


# --- The hero ---------------------------------------------------------------

HERO = {
    "o": hex_color("#2a1d3a"),  # outline
    "h": hex_color("#e8563f"),  # hat
    "s": hex_color("#f2c29b"),  # skin
    "e": hex_color("#2a1d3a"),  # eye
    "b": hex_color("#3f7fd9"),  # body
    "d": hex_color("#2c5ea8"),  # body shade
    "l": hex_color("#4a3b5c"),  # legs
    "f": hex_color("#1d1528"),  # feet
}

HEAD = [
    "................",
    ".....oooooo.....",
    "....ohhhhhho....",
    "...ohhhhhhhhoo..",
    "...ooooooooooo..",
    "....ossssseso...",
    "....osssssseo...",
    "....ossssssso...",
    ".....ooooooo....",
]

TORSO = [
    "....obbbbbbo....",
    "...obbbbbbbdo...",
    "...obbbbbbbdo...",
    "....obbbbbdo....",
]

LEGS = {
    "stand": ["....ol.lo.....", "....ol.lo.....", "...off.ffo....."],
    "stride": ["...ol...lo.....", "..ol.....lo....", ".off.....ffo..."],
    "pass": ["....ollo......", "....ollo......", "....offfo....."],
    "tuck": ["...olllllo.....", "....off.ffo....", "..............."],
}


def hero_cell(legs, bob=0):
    rows = HEAD + TORSO + [row.ljust(CELL, ".")[:CELL] for row in LEGS[legs]]
    rows = ["." * CELL] * bob + rows
    rows = (rows + ["." * CELL] * CELL)[:CELL]
    return from_art([row.ljust(CELL, ".")[:CELL] for row in rows], HERO)


def draw_hero():
    frames = [
        hero_cell("stand"),  # idle-0
        hero_cell("stand", 1),  # idle-1
        hero_cell("stride"),  # run-0
        hero_cell("pass", 1),  # run-1
        hero_cell("stride"),  # run-2 (mirrored feel from the pass frame)
        hero_cell("pass", 1),  # run-3
        hero_cell("tuck"),  # jump
        hero_cell("stride"),  # fall
    ]
    # Run-2 steps with the other foot: flip the legs only.
    for y in range(13, 16):
        frames[4][y] = frames[4][y][::-1]
    png(OUT / "hero.png", CELL * len(frames), CELL, sheet(frames))


# --- Coin and flag ----------------------------------------------------------

GOLD = [hex_color(c) for c in ("#f7c948", "#e0a82e", "#fff1a8", "#9c6b13")]


def coin_cell(half_width):
    cell = [[CLEAR] * CELL for _ in range(CELL)]
    cx, cy, radius = 7.5, 7.5, 5.5
    for y in range(CELL):
        for x in range(CELL):
            dx = (x - cx) / max(half_width, 0.5) * radius
            dy = y - cy
            distance = (dx * dx + dy * dy) ** 0.5
            if distance <= radius:
                shade = GOLD[3] if distance > radius - 1.2 else GOLD[0]
                if distance < radius - 1.2 and dx < -1 and dy < -1:
                    shade = GOLD[2]
                elif distance < radius - 1.2 and dx > 1.5:
                    shade = GOLD[1]
                cell[y][x] = shade
    return cell


def draw_coin():
    frames = [coin_cell(w) for w in (5.5, 3.5, 1.2, 3.5)]
    png(OUT / "coin.png", CELL * len(frames), CELL, sheet(frames))


POLE = hex_color("#d9d9e0")
CLOTH = [hex_color("#ffd23f"), hex_color("#e8a91c")]


def flag_cell(wave):
    height = 32
    cell = [[CLEAR] * CELL for _ in range(height)]
    for y in range(2, height):
        cell[y][3] = POLE
    cell[1][3] = GOLD[0]
    for y in range(3, 12):
        for x in range(4, 14):
            lift = 1 if (x + wave) % 4 < 2 else 0
            if y + lift < 12:
                cell[y + lift][x] = CLOTH[(x // 3 + wave) % 2]
    return cell


def draw_flag():
    frames = [flag_cell(0), flag_cell(2)]
    png(OUT / "flag.png", CELL * len(frames), 32, sheet(frames, cell_h=32))


# --- Particles --------------------------------------------------------------


def draw_particle():
    """A soft white dot; an emitter's color ramp tints it."""
    size = 8
    centre = (size - 1) / 2
    pixels = []
    for y in range(size):
        row = []
        for x in range(size):
            distance = ((x - centre) ** 2 + (y - centre) ** 2) ** 0.5 / (size / 2)
            alpha = max(0.0, 1.0 - distance) ** 0.7
            row.append((255, 255, 255, round(alpha * 255)))
        pixels.append(row)
    png(OUT / "particle.png", size, size, pixels)


# --- Panel ------------------------------------------------------------------


def draw_panel():
    """A wooden frame round a dark slate: sliced, so any size keeps its frame.

    The frame is the outer 6 pixels, the border `panel.sheet` records; the
    slate inside is flat so stretching it shows nothing.
    """
    size = 24
    frame = 6
    outline = hex_color("#3b2412")
    slate = (34, 30, 44, 225)
    pixels = []
    for y in range(size):
        row = []
        for x in range(size):
            edge = min(x, y, size - 1 - x, size - 1 - y)
            corner = min(x, size - 1 - x) + min(y, size - 1 - y)
            if corner < 2:
                row.append(CLEAR)
            elif edge == 0 or corner == 2:
                row.append(outline)
            elif edge < frame - 2:
                # Grain runs along each side, and the top and left catch light.
                lit = (y == edge) or (x == edge and y < size - 1 - edge)
                # Stripes running along the side, so stretching it lengthens
                # the grain rather than smearing it.
                side = 0 if edge in (y, size - 1 - y) else 1
                grain = int(noise(edge, side, 7) * 2)
                row.append(PLANK[2] if lit else PLANK[grain])
            elif edge < frame - 1:
                row.append(PLANK[3])
            elif edge < frame:
                row.append(outline)
            else:
                row.append(slate)
        pixels.append(row)
    for nx, ny in ((3, 3), (size - 4, 3), (3, size - 4), (size - 4, size - 4)):
        pixels[ny][nx] = hex_color("#e8d9a8")
    png(OUT / "panel.png", size, size, pixels)


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    draw_tiles()
    draw_tile_normals()
    draw_hero()
    draw_coin()
    draw_flag()
    draw_particle()
    draw_panel()
