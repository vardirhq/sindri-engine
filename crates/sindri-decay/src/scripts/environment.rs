//! Building the environment the analyzer checks a script against.
//!
//! Derived from [`crate::surface`] rather than written out, so what a
//! script may say and what the host will answer cannot drift apart.

mod action;
mod grid;
mod physics;
mod tween;
mod world;

use grid::add_grid_surface;
use physics::add_physics_surface;
use world::add_world_surface;

use std::collections::BTreeSet;

use decay_semantic::{Environment, FunctionType, HostType, Type};
use sindri_core::{ComponentSchemaRegistry, World};

use crate::{
    ScriptComponent,
    audio_host::AUDIO,
    surface::{
        ANIMATION, ANIMATION_CALLS, AnimationCall, CONSTANTS, EFFECTS, EFFECTS_CALLS, ENTITY,
        EffectsCall, FUNCTIONS, GAME, GAME_CALLS, GAMEPAD, GameCall, HostFunction, Node, PRINT,
        PROFILE, PROFILE_CALLS, PROFILES, ProfileCall, RANDOM, RANDOM_CALLS, RandomCall, SAVE,
        SAVE_CALLS, SCENE, SCENE_CALLS, SEQUENCE, SEQUENCE_CALLS, SaveCall, SceneCall,
        SequenceCall, THIS, THROUGH_REFERENCE, TIME, TIME_VALUES, UI, UI_CALLS, UiCall,
        gamepad_type,
    },
};

/// What a Decay script may name, as types the analyzer can check.
///
/// Registered here rather than being builtins of the language, which is the
/// boundary Decay is built around: `decay-semantic` knows that `sin` exists
/// only because this said so, and knows nothing about what it does.
///
/// Every entry is derived from the same host surface the runtime implements. A
/// path the analyzer accepts and the host cannot answer is a clean compile
/// followed by a runtime failure, so namespaces are described and implemented
/// as one feature change.
#[must_use]
pub fn environment() -> Environment {
    let mut environment = Environment::new();

    for (name, node) in THIS {
        environment.add_this_value(*name, describe_node(node));
    }
    for (name, ty) in collect_types() {
        environment.add_type(name, ty);
    }

    for (name, _) in CONSTANTS {
        environment.add_value(*name, Type::F32);
    }
    for (name, function) in FUNCTIONS {
        environment.add_function(
            *name,
            FunctionType {
                params: match function {
                    HostFunction::Unary(_) => vec![Type::F32],
                    HostFunction::Binary(_) => vec![Type::F32, Type::F32],
                    HostFunction::Ternary(_) => vec![Type::F32, Type::F32, Type::F32],
                },
                return_type: Type::F32,
            },
        );
    }

    // `print` takes anything, because a script has no way to turn a number into
    // a string -- Decay has no conversions and `+` does not concatenate -- so a
    // print that only took text could not report a value.
    environment.add_function(
        PRINT,
        FunctionType {
            params: vec![Type::Unknown],
            return_type: Type::Unit,
        },
    );

    super::person_surface::add_input_surface(&mut environment);
    environment.add_type(GAMEPAD, gamepad_type());
    environment.add_value(GAMEPAD, Type::Named(GAMEPAD.to_owned()));

    let mut game = HostType::new();
    for (name, call) in GAME_CALLS {
        game = game.with_function(
            *name,
            FunctionType {
                params: vec![Type::String, Type::F32],
                return_type: match call {
                    GameCall::Get => Type::F32,
                    GameCall::Set => Type::Unit,
                },
            },
        );
    }
    environment.add_type(GAME, game);
    environment.add_value(GAME, Type::Named(GAME.to_owned()));

    let mut time = HostType::new();
    for (name, _) in TIME_VALUES {
        time = time.with_value(*name, Type::F32);
    }
    environment.add_type(TIME, time);
    environment.add_value(TIME, Type::Named(TIME.to_owned()));

    add_world_surface(&mut environment);
    tween::add_tween_surface(&mut environment);
    action::add_action_surface(&mut environment);
    add_profile_surface(&mut environment);

    super::person_surface::add_pointer_surface(&mut environment);
    super::person_surface::add_viewport_surface(&mut environment);
    super::person_surface::add_aim_surface(&mut environment);
    super::person_surface::add_gesture_surface(&mut environment);
    super::person_surface::add_camera_surface(&mut environment);
    add_physics_surface(&mut environment);
    crate::surface::physics3d::add_surface(&mut environment);
    add_ui_surface(&mut environment);
    add_animation_surface(&mut environment);
    add_sequence_surface(&mut environment);
    add_random_surface(&mut environment);
    add_save_surface(&mut environment);
    add_effects_surface(&mut environment);
    add_grid_surface(&mut environment);
    add_scene_surface(&mut environment);
    add_audio_surface(&mut environment);

    environment
}

