# Standard material factors

This is the numeric foundation of the planned Explorer material track, with
native/browser feature-control and GPU evidence. Game adoption and the complete
shared material asset/texture/alpha contract remain open. Face visibility has
an independent shared override, described below.

`sindri-render::MaterialFactors` owns linear `base_color` RGBA, `metallic`,
`roughness`, and linear RGB `emissive`. Base color, metallic and roughness must
be finite in 0–1; emission must be finite and nonnegative and may exceed one.
A typed `MaterialError` names the invalid factor. Frame preparation checks
per-pass factors before GPU work; the renderer setter also checks before
replacing its current selection. Roughness is bounded below by 0.045 in the
GGX shader to avoid a singular highlight.

`material.wgsl` supplies one direct GGX response with the existing ambient
approximation for cubes, inline meshes, cached geometry (including voxels) and
imported model primitives. Directional shadows attenuate direct light; emission
lights the surface itself, then shared fog is applied. Emission is independent
of ambient and directional intensity. It does not illuminate neighbors; local
lights remain separate work. HDR emission reaches the target, so an 8-bit target
clips above one; authored bloom/post delivery in shipped players remains open.
There is no environment-map lighting or normal/metallic/roughness/emissive map.

## Scene authoring

Attach `sindri.material` to the entity owning a mesh, model or voxel world:

```json
"sindri.material": {
  "base_color": [0.85, 0.5, 0.08, 1.0],
  "metallic": 1.0,
  "roughness": 0.2,
  "emissive": [0.0, 0.0, 0.0]
}
```

Missing fields use white base color, nonmetallic, roughness one and zero
emission. These are absolute factors, not multipliers on imported primitive
factors. A model override replaces factors across all its primitives while
keeping their embedded textures, alpha masks, hierarchy and shared
geometry. Remove the component to restore imported factors. Alpha factors
participate in imported masks in both color and shadow draws. Opaque imported
materials continue ignoring alpha for coverage. Missing `sidedness` preserves
imported face visibility. This component does not add blended transparency or
unify geometry-owned alpha policy.

Without the component, existing textured world geometry retains its Lambert
appearance; imported models retain their authored factors and existing GGX
response. Opting in selects the common standard path. Migration of legacy
looks, shared texture/material assets and per-node model overrides remain pending.
Factors apply to the owning entity, not its children or 2D sprites/UI.

The editor's Rendering family offers Material with checked generic controls:
base-color picker, metallic/roughness numbers and numeric RGB emission (the
vector controls use X/Y/Z labels). Fields serialize through the normal checked
commands and undo history. Emission uses numbers so HDR values remain authorable.
Voxel material edits change uniforms without re-meshing resident sections.

## Evidence

`standard_materials.rs` reads back equivalent cube, inline, cached and imported
surfaces, compares common factors within two byte values, checks metallic and
roughness changes, self-emission with zero world lighting, and restoration of
legacy/imported factors. Geometry stays cached. Imported-model tests change
material alpha across frames and check color/masked shadows without uploads.
Frame tests reject nonfinite factors; scene tests reject invalid documents,
extract per-entity overrides while retaining shared assets, undo removal and
verify that voxel edits do not rebuild geometry. The generated catalog records
the schema/defaults and base-color meaning.

`prepare-standard-materials.py` derives six authored swatches from the checked
3D Benchmark and reuses its script/ready goal. Native and rebuilt Chromium
controls compare authored factors with absent overrides, including visible blue
self-emission. CI retains both pairs. At 960×540, native controls change 46,877
pixels and add 7,005 emissive-blue pixels; Chromium changes 46,873 and adds 6,990.
Captures were visually inspected. These are correctness checks, not performance
or game-completion evidence.

A native editor review on X11/software Vulkan selects Swatch-0, inspects Material,
edits roughness 1 → 0.25, saves, uses toolbar Undo to restore 1 and Redo to restore
0.25, and saves/reopens with 0.25 retained. Entering 2 is rejected and the saved
value remains 0.25. The Scene view shows the gold swatches, imported highlight
and blue emitting model. Local artifacts are `target/standard-material-inspector.png`,
`target/standard-material-editor-undo-redo.png`,
`target/standard-material-editor-invalid.png` and
`target/standard-material-editor-reopened.png`.

