//! Delivering a sound into a Sindri project, ready for a script to play.
//!
//! A clip a script plays is a string inside a program, which no export walk can
//! see, so a project lists it under `[assets] include` in `sindri.toml`. Writing
//! the WAV without that line makes a sound that plays in the editor and is
//! silently missing from the exported game; this module always does both.
//!
//! A browser cannot write into a project, so there the Lab downloads the same
//! two files and says which line to add; only the naming is shared.
#[cfg(not(target_arch = "wasm32"))]
use std::{
    fs,
    path::{Path, PathBuf},
};
#[cfg(not(target_arch = "wasm32"))]
use toml_edit::{Array, DocumentMut, Item, Table, value};

#[cfg(not(target_arch = "wasm32"))]
const MANIFEST: &str = "sindri.toml";

#[cfg(not(target_arch = "wasm32"))]
/// The nearest enclosing directory holding a `sindri.toml`.
pub(super) fn find(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(MANIFEST).is_file())
        .map(Path::to_path_buf)
}

#[cfg(not(target_arch = "wasm32"))]
/// The project's display name, or its folder name when the manifest has none.
pub(super) fn name(root: &Path) -> String {
    fs::read_to_string(root.join(MANIFEST))
        .ok()
        .and_then(|text| text.parse::<DocumentMut>().ok())
        .and_then(|doc| {
            doc.get("project")
                .and_then(|project| project.get("name"))
                .and_then(Item::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| {
            root.file_name()
                .map_or_else(|| "project".into(), |n| n.to_string_lossy().into_owned())
        })
}

/// A file-name stem a clip id can carry: lower-case letters, digits, `-` and `_`.
pub(super) fn stem(name: &str) -> Option<String> {
    let mut out = String::new();
    for c in name.trim().chars() {
        match c {
            'a'..='z' | '0'..='9' | '-' | '_' => out.push(c),
            'A'..='Z' => out.push(c.to_ascii_lowercase()),
            ' ' | '.' => out.push('-'),
            _ => {}
        }
    }
    let out = out.trim_matches('-').to_owned();
    (!out.is_empty()).then_some(out)
}

#[cfg(not(target_arch = "wasm32"))]
/// Where a sound went, and the line that plays it.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Placed {
    pub(super) clip: String,
    pub(super) wav: PathBuf,
    pub(super) source: PathBuf,
    /// Whether this call added the clip to `[assets] include`.
    pub(super) listed: bool,
}

#[cfg(not(target_arch = "wasm32"))]
/// Writes `audio/<stem>.<format>` and its editable source beside it, then
/// lists the clip. `format` is the clip's file extension: `wav` for an
/// effect, `ogg` for a song.
pub(super) fn place(
    root: &Path,
    name: &str,
    audio: &[u8],
    format: &str,
    source_extension: &str,
    source: &str,
) -> Result<Placed, String> {
    let stem = stem(name).ok_or("Name the sound with letters or digits first.")?;
    let manifest = root.join(MANIFEST);
    let text = fs::read_to_string(&manifest)
        .map_err(|e| format!("{} is not a Sindri project: {e}", root.display()))?;
    let mut doc = text
        .parse::<DocumentMut>()
        .map_err(|e| format!("sindri.toml does not parse: {e}"))?;
    // Asset ids are relative to `assets/` when a project has one, as export reads them.
    let assets = if root.join("assets").is_dir() {
        root.join("assets")
    } else {
        root.to_path_buf()
    };
    let folder = assets.join("audio");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let clip = format!("audio/{stem}.{format}");
    let wav_path = folder.join(format!("{stem}.{format}"));
    let source_path = folder.join(format!("{stem}.{source_extension}"));
    fs::write(&wav_path, audio).map_err(|e| e.to_string())?;
    fs::write(&source_path, source).map_err(|e| e.to_string())?;
    let listed = include(&mut doc, &clip);
    if listed {
        fs::write(&manifest, doc.to_string()).map_err(|e| e.to_string())?;
    }
    Ok(Placed {
        clip,
        wav: wav_path,
        source: source_path,
        listed,
    })
}

#[cfg(not(target_arch = "wasm32"))]
/// Adds `clip` to `[assets] include`, keeping every comment and table in place.
fn include(doc: &mut DocumentMut, clip: &str) -> bool {
    let assets = doc
        .entry("assets")
        .or_insert_with(|| Item::Table(Table::new()));
    let Some(assets) = assets.as_table_like_mut() else {
        return false;
    };
    let list = assets
        .entry("include")
        .or_insert_with(|| value(Array::new()));
    let Some(list) = list.as_array_mut() else {
        return false;
    };
    if list.iter().any(|item| item.as_str() == Some(clip)) {
        return false;
    }
    list.push(clip);
    true
}

/// The Decay line that plays a placed clip.
pub(super) fn snippet(clip: &str, music: bool) -> String {
    if music {
        format!("Audio.loop(\"{clip}\", 0.6);")
    } else {
        format!("Audio.play(\"{clip}\", 0.5);")
    }
}

/// The `sindri.toml` line that ships a clip a script plays by name.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn include_line(clip: &str) -> String {
    format!("[assets] include = [\"{clip}\"]")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(manifest: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp");
        fs::write(dir.path().join(MANIFEST), manifest).expect("manifest");
        fs::create_dir(dir.path().join("assets")).expect("assets");
        dir
    }

    #[test]
    fn placing_a_sound_writes_it_and_lists_it_once() {
        let dir = project("format_version = 1\n\n# keep me\n[project]\nname = \"Demo\"\n");
        let placed =
            place(dir.path(), "Big Coin!", b"RIFF", "wav", "sindrisfx", "{}").expect("placed");
        assert_eq!(placed.clip, "audio/big-coin.wav");
        assert!(placed.listed);
        assert!(dir.path().join("assets/audio/big-coin.wav").is_file());
        assert!(dir.path().join("assets/audio/big-coin.sindrisfx").is_file());
        let manifest = fs::read_to_string(dir.path().join(MANIFEST)).expect("read");
        assert!(manifest.contains("# keep me"));
        assert!(manifest.contains("\"audio/big-coin.wav\""));

        let again =
            place(dir.path(), "big-coin", b"RIFF2", "wav", "sindrisfx", "{}").expect("again");
        assert!(!again.listed);
        let manifest = fs::read_to_string(dir.path().join(MANIFEST)).expect("read");
        assert_eq!(manifest.matches("audio/big-coin.wav").count(), 1);
        assert_eq!(name(dir.path()), "Demo");
    }

    #[test]
    fn an_existing_include_list_keeps_its_entries() {
        let dir = project("[project]\nname = \"X\"\n\n[assets]\ninclude = [\"ui.weave\"]\n");
        place(dir.path(), "zap", b"RIFF", "wav", "sindrisfx", "{}").expect("placed");
        let manifest = fs::read_to_string(dir.path().join(MANIFEST)).expect("read");
        let doc = manifest.parse::<DocumentMut>().expect("parse");
        let list: Vec<_> = doc["assets"]["include"]
            .as_array()
            .expect("array")
            .iter()
            .filter_map(|item| item.as_str())
            .collect();
        assert_eq!(list, ["ui.weave", "audio/zap.wav"]);
    }

    #[test]
    fn a_folder_without_a_manifest_is_not_a_project() {
        let dir = tempfile::tempdir().expect("temp");
        assert!(place(dir.path(), "zap", b"RIFF", "wav", "sindrisfx", "{}").is_err());
        assert!(!dir.path().join("audio").exists());
        assert_eq!(find(&dir.path().join("a/b")), None);
        let inner = project("[project]\n");
        let nested = inner.path().join("assets/audio");
        fs::create_dir_all(&nested).expect("nested");
        assert_eq!(find(&nested).as_deref(), Some(inner.path()));
    }

    #[test]
    fn a_download_names_the_line_that_ships_it() {
        assert_eq!(
            include_line("audio/zap.wav"),
            "[assets] include = [\"audio/zap.wav\"]"
        );
    }

    #[test]
    fn names_become_safe_file_stems() {
        assert_eq!(stem("  Laser 2.0 "), Some("laser-2-0".into()));
        assert_eq!(stem("../../etc/passwd"), Some("etcpasswd".into()));
        assert_eq!(stem("!!"), None);
    }
}