fn add_profile_surface(environment: &mut Environment) {
    let profile = Type::Named(PROFILE.to_owned());
    let mut profiles = HostType::new();
    for (name, call) in PROFILE_CALLS {
        let (params, return_type) = match call {
            ProfileCall::Name | ProfileCall::Kind => (vec![profile.clone()], Type::String),
            ProfileCall::Number => (vec![profile.clone(), Type::String, Type::F32], Type::F32),
            ProfileCall::Text => (
                vec![profile.clone(), Type::String, Type::String],
                Type::String,
            ),
            ProfileCall::Flag => (vec![profile.clone(), Type::String, Type::Bool], Type::Bool),
            ProfileCall::Count => (vec![profile.clone(), Type::String], Type::F32),
            ProfileCall::NumberAt => (
                vec![
                    profile.clone(),
                    Type::String,
                    Type::F32,
                    Type::String,
                    Type::F32,
                ],
                Type::F32,
            ),
            ProfileCall::TextAt => (
                vec![
                    profile.clone(),
                    Type::String,
                    Type::F32,
                    Type::String,
                    Type::String,
                ],
                Type::String,
            ),
            ProfileCall::FlagAt => (
                vec![
                    profile.clone(),
                    Type::String,
                    Type::F32,
                    Type::String,
                    Type::Bool,
                ],
                Type::Bool,
            ),
        };
        profiles = profiles.with_function(
            *name,
            FunctionType {
                params,
                return_type,
            },
        );
    }
    environment.add_type(PROFILE, HostType::new());
    environment.add_type(PROFILES, profiles);
    environment.add_value(PROFILES, Type::Named(PROFILES.to_owned()));
}

/// Which authored clip an entity plays, and where it has got to.
///
/// A clip is named with text the same way an audio asset is: the scene authored
/// it, and a script picks from what the scene holds. There is no way to build
/// one, which is what keeps the editor's clip list the whole record of what an
/// entity can do.
pub(super) fn add_animation_surface(environment: &mut Environment) {
    let entity = || Type::Named(ENTITY.to_owned());
    let mut animation = HostType::new();
    for (name, call) in ANIMATION_CALLS {
        animation = animation.with_function(
            *name,
            FunctionType {
                params: match call {
                    AnimationCall::Play => vec![entity(), Type::String],
                    AnimationCall::Speed => vec![entity(), Type::F32],
                    _ => vec![entity()],
                },
                return_type: match call {
                    AnimationCall::Finished => Type::Bool,
                    AnimationCall::Frame => Type::F32,
                    AnimationCall::Clip => Type::String,
                    AnimationCall::Play
                    | AnimationCall::Restart
                    | AnimationCall::Stop
                    | AnimationCall::Speed => Type::Unit,
                },
            },
        );
    }
    environment.add_type(ANIMATION, animation);
    environment.add_value(ANIMATION, Type::Named(ANIMATION.to_owned()));
}

/// Playing an authored sequence, as `Animation` plays an authored clip.
pub(super) fn add_sequence_surface(environment: &mut Environment) {
    let entity = || Type::Named(ENTITY.to_owned());
    let mut sequence = HostType::new();
    for (name, call) in SEQUENCE_CALLS {
        sequence = sequence.with_function(
            *name,
            FunctionType {
                params: match call {
                    SequenceCall::Play | SequenceCall::Cued => vec![entity(), Type::String],
                    SequenceCall::Speed => vec![entity(), Type::F32],
                    _ => vec![entity()],
                },
                return_type: match call {
                    SequenceCall::Finished | SequenceCall::Cued => Type::Bool,
                    SequenceCall::Time => Type::F32,
                    SequenceCall::Name => Type::String,
                    SequenceCall::Play
                    | SequenceCall::Restart
                    | SequenceCall::Stop
                    | SequenceCall::Speed => Type::Unit,
                },
            },
        );
    }
    environment.add_type(SEQUENCE, sequence);
    environment.add_value(SEQUENCE, Type::Named(SEQUENCE.to_owned()));
}

