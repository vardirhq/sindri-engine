//! The projects the welcome window offers.
//!
//! A list of paths is the whole of it, and the two rules that keep such a list
//! from rotting are both here rather than in the window that draws it.
//!
//! The first is that a project is remembered by where it is and shown by what
//! it is called, so the name is stored beside the path. Reading every remembered
//! project's manifest to draw the list would mean the welcome window doing a
//! file read per row per frame, and a project on a disconnected network drive
//! would hang the window rather than appear in it.
//!
//! The second is that a project that has moved or been deleted is *shown*,
//! marked as missing, rather than quietly dropped. Silently pruning the list
//! would answer "where did my project go" with an empty row where it used to
//! be, and the editor cannot tell an unmounted volume from a deletion.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::manifest::Project;

/// How many projects the list keeps.
///
/// A welcome window is a way back to what someone is working on, not an
/// archive. Twelve is more than fits on the screen at once and far fewer than
/// the number of folders anyone has ever opened.
const MAX_REMEMBERED: usize = 12;

/// One remembered project.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecentProject {
    /// The project root, as it was opened.
    pub path: String,
    /// What its manifest called it when it was last opened.
    pub name: String,
    /// When it was last opened, in seconds since the Unix epoch.
    ///
    /// Absent from lists saved before it was recorded, which then show no time
    /// rather than a wrong one until the project is next opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<u64>,
}

impl RecentProject {
    /// Whether the project is still where it was.
    ///
    /// Asked of the directory *and* its manifest, because a project whose
    /// `sindri.toml` was deleted is as unopenable as one whose folder was, and a
    /// row that offers to open it would fail on the click rather than say so on
    /// the row.
    pub fn is_present(&self) -> bool {
        super::manifest::is_project(Path::new(&self.path))
    }
}

/// Projects in the order they were last opened, most recent first.
///
/// Read through [`RecentProjects::from`], so a preferences file written before
/// paths were normalized loses its duplicates the next time it is read.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "Vec<RecentProject>", into = "Vec<RecentProject>")]
pub struct RecentProjects(Vec<RecentProject>);

impl From<Vec<RecentProject>> for RecentProjects {
    fn from(entries: Vec<RecentProject>) -> Self {
        let mut recent = Self::default();
        for entry in entries {
            let path = normalized(&entry.path);
            if !recent.0.iter().any(|kept| kept.path == path) {
                recent.0.push(RecentProject { path, ..entry });
            }
        }
        recent.0.truncate(MAX_REMEMBERED);
        recent
    }
}

impl From<RecentProjects> for Vec<RecentProject> {
    fn from(recent: RecentProjects) -> Self {
        recent.0
    }
}

/// One spelling per project.
///
/// A project opened as `games/orbital/` and again as `games/orbital` — or from
/// another working directory — is the same project, and the welcome window
/// listed it twice. Made absolute and rebuilt from its components, which drops
/// trailing separators and `.` segments without touching the disk: a project
/// on an unmounted drive still has to be remembered by where it was.
fn normalized(path: &str) -> String {
    let path = Path::new(path);
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    absolute
        .components()
        .collect::<PathBuf>()
        .display()
        .to_string()
}

/// The current time in seconds since the Unix epoch, if the clock is sane.
pub fn now() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_secs())
}

/// How long ago something happened, as the welcome window says it.
///
/// Coarse on purpose: "3 days ago" answers "which one was I in last", and a
/// row that ticked over every second would be redrawn for nothing.
pub fn ago(then: u64, now: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;
    const MONTH: u64 = 30 * DAY;
    const YEAR: u64 = 365 * DAY;
    let elapsed = now.saturating_sub(then);
    let (count, unit) = match elapsed {
        0..MINUTE => return "just now".to_owned(),
        MINUTE..HOUR => (elapsed / MINUTE, "minute"),
        HOUR..DAY => (elapsed / HOUR, "hour"),
        DAY..WEEK => (elapsed / DAY, "day"),
        WEEK..MONTH => (elapsed / WEEK, "week"),
        MONTH..YEAR => (elapsed / MONTH, "month"),
        _ => (elapsed / YEAR, "year"),
    };
    if count == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{count} {unit}s ago")
    }
}

impl RecentProjects {
    /// Puts a project at the top of the list, where it was opened.
    ///
    /// Opening a project already on the list moves it rather than adding it
    /// again, and re-reads its name: renaming a project in its manifest should
    /// show up in the welcome window the next time it is opened, not the next
    /// time the preferences file is deleted.
    pub fn remember(&mut self, project: &Project) {
        let path = normalized(&project.root().display().to_string());
        self.0.retain(|remembered| remembered.path != path);
        self.0.insert(
            0,
            RecentProject {
                path,
                name: project.name().to_owned(),
                opened: now(),
            },
        );
        self.0.truncate(MAX_REMEMBERED);
    }

    /// Takes a project off the list.
    ///
    /// The only way a row leaves, and it is always something the user asked for.
    pub fn forget(&mut self, path: &str) {
        let path = normalized(path);
        self.0.retain(|remembered| remembered.path != path);
    }

    pub fn entries(&self) -> &[RecentProject] {
        &self.0
    }

