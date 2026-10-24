# Attribution

This tool is adapted from the **IsoGame Sprite Factory**:

- Project: <https://github.com/MadsenDev/isogame>
- Tool: <https://github.com/MadsenDev/isogame/tree/main/tools/sprite-factory>
- Licence: MIT, as declared by that project's `package.json`. The repository
  carries no separate `LICENSE` file at the time this was ported, so that
  declaration is the whole of the licence statement available.

The port is described file by file in `README.md`. In short: the isometric
camera, the shade-ramp palette, the banded material, the post-processing chain
(downsample, alpha threshold, palette snap, inner outline) and the compass
direction scheme are IsoGame's design, carried over with their reasoning intact.
The rasteriser, the recipe document, the anchor-aligned frame layout and the
Sindri sheet output are new here.

Model files follow the same source. The placement rules for an imported model
in `src/imported.ts` (turn, fit to the footprint, scale, centre on the footprint,
stand on the floor, nudge, base color to a four-shade ramp, override by material
name) are those of the Sprite Factory's `loaders.ts`; its use of Three.js's
`GLTFLoader` and `OBJLoader` to read the files is kept in the browser page only
(`web/models.js`).

## Third-party code the page ships

The page loads **three.js** (<https://threejs.org>, MIT, pinned in
`web/package.json` and `web/package-lock.json`) to read `.glb`, `.gltf` and
`.obj` files. `web/build.mjs` copies the module, its two loaders, one utility
they import and its licence into the site's `vendor/three/`. The baker itself
has no dependencies, and nothing is bundled.

Nothing IsoGame-specific was carried across: its furniture categories, its
sit/lay/use interaction spots, its stackability and collision model, and its
character layer system all describe that game, and would be another game's
vocabulary in a Sindri asset format.
