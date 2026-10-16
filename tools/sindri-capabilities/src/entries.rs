//! Capability entries: hand-written data, checked and rendered.
//!
//! The other documents this crate writes are read out of the engine. These are
//! not — what was run, and how far it got, is a judgement someone writes down —
//! but the parts of that judgement that can be checked are: a named test exists,
//! a claimed surface names its evidence, and the status agrees with the parity
//! row it says it is. `docs/capabilities/<area>.toml` is the source;
//! `docs/capabilities/<area>.md` is generated from it.

use std::{
    collections::{BTreeMap, HashSet},
    fmt::{self, Write as _},
    fs,
    path::Path,
};

use serde::Deserialize;

use crate::REGENERATE_COMMAND;

/// Where the sources live and the pages go, relative to the repository root.
pub(crate) const DIRECTORY: &str = "docs/capabilities";

const PARITY: &str = "docs/parity.md";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Area {
    title: String,
    /// Where the area sits in the index; lower first.
    order: u32,
    intro: String,
    #[serde(default, rename = "capability")]
    capabilities: Vec<Capability>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    id: String,
    group: String,
    title: String,
    summary: String,
    status: Status,
    /// The `docs/parity.md` row this entry is the evidence for, by its name.
    parity: Option<String>,
    #[serde(default)]
    tests: Vec<String>,
    #[serde(default)]
    proof: Vec<String>,
    #[serde(default)]
    docs: Vec<String>,
    #[serde(default)]
    limits: Vec<String>,
    /// How a claim with no named test or proof project was exercised. Rendered
    /// as a warning, so thin evidence is visible rather than absent.
    untested: Option<String>,
    #[serde(default)]
    detail: String,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Status {
    engine: Mark,
    editor: Mark,
    decay: Mark,
    proof: Mark,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
enum Mark {
    #[serde(rename = "yes")]
    Yes,
    #[serde(rename = "partial")]
    Partial,
    #[serde(rename = "no")]
    No,
    #[serde(rename = "n/a")]
    NotApplicable,
}

impl Mark {
    fn symbol(self) -> &'static str {
        match self {
            Self::Yes => "✅",
            Self::Partial => "🟡",
            Self::No => "❌",
            Self::NotApplicable => "—",
        }
    }

    fn from_symbol(cell: &str) -> Option<Self> {
        match cell.trim() {
            "✅" => Some(Self::Yes),
            "🟡" => Some(Self::Partial),
            "❌" => Some(Self::No),
            "—" => Some(Self::NotApplicable),
            _ => None,
        }
    }

    fn is_claim(self) -> bool {
        matches!(self, Self::Yes | Self::Partial)
    }
}

impl Status {
    fn marks(self) -> [Mark; 4] {
        [self.engine, self.editor, self.decay, self.proof]
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [engine, editor, decay, proof] = self.marks().map(Mark::symbol);
        write!(
            formatter,
            "Engine {engine} · Editor {editor} · Decay {decay} · Proof {proof}"
        )
    }
}

/// One generated page, for the caller to turn into a `GeneratedDocument`.
pub(crate) struct Page {
    pub(crate) path: String,
    pub(crate) contents: String,
}

/// The index that lists every area's page, at the path the hand-written
/// inventory had, so links to it keep working.
const INDEX: &str = "docs/capabilities.md";

/// Reads every area source, checks it, and renders its page and the index.
///
/// Every problem in every file is collected rather than stopping at the first,
/// because a person fixing a migration wants the whole list.
pub(crate) fn pages(root: &Path) -> Result<Vec<Page>, Vec<String>> {
    let parity = fs::read_to_string(root.join(PARITY))
        .map_err(|error| vec![format!("{PARITY} could not be read: {error}")])?;
    let rows = parity_rows(&parity);

    let mut sources: Vec<_> = fs::read_dir(root.join(DIRECTORY))
        .map_err(|error| vec![format!("{DIRECTORY} could not be read: {error}")])?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect();
    sources.sort();

    let mut problems = Vec::new();
    let mut areas = Vec::new();
    let mut ids = HashSet::new();
    for source in sources {
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        let name = format!("{DIRECTORY}/{stem}.toml");
        let area = match fs::read_to_string(&source)
            .map_err(|error| error.to_string())
            .and_then(|text| toml::from_str::<Area>(&text).map_err(|error| error.to_string()))
        {
            Ok(area) => area,
            Err(error) => {
                problems.push(format!("{name}: {error}"));
                continue;
            }
        };
        for capability in &area.capabilities {
            for problem in check(root, &rows, capability, &mut ids) {
                problems.push(format!("{name} `{}`: {problem}", capability.id));
            }
        }
        areas.push((stem, area));
    }

    areas.sort_by_key(|(_, area)| area.order);
    for pair in areas.windows(2) {
        if pair[0].1.order == pair[1].1.order {
            problems.push(format!(
                "{DIRECTORY}/{}.toml and {DIRECTORY}/{}.toml share `order = {}`",
                pair[0].0, pair[1].0, pair[0].1.order
            ));
        }
    }

    if !problems.is_empty() {
        return Err(problems);
    }

    let mut pages = vec![Page {
        path: INDEX.to_owned(),
        contents: render_index(&areas),
    }];
    pages.extend(areas.iter().map(|(stem, area)| Page {
        path: format!("{DIRECTORY}/{stem}.md"),
        contents: render(&format!("{DIRECTORY}/{stem}.toml"), area),
    }));
    Ok(pages)
}

/// The surface cells of every row in the parity tables, by the row's name.
fn parity_rows(parity: &str) -> BTreeMap<String, [Mark; 4]> {
    let mut rows = BTreeMap::new();
    for line in parity.lines() {
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
        if cells.len() < 5 {
            continue;
        }
        let marks: Option<Vec<Mark>> = cells[1..5]
            .iter()
            .map(|cell| Mark::from_symbol(cell))
            .collect();
        if let Some(marks) = marks {
            let name = cells[0].trim().trim_matches('*').to_owned();
            rows.insert(name, [marks[0], marks[1], marks[2], marks[3]]);
        }
    }
    rows
}

fn check(
    root: &Path,
    rows: &BTreeMap<String, [Mark; 4]>,
    capability: &Capability,
    ids: &mut HashSet<String>,
) -> Vec<String> {
    let mut problems = Vec::new();

    let id = &capability.id;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        problems.push("id must be lowercase letters, digits and hyphens".to_owned());
    }
    if !ids.insert(id.clone()) {
        problems.push("id is used by another entry".to_owned());
    }
    if capability.summary.contains('\n') {
        problems.push("summary must be one line".to_owned());
    }

    for path in capability
        .tests
        .iter()
        .chain(&capability.proof)
        .chain(&capability.docs)
    {
        if path.contains("..") || path.starts_with('/') {
            problems.push(format!("`{path}` must be a path inside the repository"));
        } else if !root.join(path).exists() {
            problems.push(format!("`{path}` does not exist"));
        }
    }

    let claims = capability.status.marks().into_iter().any(Mark::is_claim);
    let evidenced = !capability.tests.is_empty() || !capability.proof.is_empty();
    if claims && !evidenced && capability.untested.is_none() {
        problems.push(
            "claims ✅ or 🟡 but names no test, no proof project, and no `untested` note"
                .to_owned(),
        );
    }
    if evidenced && capability.untested.is_some() {
        problems.push("names its evidence, so `untested` no longer applies".to_owned());
    }

    if let Some(row) = &capability.parity {
        match rows.get(row) {
            None => problems.push(format!("{PARITY} has no row named `{row}`")),
            Some(marks) if *marks != capability.status.marks() => {
                let [engine, editor, decay, proof] = *marks;
                let parity = Status {
                    engine,
                    editor,
                    decay,
                    proof,
                };
                problems.push(format!(
                    "status is {} but {PARITY} says {parity}",
                    capability.status
                ));
            }
            Some(_) => {}
        }
    }

    problems
}

