//! Putting a model to the test before the editor offers anything it does.
//!
//! A runner that answers is not an assistant that works. Each case here hands
//! the model a small, known problem and grades the answer with Sindri's own
//! compiler, so a feature is switched on because the model did it — here, on
//! this machine — and not because a table said it should.
//!
//! Only features something in the editor uses are tested. A case for a feature
//! nothing offers would be a promise with nothing behind it.

use std::sync::atomic::AtomicBool;

use super::Feature;
use super::chat::Model;
use super::repair::{Failure, repair};

/// A small script with one mistake of a kind models are known to make in
/// Decay, and what the file is called when the model is shown it.
struct Case {
    file: &'static str,
    broken: &'static str,
}

/// Every case must be repaired for the feature to count as verified.
///
/// Two kinds of mistake rather than one, so a model that happens to get one
/// right by luck is not enough: a misspelt host name, and calling a script's
/// own function as though it were a method — the error `this.` invites from
/// anyone who has written another language.
const REPAIR_CASES: [Case; 2] = [
    Case {
        file: "banner.decay",
        broken: "script Banner {\n    var banner: Entity = null;\n\n    fn start() {\n        \
                 this.banner = Wrold.find(\"Banner\");\n    }\n}\n",
    },
    Case {
        file: "lamp.decay",
        broken: "script Lamp {\n    var level: f32 = 0.0;\n\n    fn update(dt: f32) {\n        \
                 this.level = this.level + dt;\n        this.glow(this.level);\n    }\n\n    \
                 fn glow(amount: f32) {\n        let seen = amount;\n    }\n}\n",
    },
];

/// What verification found.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    pub verified: Vec<Feature>,
    /// Why a tested feature did not pass, for the person to read.
    pub notes: Vec<String>,
}

/// Runs every case against the model.
pub fn verify(model: &mut dyn Model, cancel: &AtomicBool) -> Verdict {
    let mut notes = Vec::new();
    let mut repaired = true;
    for case in &REPAIR_CASES {
        match repair(model, case.file, case.broken, cancel) {
            Ok(_) => {}
            Err(failure) => {
                repaired = false;
                notes.push(format!(
                    "{} could not repair {}: {failure}",
                    Feature::DecayRepair.label(),
                    case.file
                ));
                // A runner that has gone away will not come back for the next
                // case, and asking again only doubles the wait.
                if matches!(failure, Failure::Model(_) | Failure::Cancelled) {
                    break;
                }
            }
        }
    }
    Verdict {
        verified: if repaired {
            vec![Feature::DecayRepair]
        } else {
            Vec::new()
        },
        notes,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use sindri_decay::check_source;

    use super::super::chat::{Message, ModelError};
    use super::*;

    const FIXES: [&str; 2] = [
        "script Banner {\n    var banner: Entity = null;\n\n    fn start() {\n        \
         this.banner = World.find(\"Banner\");\n    }\n}\n",
        "script Lamp {\n    var level: f32 = 0.0;\n\n    fn update(dt: f32) {\n        \
         this.level = this.level + dt;\n        glow(this.level);\n    }\n\n    \
         fn glow(amount: f32) {\n        let seen = amount;\n    }\n}\n",
    ];

    struct Scripted(VecDeque<String>);

    impl Model for Scripted {
        fn reply(&mut self, _: &[Message]) -> Result<String, ModelError> {
            self.0.pop_front().ok_or(ModelError::Unreachable)
        }
    }

    fn answers(sources: &[&str]) -> Scripted {
        Scripted(
            sources
                .iter()
                .map(|source| format!("```decay\n{source}```"))
                .collect(),
        )
    }

    #[test]
    fn every_case_is_broken_and_has_a_fix_the_compiler_accepts() {
        for (case, fix) in REPAIR_CASES.iter().zip(FIXES) {
            assert!(!check_source(case.broken).compiles(), "{}", case.file);
            assert!(
                check_source(fix).compiles(),
                "{}: {:?}",
                case.file,
                check_source(fix).diagnostics
            );
        }
    }

    #[test]
    fn a_model_that_repairs_every_case_is_verified_for_repair() {
        let mut model = answers(&FIXES);
        let verdict = verify(&mut model, &AtomicBool::new(false));
        assert_eq!(verdict.verified, [Feature::DecayRepair]);
        assert!(verdict.notes.is_empty());
    }

    #[test]
    fn one_case_it_cannot_repair_is_enough_to_withhold_it() {
        // The first case repaired; the second answered with its broken self
        // every round.
        let broken = REPAIR_CASES[1].broken;
        let mut model = answers(&[FIXES[0], broken, broken, broken]);
        let verdict = verify(&mut model, &AtomicBool::new(false));
        assert!(verdict.verified.is_empty());
        assert_eq!(verdict.notes.len(), 1);
        assert!(verdict.notes[0].contains("lamp.decay"));
    }

    #[test]
    fn a_runner_that_goes_away_stops_the_run_at_once() {
        let mut model = answers(&[]);
        let verdict = verify(&mut model, &AtomicBool::new(false));
        assert!(verdict.verified.is_empty());
        assert_eq!(verdict.notes.len(), 1, "{:?}", verdict.notes);
    }
}