/// What a script can change about a screen element.
///
/// The words stay in the scene and the numbers come from here: Decay cannot
/// build a string, so a HUD's template is authored and a script fills its
/// slots.
pub(super) fn add_ui_surface(environment: &mut Environment) {
    let entity = || Type::Named(ENTITY.to_owned());
    let mut ui = HostType::new();
    for (name, call) in UI_CALLS {
        ui = ui.with_function(
            *name,
            FunctionType {
                params: match call {
                    UiCall::Text | UiCall::SetInputText => vec![entity(), Type::String],
                    UiCall::Numbers => vec![entity(), Type::F32, Type::F32],
                    UiCall::Number
                    | UiCall::Fill
                    | UiCall::SliderSetValue
                    | UiCall::SetScrollOffset
                    | UiCall::SetSelected => {
                        vec![entity(), Type::F32]
                    }
                    UiCall::SetChecked => vec![entity(), Type::Bool],
                    _ => vec![entity()],
                },
                return_type: match call {
                    UiCall::SliderValue
                    | UiCall::ScrollOffset
                    | UiCall::Selected
                    | UiCall::Caret
                    | UiCall::SelectionStart
                    | UiCall::SelectionEnd => Type::F32,
                    UiCall::InputText => Type::String,
                    UiCall::Position | UiCall::Size => Type::Vec2,
                    UiCall::Hovered
                    | UiCall::Pressed
                    | UiCall::Held
                    | UiCall::SliderChanged
                    | UiCall::Checked
                    | UiCall::Changed
                    | UiCall::Submitted
                    | UiCall::Focused
                    | UiCall::Open => Type::Bool,
                    _ => Type::Unit,
                },
            },
        );
    }
    environment.add_type(UI, ui);
    environment.add_value(UI, Type::Named(UI.to_owned()));
}

/// What a script can draw from the run's stream.
pub(super) fn add_random_surface(environment: &mut Environment) {
    let mut random = HostType::new();
    for (name, call) in RANDOM_CALLS {
        random = random.with_function(
            *name,
            FunctionType {
                params: match call {
                    RandomCall::Value => Vec::new(),
                    RandomCall::Range | RandomCall::Int => vec![Type::F32, Type::F32],
                    RandomCall::Pick => vec![Type::array_of(Type::Named(ENTITY.to_owned()))],
                    RandomCall::Seed => vec![Type::F32],
                },
                return_type: match call {
                    RandomCall::Value | RandomCall::Range | RandomCall::Int => Type::F32,
                    RandomCall::Pick => Type::Named(ENTITY.to_owned()),
                    RandomCall::Seed => Type::Unit,
                },
            },
        );
    }
    environment.add_type(RANDOM, random);
    environment.add_value(RANDOM, Type::Named(RANDOM.to_owned()));
}

/// What a game remembers between runs.
pub(super) fn add_save_surface(environment: &mut Environment) {
    let mut save = HostType::new();
    for (name, call) in SAVE_CALLS {
        save = save.with_function(
            *name,
            FunctionType {
                params: match call {
                    SaveCall::Number | SaveCall::SetNumber => vec![Type::String, Type::F32],
                    SaveCall::Flag | SaveCall::SetFlag => vec![Type::String, Type::Bool],
                    SaveCall::Has => vec![Type::String],
                    _ => Vec::new(),
                },
                return_type: match call {
                    SaveCall::Number => Type::F32,
                    SaveCall::Flag
                    | SaveCall::Has
                    | SaveCall::IsNew
                    | SaveCall::IsDamaged
                    | SaveCall::IsFromNewer => Type::Bool,
                    SaveCall::SetNumber | SaveCall::SetFlag | SaveCall::Clear => Type::Unit,
                },
            },
        );
    }
    environment.add_type(SAVE, save);
    environment.add_value(SAVE, Type::Named(SAVE.to_owned()));
}

pub(super) fn add_scene_surface(environment: &mut Environment) {
    let mut scene = HostType::new();
    for (name, call) in SCENE_CALLS {
        scene = scene.with_function(
            *name,
            FunctionType {
                params: match call {
                    SceneCall::Go => vec![Type::String],
                    SceneCall::Current => Vec::new(),
                },
                return_type: match call {
                    SceneCall::Go => Type::Unit,
                    // Declared as text and answered as null before any
                    // scene has been entered, which is how `World.find`
                    // declares an entity it may not find.
                    SceneCall::Current => Type::String,
                },
            },
        );
    }
    environment.add_type(SCENE, scene);
    environment.add_value(SCENE, Type::Named(SCENE.to_owned()));
}