Color uniforms are now 368 bytes for textured geometry and 416 for models;
shadow uniforms remain 64/96. The fixed 300-prop workload remains 602/301 draws
and 603/301 passes with shadows on/off, with 138,032/115,568 uniform bytes.

## Imported emission

Static GLBs retain their authored core `emissiveFactor` through CPU decoding,
facade preparation, shared model resources and color uniforms. The glTF factor
is linear RGB in 0–1, defaults to zero and is validated before upload. A renderer
resource may use finite nonnegative HDR emission, as scene overrides already do.
Required extensions remain rejected; optional extensions, including
`KHR_materials_emissive_strength`, remain ignored with a warning. Emissive
textures still return an explicit unsupported-format error.

A scene override replaces imported emission along with the other numeric factors;
removing it restores the authored value without re-uploading geometry. Emission
is added after lighting/shadow attenuation and before fog. It leaves alpha-mask
coverage and shadow silhouettes unchanged and does not cast light onto neighbors.
This is an import/rendering slice for the planned Explorer; game adoption remains
open.

Decoder tests cover retained/default/invalid values and rejected emissive maps;
facade and resource tests cover transfer and validation. GPU readback checks
zero-light emission, override/removal and one retained geometry upload, plus
unchanged masked shadows on the receiver. `prepare-imported-emission.py` derives
GLB emission on/off controls from the checked 3D Benchmark, keeping scene
materials absent and its script/ready goal unchanged. Native and Chromium controls
exercise actual loading, decoding and rendering; CI retains both image pairs.
Uniform sizes, draw counts and geometry residency are unchanged by this slice.

At 960×540, GLB emission changes 20,962 pixels and adds 18,050 blue pixels
in the native control; rebuilt Chromium changes 20,960 and adds 18,034. Both
browser controls fetch all six assets, reach the script's ready goal and finish
without runtime/GPU errors. Native/browser pairs were visually inspected.
Local gates pass 212 scoped tests (40 GPU tests also run with a required Vulkan
adapter/timestamps), warnings-denied Clippy and WASM checks, formatting, file-size
checks and authored-color capture verification. This is correctness evidence,
not game completion or a benchmark improvement.

## Textured geometry normals

Cubes carry unit outward face normals. Inline `sindri.mesh` surfaces may supply
`surface.normals`: an array of object-space XYZ vectors with one usable finite,
nonzero normal per vertex. Missing/null normals preserve geometric flat shading.
Partial arrays, zero vectors and non-normalizable values are rejected while
validating the component. Existing position/UV/index contracts remain unchanged.

```json
"surface": {
  "vertices": [[-1,-1,0], [1,-1,0], [0,1,0]],
  "uvs": [[0,0], [1,0], [0.5,1]],
  "indices": [0,1,2],
  "normals": [[-0.5,0,1], [0.5,0,1], [0,0.5,1]]
}
```

`TexturedVertex::with_normal` also carries normals in cached render geometry;
zero is its default flat-shading sentinel. Frame preparation rejects nonfinite
normals in inline vertices or cached replacements before encoding. Shading
interpolates transformed normals and normalizes per fragment, in both the legacy
Lambert path and standard GGX. The vertex shader computes the inverse transpose
from cofactors of the model's linear transform; it handles rotation, nonuniform
scale and reflection without changing uniforms. Singular/near-singular transforms
(determinant magnitude below 1e-20) fall back to geometric normals.

World color and directional-shadow pipelines select clockwise front faces
for a negative world determinant, preserving exterior culling for reflected
cubes, inline meshes and cached geometry. Back-face culling is the default;
the shared sidedness override below also applies to these pipelines. Cutout-shadow
policy remains separate work. Legacy low-level
MVP-only draw methods still assume an identity world transform; project frames
use the separate-world-transform methods.

The textured vertex stride grows from 24 to 36 bytes, including existing ambient
occlusion. This adds 12 CPU/GPU bytes per textured vertex, including voxel
geometry; imported model vertices are unchanged. There is one extra resident
color pipeline and one extra shadow pipeline per renderer. Per-draw uniforms,
passes and queue-write counts are unchanged, and cached normal data uploads only
with geometry replacement. Packing/separate streams remain future optimization.

