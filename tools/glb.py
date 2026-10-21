"""A small, dependency-free writer of low-poly binary glTF (GLB) models.

Causeway's models are built rather than drawn: a figure is a handful of
bevelled boxes, cylinders and cones; an island is a column per cell. This turns
such a description into a self-contained GLB that `sindri-assets` decodes, with
one primitive per material and flat face normals, so every facet catches the
light on its own -- which is the whole low-poly look.

Colours are written as sRGB hex and stored linear, as glTF wants. A material
may glow (`emissive`), which the bloom picks up.
"""
import json
import math
import struct


def srgb_to_linear(channel):
    channel /= 255.0
    if channel <= 0.04045:
        return channel / 12.92
    return ((channel + 0.055) / 1.055) ** 2.4


def color(hex_color):
    hex_color = hex_color.lstrip("#")
    return [srgb_to_linear(int(hex_color[i:i + 2], 16)) for i in (0, 2, 4)]


class Material:
    def __init__(self, base, roughness=0.85, metallic=0.0, emissive=None, strength=1.0):
        self.base = color(base)
        self.roughness = roughness
        self.metallic = metallic
        self.emissive = [c * strength for c in color(emissive)] if emissive else [0.0, 0.0, 0.0]


# --- Small vector helpers ---------------------------------------------------

def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def norm(a):
    length = math.sqrt(dot(a, a)) or 1.0
    return (a[0] / length, a[1] / length, a[2] / length)


def rotation_matrix(degrees):
    """Euler XYZ in degrees, applied X then Y then Z, as the baker does."""
    rx, ry, rz = (math.radians(d) for d in degrees)
    cx, sx, cy, sy, cz, sz = math.cos(rx), math.sin(rx), math.cos(ry), math.sin(ry), math.cos(rz), math.sin(rz)
    x = ((1, 0, 0), (0, cx, -sx), (0, sx, cx))
    y = ((cy, 0, sy), (0, 1, 0), (-sy, 0, cy))
    z = ((cz, -sz, 0), (sz, cz, 0), (0, 0, 1))

    def mul(a, b):
        return tuple(tuple(sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)) for i in range(3))

    return mul(z, mul(y, x))


def apply(matrix, point):
    return tuple(sum(matrix[i][j] * point[j] for j in range(3)) for i in range(3))


