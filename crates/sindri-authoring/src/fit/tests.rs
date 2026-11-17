use super::*;

/// A tree, 20 pixels square: a crown across the top 14 rows, 2 to 17, and a
/// trunk four pixels wide, 8 to 11, down the last 6.
fn tree() -> Frame {
    let mut alpha = vec![0; 400];
    for y in 0..20 {
        for x in 0..20 {
            let crown = y < 14 && (2..18).contains(&x);
            let trunk = y >= 14 && (8..12).contains(&x);
            if crown || trunk {
                alpha[y * 20 + x] = 255;
            }
        }
    }
    Frame {
        width: 20,
        height: 20,
        alpha,
    }
}

/// Drawn a unit across, centred on its object.
const UNIT: Placement = Placement {
    size: [1.0, 1.0],
    offset: [0.0, 0.0],
    anchor: SpriteAnchor::Center,
};

#[test]
fn an_outline_is_a_box_around_what_is_drawn() {
    let fitted = fit(&tree(), &UNIT, FitTo::Outline).unwrap();
    assert_eq!(fitted.shape["shape"], "box");
    let half = fitted.shape["half_extents"].as_array().unwrap();
    assert!((half[0].as_f64().unwrap() - 0.4).abs() < 1e-5);
    assert!((half[1].as_f64().unwrap() - 0.5).abs() < 1e-5);
    assert!(fitted.offset.iter().all(|value| value.abs() < 1e-9));
    assert_eq!(fitted.outline.len(), 4);
}

#[test]
fn a_tree_is_footed_by_its_trunk_not_its_crown() {
    let fitted = fit(&tree(), &UNIT, FitTo::Footing).unwrap();
    // The trunk is 4 of 20 pixels across: a reach of 0.1 each way.
    let reach = match fitted.shape["shape"].as_str().unwrap() {
        "capsule" => {
            fitted.shape["half_height"].as_f64().unwrap() + fitted.shape["radius"].as_f64().unwrap()
        }
        "circle" => fitted.shape["radius"].as_f64().unwrap(),
        other => panic!("unexpected {other}"),
    };
    assert!((reach - 0.1).abs() < 1e-4, "reach {reach}");
    // At the foot of the picture: three rows of twenty up from its bottom edge.
    assert!(fitted.offset[0].abs() < 1e-5);
    assert!(
        (fitted.offset[1] - (-0.5 + 0.075)).abs() < 1e-4,
        "{:?}",
        fitted.offset
    );
    assert_eq!(fitted.outline.len(), usize::from(OUTLINE_POINTS));
}

#[test]
fn a_bottom_anchor_and_the_sprite_placement_move_the_fit() {
    let placed = Placement {
        size: [2.0, 4.0],
        offset: [1.0, 0.0],
        anchor: SpriteAnchor::Bottom,
    };
    let fitted = fit(&tree(), &placed, FitTo::Outline).unwrap();
    // Standing on its origin: the box runs from 0 to 4 up, centred on x = 1.
    assert!((fitted.offset[0] - 1.0).abs() < 1e-5);
    assert!((fitted.offset[1] - 2.0).abs() < 1e-5);
}

#[test]
fn a_mirrored_sprite_still_fits_a_positive_box() {
    let mirrored = Placement {
        size: [-1.0, 1.0],
        ..UNIT
    };
    let fitted = fit(&tree(), &mirrored, FitTo::Outline).unwrap();
    assert!(fitted.shape["half_extents"][0].as_f64().unwrap() > 0.0);
}

#[test]
fn nothing_drawn_fits_nothing() {
    let empty = Frame {
        width: 4,
        height: 4,
        alpha: vec![0; 16],
    };
    assert!(fit(&empty, &UNIT, FitTo::Footing).is_none());
}

#[test]
fn fitting_writes_the_first_piece_and_the_occluder() {
    let dir = tempfile::tempdir().unwrap();
    let frame = tree();
    let image = image::RgbaImage::from_fn(20, 20, |x, y| {
        image::Rgba([0, 0, 0, frame.alpha[(y * 20 + x) as usize]])
    });
    image.save(dir.path().join("tree.png")).unwrap();
    let mut components = BTreeMap::from([
        (SPRITE.to_owned(), json!({"texture": "tree.png"})),
        (
            COLLIDER.to_owned(),
            json!({"pieces": [{"shape": {"shape": "box", "half_extents": [1, 1]}}, {"name": "kept"}]}),
        ),
        (OCCLUDER.to_owned(), json!({"shape": "box"})),
    ]);
    fit_components(dir.path(), &mut components, FitTo::Outline).unwrap();
    let pieces = components[COLLIDER]["pieces"].as_array().unwrap();
    assert_eq!(pieces[0]["shape"]["half_extents"][0], json!(0.4));
    assert_eq!(pieces[1], json!({"name": "kept"}));
    assert_eq!(components[OCCLUDER]["shape"], "polygon");
    assert_eq!(components[OCCLUDER]["points"].as_array().unwrap().len(), 4);
}

#[test]
fn an_object_without_a_sprite_is_refused() {
    let mut components = BTreeMap::from([(COLLIDER.to_owned(), json!({}))]);
    assert_eq!(
        fit_components(Path::new("."), &mut components, FitTo::Outline),
        Err(FitError::NoSprite)
    );
}
