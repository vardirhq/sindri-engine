//! The check that makes the generated documents worth reading.
//!
//! Without it they are a snapshot of whatever the engine looked like the day
//! someone last remembered, which is the failure mode of every hand-maintained
//! API reference. With it, widening the Decay surface or registering a
//! component without regenerating is a failing test rather than a reference
//! that quietly lies to whoever reads it next.

use std::fs;

use sindri_capabilities::{REGENERATE_COMMAND, documents, repository_root};

#[test]
fn the_generated_documents_match_the_engine() {
    let root = repository_root();

    for document in documents().expect("the engine describes itself") {
        let found = fs::read_to_string(root.join(&document.path)).unwrap_or_else(|error| {
            panic!(
                "{} could not be read ({error}); run `{REGENERATE_COMMAND}`",
                document.path
            )
        });

        assert_eq!(
            found, document.contents,
            "{} is out of date; run `{REGENERATE_COMMAND}`",
            document.path
        );
    }
}
