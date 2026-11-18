//! Decay questions: does this draft compile in this project, and what is one
//! name in the host API.
//!
//! A draft is checked against the project's real environment, every other
//! script in it a type the draft may name, as `project check` does for files
//! on disk; so an agent checks a script before writing it, and checks a
//! change to one before saving it. The API is the generated reference the
//! analyzer itself checks against, asked one name at a time instead of
//! read whole.

use serde::Serialize;
use serde_json::Value;
use sindri_decay::{CheckPhase, ScriptSources, check_source_in};

use super::Project;
use crate::report::{Diagnostic, SCHEMA_VERSION};

/// `script check`.
#[derive(Serialize)]
pub struct ScriptChecked {
    pub schema_version: u32,
    pub file: String,
    pub ok: bool,
    /// The `script` and `component` containers it declares.
    pub declared: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Checks `source` as the project's script `file`, in place of what is on
/// disk there, if anything.
#[must_use]
pub fn check(project: &Project, file: &str, source: &str) -> ScriptChecked {
    let mut sources = ScriptSources::new();
    for (id, text) in &project.sources {
        let decay = std::path::Path::new(id)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("decay"));
        if decay && id != file {
            sources.insert(id.clone(), text.clone());
        }
    }
    sources.insert(file.to_owned(), source.to_owned());
    let checked = check_source_in(source, sources.environment());
    let diagnostics: Vec<Diagnostic> = checked
        .diagnostics
        .iter()
        .map(|found| {
            let code = match found.phase {
                CheckPhase::Syntax => format!("DECAY_SYNTAX_{}", screaming(found.code)),
                CheckPhase::Semantic => format!("DECAY_{}", screaming(found.code)),
            };
            Diagnostic::error(&code, file, found.message.clone()).at(found.line, found.column)
        })
        .collect();
    ScriptChecked {
        schema_version: SCHEMA_VERSION,
        file: file.to_owned(),
        ok: diagnostics.is_empty(),
        declared: checked.declared,
        diagnostics,
    }
}

fn screaming(code: &str) -> String {
    code.replace('-', "_").to_ascii_uppercase()
}

impl ScriptChecked {
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for found in &self.diagnostics {
            let _ = writeln!(
                out,
                "{}:{}:{}: error[{}]: {}",
                self.file,
                found.line.unwrap_or(0),
                found.column.unwrap_or(0),
                found.code,
                found.message
            );
        }
        let _ = writeln!(
            out,
            "{}: declares {}",
            if self.ok { "ok" } else { "failed" },
            if self.declared.is_empty() {
                "nothing".to_owned()
            } else {
                self.declared.join(", ")
            }
        );
        out
    }
}

/// `script api`.
#[derive(Serialize)]
pub struct ApiAnswer {
    pub schema_version: u32,
    pub name: String,
    /// Each entry the name reaches: a global (`Physics`), a type (`Entity`),
    /// a member (`Physics.mask`, `this.entity`), with the type's members when
    /// the entry is a value of one.
    pub found: Vec<Value>,
    /// Near names, when nothing matched.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nearest: Vec<String>,
}

