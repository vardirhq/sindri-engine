/**
 * Models that come from a file.
 *
 * A recipe's `"kind": "file"` model names a `.glb`, `.gltf` or `.obj` made in
 * Blender or anything else. Reading those formats is somebody else's job -- the
 * browser page does it with Three.js, which is the one dependency this tool
 * has and only the page carries it -- so this module starts from what every
 * format boils down to: triangles, each with a name and a base color.
 *
 * That split is deliberate. Everything here is plain arithmetic, so a model
 * goes through the same rasteriser, shading and palette as a model built from
 * primitives and bakes the same pixels in a browser as under Node. The file's
 * own lighting and textures are not used: a material contributes its base
 * color, which is turned into the same four-shade ramp a hand-written
 * material gets. A render downsampled to a few dozen pixels looks like mud;
 * four bands do not.
 *
 * The placement conventions follow IsoGame's Sprite Factory loader (MIT), from
 * which this baker was adapted: the model is centred on its footprint in X/Z
 * and stood on the floor, so anchors work whatever origin the file had.
 */

import { type Footprint } from './frames.ts';
import { type Mesh, type ModelSpec } from './model.ts';
import { type BandedMaterial, type MaterialSpec, bandMaterial } from './shading.ts';
import { IDENTITY, type Vec3, cross, normalize, rotationXYZ, sub, transform, vec } from './vec.ts';

/** One run of triangles that share a material, in world space and Y up. */
export interface RawPart {
  /** The material's name in the file; what a recipe overrides it by. May be empty. */
  name: string;
  /** Its base color as `#rrggbb`, in sRGB. */
  color: string;
  /** Three numbers per vertex. */
  positions: ArrayLike<number>;
  /** Three per vertex, when the file has them. Without, faces are flat shaded. */
  normals?: ArrayLike<number>;
  /** Three vertex indices per triangle. */
  indices: ArrayLike<number>;
}

export interface RawModel {
  parts: RawPart[];
}

/** A model read from a file, and how to place it. */
export interface FileModelSpec {
  kind: 'file';
  /** Path as the recipe wrote it. It names the model; the caller supplies the bytes. */
  file: string;
  /** Scale the model so it fills `fill` of the footprint. */
  autoFit: boolean;
  /** With `autoFit`, the fraction of the footprint to fill, above zero and up to one. */
  fill: number;
  /** Extra uniform scale, applied after fitting. */
  scale: number;
  /** Degrees about +Y, for a file not authored facing +X. */
  rotationY: number;
  /** Nudge in tile units, applied last. */
  offset: Vec3;
  /** Replace a material's color (or whole ramp) by the name it has in the file. */
  materials: Record<string, MaterialSpec>;
}

export type AnyModelSpec = ModelSpec | FileModelSpec;

export function isFileModel(spec: AnyModelSpec): spec is FileModelSpec {
  return 'file' in spec;
}

/** The extensions a model file may have. */
export const MODEL_EXTENSIONS = ['glb', 'gltf', 'obj'] as const;

/** The models a bake was handed, by the `file` a recipe names them with. */
export type ModelFiles = Record<string, RawModel>;

interface Gathered {
  positions: number[];
  normals: number[];
  indices: number[];
  triangleMaterial: number[];
  materials: BandedMaterial[];
  palette: string[];
  ramps: Record<string, string[]>;
}

/** Flat normals for a part that came without, which means unwelding its vertices. */
function unweld(part: RawPart): { positions: number[]; normals: number[]; indices: number[] } {
  const positions: number[] = [];
  const normals: number[] = [];
  const indices: number[] = [];
  const at = (vertex: number): Vec3 =>
    vec(part.positions[vertex * 3], part.positions[vertex * 3 + 1], part.positions[vertex * 3 + 2]);
  for (let t = 0; t + 2 < part.indices.length; t += 3) {
    const [a, b, c] = [at(part.indices[t]), at(part.indices[t + 1]), at(part.indices[t + 2])];
    const face = normalize(cross(sub(b, a), sub(c, a)));
    for (const corner of [a, b, c]) {
      indices.push(positions.length / 3);
      positions.push(corner[0], corner[1], corner[2]);
      normals.push(face[0], face[1], face[2]);
    }
  }
  return { positions, normals, indices };
}

