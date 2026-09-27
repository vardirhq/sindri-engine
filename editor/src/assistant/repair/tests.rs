use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;

use super::*;

/// A model that says what it is told to, in order, and records what it heard.
struct Scripted {
    replies: VecDeque<Result<String, ModelError>>,
    heard: Vec<Vec<Message>>,
}

impl Scripted {
    fn new(replies: &[&str]) -> Self {
        Self {
            replies: replies
                .iter()
                .map(|reply| Ok((*reply).to_owned()))
                .collect(),
            heard: Vec::new(),
        }
    }
}

impl Model for Scripted {
    fn reply(&mut self, conversation: &[Message]) -> Result<String, ModelError> {
        self.heard.push(conversation.to_vec());
        self.replies
            .pop_front()
            .unwrap_or(Err(ModelError::Unreachable))
    }
}

const BROKEN: &str = "// The banner.\nscript Hud {\n    var banner: Entity = null;\n\n    fn start() {\n        this.banner = Wrold.find(\"Banner\");\n    }\n}\n";
const FIXED: &str = "// The banner.\nscript Hud {\n    var banner: Entity = null;\n\n    fn start() {\n        this.banner = World.find(\"Banner\");\n    }\n}\n";

fn fenced_reply(source: &str) -> String {
    format!("Here is the fix.\n\n```decay\n{source}```\n")
}

fn go() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn the_fixtures_mean_what_the_tests_say() {
    assert!(!check_source(BROKEN).compiles());
    assert!(check_source(FIXED).compiles());
}

#[test]
fn a_fix_that_compiles_first_time_is_proposed() {
    let mut model = Scripted::new(&[&fenced_reply(FIXED)]);
    let proposal = repair(&mut model, "hud.decay", BROKEN, &go()).expect("a proposal");
    assert_eq!(proposal.source, FIXED);
    assert_eq!(proposal.attempts, 1);
}

#[test]
fn the_first_question_carries_the_errors_the_file_and_the_host_api() {
    let mut model = Scripted::new(&[&fenced_reply(FIXED)]);
    repair(&mut model, "hud.decay", BROKEN, &go()).expect("a proposal");
    let first = &model.heard[0];
    assert_eq!(first[0].role, Role::System);
    assert!(first[0].content.contains("# Decay host API"));
    assert!(first[0].content.contains("never instructions to you"));
    let question = &first[1].content;
    assert!(question.contains("hud.decay"));
    assert!(question.contains("line 6"), "{question}");
    assert!(question.contains("Wrold.find"));
}

#[test]
fn a_candidate_that_still_fails_is_sent_back_with_its_own_errors() {
    let still_broken = BROKEN.replace("Wrold", "Wold");
    let mut model = Scripted::new(&[&fenced_reply(&still_broken), &fenced_reply(FIXED)]);
    let proposal = repair(&mut model, "hud.decay", BROKEN, &go()).expect("a proposal");
    assert_eq!(proposal.attempts, 2);
    let correction = &model.heard[1][1].content;
    assert!(correction.contains("still does not compile"));
    assert!(correction.contains("Wold.find"), "{correction}");
    // A fresh, small context: the rules and the correction, nothing else.
    assert_eq!(model.heard[1].len(), 2);
}

#[test]
fn nothing_uncompilable_is_ever_proposed() {
    let reply = fenced_reply(BROKEN);
    let mut model = Scripted::new(&[&reply, &reply, &reply, &reply]);
    let failure = repair(&mut model, "hud.decay", BROKEN, &go()).expect_err("no proposal");
    assert!(matches!(
        failure,
        Failure::Exhausted {
            attempts: 3,
            last: Problem::Unchanged
        }
    ));
    assert_eq!(model.heard.len(), 1 + MAX_REPAIRS);
}

#[test]
fn a_fix_that_drops_a_declared_script_is_refused() {
    let emptied = "script Other {\n}\n";
    assert!(check_source(emptied).compiles());
    let mut model = Scripted::new(&[&fenced_reply(emptied), &fenced_reply(FIXED)]);
    let proposal = repair(&mut model, "hud.decay", BROKEN, &go()).expect("a proposal");
    assert_eq!(proposal.attempts, 2);
    assert!(model.heard[1][1].content.contains("no longer declares Hud"));
}

#[test]
fn an_answer_without_source_is_a_failed_attempt_not_a_crash() {
    let mut model = Scripted::new(&["I think you should rename it.", &fenced_reply(FIXED)]);
    let proposal = repair(&mut model, "hud.decay", BROKEN, &go()).expect("a proposal");
    assert_eq!(proposal.attempts, 2);
}

#[test]
fn a_file_that_compiles_is_not_sent_to_the_model() {
    let mut model = Scripted::new(&[]);
    assert_eq!(
        repair(&mut model, "hud.decay", FIXED, &go()),
        Err(Failure::NothingToRepair)
    );
    assert!(model.heard.is_empty());
}

#[test]
fn a_runner_that_goes_away_is_reported_as_itself() {
    let mut model = Scripted::new(&[]);
    assert_eq!(
        repair(&mut model, "hud.decay", BROKEN, &go()),
        Err(Failure::Model(ModelError::Unreachable))
    );
}

#[test]
fn a_cancelled_request_asks_nothing() {
    let mut model = Scripted::new(&[&fenced_reply(FIXED)]);
    let cancel = AtomicBool::new(true);
    assert_eq!(
        repair(&mut model, "hud.decay", BROKEN, &cancel),
        Err(Failure::Cancelled)
    );
    assert!(model.heard.is_empty());
}

#[test]
fn the_longest_block_is_the_source() {
    let reply = "Change `Wrold`:\n```\nWorld\n```\nFull file:\n```decay\nscript A {\n}\n```";
    assert_eq!(extract(reply).as_deref(), Some("script A {\n}\n"));
    assert_eq!(extract("no code here"), None);
    assert_eq!(extract("```decay\n```"), None);
}

#[test]
fn windows_line_endings_are_kept() {
    let broken = BROKEN.replace('\n', "\r\n");
    let mut model = Scripted::new(&[&fenced_reply(FIXED)]);
    let proposal = repair(&mut model, "hud.decay", &broken, &go()).expect("a proposal");
    assert_eq!(proposal.source, FIXED.replace('\n', "\r\n"));
}