fn render_index(areas: &[(String, Area)]) -> String {
    let mut page = format!(
        "<!-- Generated from {DIRECTORY}/*.toml by `{REGENERATE_COMMAND}`. Edit the sources, not this file. -->\n\n"
    );
    page.push_str(INDEX_PREAMBLE);
    page.push_str(
        "\n| Area | Capabilities | Not yet | No automated test |\n| --- | :-: | :-: | :-: |\n",
    );
    let mut totals = [0usize; 3];
    for (stem, area) in areas {
        let absent = area
            .capabilities
            .iter()
            .filter(|capability| !capability.status.marks().into_iter().any(Mark::is_claim))
            .count();
        let untested = area
            .capabilities
            .iter()
            .filter(|capability| capability.untested.is_some())
            .count();
        let present = area.capabilities.len() - absent;
        for (total, count) in totals.iter_mut().zip([present, absent, untested]) {
            *total += count;
        }
        let _ = writeln!(
            page,
            "| [{}](capabilities/{stem}.md) | {present} | {absent} | {untested} |",
            area.title
        );
    }
    let [present, absent, untested] = totals;
    let _ = writeln!(
        page,
        "| **All** | **{present}** | **{absent}** | **{untested}** |"
    );
    page
}

/// What the index says above its table: why this inventory exists and how it
/// is kept, which is the part of the old hand-written file that was policy
/// rather than inventory.
const INDEX_PREAMBLE: &str = "# What Sindri can do

