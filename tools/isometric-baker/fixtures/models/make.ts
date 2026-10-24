/**
 * Writes the model files the page offers as examples: `node fixtures/models/make.ts`.
 *
 * They are generated rather than drawn so there is nothing to attribute, and so
 * `test/imported.test.ts` can check they are still what the generator makes.
 */

import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

import { barrelParts, buildGlb, crateParts } from '../../test/glb.ts';

const here = fileURLToPath(new URL('./', import.meta.url));
await writeFile(`${here}crate.glb`, buildGlb(crateParts()));
await writeFile(`${here}barrel.glb`, buildGlb(barrelParts()));
