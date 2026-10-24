// Reading a model file into triangles: the one place this page uses Three.js.
//
// Three.js is the loader, and only the loader. Its renderer, lights and
// materials are not used: each mesh is flattened to world-space triangles with a
// name and a base color per material, and the baker's own rasteriser does the
// rest. That is what keeps a bake in the page the same pixels as a bake from
// the command line, which a GPU render through WebGL could not promise. Three is
// loaded on first use, so a page that never opens a model never fetches it.
//
// What a file brings is its geometry and each material's base color. Textures,
// vertex colors, lights, cameras and animation are ignored, and a skinned mesh
// is baked in its bind pose.

export const MODEL_EXTENSIONS = ['glb', 'gltf', 'obj'];

export function extensionOf(name) {
  const dot = name.lastIndexOf('.');
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase();
}

export const isModelFile = (name) => MODEL_EXTENSIONS.includes(extensionOf(name));

/** The loaders callback style, as a promise. */
const parseGltf = (loader, buffer) => new Promise((resolve, reject) => loader.parse(buffer, '', resolve, reject));

async function readScene(THREE, file) {
  const extension = extensionOf(file.name);
  if (extension === 'obj') {
    const { OBJLoader } = await import('three/addons/loaders/OBJLoader.js');
    return new OBJLoader().parse(await file.text());
  }
  const { GLTFLoader } = await import('three/addons/loaders/GLTFLoader.js');
  try {
    return (await parseGltf(new GLTFLoader(), await file.arrayBuffer())).scene;
  } catch (error) {
    const reason = error?.message ?? String(error);
    throw new Error(
      extension === 'gltf'
        ? `${file.name} could not be read (${reason}). A .gltf that points at separate .bin or image files cannot be read from one dropped file; export it as .glb or with the buffers embedded.`
        : `${file.name} could not be read (${reason})`,
    );
  }
}

/** A material's base color as `#rrggbb` in sRGB, which is what a palette is made of. */
function colorOf(THREE, material) {
  return material?.color ? `#${material.color.getHexString(THREE.SRGBColorSpace)}` : '#b0b0b0';
}

/**
 * One range of a mesh's triangles, compacted to the vertices it uses and moved
 * into world space.
 */
function partOf(THREE, mesh, material, name, range) {
  const geometry = mesh.geometry;
  const position = geometry.getAttribute('position');
  const normal = geometry.getAttribute('normal');
  const index = geometry.getIndex();
  const matrix = mesh.matrixWorld;
  const normalMatrix = new THREE.Matrix3().getNormalMatrix(matrix);

  const remap = new Map();
  const positions = [];
  const normals = [];
  const indices = [];
  const point = new THREE.Vector3();
  const vector = new THREE.Vector3();
  for (let at = range.start; at < range.start + range.count; at++) {
    const source = index ? index.getX(at) : at;
    let mapped = remap.get(source);
    if (mapped === undefined) {
      mapped = positions.length / 3;
      remap.set(source, mapped);
      point.fromBufferAttribute(position, source).applyMatrix4(matrix);
      positions.push(point.x, point.y, point.z);
      if (normal) {
        vector.fromBufferAttribute(normal, source).applyMatrix3(normalMatrix).normalize();
        normals.push(vector.x, vector.y, vector.z);
      }
    }
    indices.push(mapped);
  }
  // A mirrored node (negative scale) reverses winding, and with it the faces' outward direction.
  if (matrix.determinant() < 0) {
    for (let t = 0; t + 2 < indices.length; t += 3) [indices[t + 1], indices[t + 2]] = [indices[t + 2], indices[t + 1]];
  }
  return {
    name,
    color: colorOf(THREE, material),
    positions: Float32Array.from(positions),
    normals: normal ? Float32Array.from(normals) : undefined,
    indices: Uint32Array.from(indices),
  };
}

/** Every triangle in a file, as the baker's `RawModel`. */
export async function readModelFile(file) {
  const THREE = await import('three');
  const scene = await readScene(THREE, file);
  scene.updateMatrixWorld(true);

  const parts = [];
  let unnamed = 0;
  scene.traverse((node) => {
    if (!node.isMesh || !node.geometry?.getAttribute('position')) return;
    const materials = Array.isArray(node.material) ? node.material : [node.material];
    const total = node.geometry.getIndex() ? node.geometry.getIndex().count : node.geometry.getAttribute('position').count;
    const groups = node.geometry.groups.length > 0 ? node.geometry.groups : [{ start: 0, count: total, materialIndex: 0 }];
    for (const group of groups) {
      const material = materials[group.materialIndex] ?? materials[0];
      const name = material?.name || `Material ${++unnamed}`;
      const count = Math.min(group.count, total - group.start);
      if (count >= 3) parts.push(partOf(THREE, node, material, name, { start: group.start, count }));
    }
  });
  if (parts.length === 0) throw new Error(`${file.name} has no triangles to bake`);
  return { parts };
}
