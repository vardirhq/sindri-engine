//! Sprite sheets, and the sprites and texture that go with one.

use std::path::{Path, PathBuf};

use sindri_core::SpriteSheetDocument;

use super::kind::AssetKind;

/// Native and legacy sheet suffixes, relative to the texture they slice.
const SHEET_SUFFIX: &str = ".sheet";
const LEGACY_SHEET_SUFFIX: &str = ".sheet.json";

fn sheet_stem(name: &str) -> Option<&str> {
    name.strip_suffix(SHEET_SUFFIX)
        .or_else(|| name.strip_suffix(LEGACY_SHEET_SUFFIX))
}

/// The texture a sheet slices, when one sits beside it.
///
/// A sheet names its texture by stem rather than by extension, so this asks the
/// directory which image is there rather than guessing at `.png`.
pub fn sliced_texture_beside(sheet: &Path) -> Option<PathBuf> {
    let name = sheet.file_name()?.to_str()?;
    let stem = sheet_stem(name)?;
    let directory = sheet.parent()?;
    std::fs::read_dir(directory)
        .ok()?
        .flatten()
        .find_map(|entry| {
            let path = entry.path();
            let matches = path.file_stem().and_then(|found| found.to_str()) == Some(stem)
                && AssetKind::of_file(&path.to_string_lossy()) == AssetKind::Texture;
            matches.then_some(path)
        })
}

/// The sprites the sheet beside `texture` names, or nothing.
///
/// Prefer the native sidecar, but keep reading the old JSON-suffixed form while
/// projects migrate. A sheet that will not parse yields no sprites rather than
/// an error: the browser's job is to list a directory, and a broken sidecar is
/// something the slicer shows and fixes, not something that should empty the panel.
pub fn sprites_beside(texture: &Path) -> Vec<String> {
    let Some(stem) = texture.file_stem().and_then(|stem| stem.to_str()) else {
        return Vec::new();
    };
    let native = texture.with_file_name(format!("{stem}{SHEET_SUFFIX}"));
    let legacy = texture.with_file_name(format!("{stem}{LEGACY_SHEET_SUFFIX}"));
    let json = std::fs::read_to_string(&native)
        .or_else(|_| std::fs::read_to_string(&legacy));
    let Ok(json) = json else {
        return Vec::new();
    };
    SpriteSheetDocument::from_json(&json)
        .ok()
        .and_then(|document| document.rects().ok())
        .map(|rects| rects.into_keys().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::ProjectTree;
    use super::*;

    /// A sliced image carries its parts, so the browser can show them where a
    /// person looks for them: under the image, not loose in the directory.
    #[test]
    fn a_sliced_texture_carries_the_sprites_its_sheet_names() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        fs::write(directory.path().join("tiles.png"), []).expect("writable");
        fs::write(
            directory.path().join("tiles.sheet"),
            r#"{ "format_version": 1,
                 "grid": { "columns": 2, "rows": 1, "names": ["light", "dark"] } }"#,
        )
        .expect("writable");

        fs::write(directory.path().join("a.scene"), "{}").expect("writable");
        let tree = ProjectTree::beside(Some(&directory.path().join("a.scene")));
        let texture = tree
            .entries()
            .iter()
            .find(|entry| entry.name == "tiles.png")
            .expect("the texture is listed");
        assert_eq!(texture.sprites, vec!["dark".to_owned(), "light".to_owned()]);

        assert!(
            tree.entries()
                .iter()
                .all(|entry| entry.kind != AssetKind::Sheet),
            "the sheet is shown as its texture's sprites, so listing the file \
             as well would say the same thing twice"
        );
    }

    #[test]
    fn a_legacy_sheet_still_slices_its_texture() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        fs::write(directory.path().join("tiles.png"), []).expect("writable");
        fs::write(
            directory.path().join("tiles.sheet.json"),
            r#"{ "format_version": 1,
                 "grid": { "columns": 1, "rows": 1, "names": ["whole"] } }"#,
        )
        .expect("writable");
        assert_eq!(sprites_beside(&directory.path().join("tiles.png")), vec!["whole"]);
        assert_eq!(
            sliced_texture_beside(&directory.path().join("tiles.sheet.json")),
            Some(directory.path().join("tiles.png"))
        );
    }

    /// An orphaned sheet *is* listed, because a sidecar cutting up an image
    /// nobody can find is exactly what a browser that hides files would let you
    /// never notice.
    #[test]
    fn a_sheet_with_no_texture_is_still_listed() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        fs::write(
            directory.path().join("gone.sheet"),
            r#"{ "format_version": 1, "grid": { "columns": 1, "rows": 1 } }"#,
        )
        .expect("writable");

        fs::write(directory.path().join("a.scene"), "{}").expect("writable");
        let tree = ProjectTree::beside(Some(&directory.path().join("a.scene")));
        assert!(
            tree.entries()
                .iter()
                .any(|entry| entry.kind == AssetKind::Sheet && entry.name == "gone.sheet"),
            "a sheet slicing nothing is worth seeing"
        );
    }

    /// A sheet that will not parse leaves its texture unsliced rather than
    /// emptying the panel: listing a directory is the browser's job, and a
    /// broken sidecar is the slicer's to show.
    #[test]
    fn a_broken_sheet_leaves_its_texture_unsliced() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        fs::write(directory.path().join("tiles.png"), []).expect("writable");
        fs::write(directory.path().join("tiles.sheet"), "{ not json").expect("writable");

        fs::write(directory.path().join("a.scene"), "{}").expect("writable");
        let tree = ProjectTree::beside(Some(&directory.path().join("a.scene")));
        let texture = tree
            .entries()
            .iter()
            .find(|entry| entry.name == "tiles.png")
            .expect("the texture is still listed");
        assert!(texture.sprites.is_empty());
    }
}
