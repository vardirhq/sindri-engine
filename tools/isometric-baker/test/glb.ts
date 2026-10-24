/**
 * Small models, and the `.glb` files they are written as.
 *
 * The baker reads a model's triangles from whatever the page's loader makes of
 * a file, so the tests that exercise placement build triangles directly, and
 * the browser test needs real files for the loader to read. Both come from
 * here, so the file a person can try on the page is the model the unit tests
 * measure. `fixtures/models/make.ts` writes them; `test/imported.test.ts`
 * checks the committed files are still what this makes.
 *
 * Sizes are deliberately not in tile units: a crate made in metres is 1.6
 * across, which is what an auto-fitting recipe is for.
 */

import { type RawPart } from '../src/imported.ts';

type Triple = [number, number, number];

/** An axis-aligned box with flat faces, as a part. */
export function boxPart(name: string, color: string, size: Triple, centre: Triple): RawPart {
  const [sx, sy, sz] = size.map((v) => v / 2);
  const faces: { normal: Triple; corners: Triple[] }[] = [
    { normal: [1, 0, 0], corners: [[1, -1, -1], [1, 1, -1], [1, 1, 1], [1, -1, 1]] },
    { normal: [-1, 0, 0], corners: [[-1, -1, 1], [-1, 1, 1], [-1, 1, -1], [-1, -1, -1]] },
    { normal: [0, 1, 0], corners: [[-1, 1, -1], [-1, 1, 1], [1, 1, 1], [1, 1, -1]] },
    { normal: [0, -1, 0], corners: [[-1, -1, 1], [-1, -1, -1], [1, -1, -1], [1, -1, 1]] },
    { normal: [0, 0, 1], corners: [[-1, -1, 1], [1, -1, 1], [1, 1, 1], [-1, 1, 1]] },
    { normal: [0, 0, -1], corners: [[1, -1, -1], [-1, -1, -1], [-1, 1, -1], [1, 1, -1]] },
  ];
  const positions: number[] = [];
  const normals: number[] = [];
  const indices: number[] = [];
  for (const face of faces) {
    const base = positions.length / 3;
    for (const [x, y, z] of face.corners) {
      positions.push(centre[0] + x * sx, centre[1] + y * sy, centre[2] + z * sz);
      normals.push(...face.normal);
    }
    indices.push(base, base + 1, base + 2, base, base + 2, base + 3);
  }
  return { name, color, positions, normals, indices };
}

/** An upright cylinder with smooth sides, as a part. */
export function cylinderPart(
  name: string,
  color: string,
  radius: number,
  height: number,
  centre: Triple,
  segments = 16,
): RawPart {
  const positions: number[] = [];
  const normals: number[] = [];
  const indices: number[] = [];
  const bottom = centre[1] - height / 2;
  const top = centre[1] + height / 2;
  for (let s = 0; s <= segments; s++) {
    const angle = (s / segments) * Math.PI * 2;
    const [c, n] = [Math.cos(angle), Math.sin(angle)];
    positions.push(centre[0] + c * radius, bottom, centre[2] + n * radius);
    normals.push(c, 0, n);
    positions.push(centre[0] + c * radius, top, centre[2] + n * radius);
    normals.push(c, 0, n);
  }
  for (let s = 0; s < segments; s++) {
    const a = s * 2;
    indices.push(a, a + 1, a + 3, a, a + 3, a + 2);
  }
  // Flat caps, each with its own vertices so the rim stays crisp.
  for (const [y, ny] of [[top, 1], [bottom, -1]] as const) {
    const middle = positions.length / 3;
    positions.push(centre[0], y, centre[2]);
    normals.push(0, ny, 0);
    for (let s = 0; s <= segments; s++) {
      const angle = (s / segments) * Math.PI * 2;
      positions.push(centre[0] + Math.cos(angle) * radius, y, centre[2] + Math.sin(angle) * radius);
      normals.push(0, ny, 0);
    }
    for (let s = 0; s < segments; s++) {
      const [a, b] = [middle + 1 + s, middle + 2 + s];
      if (ny > 0) indices.push(middle, b, a);
      else indices.push(middle, a, b);
    }
  }
  return { name, color, positions, normals, indices };
}

