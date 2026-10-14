# Sprite masks

A sprite mask is a picture placed in the world whose shape decides where other
sprites draw. It is never drawn itself. A keyhole moving over a dark scene, a
spotlight, a window onto another layer, a health orb filling inside a round
frame: the shape is the mask's alpha, and each sprite says whether it shows
inside it or outside it.

## Authoring

`sindri.sprite_mask` sits on an entity and is placed by its transform the way a
sprite is, anchor included:

| Field | Meaning | Default |
| --- | --- | --- |
| `texture` | The picture whose alpha is the shape, as a sprite names one (`sheet.png#hole` works) | — |
| `name` | What masked sprites name to meet it; empty is a name too | `""` |

A `sindri.sprite` opts in:

| Field | Meaning | Default |
| --- | --- | --- |
| `mask` | `none`, `inside` (draws only where the mask is opaque) or `outside` (only where it is not) | `none` |
| `mask_name` | The `name` of the mask it meets | `""` |

Sprites find their mask by name rather than by being its children, because what
a keyhole reveals stays still while the keyhole moves. A sprite masked `inside`
a mask the scene does not have draws nothing; one masked `outside` it draws
whole. If two masks share a name, the first found is used.

Coverage is the mask's alpha, so a soft-edged mask gives a soft edge. Outside
the mask's quad counts as clear.

## How it draws

The mask is batch state, not per-sprite data: a batch carries one mask texture
and the matrix from the world into the mask's quad, and the fragment stage
multiplies each pixel's alpha by what the mask lets through. Sprites masked by
the same mask the same way still share a draw, and an unmasked sprite pays
nothing. Opaque (depth-writing) sprites discard what the mask hides.

There is no stencil buffer involved, so masks do not need a depth format with
stencil and nothing about masking reaches the 3D renderer.

## Limits

World sprites only: UI images, tilemaps, particles and shapes are not masked.
One mask per sprite, so masks cannot be combined into a union or intersection.

## Proof

`examples/sprite-masks`: a keyhole wanders over a night sky. The night is
masked outside it, so the sunny meadow under it shows through the hole, and a
gem is masked inside it, so it is only ever seen through the hole.
`game/tests/the_sprite_mask_demo_works.rs` plays it, and
`crates/sindri-gpu/tests/sprite_masks.rs` reads back pixels inside and outside a
mask.
