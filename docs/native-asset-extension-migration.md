# Native authored asset extensions

Sindri-authored documents should identify what they are, not which serialization happens to be used internally.

Canonical extensions:

- scene: `.scene`
- prefab: `.prefab`
- profile: `.profile`
- tile set: `.tileset`
- sprite sheet: `.sheet`

The current JSON-suffixed forms remain accepted as legacy input during migration:

- `.scene.json`
- `.prefab.json`
- `.profile.json`
- `.tileset.json`
- `.sheet.json`

Newly authored assets and Sindri's own examples/fixtures should use the canonical forms. JSON remains the serialization format for now; changing the extension does not introduce a new parser or file syntax.

The migration is complete only when editor creation/opening, project browsing, runtime loading, manifests/export, Decay tooling, sprite-sheet derivation, tests, examples, and documentation all use the canonical names while old projects still load through the legacy aliases.