An inventory of what exists and what is missing. `ROADMAP.md` says what is
planned and in what order; `docs/parity.md` judges Sindri against what an
engine is expected to do; this says what is true today, and what proves it.

It exists because those are easy to confuse. A roadmap full of ticked boxes
reads like a capable engine, and an editor full of buttons looks like a working
tool. Twice now something has been described as working because it was written
down rather than because it ran.

## Keeping this current

The inventory is data: one TOML file per area in `docs/capabilities/`, from
which this index and each area's page are generated. **Edit the TOML in the
same commit as the change it describes**, then regenerate with
`cargo run -p sindri-capabilities -- --write`; `cargo test --workspace` fails
when a page is stale.

The generator refuses an entry that:

- names a test, proof project or document that does not exist;
- claims ✅ or 🟡 on any surface with no test and no proof project behind it,
  unless it says in `untested` how it was exercised instead — and then the page
  says so in a warning;
- names a `parity` row that `docs/parity.md` does not have, or whose
  Engine / Editor / Decay / Proof cells disagree with its `status`.

Update an entry when a change adds, removes or wires up a capability, turns
something under *Not yet* into something that is, or finds that an entry is
wrong. Every entry should be something someone ran, not something read in a
roadmap or inferred from a type signature. If you cannot demonstrate an entry,
correct it or delete it.

## Areas
";

fn render(source: &str, area: &Area) -> String {
    let mut page = String::new();
    let _ = writeln!(
        page,
        "<!-- Generated from {source} by `{REGENERATE_COMMAND}`. Edit the source, not this file. -->\n"
    );
    let _ = writeln!(page, "# {}\n", area.title);
    let _ = writeln!(page, "{}", area.intro.trim());
    page.push_str(
        "\nEach entry links its tests and the projects that exercise it. Surface \
         marks match the row in [parity](../parity.md) where one is named: \
         ✅ works and is exercised · 🟡 a useful slice, with the gap named · \
         ❌ absent · — not applicable.\n",
    );

    let mut groups: Vec<(&str, Vec<&Capability>)> = Vec::new();
    for capability in &area.capabilities {
        match groups
            .iter_mut()
            .find(|(name, _)| *name == capability.group)
        {
            Some((_, members)) => members.push(capability),
            None => groups.push((&capability.group, vec![capability])),
        }
    }

    page.push_str(
        "\n| Capability | Engine | Editor | Decay | Proof |\n| --- | :-: | :-: | :-: | :-: |\n",
    );
    for (group, members) in &groups {
        let _ = writeln!(page, "| **{group}** | | | | |");
        for capability in members {
            let [engine, editor, decay, proof] = capability.status.marks().map(Mark::symbol);
            let _ = writeln!(
                page,
                "| [{}](#{}) | {engine} | {editor} | {decay} | {proof} |",
                capability.title, capability.id
            );
        }
    }

    for (group, members) in &groups {
        let _ = writeln!(page, "\n## {group}");
        for capability in members {
            render_capability(&mut page, capability);
        }
    }

    page
}

fn render_capability(page: &mut String, capability: &Capability) {
    let _ = writeln!(page, "\n<a id=\"{}\"></a>\n", capability.id);
    let _ = writeln!(page, "### {}\n", capability.title);
    let _ = writeln!(page, "{}\n", capability.summary);
    let _ = write!(page, "{}", capability.status);
    if let Some(row) = &capability.parity {
        let _ = write!(page, " — parity row *{row}*");
    }
    page.push('\n');

    for (label, paths) in [
        ("Tested by", &capability.tests),
        ("Exercised in", &capability.proof),
        ("See", &capability.docs),
    ] {
        if !paths.is_empty() {
            let links: Vec<String> = paths
                .iter()
                .map(|path| format!("[`{path}`](../../{path})"))
                .collect();
            let _ = write!(page, "\n**{label}:** {}\n", links.join(", "));
        }
    }

    if let Some(untested) = &capability.untested {
        let _ = write!(
            page,
            "\n> ⚠️ **No automated test names this.** {untested}\n"
        );
    }

    if !capability.limits.is_empty() {
        page.push_str("\n**Limits:**\n\n");
        for limit in &capability.limits {
            let _ = writeln!(page, "- {limit}");
        }
    }

    let detail = capability.detail.trim();
    if !detail.is_empty() {
        let _ = write!(
            page,
            "\n<details markdown=\"1\">\n<summary>Evidence</summary>\n\n{detail}\n\n</details>\n"
        );
    }
}
