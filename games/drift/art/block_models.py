"""Reproducible, cell-bounded metalwork for DRIFT's voxel block set."""

from glb import Material, Mesh

MATERIALS = {
    "seal": Material("#39454b", roughness=0.83, metallic=0.22),
    "enamel": Material("#c1c6b8", roughness=0.61, metallic=0.18),
    "edge": Material("#71827e", roughness=0.42, metallic=0.48),
    "steel": Material("#687a84", roughness=0.66, metallic=0.44),
    "tread": Material("#86999c", roughness=0.49, metallic=0.57),
    "primer": Material("#52666a", roughness=0.72, metallic=0.29),
    "rib": Material("#657977", roughness=0.54, metallic=0.45),
    "ceramic": Material("#c0bbab", roughness=0.66, metallic=0.14),
    "fastener": Material("#a5ada0", roughness=0.48, metallic=0.58),
    "oxide": Material("#b26340", roughness=0.61, metallic=0.22),
    "amber": Material("#dba64c", roughness=0.64, metallic=0.15),
    "lens": Material("#dedfc1", roughness=0.33, emissive="#ffedbd"),
}

# Face-local U runs across the panel, V runs up it, D is depth into the cell.
# Opposite vertical faces share U/V coordinates so paint bands meet at corners.
SIDES = ("front", "back", "right", "left")
HORIZONTAL = ("top", "bottom")


def point(face, u, v, d):
    return {
        "front": (u, v, 1 - d),
        "back": (u, v, d),
        "right": (1 - d, v, u),
        "left": (d, v, u),
        "top": (u, 1 - d, v),
        "bottom": (u, d, v),
    }[face]


def surface(mesh, face, material, points):
    if len(points) < 3:
        return
    u = sum(p[0] for p in points) / len(points)
    v = sum(p[1] for p in points) / len(points)
    mesh.polygon(material, [point(face, *p) for p in points], outward=point(face, u, v, 1))


def rectangle(bounds, depth):
    left, bottom, right, top = bounds
    return [(left, bottom, depth), (right, bottom, depth),
            (right, top, depth), (left, top, depth)]


def panel(mesh, face, center, *, outer=0.025, inner=0.075, depth=0.032,
          rim="seal", bevel="edge"):
    """Welded perimeter and sloped shoulders around a recessed insert."""
    def square(inset, z):
        return rectangle((inset, inset, 1 - inset, 1 - inset), z)

    rings = [square(0, 0), square(outer, 0), square(inner, depth)]
    for outside, inside, material in zip(rings, rings[1:], (rim, bevel)):
        for i in range(4):
            j = (i + 1) % 4
            surface(mesh, face, material, [outside[i], outside[j], inside[j], inside[i]])
    surface(mesh, face, center, rings[-1])


def relief(mesh, face, material, outline, center, *, lip, recess, inset):
    """A small stamped feature with a flat center and four bevel faces."""
    cx, cy = center
    outer = [(u, v, recess) for u, v in outline]
    inner = [(cx + (u - cx) * inset, cy + (v - cy) * inset, lip) for u, v in outline]
    for i in range(len(outer)):
        j = (i + 1) % len(outer)
        surface(mesh, face, material, [outer[i], outer[j], inner[j], inner[i]])
    surface(mesh, face, material, inner)


def square_fastener(mesh, face, u, v, depth):
    outline = [(u - 0.024, v), (u, v - 0.024), (u + 0.024, v), (u, v + 0.024)]
    relief(mesh, face, "fastener", outline, (u, v), lip=depth - 0.014,
           recess=depth, inset=0.62)


def painted_stripes(mesh, face, *, diagonal):
    """Paint tracks continue through every cell boundary in the row."""
    if not diagonal:
        rows = ((0, 0), (0.30, 0), (0.32, 0.017),
                (0.49, 0.017), (0.51, 0), (1, 0))
        for (low, low_d), (high, high_d), material in zip(
            rows, rows[1:], ("enamel", "edge", "oxide", "edge", "enamel")
        ):
            surface(mesh, face, material,
                    [(0, low, low_d), (1, low, low_d), (1, high, high_d), (0, high, high_d)])
        return

    # Clip one diagonal band at a time to the square. The period is 1/3 cell,
    # so another block continues yellow and graphite at both ends of the face.
    def clip(points, limit, keep):
        clipped = []
        for a, b in zip(points[-1:] + points[:-1], points):
            da, db = a[0] - a[1] - limit, b[0] - b[1] - limit
            if keep(da) != keep(db):
                t = da / (da - db)
                clipped.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
            if keep(db):
                clipped.append(b)
        unique = []
        for p in clipped:
            if not unique or p != unique[-1]:
                unique.append(p)
        if len(unique) > 1 and unique[0] == unique[-1]:
            unique.pop()
        return unique

    for i in range(-3, 3):
        for start, end, material in ((0, 0.57, "amber"), (0.57, 1, "seal")):
            outline = clip([(0, 0), (1, 0), (1, 1), (0, 1)], (i + start) / 3, lambda d: d > 0)
            outline = clip(outline, (i + end) / 3, lambda d: d <= 0)
            surface(mesh, face, material, [(u, v, 0.007) for u, v in outline])