class Mesh:
    """Polygons grouped by material, each polygon flat-shaded."""

    def __init__(self):
        self.groups = {}

    def polygon(self, material, points, outward=None):
        """A convex polygon. `outward` is a point it must face away from."""
        if len(points) < 3:
            return
        normal = norm(cross(sub(points[1], points[0]), sub(points[2], points[0])))
        if outward is not None:
            centre = tuple(sum(p[i] for p in points) / len(points) for i in range(3))
            if dot(normal, sub(centre, outward)) < 0:
                points = list(reversed(points))
                normal = (-normal[0], -normal[1], -normal[2])
        group = self.groups.setdefault(material, ([], [], []))
        positions, normals, indices = group
        start = len(positions)
        for point in points:
            positions.append(point)
            normals.append(normal)
        for i in range(1, len(points) - 1):
            indices.extend((start, start + i, start + i + 1))

    def transformed(self, material, polygons, position=(0, 0, 0), rotation=None, centre=(0, 0, 0)):
        matrix = rotation_matrix(rotation) if rotation else None
        for points in polygons:
            moved = []
            for p in points:
                if matrix:
                    p = apply(matrix, p)
                moved.append((p[0] + position[0], p[1] + position[1], p[2] + position[2]))
            inside = centre
            if matrix:
                inside = apply(matrix, inside)
            inside = (inside[0] + position[0], inside[1] + position[1], inside[2] + position[2])
            self.polygon(material, moved, inside)

    # --- Shapes ---------------------------------------------------------------

    def box(self, material, position, size, rotation=None, bevel=0.0, bevel_bottom=True):
        """A box centred on `position`, its edges chamfered by `bevel`."""
        hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
        b = min(bevel, hx * 0.9, hy * 0.9, hz * 0.9)
        self.transformed(material, box_polygons(hx, hy, hz, b, bevel_bottom), position, rotation)

    def cylinder(self, material, position, radius, height, top=None, segments=8, rotation=None):
        """Centred on `position`; `top` tapers it into a frustum, 0 is a cone."""
        top = radius if top is None else top
        rings = []
        for y, r in ((-height / 2, radius), (height / 2, top)):
            ring = []
            for i in range(segments):
                a = 2 * math.pi * (i + 0.5) / segments
                ring.append((math.cos(a) * r, y, math.sin(a) * r))
            rings.append(ring)
        polygons = []
        for i in range(segments):
            j = (i + 1) % segments
            if top > 1e-6:
                polygons.append([rings[0][i], rings[0][j], rings[1][j], rings[1][i]])
            else:
                polygons.append([rings[0][i], rings[0][j], (0, height / 2, 0)])
        polygons.append(list(reversed(rings[0])))
        if top > 1e-6:
            polygons.append(rings[1])
        self.transformed(material, polygons, position, rotation)

    def cone(self, material, position, radius, height, segments=8, rotation=None):
        self.cylinder(material, position, radius, height, top=0.0, segments=segments, rotation=rotation)

    def sphere(self, material, position, radius, rings=4, segments=8, squash=1.0):
        """A low-poly ball; `squash` flattens it vertically."""
        polygons = []
        points = []
        for r in range(rings + 1):
            phi = math.pi * r / rings
            row = []
            for s in range(segments):
                theta = 2 * math.pi * (s + 0.5 * (r % 2)) / segments
                row.append((math.sin(phi) * math.cos(theta) * radius,
                            math.cos(phi) * radius * squash,
                            math.sin(phi) * math.sin(theta) * radius))
            points.append(row)
        for r in range(rings):
            for s in range(segments):
                n = (s + 1) % segments
                a, b, c, d = points[r][s], points[r][n], points[r + 1][n], points[r + 1][s]
                if r == 0:
                    polygons.append([a, c, d])
                elif r == rings - 1:
                    polygons.append([a, b, d])
                else:
                    polygons.append([a, b, c])
                    polygons.append([a, c, d])
        self.transformed(material, polygons, position)

    def wedge(self, material, position, size, rotation=None):
        """A roof: a triangular prism, its ridge along Z at the top."""
        hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
        a, b, c = (-hx, -hy, -hz), (hx, -hy, -hz), (0, hy, -hz)
        d, e, f = (-hx, -hy, hz), (hx, -hy, hz), (0, hy, hz)
        polygons = [[a, b, c], [d, f, e], [a, d, e, b], [a, c, f, d], [b, e, f, c]]
        self.transformed(material, polygons, position, rotation)

    # --- Writing --------------------------------------------------------------

    def write(self, path, materials, name="model"):
        """Writes a GLB of this mesh; `materials` maps a name to a Material."""
        binary = bytearray()
        accessors, views, primitives, gltf_materials = [], [], [], []
        names = [m for m in self.groups if self.groups[m][2]]
        for index, material_name in enumerate(names):
            positions, normals, indices = self.groups[material_name]
            material = materials[material_name]
            gltf_materials.append({
                "name": material_name,
                "pbrMetallicRoughness": {
                    "baseColorFactor": material.base + [1.0],
                    "metallicFactor": material.metallic,
                    "roughnessFactor": material.roughness,
                },
                "emissiveFactor": [min(1.0, c) for c in material.emissive],
            })
            if max(material.emissive) > 1.0:
                strength = max(material.emissive)
                gltf_materials[-1]["emissiveFactor"] = [c / strength for c in material.emissive]
                gltf_materials[-1].setdefault("extensions", {})["KHR_materials_emissive_strength"] = {
                    "emissiveStrength": strength}
            attributes = {}
            for attribute, data in (("POSITION", positions), ("NORMAL", normals)):
                offset = len(binary)
                for value in data:
                    binary += struct.pack("<3f", *value)
                views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(data) * 12, "target": 34962})
                accessor = {"bufferView": len(views) - 1, "componentType": 5126, "count": len(data), "type": "VEC3"}
                if attribute == "POSITION":
                    accessor["min"] = [min(p[i] for p in data) for i in range(3)]
                    accessor["max"] = [max(p[i] for p in data) for i in range(3)]
                accessors.append(accessor)
                attributes[attribute] = len(accessors) - 1
            offset = len(binary)
            for value in indices:
                binary += struct.pack("<I", value)
            while len(binary) % 4:
                binary += b"\0"
            views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(indices) * 4, "target": 34963})
            accessors.append({"bufferView": len(views) - 1, "componentType": 5125, "count": len(indices), "type": "SCALAR"})
            primitives.append({"attributes": attributes, "indices": len(accessors) - 1, "material": index})
        document = {
            "asset": {"version": "2.0", "generator": "Causeway tools/glb.py"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0, "name": name}],
            "meshes": [{"name": name, "primitives": primitives}],
            "materials": gltf_materials,
            "accessors": accessors,
            "bufferViews": views,
            "buffers": [{"byteLength": len(binary)}],
        }
        if any("extensions" in m for m in gltf_materials):
            document["extensionsUsed"] = ["KHR_materials_emissive_strength"]
        text = json.dumps(document, separators=(",", ":")).encode()
        while len(text) % 4:
            text += b" "
        with open(path, "wb") as out:
            out.write(struct.pack("<4sII", b"glTF", 2, 12 + 8 + len(text) + 8 + len(binary)))
            out.write(struct.pack("<I4s", len(text), b"JSON"))
            out.write(text)
            out.write(struct.pack("<I4s", len(binary), b"BIN\0"))
            out.write(bytes(binary))


