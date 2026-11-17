//! Whether the file under an open scene has changed since it was read.
//!
//! An agent, a script or another editor may write the scene while it is open.
//! The editor used to notice only when told to reload, and its next save then
//! wrote over whatever had been done. This remembers what the file held when
//! it was last read or written — its size, its time and a hash of its bytes —
//! so a look costs one `stat` while nothing has changed, and a file that was
//! only touched, or rewritten with the same bytes, is not news.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

/// What the file held when it was last agreed with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
    hash: u64,
}

impl Stamp {
    /// The file as it is now, `None` when it cannot be read.
    pub(super) fn of(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let metadata = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: metadata.modified().ok(),
            len: metadata.len(),
            hash: hash(&bytes),
        })
    }
}

fn hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// What a look at the file found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnDisk {
    /// What the editor last read or wrote, or a scene with no file.
    Same,
    /// Different bytes from those the editor last read or wrote.
    Changed,
    /// The file is no longer there, or can no longer be read.
    Gone,
}

/// Compares the file with what was agreed, and agrees with it again when
/// only its time moved.
pub(super) fn look(path: &Path, seen: &mut Option<Stamp>) -> OnDisk {
    let Some(was) = seen.as_ref() else {
        return OnDisk::Same;
    };
    let Ok(metadata) = std::fs::metadata(path) else {
        return OnDisk::Gone;
    };
    if metadata.len() == was.len && metadata.modified().ok() == was.modified {
        return OnDisk::Same;
    }
    let Some(now) = Stamp::of(path) else {
        return OnDisk::Gone;
    };
    if now.hash == was.hash && now.len == was.len {
        *seen = Some(now);
        return OnDisk::Same;
    }
    OnDisk::Changed
}
