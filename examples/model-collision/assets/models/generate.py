"""Reproduce the lab's static GLB: a ground plane and a vertical wall."""
import json
from pathlib import Path
import struct

vertices = [(-6, 0, -4), (6, 0, -4), (6, 0, 4), (-6, 0, 4),
            (0, 0, -1), (0, 2, -1), (0, 2, 1), (0, 0, 1)]
normals = [(0, 1, 0)] * 4 + [(-1, 0, 0)] * 4
indices = [0, 2, 1, 0, 3, 2, 0, 2, 1, 0, 3, 2]
blob = b''.join(struct.pack('<3f', *v) for v in vertices + normals)
blob += struct.pack('<12H', *indices)
views = [{'buffer': 0, 'byteOffset': offset, 'byteLength': length}
         for offset, length in [(0, 96), (96, 96), (192, 24)]]
accessors = []
for offset in [0, 48]:
    points = vertices[offset // 12:offset // 12 + 4]
    accessors.append({'bufferView': 0, 'byteOffset': offset, 'componentType': 5126,
                      'count': 4, 'type': 'VEC3',
                      'min': [min(p[i] for p in points) for i in range(3)],
                      'max': [max(p[i] for p in points) for i in range(3)]})
for offset in [0, 48]:
    accessors.append({'bufferView': 1, 'byteOffset': offset, 'componentType': 5126,
                      'count': 4, 'type': 'VEC3'})
for offset in [0, 12]:
    accessors.append({'bufferView': 2, 'byteOffset': offset, 'componentType': 5123,
                      'count': 6, 'type': 'SCALAR'})
document = {'asset': {'version': '2.0'}, 'scene': 0,
            'scenes': [{'nodes': [0]}], 'nodes': [{'name': 'Level', 'mesh': 0}],
            'buffers': [{'byteLength': len(blob)}], 'bufferViews': views,
            'accessors': accessors,
            'materials': [{'doubleSided': True, 'pbrMetallicRoughness': {
                'baseColorFactor': color, 'metallicFactor': 0, 'roughnessFactor': 1}}
                for color in [[0.12, 0.4, 0.24, 1], [0.8, 0.24, 0.07, 1]]],
            'meshes': [{'primitives': [
                {'attributes': {'POSITION': i, 'NORMAL': i + 2},
                 'indices': i + 4, 'material': i} for i in range(2)]}]}
encoded = json.dumps(document, separators=(',', ':')).encode()
encoded += b' ' * (-len(encoded) % 4)
body = struct.pack('<II', len(encoded), 0x4E4F534A) + encoded
body += struct.pack('<II', len(blob), 0x004E4942) + blob
Path(__file__).with_name('level.glb').write_bytes(
    struct.pack('<III', 0x46546C67, 2, len(body) + 12) + body)