/// Looks `name` up in the host API.
#[must_use]
pub fn api(name: &str) -> ApiAnswer {
    let reference = sindri_capabilities::reference("decay")
        .ok()
        .flatten()
        .unwrap_or_default();
    let list = |key: &str| {
        reference
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let (globals, types, this) = (list("globals"), list("types"), list("this"));
    let named = |entries: &[Value], wanted: &str| {
        entries
            .iter()
            .find(|entry| entry.get("name").and_then(Value::as_str) == Some(wanted))
            .cloned()
    };
    let type_of = |entry: &Value| {
        entry
            .get("type")
            .and_then(Value::as_str)
            .and_then(|kind| named(&types, kind))
    };
    let mut found = Vec::new();
    match name.split_once('.') {
        None => {
            if let Some(global) = named(&globals, name) {
                let mut global = global;
                if let Some(kind) = type_of(&global) {
                    global["members"] = kind.get("members").cloned().unwrap_or_default();
                }
                found.push(global);
            }
            if let Some(kind) = named(&types, name) {
                found.push(kind);
            }
        }
        Some((owner, member)) => {
            let members = if owner == "this" {
                this.clone()
            } else {
                named(&globals, owner)
                    .and_then(|global| type_of(&global))
                    .or_else(|| named(&types, owner))
                    .and_then(|kind| kind.get("members").and_then(Value::as_array).cloned())
                    .unwrap_or_default()
            };
            if let Some(mut entry) = named(&members, member) {
                entry["owner"] = Value::from(owner);
                found.push(entry);
            }
        }
    }
    let nearest = if found.is_empty() {
        nearest(name, &globals, &types, &this)
    } else {
        Vec::new()
    };
    ApiAnswer {
        schema_version: SCHEMA_VERSION,
        name: name.to_owned(),
        found,
        nearest,
    }
}

/// The names nearest `name`, members by their owner, at most five.
fn nearest(name: &str, globals: &[Value], types: &[Value], this: &[Value]) -> Vec<String> {
    let names = |entries: &[Value], prefix: &str| -> Vec<String> {
        entries
            .iter()
            .filter_map(|entry| entry.get("name").and_then(Value::as_str))
            .map(|entry| format!("{prefix}{entry}"))
            .collect()
    };
    let mut every = names(globals, "");
    every.extend(names(types, ""));
    every.extend(names(this, "this."));
    for kind in types {
        let owner = kind.get("name").and_then(Value::as_str).unwrap_or_default();
        let members = kind
            .get("members")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        every.extend(names(&members, &format!("{owner}.")));
    }
    let wanted = name.to_lowercase();
    let mut scored: Vec<(usize, String)> = every
        .into_iter()
        .map(|candidate| {
            let lower = candidate.to_lowercase();
            let score = if lower.contains(&wanted) {
                0
            } else {
                distance(&wanted, &lower)
            };
            (score, candidate)
        })
        .filter(|(score, _)| *score <= 3)
        .collect();
    scored.sort();
    scored.dedup_by(|a, b| a.1 == b.1);
    scored.into_iter().take(5).map(|(_, name)| name).collect()
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, right) in b.iter().enumerate() {
            let here = row[j + 1];
            row[j + 1] = (row[j + 1] + 1)
                .min(row[j] + 1)
                .min(previous + usize::from(left != *right));
            previous = here;
        }
    }
    row[b.len()]
}

impl ApiAnswer {
    #[must_use]
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for entry in &self.found {
            let text = |key: &str| entry.get(key).and_then(Value::as_str).unwrap_or_default();
            let owner = entry
                .get("owner")
                .and_then(Value::as_str)
                .map(|owner| format!("{owner}."))
                .unwrap_or_default();
            let _ = writeln!(out, "{owner}{}{}", text("name"), signature(entry));
            if !text("description").is_empty() {
                let _ = writeln!(out, "  {}", text("description"));
            }
            for member in entry
                .get("members")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let name = member
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let _ = writeln!(out, "  .{name}{}", signature(member));
            }
        }
        if self.found.is_empty() {
            let _ = writeln!(out, "no {} in the host API", self.name);
            if !self.nearest.is_empty() {
                let _ = writeln!(out, "nearest: {}", self.nearest.join(", "));
            }
        }
        out
    }
}

/// `(a: T, b: U) -> R` for a function, `: T` for a value.
fn signature(entry: &Value) -> String {
    let strings = |key: &str| -> Vec<String> {
        entry
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    };
    if entry.get("kind").and_then(Value::as_str) == Some("function") {
        let names = strings("parameter_names");
        let kinds = strings("parameters");
        let parameters: Vec<String> = kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| match names.get(index) {
                Some(name) => format!("{name}: {kind}"),
                None => kind.clone(),
            })
            .collect();
        let returns = entry
            .get("returns")
            .and_then(Value::as_str)
            .unwrap_or("nothing");
        format!("({}) -> {returns}", parameters.join(", "))
    } else {
        entry
            .get("type")
            .and_then(Value::as_str)
            .map(|kind| format!(": {kind}"))
            .unwrap_or_default()
    }
}
