// Builds the baker's web page: node web/build.mjs <out-dir>
//
// The baker is TypeScript that Node runs by stripping types, and a browser
// cannot do that, so this strips them ahead of time with Node's own
// `stripTypeScriptTypes` -- no compiler and no dependencies, in keeping with the
// tool. Every source file is copied to <out>/baker/ as JavaScript with its
// `.ts` imports renamed to `.js`, except two:
//
//   src/cli.ts  is the command line, and reads files;
//   src/png.ts  is replaced by web/png.ts, because it needs `node:zlib`.
//
// web/zip.ts joins them, then the page's own modules and the example recipes
// and models. The page reads model files with Three.js, which is the page's only
// dependency (`web/package.json`, installed with `npm ci --prefix web`): the
// three files of it the page uses are copied to vendor/three/, and an import map
// in index.html points at them. Nothing is bundled, so what ships is what is
// pinned.
import { copyFile, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const tool = fileURLToPath(new URL('..', import.meta.url));

/** Recipes the page offers to start from: the fixtures, and games' own. */
export const EXAMPLES = [
  ['fixtures/project/prefabs/standing-stone.isobake', 'standing-stone.isobake'],
  ['fixtures/models/crate.isobake', 'crate.isobake'],
  ['fixtures/models/barrel.isobake', 'barrel.isobake'],
  ['../../games/orbital-baked/assets/textures/detonation.isobake', 'detonation.isobake'],
  ['../../games/voxel-lab/recipes/block-tops.isobake', 'block-tops.isobake'],
  ['../../games/voxel-lab/recipes/ground-side.isobake', 'ground-side.isobake'],
];

/** The page's own modules, copied as they are. */
const PAGE_FILES = ['index.html', 'app.js', 'controls.js', 'context.js', 'models.js'];

/** The model files those recipes name, served beside them as `examples/models/`. */
export const EXAMPLE_MODELS = ['fixtures/models/crate.glb', 'fixtures/models/barrel.glb'];

/** What the page takes from Three.js, and where it goes under vendor/three/. */
const VENDORED = [
  ['build/three.module.min.js', 'three.module.min.js'],
  ['examples/jsm/loaders/GLTFLoader.js', 'addons/loaders/GLTFLoader.js'],
  ['examples/jsm/loaders/OBJLoader.js', 'addons/loaders/OBJLoader.js'],
  ['examples/jsm/utils/BufferGeometryUtils.js', 'addons/utils/BufferGeometryUtils.js'],
  ['LICENSE', 'LICENSE'],
];

async function vendor(out) {
  const root = join(tool, 'web', 'node_modules', 'three');
  try {
    await readFile(join(root, 'package.json'));
  } catch {
    throw new Error('Three.js is not installed: run `npm ci --prefix tools/isometric-baker/web` first');
  }
  for (const [from, to] of VENDORED) {
    await mkdir(dirname(join(out, 'vendor', 'three', to)), { recursive: true });
    await copyFile(join(root, from), join(out, 'vendor', 'three', to));
  }
}

async function compile(from, to) {
  const source = await readFile(from, 'utf8');
  const stripped = stripTypeScriptTypes(source, { mode: 'strip' });
  const js = stripped.replace(/(from\s+['"]\.{1,2}\/[^'"]*)\.ts(['"])/g, '$1.js$2');
  await writeFile(to, js);
}

export async function build(outDir, { vendor: withVendor = true } = {}) {
  const out = resolve(outDir);
  await rm(out, { recursive: true, force: true });
  await mkdir(join(out, 'baker'), { recursive: true });
  await mkdir(join(out, 'examples'), { recursive: true });

  const replaced = new Set(['cli.ts', 'png.ts']);
  for (const file of await readdir(join(tool, 'src'))) {
    if (!file.endsWith('.ts') || replaced.has(file)) continue;
    await compile(join(tool, 'src', file), join(out, 'baker', file.replace(/\.ts$/, '.js')));
  }
  // web/png.ts reaches back for `../src/image.ts`; in the output that sits beside it.
  for (const file of ['png.ts', 'zip.ts', 'unzip.ts']) {
    const source = await readFile(join(tool, 'web', file), 'utf8');
    const flat = source.replaceAll("'../src/", "'./");
    const stripped = stripTypeScriptTypes(flat, { mode: 'strip' });
    await writeFile(
      join(out, 'baker', file.replace(/\.ts$/, '.js')),
      stripped.replace(/(from\s+['"]\.{1,2}\/[^'"]*)\.ts(['"])/g, '$1.js$2'),
    );
  }

  for (const file of PAGE_FILES) await copyFile(join(tool, 'web', file), join(out, file));
  for (const [from] of EXAMPLES) await copyFile(join(tool, from), join(out, 'examples', basename(from)));
  await mkdir(join(out, 'examples', 'models'), { recursive: true });
  for (const from of EXAMPLE_MODELS) await copyFile(join(tool, from), join(out, 'examples', 'models', basename(from)));
  if (withVendor) await vendor(out);
  await writeFile(
    join(out, 'examples', 'index.json'),
    `${JSON.stringify(EXAMPLES.map(([, name]) => name))}\n`,
  );
  return out;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const target = process.argv[2];
  if (!target) throw new Error('usage: node web/build.mjs <out-dir>');
  console.log(`built the baker page in ${await build(target)}`);
}