/// Short-lived visual flecks a script can throw.
pub(super) fn add_effects_surface(environment: &mut Environment) {
    let mut effects = HostType::new();
    for (name, call) in EFFECTS_CALLS {
        effects = effects.with_function(
            *name,
            FunctionType {
                params: match call {
                    EffectsCall::Burst => vec![Type::Named(ENTITY.to_owned())],
                    EffectsCall::BurstAt => {
                        vec![Type::Named(ENTITY.to_owned()), Type::F32, Type::F32]
                    }
                    EffectsCall::Live => Vec::new(),
                },
                // How many flecks were made, which is fewer than asked for when
                // the pool is full. A game can watch it and turn itself down.
                return_type: Type::F32,
            },
        );
    }
    environment.add_type(EFFECTS, effects);
    environment.add_value(EFFECTS, Type::Named(EFFECTS.to_owned()));
}

pub(super) fn add_audio_surface(environment: &mut Environment) {
    let audio = HostType::new()
        .with_function(
            "play",
            FunctionType {
                params: vec![Type::String, Type::F32],
                return_type: Type::Unit,
            },
        )
        .with_function(
            "loop",
            FunctionType {
                params: vec![Type::String, Type::F32],
                return_type: Type::Unit,
            },
        )
        .with_function(
            "play_on",
            FunctionType {
                params: vec![Type::String, Type::String, Type::F32],
                return_type: Type::Unit,
            },
        )
        .with_function(
            "loop_on",
            FunctionType {
                params: vec![Type::String, Type::String, Type::F32],
                return_type: Type::Unit,
            },
        )
        .with_function(
            "set_volume",
            FunctionType {
                params: vec![Type::String, Type::F32],
                return_type: Type::Unit,
            },
        )
        .with_function(
            "volume",
            FunctionType {
                params: vec![Type::String],
                return_type: Type::F32,
            },
        )
        .with_function(
            "stop_all",
            FunctionType {
                params: Vec::new(),
                return_type: Type::Unit,
            },
        )
        .with_function(
            "pause_all",
            FunctionType {
                params: Vec::new(),
                return_type: Type::Unit,
            },
        )
        .with_function(
            "resume_all",
            FunctionType {
                params: Vec::new(),
                return_type: Type::Unit,
            },
        );
    environment.add_type(AUDIO, audio);
    environment.add_value(AUDIO, Type::Named(AUDIO.to_owned()));
}

/// The type a node has: a group is its name, a leaf is a number.
pub(super) fn describe_node(node: &Node) -> Type {
    match node {
        // The transform's position and scale are the language's own `Vec3`,
        // not a host type of that name: a script can hold, add and pass one.
        Node::Group(name, _) if *name == crate::surface::names::VEC3 => Type::Vec3,
        // And a color is the language's `Color`: held, blended and assigned
        // whole, and still read channel by channel.
        Node::Group(name, _) if *name == crate::surface::names::RGBA => Type::Color,
        Node::Group(name, _) => Type::Named((*name).to_owned()),
        Node::Leaf(_) => Type::F32,
        Node::Handle(_) => Type::Named(ENTITY.to_owned()),
    }
}

/// Every named type the surface tree mentions, with its members.
pub(super) fn collect_types() -> Vec<(String, HostType)> {
    pub(super) fn walk(
        members: &'static [(&'static str, Node)],
        into: &mut Vec<(String, HostType)>,
    ) {
        for (_, node) in members {
            let Node::Group(name, nested) = node else {
                continue;
            };
            if *name == crate::surface::names::VEC3 || *name == crate::surface::names::RGBA {
                continue;
            }
            let mut ty = HostType::new();
            for (field, child) in *nested {
                ty = if child.is_read_only() {
                    ty.with_read_only_value(*field, describe_node(child))
                } else {
                    ty.with_value(*field, describe_node(child))
                };
            }
            if *name == crate::surface::TRANSFORM {
                ty = crate::surface::transform::add_calls(ty);
            }
            into.push(((*name).to_owned(), ty));
            walk(nested, into);
        }
    }
    let mut types = Vec::new();
    walk(THIS, &mut types);

    let mut entity = HostType::new();
    for (field, node) in THROUGH_REFERENCE {
        entity = entity.with_value(*field, describe_node(node));
    }
    types.push((ENTITY.to_owned(), entity));
    types
}

pub fn referenced_sources(world: &World, components: &ComponentSchemaRegistry) -> BTreeSet<String> {
    components
        .query::<ScriptComponent>(world)
        .unwrap_or_default()
        .into_iter()
        .map(|(_, component)| component.source)
        .collect()
}