function gather(raw: RawModel, spec: FileModelSpec): Gathered {
  const out: Gathered = {
    positions: [],
    normals: [],
    indices: [],
    triangleMaterial: [],
    materials: [],
    palette: [],
    ramps: {},
  };
  const rotation = spec.rotationY ? rotationXYZ(vec(0, spec.rotationY, 0)) : IDENTITY;
  const cache = new Map<string, number>();

  for (const part of raw.parts) {
    const key = `${part.name}|${part.color.toLowerCase()}`;
    let material = cache.get(key);
    if (material === undefined) {
      const banded = bandMaterial(spec.materials[part.name] ?? { color: part.color });
      material = out.materials.length;
      out.materials.push(banded);
      out.palette.push(...banded.ramp);
      out.ramps[part.name || part.color] = banded.ramp;
      cache.set(key, material);
    }

    const welded = part.normals !== undefined && part.normals.length === part.positions.length;
    const source = welded
      ? { positions: part.positions, normals: part.normals as ArrayLike<number>, indices: part.indices }
      : unweld(part);
    const base = out.positions.length / 3;
    for (let i = 0; i + 2 < source.positions.length; i += 3) {
      const p = transform(rotation, vec(source.positions[i], source.positions[i + 1], source.positions[i + 2]));
      const n = transform(rotation, vec(source.normals[i], source.normals[i + 1], source.normals[i + 2]));
      out.positions.push(p[0], p[1], p[2]);
      out.normals.push(n[0], n[1], n[2]);
    }
    for (let t = 0; t + 2 < source.indices.length; t += 3) {
      out.indices.push(base + source.indices[t], base + source.indices[t + 1], base + source.indices[t + 2]);
      out.triangleMaterial.push(material);
    }
  }
  return out;
}

/**
 * Turn a file's triangles into the mesh a bake rasterises.
 *
 * Order matters and follows the Sprite Factory: turn, fit, scale, then centre
 * on the footprint, stand on the floor and nudge. Fitting and centring are
 * measured on the turned model, so a model turned a quarter fills the
 * footprint the way it will be seen.
 */
export function meshFromRaw(raw: RawModel, spec: FileModelSpec, footprint: Footprint): Mesh {
  const gathered = gather(raw, spec);
  if (gathered.triangleMaterial.length === 0) {
    throw new Error(`${spec.file} has no triangles to bake`);
  }
  if (gathered.materials.length > 0xffff) {
    throw new Error(`a model may use at most 65536 materials, got ${gathered.materials.length}`);
  }

  const { positions } = gathered;
  const low = [Infinity, Infinity, Infinity];
  const high = [-Infinity, -Infinity, -Infinity];
  for (let i = 0; i < positions.length; i += 3) {
    for (let axis = 0; axis < 3; axis++) {
      low[axis] = Math.min(low[axis], positions[i + axis]);
      high[axis] = Math.max(high[axis], positions[i + axis]);
    }
  }

  let scale = spec.scale;
  if (spec.autoFit) {
    const fit = Math.min(
      high[0] > low[0] ? (footprint.width * spec.fill) / (high[0] - low[0]) : Infinity,
      high[2] > low[2] ? (footprint.height * spec.fill) / (high[2] - low[2]) : Infinity,
    );
    if (Number.isFinite(fit) && fit > 0) scale *= fit;
  }

  const centreX = (low[0] + high[0]) / 2;
  const centreZ = (low[2] + high[2]) / 2;
  for (let i = 0; i < positions.length; i += 3) {
    positions[i] = (positions[i] - centreX) * scale + spec.offset[0];
    positions[i + 1] = (positions[i + 1] - low[1]) * scale + spec.offset[1];
    positions[i + 2] = (positions[i + 2] - centreZ) * scale + spec.offset[2];
  }

  return {
    positions: Float64Array.from(positions),
    normals: Float64Array.from(gathered.normals),
    indices: Uint32Array.from(gathered.indices),
    triangleMaterial: Uint16Array.from(gathered.triangleMaterial),
    materials: gathered.materials,
    palette: gathered.palette,
    ramps: gathered.ramps,
  };
}