    /// The most recently opened project, whether or not it is still there.
    ///
    /// Whether it is still there is the caller's question: opening on launch
    /// falls back to the welcome window, while the window itself shows the row
    /// as missing.
    pub fn most_recent(&self) -> Option<&RecentProject> {
        self.0.first()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use sindri_core::SceneDocument;

    use super::*;

    fn project(root: &Path, name: &str) -> Project {
        Project::create(root, name, &SceneDocument::default()).expect("a project is created")
    }

    #[test]
    fn the_most_recently_opened_project_is_first() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let first = project(&directory.path().join("one"), "One");
        let second = project(&directory.path().join("two"), "Two");

        let mut recent = RecentProjects::default();
        recent.remember(&first);
        recent.remember(&second);

        let names: Vec<&str> = recent
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, ["Two", "One"]);
    }

    #[test]
    fn opening_a_remembered_project_moves_it_rather_than_repeating_it() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let first = project(&directory.path().join("one"), "One");
        let second = project(&directory.path().join("two"), "Two");

        let mut recent = RecentProjects::default();
        recent.remember(&first);
        recent.remember(&second);
        recent.remember(&first);

        assert_eq!(recent.entries().len(), 2, "a project appears once");
        assert_eq!(recent.most_recent().expect("a first row").name, "One");
    }

    #[test]
    fn a_renamed_project_is_listed_under_its_new_name() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path().join("game");
        let before = project(&root, "Working Title");
        let mut recent = RecentProjects::default();
        recent.remember(&before);

        std::fs::write(
            super::super::manifest::manifest_path(&root),
            "format_version = 1\n\n[project]\nname = \"Gather\"\n",
        )
        .expect("the manifest is rewritten");
        let after = Project::open(&root).expect("the renamed project opens");
        recent.remember(&after);

        assert_eq!(recent.entries().len(), 1);
        assert_eq!(recent.most_recent().expect("a first row").name, "Gather");
    }

    /// The welcome window listed every project twice: once as it was opened
    /// from a path with a trailing separator, once without.
    #[test]
    fn one_project_is_one_row_however_its_path_was_written() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path().join("orbital");
        project(&root, "Orbital");
        let mut recent = RecentProjects::default();
        for spelling in [
            root.display().to_string(),
            format!("{}/", root.display()),
            format!("{}/.", root.display()),
        ] {
            recent.remember(&Project::open(Path::new(&spelling)).expect("the project opens"));
        }
        assert_eq!(recent.entries().len(), 1, "{:?}", recent.entries());
    }

    /// A preferences file written before paths were normalized already holds
    /// both rows, and reading it keeps one.
    #[test]
    fn duplicates_already_saved_are_merged_when_read() {
        let saved = r#"[
            {"path": "/games/orbital/", "name": "Orbital"},
            {"path": "/games/orbital", "name": "Orbital"},
            {"path": "/games/platformer", "name": "Platformer"}
        ]"#;
        let recent: RecentProjects = serde_json::from_str(saved).expect("the list reads");
        let paths: Vec<&str> = recent
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths.len(), 2, "{paths:?}");
        assert_eq!(recent.most_recent().expect("a first row").name, "Orbital");
    }

    #[test]
    fn times_are_said_in_the_largest_whole_unit() {
        assert_eq!(ago(100, 130), "just now");
        assert_eq!(ago(0, 60), "1 minute ago");
        assert_eq!(ago(0, 3 * 3600 + 59), "3 hours ago");
        assert_eq!(ago(0, 2 * 86_400), "2 days ago");
        assert_eq!(ago(0, 15 * 86_400), "2 weeks ago");
        assert_eq!(ago(0, 400 * 86_400), "1 year ago");
        assert_eq!(ago(500, 100), "just now", "a clock that went back");
    }

    #[test]
    fn a_list_saved_without_times_still_reads() {
        let saved = r#"[{"path":"/projects/orbital","name":"Orbital"}]"#;
        let recent: RecentProjects = serde_json::from_str(saved).expect("the list reads");
        assert_eq!(recent.entries()[0].opened, None);
    }

    #[test]
    fn the_list_stops_growing() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let mut recent = RecentProjects::default();
        for index in 0..MAX_REMEMBERED + 5 {
            let root = directory.path().join(format!("project-{index}"));
            recent.remember(&project(&root, &format!("Project {index}")));
        }
        assert_eq!(recent.entries().len(), MAX_REMEMBERED);
        assert_eq!(
            recent.most_recent().expect("a first row").name,
            format!("Project {}", MAX_REMEMBERED + 4),
            "and it is the newest that survives"
        );
    }

    #[test]
    fn a_project_that_moved_is_still_listed_and_says_it_is_missing() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path().join("gone");
        let opened = project(&root, "Gone");
        let mut recent = RecentProjects::default();
        recent.remember(&opened);
        std::fs::remove_dir_all(&root).expect("the project is removed");

        let entry = recent.most_recent().expect("the row is still there");
        assert!(
            !entry.is_present(),
            "an empty row where a project used to be answers nothing"
        );
        recent.forget(&entry.path.clone());
        assert!(
            recent.is_empty(),
            "and it leaves only when the user asks it to"
        );
    }

    #[test]
    fn a_project_whose_manifest_was_deleted_reads_as_missing() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path().join("hollow");
        let opened = project(&root, "Hollow");
        let mut recent = RecentProjects::default();
        recent.remember(&opened);
        std::fs::remove_file(super::super::manifest::manifest_path(&root))
            .expect("the manifest is removed");

        assert!(
            !recent.most_recent().expect("the row").is_present(),
            "a folder that is no longer a project cannot be opened as one"
        );
    }
}
