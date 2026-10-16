"""Regenerate the tiny static model fixture using only Python's standard library."""
import json
from pathlib import Path
import struct
import zlib

binary = bytearray()
views = []
accessors = []

def view(data):
    binary.extend(b'\0' * (-len(binary) % 4))
    index = len(views)
    views.append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': len(data)})
    binary.extend(data)
    return index

def accessor(data, component, kind, **extra):
    index = len(accessors)
    accessors.append({'bufferView': view(data), 'componentType': component,
                      'count': 3, 'type': kind, **extra})
    return index

positions = accessor(struct.pack('<9f', 0, 0, 0, 1, 0, 0, 0, 1, 0), 5126,
                     'VEC3', min=[0, 0, 0], max=[1, 1, 0])
normals = accessor(struct.pack('<9f', 0, 0, 1, 0, 0, 1, 0, 0, 1), 5126, 'VEC3')
uvs = accessor(struct.pack('<6f', 0, 0, 1, 0, 0, 1), 5126, 'VEC2')
short = accessor(struct.pack('<3H', 0, 1, 2), 5123, 'SCALAR')
long = accessor(struct.pack('<3I', 2, 1, 0), 5125, 'SCALAR')

def png_chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
png = b'\x89PNG\r\n\x1a\n' + png_chunk(b'IHDR', struct.pack('>2I5B', 1, 1, 8, 6, 0, 0, 0))
png += png_chunk(b'IDAT', zlib.compress(b'\x00\xff\x80\x00\xff')) + png_chunk(b'IEND', b'')
image = view(png)
attributes = {'POSITION': positions, 'NORMAL': normals, 'TEXCOORD_0': uvs}
document = {
    'asset': {'version': '2.0', 'generator': 'Sindri deterministic model fixture'},
    'scene': 0, 'scenes': [{'nodes': [0]}],
    'nodes': [
        {'name': 'root', 'translation': [1, 2, 3], 'children': [1, 2]},
        {'name': 'child', 'translation': [0, 1, 0], 'scale': [2, 3, 4], 'mesh': 0},
        {'name': 'reused', 'mesh': 0, 'children': [3]},
        {'name': 'non-indexed', 'mesh': 1},
    ],
    'meshes': [
        {'name': 'two-primitives', 'primitives': [
            {'attributes': attributes, 'indices': short, 'material': 0},
            {'attributes': attributes, 'indices': long, 'material': 1},
        ]},
        {'primitives': [{'attributes': {'POSITION': positions}, 'material': 0}]},
    ],
    'materials': [
        {'name': 'orange', 'pbrMetallicRoughness': {'baseColorFactor': [1, 0.5, 0, 1], 'metallicFactor': 0.3, 'roughnessFactor': 0.7}},
        {'name': 'textured', 'pbrMetallicRoughness': {'baseColorTexture': {'index': 0}}},
    ],
    'images': [{'bufferView': image, 'mimeType': 'image/png'}],
    'textures': [{'source': 0}],
    'bufferViews': views, 'accessors': accessors,
    'buffers': [{'byteLength': len(binary)}],
}
encoded = json.dumps(document, separators=(',', ':'), sort_keys=True).encode()
encoded += b' ' * (-len(encoded) % 4)
binary.extend(b'\0' * (-len(binary) % 4))
output = struct.pack('<III', 0x46546c67, 2, 12 + 8 + len(encoded) + 8 + len(binary))
output += struct.pack('<II', len(encoded), 0x4e4f534a) + encoded
output += struct.pack('<II', len(binary), 0x004e4942) + binary
Path(__file__).with_name('static-model.glb').write_bytes(output)
