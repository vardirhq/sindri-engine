# Imported model occlusion silhouettes

Causeway adds this general rendering capability to keep wardens and Hollow
readable behind raised terrain without changing their lit low-poly artwork.

An optional `sindri.model.occluded_color` is a flat RGBA array with finite
channels in 0..=1. An absent or null value preserves the ordinary model path.
Causeway authors muted gold for wardens and rust for Hollow in `tools/level.py`.

The frame draws ordinary opaque geometry first, then opted-in subjects' hidden
fragments with a greater-depth test, alpha blending and no depth writes, then
those subjects normally lit with the usual less-depth test. World transparency
and the HUD follow. Subject primitives never contribute depth to the silhouette
test, so overlapping limbs do not reveal themselves. Normal subjects still cast
shadows. Both passes share geometry, transforms, textures and alpha cutouts;
mirrored instances retain their material facing rules.

This is a filled silhouette rather than an edge outline. Subjects reveal through
opaque non-subject geometry, including buildings, but not through each other.
Overlapping hidden fragments can accumulate alpha. Transparent world effects can
cover the result. This is authored through JSON; editor color authoring and a
Decay setter have not been exercised. Causeway's body picking is approximate
cylinder picking, with silhouetted bodies taking priority over terrain; it is
not a pixel-perfect model raycast.

Existing world text is also limited: world-text extraction uses XY coordinates
and drops Z, so Causeway's health/order labels do not follow this XZ battlefield
correctly. The selected-unit HUD still supplies health and skill information.
