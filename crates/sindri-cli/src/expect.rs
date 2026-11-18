//! `expect:` steps: what a run must have reached by the time it gets there.
//!
//! A run that reports numbers leaves the agent to read them and decide; one
//! that states what it expects is a test the agent wrote, and fails on its
//! own when the game stops doing it. Each expectation is checked where it
//! stands in the steps, so `wait:2 expect:score>=1` means "by two seconds
//! in", and a failed one is recorded with what was there instead rather than
//! ending the run, so one run answers every question it asks.
//!
//! ```text
//! expect:<board name><op><number>   a shared board value: expect:wood>=2
//! expect:<entity>.x<op><number>     where an entity is, by stable ID: expect:wick.x>13
//! expect:<entity>.y<op><number>
//! expect:scene=<name>               the scene playing, by file name: expect:scene=wilds
//! expect:printed=<text>             some script has printed a line holding <text>
//! ```
//!
//! `<op>` is one of `=`, `!=`, `<`, `<=`, `>`, `>=`.

use serde::Serialize;

/// One expectation, read.
#[derive(Clone, Debug, PartialEq)]
pub struct Expectation {
    pub subject: Subject,
    pub op: Op,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    Board(String),
    Position { entity: String, axis: usize },
    Scene,
    Printed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

/// What a run found when it reached an expectation.
#[derive(Clone, Debug, Serialize)]
pub struct Checked {
    pub frame: u32,
    /// The step as written.
    pub step: String,
    pub passed: bool,
    /// What was there: a number, a scene, or `nothing` when it is missing.
    pub actual: String,
}

/// What an expectation is checked against, at the moment it is reached.
pub trait State {
    fn board(&self, name: &str) -> Option<f64>;
    fn position(&self, entity: &str) -> Option<[f32; 2]>;
    fn scene(&self) -> Option<String>;
    fn printed(&self) -> Vec<String>;
}

/// Reads `expect:`'s argument.
///
/// # Errors
/// No operator, an empty side, or a number where one is needed that is not.
pub fn parse(text: &str) -> Result<Expectation, String> {
    const OPS: [(&str, Op); 6] = [
        (">=", Op::GreaterOrEqual),
        ("<=", Op::LessOrEqual),
        ("!=", Op::NotEqual),
        ("=", Op::Equal),
        (">", Op::Greater),
        ("<", Op::Less),
    ];
    let (at, spelling, op) = OPS
        .iter()
        .filter_map(|(spelling, op)| text.find(spelling).map(|at| (at, *spelling, *op)))
        // The leftmost operator, and of two at one place the longer: `>=`
        // rather than `>` followed by `=1`.
        .min_by_key(|(at, spelling, _)| (*at, std::cmp::Reverse(spelling.len())))
        .ok_or_else(|| format!("expect:{text} has no comparison; write expect:<what><op><value>, op one of = != < <= > >="))?;
    let (left, value) = (text[..at].trim(), text[at + spelling.len()..].trim());
    if left.is_empty() || value.is_empty() {
        return Err(format!(
            "expect:{text} needs something on each side of {spelling}"
        ));
    }
    let subject = match left {
        "scene" => Subject::Scene,
        "printed" => Subject::Printed,
        _ => match left.rsplit_once('.') {
            Some((entity, "x")) if !entity.is_empty() => Subject::Position {
                entity: entity.to_owned(),
                axis: 0,
            },
            Some((entity, "y")) if !entity.is_empty() => Subject::Position {
                entity: entity.to_owned(),
                axis: 1,
            },
            _ => Subject::Board(left.to_owned()),
        },
    };
    let textual = matches!(subject, Subject::Scene | Subject::Printed);
    if textual && !matches!(op, Op::Equal | Op::NotEqual) {
        return Err(format!(
            "expect:{text}: {left} is compared with = or != only"
        ));
    }
    if !textual && value.parse::<f64>().is_err() {
        return Err(format!("expect:{text}: {value} is not a number"));
    }
    Ok(Expectation {
        subject,
        op,
        value: value.to_owned(),
    })
}

impl Expectation {
    /// Whether `state` meets it, and what was there.
    pub fn check(&self, state: &impl State) -> (bool, String) {
        match &self.subject {
            Subject::Board(name) => self.number(state.board(name)),
            Subject::Position { entity, axis } => {
                self.number(state.position(entity).map(|at| f64::from(at[*axis])))
            }
            Subject::Scene => {
                let scene = state.scene();
                let found = scene
                    .as_deref()
                    .is_some_and(|scene| names(scene, &self.value));
                (
                    found == (self.op == Op::Equal),
                    scene.unwrap_or_else(|| "nothing".to_owned()),
                )
            }
            Subject::Printed => {
                let printed = state.printed();
                let found = printed.iter().any(|line| line.contains(&self.value));
                let actual = match printed.last() {
                    Some(last) => format!("{} lines, the last {last:?}", printed.len()),
                    None => "nothing printed".to_owned(),
                };
                (found == (self.op == Op::Equal), actual)
            }
        }
    }

