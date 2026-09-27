//! What a project's scripts declare, and what another script may say about them.

use decay_ir::lower_with_environment;
use decay_semantic::Type;

use super::*;
use crate::environment;

const BOLT: &str = "script Bolt {
    var damage: f32 = 1.0;
    var speed = 12.0;
    var heading = Vec2(0.0, 1.0);
    let label = \"bolt\";
    var owner: Entity = null;
    fn start() {}
    fn update(dt: f32) {}
    fn hit(amount: f32) {}
}";

fn diagnostics(sources: &[&str], source: &str) -> Vec<String> {
    let mut environment = environment();
    let project = Project::read(sources.iter().copied(), &environment);
    project.describe(&mut environment);
    lower_with_environment(source, &environment)
        .analysis
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[test]
fn a_script_s_fields_and_messages_are_declared_with_their_types() {
    let project = Project::read([BOLT], &environment());
    let bolt = &project.scripts["Bolt"];
    assert_eq!(
        bolt.fields,
        [
            ("damage".to_owned(), Type::F32),
            ("speed".to_owned(), Type::F32),
            ("heading".to_owned(), Type::Vec2),
            ("label".to_owned(), Type::String),
            ("owner".to_owned(), Type::Named("Entity".to_owned())),
        ]
    );
    // Lifecycle functions are the engine's to call, not messages.
    assert_eq!(bolt.messages, [("hit".to_owned(), vec![Type::F32])]);
}

#[test]
fn another_script_reads_writes_and_messages_one_by_type() {
    let turret = "script Turret {
        fn update(dt: f32) {
            let bolt = Bolt.on(this.entity);
            if bolt != null {
                bolt.damage += 1.0;
                bolt.heading = Vec2(1.0, 0.0);
                bolt.hit(2.0);
            }
        }
    }";
    assert_eq!(diagnostics(&[BOLT, turret], turret), Vec::<String>::new());
}

#[test]
fn a_mistake_about_another_script_is_a_compile_error() {
    let said = |body: &str| {
        let source = format!("script Turret {{ fn update(dt: f32) {{ {body} }} }}");
        diagnostics(&[BOLT, &source], &source).join("\n")
    };
    assert!(said("let b = Bolt.on(this.entity); b.damge = 1.0;").contains("no member `damge`"));
    assert!(said("let b = Bolt.on(this.entity); b.damage = \"lots\";").contains("cannot assign"));
    assert!(said("let b = Bolt.on(this.entity); b.hit();").contains("expected 1 argument"));
    assert!(
        said("let b = Bolt.on(this.entity); let x = b.hit(1.0) + 1.0;").contains("expected `f32`")
    );
    assert!(said("let b = Blot.on(this.entity);").contains("unknown name `Blot`"));
}

/// A name nobody can tell apart is not offered: which of two `Bolt`s a script
/// meant would be a guess.
#[test]
fn a_name_declared_twice_or_taken_by_the_engine_is_left_out() {
    let project = Project::read([BOLT, BOLT, "script World {}"], &environment());
    assert!(!project.scripts.contains_key("Bolt"));
    assert!(!project.scripts.contains_key("World"));
}

/// A program compiled against one shape is recompiled against another, and
/// only a change to a declaration changes the shape.
#[test]
fn the_key_follows_declarations_not_bodies() {
    let reserved = environment();
    let key = |source: &str| Project::read([source], &reserved).key().to_owned();
    let first = key("script A { var x: f32 = 1.0; fn f() { let y = 1.0; } }");
    let body_changed = key("script A { var x: f32 = 1.0; fn f() { let y = 2.0; } }");
    let field_added = key("script A { var x: f32 = 1.0; var z: f32 = 0.0; fn f() {} }");
    assert_eq!(first, body_changed);
    assert_ne!(first, field_added);
}
