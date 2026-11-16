use super::*;
use crate::{VoxelBlock, VoxelEdit, VoxelGeneratorDocument, VoxelWorldComponent};

/// A flat world of material 1, with `edits` on top.
fn ground(edits: Vec<VoxelEdit>) -> VoxelGround {
    let mut component: VoxelWorldComponent = serde_json::from_str("{}").expect("a bare world");
    component.generator = VoxelGeneratorDocument::LayeredTerrain {
        seed: 7,
        base_height: 4,
        height_variation: 0,
        surface_voxel: VoxelBlock::Material(1),
        subsurface_voxel: VoxelBlock::Material(2),
        deep_voxel: VoxelBlock::Material(3),
        subsurface_depth: 2,
    };
    component.edits = edits;
    VoxelGround::of(&component, None).expect("the world builds")
}

fn trees(chance: f32) -> ScatterComponent {
    ScatterComponent {
        seed: 11,
        rules: vec![ScatterRule {
            on: vec!["1".to_owned()],
            prefabs: vec!["tree-a.prefab".to_owned(), "tree-b.prefab".to_owned()],
            spacing: 3,
            chance,
            flip: true,
        }],
        ..ScatterComponent::default()
    }
}

#[test]
fn the_default_fills_in_one_rule_and_is_accepted() {
    let scatter = ScatterComponent::default();
    assert_eq!(scatter.rules.len(), 1);
    assert_eq!(scatter.validate(), Ok(()));
}

/// The same seed dresses the same land the same way, however it is asked:
/// a part of it holds what the whole holds there.
#[test]
fn the_same_seed_stands_the_same_things_in_the_same_places() {
    let ground = ground(Vec::new());
    let scatter = trees(0.6);
    let whole = placements(&scatter, &ground, [-30, -30], [29, 29]);
    assert_eq!(whole, placements(&scatter, &ground, [-30, -30], [29, 29]));
    let part = placements(&scatter, &ground, [0, 0], [9, 9]);
    let in_part: Vec<_> = whole
        .iter()
        .filter(|p| (0..=9).contains(&p.key[0]) && (0..=9).contains(&p.key[1]))
        .cloned()
        .collect();
    assert_eq!(part, in_part, "asked piece by piece, the land is the same");
    // About as thick as asked: 400 patches of 3 by 3, three in five holding one.
    assert!((180..300).contains(&whole.len()), "{}", whole.len());
    assert!(whole.iter().any(|p| p.flipped) && whole.iter().any(|p| !p.flipped));
    assert!(whole.iter().any(|p| p.prefab == "tree-a.prefab"));
    assert!(whole.iter().any(|p| p.prefab == "tree-b.prefab"));
    assert!(
        whole
            .iter()
            .all(|p| p.offset.iter().all(|o| o.abs() <= 0.34))
    );
}

/// No two things stand closer than their spacing allows: one to a patch.
#[test]
fn one_thing_to_a_patch() {
    let scatter = trees(1.0);
    let all = placements(&scatter, &ground(Vec::new()), [0, 0], [29, 29]);
    assert_eq!(all.len(), 100, "every patch of 3 by 3 holds one");
    let mut patches: Vec<_> = all.iter().map(|p| (p.key[0] / 3, p.key[1] / 3)).collect();
    patches.sort_unstable();
    patches.dedup();
    assert_eq!(patches.len(), 100);
}

/// A thing grows only on the ground its rule names, and what was taken away
/// does not grow back.
#[test]
fn only_on_its_own_ground_and_not_where_taken() {
    let scatter = trees(1.0);
    let flat = ground(Vec::new());
    let first = placements(&scatter, &flat, [0, 0], [2, 2]);
    assert_eq!(first.len(), 1);
    let [column, row, _] = first[0].key;
    let level = flat.surface(column, row).expect("solid").0;
    let paved = ground(vec![VoxelEdit {
        at: [column, level + 1, row],
        block: VoxelBlock::Material(2),
    }]);
    assert!(
        placements(&scatter, &paved, [0, 0], [2, 2]).is_empty(),
        "not on other ground"
    );
    let mut felled = scatter.clone();
    felled.taken.push(first[0].key);
    assert!(
        placements(&felled, &flat, [0, 0], [2, 2]).is_empty(),
        "not where taken"
    );
    let mut cleared = scatter.clone();
    #[allow(clippy::cast_precision_loss)]
    cleared
        .clearings
        .push([column as f32 + 0.5, row as f32 + 0.5, 0.5]);
    assert!(
        placements(&cleared, &flat, [0, 0], [2, 2]).is_empty(),
        "nor in a clearing"
    );
}

