<!-- Generated from docs/capabilities/assets.toml by `cargo run -p sindri-capabilities -- --write`. Edit the source, not this file. -->

# Assets

Logical asset IDs, the asynchronous load queue, typed decoders, manifests and
hot reload. `docs/asset-foundation.md` is the contract.

Each entry links its tests and the projects that exercise it. Surface marks match the row in [parity](../parity.md) where one is named: ✅ works and is exercised · 🟡 a useful slice, with the gap named · ❌ absent · — not applicable.

| Capability | Engine | Editor | Decay | Proof |
| --- | :-: | :-: | :-: | :-: |
| **Loading** | | | | |
| [Asset IDs, manifests, async load](#asset-ids-and-loading) | ✅ | ✅ | 🟡 | ✅ |
| [Identical image decoding on every target](#image-decoding) | ✅ | — | — | — |
| [Project fonts](#project-fonts) | ✅ | ✅ | — | ✅ |
| [Reusable data profiles](#profiles) | ✅ | ✅ | ✅ | ✅ |
| [Hot reload](#hot-reload) | ✅ | 🟡 | 🟡 | ✅ |

## Loading

<a id="asset-ids-and-loading"></a>

### Asset IDs, manifests, async load

Validated logical IDs, a bounded generation-checked load queue, typed decoders and hash-checked manifests.

Engine ✅ · Editor ✅ · Decay 🟡 · Proof ✅ — parity row *Asset IDs, manifests, async load*

**Tested by:** [`crates/sindri-core/src/asset/tests/id.rs`](../../crates/sindri-core/src/asset/tests/id.rs), [`crates/sindri-core/src/asset/tests/store.rs`](../../crates/sindri-core/src/asset/tests/store.rs), [`crates/sindri-assets/src/queue/tests.rs`](../../crates/sindri-assets/src/queue/tests.rs), [`crates/sindri-assets/src/loader/tests.rs`](../../crates/sindri-assets/src/loader/tests.rs), [`crates/sindri-assets/src/manifest/tests.rs`](../../crates/sindri-assets/src/manifest/tests.rs), [`crates/sindri-gpu/tests/texture_registry.rs`](../../crates/sindri-gpu/tests/texture_registry.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

**See:** [`docs/asset-foundation.md`](../../docs/asset-foundation.md)

<details markdown="1">
<summary>Evidence</summary>

`AssetId` is a validated relative logical ID, never a path or URL. Sources
resolve it against a root: the filesystem source canonicalises to catch symlink
escapes, and `UrlRoot` percent-encodes and normalises bases with rules
exercised on every target rather than only in a browser. The load queue is
bounded, rejects duplicates, and carries a generation token so a late
completion cannot overwrite a replacement. Textures, fonts, sheets, scripts,
and scene JSON decode through typed decoders. Nothing pretends browser I/O is
synchronous.

A `TextureId` is a generation-checked slot handle, so the renderer's texture
registry can release one texture and reuse its slot without a handle nobody
updated drawing whatever lands there next.

`AssetLoader` drives all of that in one place — request, enqueue, drain,
decode, apply — so a caller does not have to know the order. Requesting is
idempotent, a failure is reported once rather than retried forever, and
`retain` releases what is no longer wanted and says which IDs went, so a host
can drop whatever it built from them. GPU upload stays with the host, which is
the only thing that owns a device.

`AssetManifest` records what a project ships — each asset's length and the
SHA-256 of its stored bytes — as a versioned, ID-ordered file. A loader given
one holds arriving bytes to it, so a truncated response or a stale cache entry
is an error naming the asset rather than a picture from last week; an asset the
manifest does not list still loads. The editor picks one up from the directory
a scene lives in. The generic browser host sizes each kind's bounded request
queue from that manifest, so projects with more than the asset pipeline's
default 64 textures or other same-kind assets still load without increasing
fetch concurrency.

</details>

<a id="image-decoding"></a>

### Identical image decoding on every target

Every PNG colour type, 16-bit, interlaced and JPEG images decode pixel-identically natively and on wasm32.

Engine ✅ · Editor — · Decay — · Proof —

**Tested by:** [`crates/sindri-assets/tests/decode_compatibility.rs`](../../crates/sindri-assets/tests/decode_compatibility.rs), [`crates/sindri-assets/src/decode/tests.rs`](../../crates/sindri-assets/src/decode/tests.rs)

<details markdown="1">
<summary>Evidence</summary>

A corpus of deliberately awkward images — every PNG colour type, sixteen bits
per channel, an interlaced encoding, and a JPEG — is decoded and checked pixel
by pixel on both native and `wasm32-unknown-unknown`, so a texture cannot
decode one way in the editor and another in the browser.

</details>

<a id="project-fonts"></a>

### Project fonts

OpenType faces are validated and bound under the scene's logical reference, so no build substitutes an installed font.

Engine ✅ · Editor ✅ · Decay — · Proof ✅ — parity row *Project fonts*

**Tested by:** [`crates/sindri-assets/src/decode/tests.rs`](../../crates/sindri-assets/src/decode/tests.rs)

**Exercised in:** [`games/orbital-baked`](../../games/orbital-baked)

<details markdown="1">
<summary>Evidence</summary>

`FontAssetDecoder` validates OpenType bytes and records their declared family;
text binds that project-owned face under the scene's logical asset reference,
so native and browser builds never substitute different installed fonts.

</details>

<a id="profiles"></a>

### Reusable data profiles

Versioned `.profile` assets hold nested data that many script components read without creating entities.

Engine ✅ · Editor ✅ · Decay ✅ · Proof ✅ — parity row *Reusable data profiles*

**Tested by:** [`crates/sindri-decay/tests/a_script_reads_a_profile.rs`](../../crates/sindri-decay/tests/a_script_reads_a_profile.rs)

**Exercised in:** [`games/lamplighter`](../../games/lamplighter)

**See:** [`docs/profiles.md`](../../docs/profiles.md)

<details markdown="1">
<summary>Evidence</summary>

`ProfileDocument` is the reusable-data counterpart to a prefab: a versioned
`.profile` asset with an author label, an optional game-defined type, and a
nested JSON-shaped value payload. `ProfileAssetDecoder` validates it through
the same native/browser loading path, and the manifest records it as
`profile`. Unlike a prefab it creates no entity; many script components can
hold the same logical profile reference and read one loaded document.

</details>

<a id="hot-reload"></a>

### Hot reload

On native, `AssetWatch` notices a changed file behind a loaded asset and `AssetLoader::reload` loads it again.

Engine ✅ · Editor 🟡 · Decay 🟡 · Proof ✅ — parity row *Hot reload*

**Tested by:** [`crates/sindri-assets/src/loader/tests.rs`](../../crates/sindri-assets/src/loader/tests.rs)

**Exercised in:** [`games/lamplighter`](../../games/lamplighter)

<details markdown="1">
<summary>Evidence</summary>

On native, `AssetWatch` notices when the file behind a loaded asset changes and
`AssetLoader::reload` loads it again, by polling modification time and length
rather than subscribing to filesystem events.

</details>