def plating(kind):
    mesh = Mesh()
    for face in SIDES + HORIZONTAL:
        if kind == "hull":
            panel(mesh, face, "enamel")
        elif kind == "hull-stripe" and face in SIDES:
            painted_stripes(mesh, face, diagonal=False)
        elif kind == "hazard" and face != "bottom":
            painted_stripes(mesh, face, diagonal=True)
        else:
            panel(mesh, face, "enamel" if kind == "hull-stripe" else "seal")
    return mesh


def deck():
    mesh = Mesh()
    for face in SIDES + ("bottom",):
        surface(mesh, face, "seal", rectangle((0, 0, 1, 1), 0))
    panel(mesh, "top", "steel", outer=0.035, inner=0.065, depth=0.019, bevel="tread")
    for row in range(3):
        for col in range(3):
            u, v = 0.24 + col * 0.26, 0.24 + row * 0.26
            sign = (-1) ** (row + col)
            outline = [(u - 0.094, v - sign * 0.055), (u + 0.061, v + sign * 0.094),
                       (u + 0.094, v + sign * 0.055), (u - 0.061, v - sign * 0.094)]
            relief(mesh, "top", "tread", outline, (u, v), lip=0.001, recess=0.019, inset=0.72)
    return mesh


def bulkhead():
    mesh = Mesh()
    for face in SIDES + HORIZONTAL:
        panel(mesh, face, "primer" if face in HORIZONTAL else "ceramic",
              outer=0.095, inner=0.15, depth=0.073, bevel="steel")
        if face in SIDES:
            for u, v in ((0.235, 0.235), (0.765, 0.235), (0.235, 0.765), (0.765, 0.765)):
                square_fastener(mesh, face, u, v, 0.073)
    return mesh


def frame():
    mesh = Mesh()
    for face in SIDES + HORIZONTAL:
        panel(mesh, face, "primer", outer=0.075, inner=0.16, depth=0.082, bevel="rib")
        outline = [(0.23, 0.41), (0.77, 0.41), (0.77, 0.59), (0.23, 0.59)]
        relief(mesh, face, "rib", outline, (0.5, 0.5), lip=0.029, recess=0.082, inset=0.86)
    return mesh


def light():
    mesh = Mesh()
    for face in SIDES + ("top",):
        panel(mesh, face, "primer", rim="seal", bevel="steel")
    panel(mesh, "bottom", "seal", outer=0.09, inner=0.17, depth=0.09, bevel="steel")
    # Three diffusers float just above the recessed gasket with dark ribs between.
    for u in (0.33, 0.5, 0.67):
        outline = [(u - 0.071, 0.22), (u + 0.071, 0.22),
                   (u + 0.071, 0.78), (u - 0.071, 0.78)]
        relief(mesh, "bottom", "lens", outline, (u, 0.5), lip=0.048, recess=0.09, inset=0.83)
    return mesh


def planar_uv(position, normal):
    """Project each face onto one cell of UV space."""
    axis = max(range(3), key=lambda i: abs(normal[i]))
    if axis == 1:
        return position[0], position[2]
    return position[2 if axis == 0 else 0], 1 - position[1]


def texture_slots(maps, finishes):
    return {material: (maps[finish], planar_uv) for material, finish in finishes.items()}


FINISHES = {
    "enamel": "enamel", "oxide": "enamel", "amber": "enamel",
    "edge": "brushed", "steel": "brushed", "tread": "brushed", "fastener": "brushed",
    "primer": "primer", "seal": "primer", "rib": "primer",
    "ceramic": "ceramic", "lens": "lens",
}


def write_models(directory, maps):
    """Builds seven structural models. The eighth is the existing vent."""
    directory.mkdir(parents=True, exist_ok=True)
    textures = texture_slots(maps, FINISHES)
    for name in ("hull", "hull-stripe", "hazard"):
        plating(name).write(directory / f"{name}-block.glb", MATERIALS,
                            name=f"{name}-block", textures=textures)
    for make in (deck, bulkhead, frame, light):
        name = f"{make.__name__}-block"
        make().write(directory / f"{name}.glb", MATERIALS, name=name, textures=textures)
