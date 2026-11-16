//! A map of the wilds in letters, to look at a seed's land without the
//! renderer: where the sea, the shore and each biome lie around the start.
//!
//! Ignored by default: it prints rather than checks. Run it with
//! `cargo test -p lamplighter --test wilds_map -- --ignored --nocapture`, and
//! set `STEP` (squares a letter covers, 1 by default), `SEEDS` (comma
//! separated, the scene's own by default), `RELIEF` and `FEATURE` to try
//! others.
//!
//! `~` water, `s` sand, `.` meadow, `t` deadwood, `h` heath, `b` bog, `m` mud,
//! `^` stone and rock, `?` anything else, `@` where Wick starts, `H` a hamlet's
//! middle.

use lamplighter::Run;
use sindri_scene::{VoxelGeneratorDocument, VoxelGround, VoxelWorldComponent};

fn setting<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok()?.parse().ok()
}

#[test]
#[ignore = "a picture, not a check"]
fn wilds_map() {
    let mut run = Run::open_scene("wilds.scene").expect("the wilds open");
    let ground = run.entity("ground").expect("the ground");
    let payload =
        run.world.get(ground).expect("it exists").components["sindri.voxel_world"].clone();
    let mut land: VoxelWorldComponent = serde_json::from_value(payload).expect("a world");
    let scatter: sindri_scene::ScatterComponent = serde_json::from_value(
        run.world.get(ground).expect("it exists").components["sindri.scatter"].clone(),
    )
    .expect("a scatter");
    let start = run
        .world
        .world_transform(run.entity("wick").expect("Wick"))
        .expect("placed")
        .position;
    let origin = run.world.world_transform(ground).expect("placed").position;
    #[allow(clippy::cast_possible_truncation)]
    let middle = [
        (start[0] - origin[0]).floor() as i32,
        (origin[1] - start[1]).floor() as i32,
    ];
    let step: i32 = setting("STEP").unwrap_or(1);
    let seeds: Vec<u64> = std::env::var("SEEDS")
        .map(|seeds| seeds.split(',').filter_map(|s| s.parse().ok()).collect())
        .unwrap_or_default();
    run.draw_with(|_, _, session| {
        let VoxelGeneratorDocument::NaturalTerrain(authored) = &land.generator else {
            return Err("the wilds are not a natural terrain".to_owned());
        };
        let seeds = if seeds.is_empty() {
            vec![authored.seed]
        } else {
            seeds.clone()
        };
        for seed in seeds {
            if let VoxelGeneratorDocument::NaturalTerrain(natural) = &mut land.generator {
                natural.seed = seed;
                if let Some(relief) = setting("RELIEF") {
                    natural.relief = relief;
                }
                if let Some(feature) = setting("FEATURE") {
                    natural.feature_size = feature;
                }
            }
            let ground = VoxelGround::of(&land, Some(session.tile_sets()))
                .map_err(|error| error.to_string())?;
            println!("seed {seed}, {step} squares a letter");
            let hamlets: Vec<[i32; 2]> = sindri_scene::scatter_sites(
                &scatter,
                &ground,
                [middle[0] - 40 * step, middle[1] - 25 * step],
                [middle[0] + 40 * step, middle[1] + 25 * step],
            )
            .iter()
            .map(|site| {
                #[allow(clippy::cast_possible_truncation)]
                let square = [site.middle[0].floor() as i32, site.middle[1].floor() as i32];
                println!("a hamlet at {square:?}, start {middle:?}");
                square
            })
            .collect();
            for row in -25..25 {
                let line: String = (-40..40)
                    .map(|column| {
                        if row == 0 && column == 0 {
                            return '@';
                        }
                        let (c, r) = (middle[0] + column * step, middle[1] + row * step);
                        let near = |at: &[i32; 2]| {
                            (at[0] - c).abs() < step.max(1) && (at[1] - r).abs() < step.max(1)
                        };
                        if hamlets
                            .iter()
                            .any(|at| near(at) && at[0] >= c && at[1] >= r)
                        {
                            return 'H';
                        }
                        match ground.surface(c, r) {
                            None | Some((_, _, false)) => '~',
                            Some((level, _, true)) => match ground.block([c, level, r]).as_str() {
                                "grass" => '.',
                                "deadgrass" => 't',
                                "earth" => 'h',
                                "moss" => 'b',
                                "mud" => 'm',
                                "stone" | "rock" => '^',
                                "sand" => 's',
                                _ => '?',
                            },
                        }
                    })
                    .collect();
                println!("{line}");
            }
        }
        Ok(((), sindri_scene::UiTextSizes::new()))
    })
    .expect("the map");
}
