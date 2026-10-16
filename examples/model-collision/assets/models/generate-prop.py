"""Reproduce a small volumetric GLB prop with shared indexed cube vertices."""
import json
from pathlib import Path
import struct

points = [(x, y, z) for x in (-0.5, 0.5) for y in (-0.5, 0.5) for z in (-0.5, 0.5)]
indices = [0, 1, 3, 0, 3, 2, 4, 6, 7, 4, 7, 5,
           0, 4, 5, 0, 5, 1, 2, 3, 7, 2, 7, 6,
           0, 2, 6, 0, 6, 4, 1, 5, 7, 1, 7, 3]
blob = b''.join(struct.pack('<3f', *p) for p in points) + struct.pack('<36H', *indices)
document = {'asset': {'version': '2.0'}, 'scene': 0,
            'scenes': [{'nodes': [0]}], 'nodes': [{'name': 'Prop', 'mesh': 0}],
            'buffers': [{'byteLength': len(blob)}],
            'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': 96},
                            {'buffer': 0, 'byteOffset': 96, 'byteLength': 72}],
            'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 8,
                           'type': 'VEC3', 'min': [-0.5] * 3, 'max': [0.5] * 3},
                          {'bufferView': 1, 'componentType': 5123, 'count': 36, 'type': 'SCALAR'}],
            'materials': [{'pbrMetallicRoughness': {'baseColorFactor': [0.4, 0.12, 0.8, 1],
                            'metallicFactor': 0, 'roughnessFactor': 1}}],
            'meshes': [{'primitives': [{'attributes': {'POSITION': 0}, 'indices': 1, 'material': 0}]}]}
encoded = json.dumps(document, separators=(',', ':')).encode()
encoded += b' ' * (-len(encoded) % 4)
body = struct.pack('<II', len(encoded), 0x4E4F534A) + encoded
body += struct.pack('<II', len(blob), 0x004E4942) + blob
Path(__file__).with_name('prop.glb').write_bytes(struct.pack('<III', 0x46546C67, 2, len(body) + 12) + body)
