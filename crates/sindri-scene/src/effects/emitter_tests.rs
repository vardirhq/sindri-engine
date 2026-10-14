//! What an emitter throws, how many, and where they go.

use std::time::Duration;

use sindri_core::{EntityData, EntityId, SceneComponent, Transform3D, World};

use super::super::{Effects2d, ParticleSpace, ParticlesComponent};
use crate::SceneExtractor;

const STEP: Duration = Duration::from_micros(16_667);

fn world_with(particles: &ParticlesComponent) -> (World, EntityId) {
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ParticlesComponent::TYPE_NAME.to_owned(),
            serde_json::to_value(particles).unwrap(),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    (world, entity)
}

fn fountain() -> ParticlesComponent {
    ParticlesComponent {
        texture: "spark.png".to_owned(),
        rate: 30.0,
        lifetime: 10.0,
        ..ParticlesComponent::default()
    }
}

fn run(effects: &mut Effects2d, world: &World, steps: usize) {
    let components = SceneExtractor::new().unwrap();
    for _ in 0..steps {
        effects.advance(STEP);
        let problems = effects.emit(world, components.components(), STEP).unwrap();
        assert!(problems.is_empty(), "{problems:?}");
    }
}

#[test]
fn an_emitter_throws_its_rate_each_second() {
    let (world, entity) = world_with(&fountain());
    let mut effects = Effects2d::default();
    run(&mut effects, &world, 60);
    let thrown = effects.emitter_live(entity);
    assert!((29..=31).contains(&thrown), "{thrown} in a second at 30 a second");
}

#[test]
fn a_stopped_emitter_throws_nothing() {
    let (world, _) = world_with(&ParticlesComponent {
        playing: false,
        ..fountain()
    });
    let mut effects = Effects2d::default();
    run(&mut effects, &world, 60);
    assert_eq!(effects.live(), 0);
}

#[test]
fn an_emitter_keeps_to_its_cap() {
    let (world, entity) = world_with(&ParticlesComponent {
        rate: 600.0,
        max: 25,
        ..fountain()
    });
    let mut effects = Effects2d::default();
    run(&mut effects, &world, 30);
    assert_eq!(effects.emitter_live(entity), 25);
}

#[test]
fn particles_die_and_are_counted_out() {
    let (world, entity) = world_with(&ParticlesComponent {
        lifetime: 0.1,
        lifetime_spread: 0.0,
        ..fountain()
    });
    let mut effects = Effects2d::default();
    run(&mut effects, &world, 60);
    let live = effects.emitter_live(entity);
    assert!(live <= 4, "{live} alive with a tenth of a second each");
    assert_eq!(usize::try_from(live).unwrap(), effects.live());
}

#[test]
fn gravity_pulls_and_the_ramp_colors_them() {
    let (world, _) = world_with(&ParticlesComponent {
        speed: 0.0,
        gravity: [0.0, -10.0],
        colors: vec![[1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]],
        lifetime: 1.0,
        lifetime_spread: 0.0,
        ..fountain()
    });
    let mut effects = Effects2d::default();
    run(&mut effects, &world, 30);
    let oldest = effects
        .flecks()
        .iter()
        .min_by(|a, b| a.remaining.total_cmp(&b.remaining))
        .copied()
        .unwrap();
    assert!(oldest.position[1] < -0.5, "fell: {:?}", oldest.position);
    let color = effects.color_of(&oldest);
    assert!(color[2] > 0.3 && color[0] < 0.7, "halfway along its ramp: {color:?}");
}

#[test]
fn local_particles_follow_their_emitter() {
    let local = ParticlesComponent {
        speed: 0.0,
        space: ParticleSpace::Local,
        ..fountain()
    };
    let (mut world, entity) = world_with(&local);
    let mut effects = Effects2d::default();
    run(&mut effects, &world, 10);
    let before = effects.flecks()[0].position;
    world.get_mut(entity).unwrap().transform_3d.as_mut().unwrap().position[0] = 3.0;
    run(&mut effects, &world, 1);
    let after = effects.flecks()[0].position;
    assert!((after[0] - before[0] - 3.0).abs() < 1.0e-4, "{before:?} to {after:?}");
}

#[test]
fn the_same_run_throws_the_same_particles() {
    let (world, _) = world_with(&ParticlesComponent {
        spread: 360.0,
        ..fountain()
    });
    let positions = || {
        let mut effects = Effects2d::default();
        run(&mut effects, &world, 20);
        effects
            .flecks()
            .iter()
            .map(|fleck| fleck.position.map(f32::to_bits))
            .collect::<Vec<_>>()
    };
    assert_eq!(positions(), positions());
}

#[test]
fn an_emitter_the_engine_cannot_use_is_named() {
    let (world, _) = world_with(&ParticlesComponent {
        lifetime: -1.0,
        ..fountain()
    });
    let components = SceneExtractor::new().unwrap();
    let problems = Effects2d::default()
        .emit(&world, components.components(), STEP)
        .unwrap();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("lifetime"), "{problems:?}");
}
