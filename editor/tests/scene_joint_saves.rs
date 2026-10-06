//! Editor save paths remap spawned constraints without changing the running world.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, PrefabDocument, SceneComponent, SceneDocument,
    SceneEntityId, World,
};
use sindri_editor::{prefab::subtree_prefab, scene_file::SceneFile};
use sindri_scene::{SceneExtractor, ScenePhysics2d};

const KINDS: [&str; 4] = [
    "sindri.physics2d.distance_joint",
    "sindri.physics2d.hinge_joint",
    "sindri.physics2d.slider_joint",
    "sindri.physics2d.spring_joint",
];
const STEP: Duration = Duration::from_nanos(16_666_667);

fn fixture(kind: &str) -> (World, sindri_core::SpawnedPrefab, ComponentSchemaRegistry) {
    let registry = SceneExtractor::new().unwrap().components().clone();
    let mut payload = registry.default_payload(kind).unwrap().clone();
    payload["first"] = json!("anchor");
    payload["second"] = json!("body");
    payload["future_field"] = json!(42);
    let collider = registry
        .default_payload("sindri.physics2d.collider")
        .unwrap()
        .clone();
    let rigid_body = registry
        .default_payload("sindri.physics2d.rigid_body")
        .unwrap()
        .clone();
    let prefab: PrefabDocument = serde_json::from_value(json!({
        "format_version": 1, "entities": [
            {"id": "anchor", "components": {
                "sindri.physics2d.collider": collider
            }},
            {"id": "body", "parent": "anchor", "components": {
                "sindri.physics2d.collider": collider,
                "sindri.physics2d.rigid_body": rigid_body
            }},
            {"id": "joint", "parent": "anchor", "components": {(kind): payload}}
        ]
    }))
    .unwrap();
    let mut world = World::default();
    let first = world.spawn_prefab(&prefab).unwrap();
    world.spawn_prefab(&prefab).unwrap();
    world.assign_missing_source_ids("saved").unwrap();
    (world, first, registry)
}

#[test]
fn save_as_save_and_subtree_prefabs_keep_every_joint_instance_isolated() {
    for kind in KINDS {
        let (world, first, registry) = fixture(kind);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mechanisms.scene");
        let mut file = SceneFile::detached(SceneDocument::default());
        file.save_as(&path, &world, &registry).unwrap();
        let original = std::fs::read_to_string(&path).unwrap();
        file.save(&world, &registry).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let loaded = SceneFile::open(&path).unwrap();
        let mut reopened = World::from_scene(loaded.document()).unwrap().world;
        let owners: Vec<_> = reopened
            .entities()
            .filter_map(|(owner, data)| data.components.contains_key(kind).then_some(owner))
            .collect();
        assert_eq!(owners.len(), 2);
        let endpoints: Vec<_> = owners
            .iter()
            .map(|owner| {
                let payload = &reopened.get(*owner).unwrap().components[kind];
                assert_eq!(payload["future_field"], 42);
                reopened
                    .resolve_entity_reference(*owner, payload["second"].as_str().unwrap())
                    .unwrap()
            })
            .collect();
        assert_ne!(endpoints[0], endpoints[1]);
        let mut physics = ScenePhysics2d::top_down().unwrap();
        physics.step(&mut reopened, &registry, STEP).unwrap();
        assert_eq!(physics.world().joint_count(), 2, "{kind}");
        let owner = first.by_source_id[&SceneEntityId::new("joint").unwrap()];
        assert_eq!(world.get(owner).unwrap().components[kind]["second"], "body");
        assert!(world.get(owner).unwrap().prefab_identity.is_some());
        let prefab = subtree_prefab(&world, first.root, file.prefabs(), &registry).unwrap();
        let mut placed = World::default();
        placed.spawn_prefab(&prefab).unwrap();
        placed.spawn_prefab(&prefab).unwrap();
        let mut physics = ScenePhysics2d::top_down().unwrap();
        physics.step(&mut placed, &registry, STEP).unwrap();
        assert_eq!(physics.world().joint_count(), 2, "saved prefab {kind}");
    }
}

#[test]
fn invalid_references_never_overwrite_files_or_adopt_save_as_paths() {
    for problem in ["missing", "malformed", "unstable"] {
        let (mut world, first, registry) = fixture(KINDS[1]);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.scene");
        let other = directory.path().join("other.scene");
        let mut file = SceneFile::detached(SceneDocument::default());
        file.save_as(&path, &world, &registry).unwrap();
        let original = std::fs::read_to_string(&path).unwrap();
        let agreed = file.document().clone();
        let owner = first.by_source_id[&SceneEntityId::new("joint").unwrap()];
        if problem == "unstable" {
            let target = first.by_source_id[&SceneEntityId::new("body").unwrap()];
            world.get_mut(target).unwrap().source_id = None;
        } else {
            world
                .get_mut(owner)
                .unwrap()
                .components
                .get_mut(KINDS[1])
                .unwrap()["second"] = if problem == "missing" {
                json!("gone")
            } else {
                json!(17)
            };
        }
        assert!(file.save(&world, &registry).is_err(), "{problem}");
        assert!(
            file.save_as(&other, &world, &registry).is_err(),
            "{problem}"
        );
        assert_eq!(file.path(), Some(path.as_path()));
        assert_eq!(file.document(), &agreed);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(!other.exists());
        assert!(subtree_prefab(&world, first.root, file.prefabs(), &registry).is_err());
    }
}