def box_polygons(hx, hy, hz, b, bevel_bottom=True):
    """The faces of a box with every edge chamfered by `b`, or a plain box."""
    if b <= 0:
        corners = [(sx * hx, sy * hy, sz * hz) for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
        faces = []
        for axis in range(3):
            for sign in (-1, 1):
                face = [c for c in corners if c[axis] * sign > 0]
                # Order the four corners round the face.
                u, v = [a for a in range(3) if a != axis]
                face.sort(key=lambda p: math.atan2(p[v], p[u]))
                faces.append(face)
        return faces
    polygons = []
    half = (hx, hy, hz)

    def point(sx, sy, sz, on):
        # The corner (sx, sy, sz)'s point lying on the face of axis `on`.
        p = []
        for axis, s in enumerate((sx, sy, sz)):
            h = half[axis]
            p.append(s * (h if axis == on else h - b))
        return tuple(p)

    bottom_b = b if bevel_bottom else 0.0
    signs = (-1, 1)
    for axis in range(3):
        for sign in signs:
            u, v = [a for a in range(3) if a != axis]
            face = []
            for su, sv in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
                s = [0, 0, 0]
                s[axis], s[u], s[v] = sign, su, sv
                face.append(point(*s, axis))
            polygons.append(face)
    for a, c in ((0, 1), (0, 2), (1, 2)):
        other = 3 - a - c
        for sa in signs:
            for sc in signs:
                quad = []
                for so in (-1, 1):
                    s = [0, 0, 0]
                    s[a], s[c], s[other] = sa, sc, so
                    quad.append((tuple(s), a))
                s0, s1 = quad[0][0], quad[1][0]
                polygons.append([point(*s0, a), point(*s1, a), point(*s1, c), point(*s0, c)])
    for sx in signs:
        for sy in signs:
            for sz in signs:
                polygons.append([point(sx, sy, sz, 0), point(sx, sy, sz, 1), point(sx, sy, sz, 2)])
    if not bevel_bottom:
        # Flatten the lower chamfers into the bottom face: a column standing on
        # the ground shows no seam under it.
        flattened = []
        for polygon in polygons:
            flattened.append([(p[0], -hy if p[1] < 0 else p[1], p[2]) for p in polygon])
        polygons = [p for p in flattened if len({(round(q[0], 6), round(q[1], 6), round(q[2], 6)) for q in p}) >= 3]
    return polygons
