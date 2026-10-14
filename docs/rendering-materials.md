# Standard material factors

This is the numeric foundation of the planned Explorer material track, with
native/browser feature-control and GPU evidence. Game adoption and the complete
shared material asset/texture/alpha/sidedness contract remain open.

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
keeping their embedded textures, alpha masks, sidedness, hierarchy and shared
geometry. Remove the component to restore imported factors. Alpha factors
participate in imported masks in both color and shadow draws. Opaque imported
materials continue ignoring alpha for coverage. This component does not add
blended transparency or unify geometry-owned alpha/sidedness policy.

Without the component, existing textured world geometry retains its Lambert
appearance; imported models retain their authored factors and existing GGX
response. Opting in selects the common standard path. Migration of legacy
looks, shared texture/material assets, authored normals for cubes/inline meshes,
per-node model overrides and GLB emissive-factor decoding remain pending.
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