struct WindmillFixture {
    file: SceneFile,
    world: World,
    registry: ComponentSchemaRegistry,
    sources: sindri_decay::ScriptSources,
    prefabs: sindri_decay::PrefabSources,
}

fn spawned_windmill() -> WindmillFixture {
    use sindri_decay::{
        Physics2d, PrefabSources, ScriptComponent, ScriptFrame, ScriptSources, Scripts,
    };
    use sindri_platform::{InputEvent, InputState, Key};

    let assets =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../games/platformer/assets");
    let file = SceneFile::open(assets.join("platformer.scene")).unwrap();
    let mut extractor = SceneExtractor::new().unwrap();
    extractor.register::<ScriptComponent>("Script").unwrap();
    let registry = extractor.components();
    let mut prefabs = PrefabSources::new();
    for (id, prefab) in file.prefabs().iter() {
        prefabs.insert(id, prefab.clone());
    }
    let mut sources = ScriptSources::new();
    for name in ["windmill-setup", "windmill"] {
        let id = format!("scripts/{name}.decay");
        sources.insert(&id, std::fs::read_to_string(assets.join(&id)).unwrap());
    }
    let mut world = World::default();
    world.spawn(EntityData {
        components: [(
            ScriptComponent::TYPE_NAME.into(),
            json!({
                "source": "scripts/windmill-setup.decay", "script": "WindmillSetup",
                "properties": {"template": "prefabs/windmill-kit.prefab"}
            }),
        )]
        .into(),
        ..EntityData::default()
    });
    let mut input = InputState::default();
    input.apply(InputEvent::KeyPressed(Key::V));
    let mut scripts = Scripts::new();
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let (backend, events) = physics.for_scripts();
    let report = scripts.advance(
        &mut world,
        registry,
        ScriptFrame::new(&sources, &input, 1.0 / 60.0)
            .with_prefabs(&prefabs)
            .with_physics(Physics2d {
                world: backend,
                events,
            }),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    world.assign_missing_source_ids("game-save").unwrap();
    WindmillFixture {
        file,
        world,
        registry: registry.clone(),
        sources,
        prefabs,
    }
}

fn copy_windmill_assets(
    file: &SceneFile,
    destination: &std::path::Path,
    sources: &mut sindri_decay::ScriptSources,
) {
    let original = file.anchor().unwrap().parent().unwrap();
    std::fs::create_dir_all(destination.join("scripts")).unwrap();
    std::fs::create_dir_all(destination.join("prefabs")).unwrap();
    for (id, _) in file.prefabs().iter() {
        std::fs::copy(original.join(id), destination.join(id)).unwrap();
    }
    for name in ["windmill-setup", "windmill"] {
        let id = format!("scripts/{name}.decay");
        let path = destination.join(&id);
        std::fs::copy(original.join(&id), &path).unwrap();
        sources.insert(&id, std::fs::read_to_string(path).unwrap());
    }
}

#[test]
fn decay_spawned_platformer_windmill_reopens_through_the_editor_save_path() {
    use sindri_decay::{Physics2d, ScriptFrame, Scripts};
    use sindri_platform::InputState;
    let WindmillFixture {
        mut file,
        world,
        registry,
        mut sources,
        prefabs,
    } = spawned_windmill();
    let directory = tempfile::tempdir().unwrap();
    copy_windmill_assets(&file, directory.path(), &mut sources);
    let path = directory.path().join("windmill.scene");
    file.save_as(&path, &world, &registry).unwrap();
    let loaded = SceneFile::open(&path).unwrap();
    let mut reopened = World::from_scene(loaded.document()).unwrap().world;
    let named = |name| {
        reopened
            .entities()
            .find_map(|(entity, data)| (data.name.as_deref() == Some(name)).then_some(entity))
            .unwrap()
    };
    let rotor = named("Spawned windmill rotor");
    let axle = named("Spawned windmill axle");
    let input = InputState::default();
    let mut scripts = Scripts::new();
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let mut forward = false;
    let mut backward = false;
    for _ in 0..360 {
        physics.step(&mut reopened, &registry, STEP).unwrap();
        let (backend, events) = physics.for_scripts();
        let report = scripts.advance(
            &mut reopened,
            &registry,
            ScriptFrame::new(&sources, &input, 1.0 / 60.0)
                .with_prefabs(&prefabs)
                .with_physics(Physics2d {
                    world: backend,
                    events,
                }),
        );
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let speed = physics.world().angular_velocity(rotor).unwrap();
        forward |= speed > 1.0;
        backward |= speed < -1.0;
        assert_eq!(physics.world().joint_count(), 1);
        let [x, y] = reopened.world_transform(rotor).unwrap().position_2d();
        let [ax, ay] = reopened.world_transform(axle).unwrap().position_2d();
        assert!((x - ax).hypot(y - ay) < 0.02);
        assert!((ax - 5.2).abs() < 1e-4);
    }
    assert!(forward && backward);
}
