//! What a manifest accepts, and what it refuses to describe.

use super::*;

fn id(value: &str) -> AssetId {
    AssetId::new(value).expect("test asset IDs are valid")
}

#[test]
fn the_hash_is_a_sha256_anyone_else_would_get() {
    assert_eq!(
        ContentHash::of(b"abc").to_string(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        ContentHash::of(b"").to_string(),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn a_hash_round_trips_through_its_text() {
    let hash = ContentHash::of(b"some bytes");
    assert_eq!(hash.to_string().parse::<ContentHash>(), Ok(hash));
}

#[test]
fn a_hash_of_another_algorithm_is_refused() {
    assert!(matches!(
        "sha512:00".parse::<ContentHash>(),
        Err(ManifestError::UnknownAlgorithm(_))
    ));
    assert!(matches!(
        "sha256:nothex".parse::<ContentHash>(),
        Err(ManifestError::MalformedHash(_))
    ));
    assert!(matches!(
        "sha256:zz7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            .parse::<ContentHash>(),
        Err(ManifestError::MalformedHash(_))
    ));
}

#[test]
fn bytes_that_do_not_match_are_rejected() {
    let mut manifest = AssetManifest::new();
    manifest.insert(id("textures/badge.png"), b"the real bytes");
    assert_eq!(
        manifest.verify(&id("textures/badge.png"), b"the real bytes"),
        Ok(())
    );
    let truncated = manifest
        .verify(&id("textures/badge.png"), b"the real")
        .expect_err("a short response is not the asset");
    assert!(truncated.message().contains("14 bytes and 8 arrived"));
    let swapped = manifest
        .verify(&id("textures/badge.png"), b"other bytes!!!")
        .expect_err("the same length is not the same asset");
    assert!(swapped.message().contains("sha256:"));
}

#[test]
fn an_asset_the_manifest_does_not_mention_passes() {
    let manifest = AssetManifest::new();
    assert_eq!(manifest.verify(&id("anything.png"), b"whatever"), Ok(()));
}

#[test]
fn a_manifest_round_trips_through_its_file() {
    let mut manifest = AssetManifest::new();
    manifest.insert(id("textures/badge.png"), b"badge");
    manifest.insert(id("demo.scene"), b"{}");
    let text = manifest.to_canonical_json().expect("a manifest serializes");
    assert!(text.ends_with('\n'));
    assert_eq!(AssetManifest::from_json(&text), Ok(manifest));
}

#[test]
fn the_file_is_ordered_by_asset_id() {
    let mut one = AssetManifest::new();
    one.insert(id("zebra.png"), b"z");
    one.insert(id("alpha.png"), b"a");
    let mut other = AssetManifest::new();
    other.insert(id("alpha.png"), b"a");
    other.insert(id("zebra.png"), b"z");
    assert_eq!(one.to_canonical_json(), other.to_canonical_json());
}

#[test]
fn the_first_inserted_scene_remains_the_entry_scene() {
    let mut manifest = AssetManifest::new();
    manifest.insert(id("orbital.scene"), b"main");
    manifest.insert(id("combat-lab.scene"), b"lab");
    let text = manifest.to_canonical_json().expect("manifest serializes");
    let restored = AssetManifest::from_json(&text).expect("manifest round trips");
    assert_eq!(
        restored
            .ids_of(AssetKind::Scene)
            .map(AssetId::as_str)
            .collect::<Vec<_>>(),
        ["orbital.scene", "combat-lab.scene"]
    );
}

#[test]
fn an_unsupported_version_is_refused() {
    let text = r#"{ "format_version": 99, "assets": {} }"#;
    assert_eq!(
        AssetManifest::from_json(text),
        Err(ManifestError::UnsupportedVersion(99))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_directory_becomes_the_names_a_scene_would_write() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("textures")).unwrap();
    std::fs::write(root.join("demo.scene"), b"{}").unwrap();
    std::fs::write(root.join("textures/badge.png"), b"badge bytes").unwrap();
    std::fs::write(root.join(".hidden"), b"tooling").unwrap();
    std::fs::write(root.join(MANIFEST_FILE_NAME), b"stale").unwrap();

    let manifest = AssetManifest::of_directory(root).expect("the project reads");
    assert_eq!(
        manifest.assets().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        ["demo.scene", "textures/badge.png"]
    );
}

#[test]
fn a_scanned_manifest_says_what_its_native_assets_are() {
    for (asset, expected) in [
        ("demo.scene", AssetKind::Scene),
        ("prefabs/bullet.prefab", AssetKind::Prefab),
        ("profiles/player.profile", AssetKind::Profile),
        ("textures/player.sheet", AssetKind::Sheet),
        ("tiles/world.tileset", AssetKind::TileSet),
        ("scripts/player.decay", AssetKind::Script),
        ("textures/player.png", AssetKind::Texture),
        ("fonts/Inter.ttf", AssetKind::Font),
        ("audio/theme.wav", AssetKind::Audio),
        ("fonts/Inter-OFL.txt", AssetKind::Other),
    ] {
        assert_eq!(AssetKind::for_id(asset), expected, "{asset}");
    }
}

#[test]
fn legacy_json_suffixed_authored_assets_are_still_classified() {
    for (asset, expected) in [
        ("demo.scene.json", AssetKind::Scene),
        ("prefabs/bullet.prefab.json", AssetKind::Prefab),
        ("profiles/player.profile.json", AssetKind::Profile),
        ("textures/player.sheet.json", AssetKind::Sheet),
        ("tiles/world.tileset.json", AssetKind::TileSet),
    ] {
        assert_eq!(AssetKind::for_id(asset), expected, "{asset}");
    }
    assert_eq!(AssetKind::for_id("data.json"), AssetKind::Other);
}