/** A wooden crate with a metal lid and base, 1.6 across: made in metres. */
export function crateParts(): RawPart[] {
  return [
    boxPart('Wood', '#b07a42', [1.5, 1.3, 1.5], [0, 0.75, 0]),
    boxPart('Metal', '#6b7280', [1.6, 0.14, 1.6], [0, 1.47, 0]),
    boxPart('Metal', '#6b7280', [1.6, 0.14, 1.6], [0, 0.07, 0]),
  ];
}

/** A barrel with two hoops. The sides are smooth, which is what terraces under banded light. */
export function barrelParts(): RawPart[] {
  return [
    cylinderPart('Staves', '#9a6a3a', 0.55, 1.5, [0, 0.75, 0], 20),
    cylinderPart('Hoop', '#444b55', 0.58, 0.1, [0, 0.35, 0], 20),
    cylinderPart('Hoop', '#444b55', 0.58, 0.1, [0, 1.15, 0], 20),
  ];
}

/** sRGB `#rrggbb` to the linear factor glTF stores. */
function linear(hex: string): number[] {
  const channel = (offset: number) => {
    const c = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return [channel(1), channel(3), channel(5), 1];
}

/** Write parts as a binary glTF 2.0 file: one mesh, one primitive per part, one material each. */
export function buildGlb(parts: RawPart[]): Uint8Array {
  const chunks: Uint8Array[] = [];
  const bufferViews: object[] = [];
  const accessors: object[] = [];
  let length = 0;

  const add = (bytes: Uint8Array, target: number): number => {
    const padded = new Uint8Array(Math.ceil(bytes.length / 4) * 4);
    padded.set(bytes);
    bufferViews.push({ buffer: 0, byteOffset: length, byteLength: bytes.length, target });
    chunks.push(padded);
    length += padded.length;
    return bufferViews.length - 1;
  };

  const primitives = parts.map((part, material) => {
    const positions = Float32Array.from(part.positions);
    const normals = Float32Array.from(part.normals ?? []);
    const indices = Uint16Array.from(part.indices);
    const lows = [0, 1, 2].map((axis) => Math.min(...positions.filter((_, i) => i % 3 === axis)));
    const highs = [0, 1, 2].map((axis) => Math.max(...positions.filter((_, i) => i % 3 === axis)));
    accessors.push({
      bufferView: add(new Uint8Array(positions.buffer), 34962),
      componentType: 5126,
      count: positions.length / 3,
      type: 'VEC3',
      min: lows,
      max: highs,
    });
    accessors.push({
      bufferView: add(new Uint8Array(normals.buffer), 34962),
      componentType: 5126,
      count: normals.length / 3,
      type: 'VEC3',
    });
    accessors.push({
      bufferView: add(new Uint8Array(indices.buffer), 34963),
      componentType: 5123,
      count: indices.length,
      type: 'SCALAR',
    });
    const first = accessors.length - 3;
    return { attributes: { POSITION: first, NORMAL: first + 1 }, indices: first + 2, material };
  });

  const json = JSON.stringify({
    asset: { version: '2.0', generator: 'sindri isometric baker test fixtures' },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ mesh: 0 }],
    meshes: [{ primitives }],
    materials: parts.map((part) => ({
      name: part.name,
      pbrMetallicRoughness: { baseColorFactor: linear(part.color), metallicFactor: 0, roughnessFactor: 1 },
    })),
    accessors,
    bufferViews,
    buffers: [{ byteLength: length }],
  });
  const jsonBytes = new TextEncoder().encode(json.padEnd(Math.ceil(json.length / 4) * 4, ' '));

  const out = new Uint8Array(12 + 8 + jsonBytes.length + 8 + length);
  const view = new DataView(out.buffer);
  view.setUint32(0, 0x46546c67, true); // "glTF"
  view.setUint32(4, 2, true);
  view.setUint32(8, out.length, true);
  view.setUint32(12, jsonBytes.length, true);
  view.setUint32(16, 0x4e4f534a, true); // "JSON"
  out.set(jsonBytes, 20);
  const binary = 20 + jsonBytes.length;
  view.setUint32(binary, length, true);
  view.setUint32(binary + 4, 0x004e4942, true); // "BIN\0"
  let at = binary + 8;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.length;
  }
  return out;
}
