// Reading a model file into triangles: the one place this page uses Three.js.
//
// Three.js is the loader, and only the loader. Its renderer, lights and
// materials are not used: each mesh is flattened to world-space triangles with a
// name and a base color per material, and the baker's own rasteriser does the
// rest. That is what keeps a bake in the page the same pixels as a bake from
// the command line, which a GPU render through WebGL could not promise. Three is
// loaded on first use, so a page that never opens a model never fetches it.
//
// What a file brings is its geometry and each material's base color, which for a
// textured material is the color its texture gives each triangle (read once, at
// the triangle's middle, and cut down to a few colors by the baker). Normal and
// other maps, vertex colors, lights, cameras and animation are ignored, and a
// skinned mesh is baked in its bind pose.
//
// A .gltf is a description that points at other files: a .bin of geometry and
// usually some images. Those are read from `companions`, the other files that
// were chosen or dropped with it, found by name wherever they were in the folder.

export const MODEL_EXTENSIONS = ['glb', 'gltf', 'obj'];

export function extensionOf(name) {
  const dot = name.lastIndexOf('.');
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase();
}

export const isModelFile = (name) => MODEL_EXTENSIONS.includes(extensionOf(name));

/** The loaders callback style, as a promise. */
const parseGltf = (loader, data) => new Promise((resolve, reject) => loader.parse(data, '', resolve, reject));

const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'];
const nameOf = (path) => decodeURIComponent(path.split(/[\\/]/).pop().split('?')[0]).toLowerCase();

/** A transparent pixel, standing in for a texture that was not chosen: colors then come from the material alone. */
const BLANK_IMAGE =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==';

/** Say what a .gltf is missing, in the words a person can act on. */
function missingMessage(file, missing) {
  const list = missing.length <= 3 ? missing.join(', ') : `${missing.slice(0, 3).join(', ')} and ${missing.length - 3} more`;
  return (
    `${file.name} needs ${list}. A .gltf points at other files: choose them together with it ` +
    '(Open folder… takes the whole folder it came in, and so does dropping the folder), or export a .glb, which holds everything in one file.'
  );
}

async function readGltf(THREE, file, companions) {
  const isGlb = extensionOf(file.name) === 'glb';
  const data = isGlb ? await file.arrayBuffer() : await file.text();
  const byName = new Map(companions.map((companion) => [companion.name.toLowerCase(), companion]));

  if (!isGlb) {
    let json;
    try {
      json = JSON.parse(data);
    } catch {
      throw new Error(`${file.name} could not be read (it is not valid glTF JSON)`);
    }
    const external = (list) => (list ?? []).map((entry) => entry.uri).filter((uri) => uri && !uri.startsWith('data:'));
    // Geometry cannot be done without; images only color it, and a missing one is stood in for.
    const missing = external(json.buffers).filter((uri) => !byName.has(nameOf(uri)));
    if (missing.length > 0) throw new Error(missingMessage(file, missing.map((uri) => uri.split('/').pop())));
  }

  const urls = [];
  const manager = new THREE.LoadingManager();
  manager.setURLModifier((url) => {
    if (url.startsWith('data:') || url.startsWith('blob:')) return url;
    const companion = byName.get(nameOf(url));
    if (companion) {
      const blob = URL.createObjectURL(companion);
      urls.push(blob);
      return blob;
    }
    return IMAGE_EXTENSIONS.includes(extensionOf(nameOf(url))) ? BLANK_IMAGE : url;
  });
  const { GLTFLoader } = await import('three/addons/loaders/GLTFLoader.js');
  try {
    return (await parseGltf(new GLTFLoader(manager), data)).scene;
  } catch (error) {
    throw new Error(`${file.name} could not be read (${error?.message ?? error})`);
  } finally {
    // The parse has finished with these once it settles.
    setTimeout(() => urls.forEach((url) => URL.revokeObjectURL(url)), 0);
  }
}

async function readScene(THREE, file, companions) {
  if (extensionOf(file.name) === 'obj') {
    const { OBJLoader } = await import('three/addons/loaders/OBJLoader.js');
    return new OBJLoader().parse(await file.text());
  }
  return readGltf(THREE, file, companions);
}

/** A material's base color as `#rrggbb` in sRGB, which is what a palette is made of. */
function colorOf(THREE, material) {
  return material?.color ? `#${material.color.getHexString(THREE.SRGBColorSpace)}` : '#b0b0b0';
}

const pixelCache = new WeakMap();

/** A texture's pixels as bytes, or null where there is no canvas to read them with. */
function pixelsOf(texture) {
  if (pixelCache.has(texture)) return pixelCache.get(texture);
  let pixels = null;
  const image = texture.image;
  const width = image?.width || image?.naturalWidth;
  const height = image?.height || image?.naturalHeight;
  if (width && height) {
    let canvas = null;
    if (typeof OffscreenCanvas !== 'undefined') canvas = new OffscreenCanvas(width, height);
    else if (typeof document !== 'undefined') Object.assign((canvas = document.createElement('canvas')), { width, height });
    const context = canvas?.getContext('2d', { willReadFrequently: true });
    if (context) {
      try {
        context.drawImage(image, 0, 0);
        pixels = { width, height, data: context.getImageData(0, 0, width, height).data };
      } catch {
        pixels = null;
      }
    }
  }
  pixelCache.set(texture, pixels);
  return pixels;
}

/**
 * The color a texture gives each triangle in a range: its pixel at the middle of
 * the triangle's UVs, times the material's own color. Three bytes per triangle.
 */
function triangleColors(THREE, mesh, material, range) {
  const texture = material?.map;
  const uv = mesh.geometry.getAttribute('uv');
  const pixels = texture && uv ? pixelsOf(texture) : null;
  if (!pixels) return undefined;

  const index = mesh.geometry.getIndex();
  const at = (i) => (index ? index.getX(i) : i);
  const factor = material.color ? material.color.getRGB({ r: 1, g: 1, b: 1 }, THREE.SRGBColorSpace) : { r: 1, g: 1, b: 1 };
  const clampS = texture.wrapS === THREE.ClampToEdgeWrapping;
  const clampT = texture.wrapT === THREE.ClampToEdgeWrapping;
  const wrap = (value, clamp) => (clamp ? Math.min(Math.max(value, 0), 1) : value - Math.floor(value));
  const colors = new Uint8Array(range.count);
  for (let t = 0; t < range.count / 3; t++) {
    let u = 0;
    let v = 0;
    for (let corner = 0; corner < 3; corner++) {
      const vertex = at(range.start + t * 3 + corner);
      u += uv.getX(vertex) / 3;
      v += uv.getY(vertex) / 3;
    }
    u = wrap(u, clampS);
    v = wrap(texture.flipY ? 1 - v : v, clampT);
    const x = Math.min(pixels.width - 1, Math.floor(u * pixels.width));
    const y = Math.min(pixels.height - 1, Math.floor(v * pixels.height));
    const from = (y * pixels.width + x) * 4;
    colors[t * 3] = Math.round(pixels.data[from] * factor.r);
    colors[t * 3 + 1] = Math.round(pixels.data[from + 1] * factor.g);
    colors[t * 3 + 2] = Math.round(pixels.data[from + 2] * factor.b);
  }
  return colors;
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
    colors: triangleColors(THREE, mesh, material, range),
  };
}

/** Every triangle in a file, as the baker's `RawModel`. */
export async function readModelFile(file, { companions = [] } = {}) {
  const THREE = await import('three');
  const scene = await readScene(THREE, file, companions);
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