Cube tests verify outward unit normals and triangle winding. Scene tests exercise
validation, extraction and edit/Undo/Redo restoration. GPU readback compares
interpolated inline/cached normals with imported model inverse-transpose shading
under rotation, nonuniform scale and reflection, and checks reflected cube color
and shadows, including open inline/cached casters whose wrong winding would
remove the entire shadow. Native/browser smooth-versus-flat reflected ellipsoid controls reuse the
3D Benchmark's assets and script/ready goal; CI retains captures. This is feature
proof for Explorer's material track. Editor authoring review, Decay access and
in-tree game adoption remain open.

Final 960×540 controls change 1,449 native pixels and 1,451 Chromium pixels,
with maximum channel difference 45 in both. Both browser controls fetch all six
assets and reach ready without runtime/GPU errors. Captures were visually
inspected. Local checks pass 698 scoped tests plus 11 catalog tests (709 unique),
including 43 GPU tests repeated with required Vulkan/timestamps; native Clippy,
WASM, authored-color capture, formatting and file-size gates pass. Catalog
regeneration produces no changes. The fixed 300-prop on/off benchmark reconfirms
602/301 draws, 603/301 passes and 138,032/115,568 uniform bytes with valid GPU
timing. These are correctness/counter checks, not a performance improvement.

## Face visibility

`sindri.material.sidedness` is a checked choice: `inherit` (the default),
`single`, or `double`. It applies to the owning cube, inline surface, cached
voxel sections, or every primitive of an imported model. Other entities and
children retain their own settings. Unknown spellings and nonstring values
reject the component. The editor exposes the choice through checked commands
and undo/redo, using the same registration as generated component documentation.

```json
"sindri.material": {
  "base_color": [0.85, 0.4, 0.06, 1.0],
  "roughness": 0.7,
  "sidedness": "double"
}
```

`inherit` preserves each imported primitive's authored `doubleSided`; other
world geometry is single-sided. Explicit `single` can override an imported
double-sided primitive. Removing the component restores inherited visibility
alongside its existing factor restoration. Existing numeric-only overrides keep
their prior behavior because omitted sidedness inherits. Alpha masks, texture
ownership and legacy-look opt-in are unchanged.

`FramePass.sidedness` carries renderer-owned `MaterialSidedness` independently
of numeric factors. Both color and shadow encoding reset it for every pass.
Textured pipelines select front-face orientation and culling independently:
reflections preserve the outward side, and double-sided draws disable culling.
Visible backs flip interpolated authored normals toward the viewed side; flat
geometric normals already face the viewer. Imported color/shadow shaders use
the resolved policy with their existing orientation and normal logic.

Changing sidedness does not replace geometry, remesh voxels or upload model
assets. Two additional color and two additional shadow pipelines per textured
renderer cover double-sided reflected/unreflected draws. Uniform layouts,
draw/pass counts and per-draw queue writes are unchanged. This is face visibility,
not blended transparency, cutout-shadow unification or shared material assets.

GPU readbacks exercise inherited/forced/restored model settings, lit backs of
transient/retained/cached surfaces, a cube viewed from inside, reflected geometry,
matching directional-shadow coverage and geometry residency. Scene tests cover
checked removal/undo/redo and voxel overrides without remeshing. Lighting Lab
adds two back-facing orange panels: `Single-sided panel` is hidden from its
camera; `Double-sided panel` is visible and receives lighting on its back.
The `prepare-lighting3d.py --single-sided` control changes only the latter's
material policy. Native and real WebGPU capture comparisons require the visible
orange panel to disappear in that control. This is feature proof for the planned
Explorer material track; in-tree game adoption remains open.

Native review of the paired panels measures 3,550 changed/orange pixels at
960×540 when the double-sided control becomes single-sided. The 300-prop
baseline remains byte-identical, with 602/301 shadow-on/off draws and no
settled geometry uploads; offscreen and native-editor authored-color checks pass.
In a scratch Lighting Lab project, the editor choice hides/restores the panel,
Undo/Redo restores each policy, and saving and reopening preserves `single`.
Native Play reaches the existing lighting goal with that saved policy.

Real Chromium/WebGPU delivery measures the same 3,550 changed/orange pixels.
The lights-off control removes 91,020 brightened pixels, including 5,059 red
and 69,820 blue pixels; all three Lighting Lab exports fetch their six assets,
exercise the live point/spot phases and reach the scripted goal without GPU or
runtime errors. The existing browser benchmark remains byte-identical.
