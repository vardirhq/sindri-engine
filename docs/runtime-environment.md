# Runtime ambient and background

This is feature proof for Explorer and external Low Tide 3D's planned lighting
track. Lighting Lab (`examples/lighting3d`) exercises it; in-tree game adoption
and a full day/night clock remain open.

`this.environment` and `entity.environment` refer to an existing
`sindri.environment` component. They expose `ambient_intensity` as a number,
`ambient_color` as Color and `background` as Color. Both colors support whole
assignments and r/g/b/a channel access. Ambient alpha reads one and must remain
one; background retains authored RGBA. The existing authored color range remains
finite f32, without a new clamp or nonnegative constraint. Ambient intensity is
finite and nonnegative. Fog, shadows and post-processing controls are not exposed
by this API.

Reads resolve omitted component defaults. Writes clone the payload, check the
existing and complete candidate with the scene validator, then replace it only
on success. Non-finite/overflowing numbers, invalid ambient alpha, negative
intensity and invalid preexisting settings report runtime errors and leave the
old payload untouched. Whole colors commit once, preserving atomic rejection.
Channel edits preserve other authored channels; omitted colors materialize their
defaults only when edited. Unrelated fields and omitted settings remain intact.
Integer-authored JSON slots accept fractional writes without rounding.

Access neither creates a component nor selects the world's active environment.
Missing components and stale handles are errors. Writes can address inactive
environment entities; scene extraction still chooses the active environment,
ignores inactive ancestors and rejects multiple active environments. No second
environment store or time-of-day service is introduced.

The shared Session runs scripts before extraction. The next frame derives ambient
lighting and clear color from the updated world through the existing native,
browser and editor paths. No restart, renderer uniform layout change, geometry
upload, new pipeline or resource rebuild is required.

Lighting Lab changes ambient intensity/color and background from blue to warm
during its point/spot phases, then restores the original environment at its ready
goal. Its open/compile/play test checks both goals and intermediate extracted
lighting and frame clear. The phase image comparison checks the changing clear
color as well as the colored geometry. Final lit/off controls retain the same
ambient and background, isolating local-light illumination. This is separate
from full sunrise/midday/sunset/night transitions, mounted-GLB sun-shadow proof,
fog/post delivery and game adoption.


Local validation passes 535 scoped LSP/game/Decay/catalogue tests, warning-denied
workspace Clippy, a workspace WASM check and typed preflight with zero runtime
reminders. Native phase backgrounds change from RGB [33,44,97] to [97,44,33];
excluding the background, surfaces add 111,271 red pixels and remove 154,887 blue
pixels. Final lit/off controls retain 91,624 brightened pixels. Native editor
Play visibly shows both environment phases and the ready goal; Stop/save restores
the original environment payload, all 13 authored entities and their disabled
black lights. Native/editor authored-color verification also passes.

Rebuilt WebGPU Chromium captures show the same RGB [33,44,97] to [97,44,33]
background transition. Excluding the sampled background, the on-control phases
add 109,022 red surface pixels and remove 154,857 blue pixels. Both lit/dark
exports fetch six assets, reach ready and report no runtime/GPU errors; the final
comparison retains 91,020 brightened pixels. The original 300-prop browser
workload and its shadow comparison pass (2,088 darkened pixels). The phase checker
samples inside the background to avoid the browser canvas's CSS border.