#[test]
fn a_scatter_that_places_nothing_sensible_is_refused() {
    assert_eq!(trees(0.5).validate(), Ok(()));
    let mut wide = trees(0.5);
    wide.reach = 0;
    assert_eq!(wide.validate(), Err(ScatterError::Reach));
    let mut sure = trees(1.5);
    assert_eq!(sure.validate(), Err(ScatterError::Chance(0)));
    sure.rules[0].chance = 1.0;
    sure.rules[0].spacing = 0;
    assert_eq!(sure.validate(), Err(ScatterError::Spacing(0)));
    sure.rules[0].spacing = 2;
    sure.rules[0].prefabs = vec![String::new()];
    assert_eq!(
        sure.validate(),
        Ok(()),
        "a rule still to be filled in is not refused"
    );
    assert!(
        placements(&sure, &ground(Vec::new()), [0, 0], [9, 9]).is_empty(),
        "but it makes nothing"
    );
}

fn hamlet() -> ScatterComponent {
    let piece = |name: &str, at: [f32; 2]| SitePiece {
        prefabs: vec![format!("{name}.prefab")],
        at,
        chance: 1.0,
    };
    ScatterComponent {
        sites: vec![ScatterSite {
            name: "hamlet".to_owned(),
            on: vec!["1".to_owned()],
            spacing: 40,
            chance: 1.0,
            radius: 6.0,
            pieces: vec![
                piece("well", [0.0, 0.0]),
                piece("house", [3.0, 2.0]),
                SitePiece {
                    chance: 0.0,
                    ..piece("cart", [-2.0, -2.0])
                },
            ],
        }],
        ..trees(1.0)
    }
}

/// A site stands its layout whole round a middle the seed chooses, the same
/// way every time, and clears what the rules grow inside it.
#[test]
fn a_site_stands_its_layout_and_clears_round_it() {
    let flat = ground(Vec::new());
    let scatter = hamlet();
    let found = crate::scatter_sites(&scatter, &flat, [0, 0], [39, 39]);
    assert_eq!(found.len(), 1, "one to a patch, sure to be there");
    let middle = found[0].middle;
    let all = placements(&scatter, &flat, [0, 0], [39, 39]);
    let pieces: Vec<_> = all.iter().filter(|p| p.key[2] < 0).collect();
    assert_eq!(
        pieces.len(),
        2,
        "the well and the house; the cart never stands"
    );
    let well = pieces
        .iter()
        .find(|p| p.prefab == "well.prefab")
        .expect("a well");
    #[allow(clippy::cast_precision_loss)]
    let at = [
        well.key[0] as f32 + 0.5 + well.offset[0],
        well.key[1] as f32 + 0.5 - well.offset[1],
    ];
    assert!((at[0] - middle[0]).abs() < 1e-4 && (at[1] - middle[1]).abs() < 1e-4);
    let house = pieces
        .iter()
        .find(|p| p.prefab == "house.prefab")
        .expect("a house");
    let east = if found[0].mirrored { -3.0 } else { 3.0 };
    #[allow(clippy::cast_possible_truncation)]
    let square = [
        (middle[0] + east).floor() as i32,
        (middle[1] - 2.0).floor() as i32,
    ];
    assert_eq!(
        [house.key[0], house.key[1]],
        square,
        "north and east of the well"
    );
    #[allow(clippy::cast_precision_loss)]
    let inside = all.iter().filter(|p| p.key[2] >= 0).any(|p| {
        (p.key[0] as f32 + 0.5 - middle[0]).hypot(p.key[1] as f32 + 0.5 - middle[1]) < 6.0
    });
    assert!(!inside, "no tree grows in the hamlet");
    assert!(all.iter().any(|p| p.key[2] >= 0), "but they grow round it");
    assert_eq!(all, placements(&scatter, &flat, [0, 0], [39, 39]));
    let mut taken = scatter.clone();
    taken.taken.push(well.key);
    let left = placements(&taken, &flat, [0, 0], [39, 39]);
    assert!(
        !left.iter().any(|p| p.prefab == "well.prefab"),
        "taken stays taken"
    );
}

/// A site is only built where all of it can be walked on.
#[test]
fn a_site_is_not_built_where_it_cannot_be_walked() {
    let scatter = hamlet();
    let mut wrong = scatter.clone();
    wrong.sites[0].on = vec!["2".to_owned()];
    assert!(crate::scatter_sites(&wrong, &ground(Vec::new()), [0, 0], [39, 39]).is_empty());
    let mut tight = scatter.clone();
    tight.sites[0].spacing = 12;
    assert_eq!(
        tight.validate(),
        Err(ScatterError::Site(
            super::super::scatter_sites::SiteError::Spacing(0)
        ))
    );
    tight.sites[0].spacing = 40;
    tight.sites[0].pieces[0].at = [7.0, 0.0];
    assert!(tight.validate().is_err(), "a piece outside its radius");
}