    fn number(&self, actual: Option<f64>) -> (bool, String) {
        let Some(actual) = actual else {
            return (false, "nothing".to_owned());
        };
        let wanted: f64 = self.value.parse().unwrap_or(f64::NAN);
        // Positions are f32 and the board holds what scripts wrote, so
        // equality allows for the last bits of a float.
        let near = (actual - wanted).abs() <= 1e-4 * wanted.abs().max(1.0);
        let passed = match self.op {
            Op::Equal => near,
            Op::NotEqual => !near,
            Op::Less => actual < wanted,
            Op::LessOrEqual => actual <= wanted || near,
            Op::Greater => actual > wanted,
            Op::GreaterOrEqual => actual >= wanted || near,
        };
        (passed, format!("{actual}"))
    }
}

/// Whether the scene `id` is the one called `name`: by its whole ID, its file
/// name, or its file name without `.scene`.
fn names(id: &str, name: &str) -> bool {
    let file = id.rsplit(['/', '\\']).next().unwrap_or(id);
    id == name || file == name || file.strip_suffix(".scene") == Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl State for Fake {
        fn board(&self, name: &str) -> Option<f64> {
            (name == "wood").then_some(2.0)
        }
        fn position(&self, entity: &str) -> Option<[f32; 2]> {
            (entity == "wick").then_some([13.5, 6.0])
        }
        fn scene(&self) -> Option<String> {
            Some("assets/wilds.scene".to_owned())
        }
        fn printed(&self) -> Vec<String> {
            vec!["lit the lamp".to_owned()]
        }
    }

    fn passes(text: &str) -> bool {
        parse(text).unwrap().check(&Fake).0
    }

    #[test]
    fn each_kind_of_expectation_reads_and_checks() {
        for holds in [
            "wood>=2",
            "wood=2",
            "wood<3",
            "wood!=5",
            "wick.x>13",
            "wick.y<=6",
            "scene=wilds",
            "scene=assets/wilds.scene",
            "scene!=title",
            "printed=lamp",
            "printed!=gloom",
        ] {
            assert!(passes(holds), "{holds} should hold");
        }
        for fails in [
            "wood>2",
            "wood=3",
            "wick.x<13",
            "scene=title",
            "printed=gloom",
            "stone>=0",
            "ghost.x>0",
        ] {
            assert!(!passes(fails), "{fails} should fail");
        }
        assert_eq!(parse("stone>=0").unwrap().check(&Fake).1, "nothing");
    }

    #[test]
    fn what_does_not_read_says_why() {
        for wrong in ["wood", "=2", "wood>=", "wood>=lots", "scene>2", "printed<x"] {
            assert!(parse(wrong).is_err(), "{wrong} should not read");
        }
    }
}
