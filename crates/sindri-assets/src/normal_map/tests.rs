use std::path::Path;

use sindri_core::SpriteSheetDocument;

use super::*;

/// A `width` by `height` picture whose gray at each texel is `gray(x, y)`.
fn picture(width: u32, height: u32, gray: impl Fn(u32, u32) -> u8) -> Vec<u8> {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let value = gray(x, y);
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }
    rgba
}

fn make(width: u32, height: u32, rgba: &[u8], settings: &NormalMapSettings) -> Vec<u8> {
    let source = Pixels::new(width, height, rgba).expect("a whole picture");
    generate(source, None, settings, &[]).expect("a map")
}

fn texel(map: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    [map[at], map[at + 1], map[at + 2], map[at + 3]]
}

#[test]
fn a_flat_picture_makes_a_flat_map_of_the_same_size() {
    let rgba = picture(5, 3, |_, _| 90);
    let map = make(5, 3, &rgba, &NormalMapSettings::default());
    assert_eq!(map.len(), rgba.len());
    assert!(map.chunks_exact(4).all(|texel| texel == FLAT_NORMAL));
}

#[test]
fn brightness_rising_to_the_right_faces_left() {
    let rgba = picture(5, 5, |x, _| u8::try_from(x * 50).unwrap());
    let map = make(5, 5, &rgba, &NormalMapSettings::default());
    let [red, green, blue, _] = texel(&map, 5, 2, 2);
    assert!(red < 128, "faces left: {red}");
    assert_eq!(green, 128);
    assert!(blue > 128 && blue < 255);
}

#[test]
fn brightness_rising_down_the_rows_faces_up_and_green_down_flips_it() {
    let rgba = picture(5, 5, |_, y| u8::try_from(y * 50).unwrap());
    let up = make(5, 5, &rgba, &NormalMapSettings::default());
    assert!(texel(&up, 5, 2, 2)[1] > 128, "faces up the world");
    let down = make(
        5,
        5,
        &rgba,
        &NormalMapSettings {
            green: GreenAxis::Down,
            ..NormalMapSettings::default()
        },
    );
    assert!(texel(&down, 5, 2, 2)[1] < 128);
    assert_eq!(texel(&up, 5, 2, 2)[0], texel(&down, 5, 2, 2)[0]);
}

#[test]
fn strength_steepens_and_zero_is_flat() {
    let rgba = picture(5, 1, |x, _| u8::try_from(x * 40).unwrap());
    let red = |strength| {
        let settings = NormalMapSettings {
            strength,
            ..NormalMapSettings::default()
        };
        texel(&make(5, 1, &rgba, &settings), 5, 2, 0)[0]
    };
    assert_eq!(red(0.0), 128);
    assert!(red(1.0) < 128);
    assert!(red(4.0) < red(1.0));
}

#[test]
fn invert_turns_the_slope_round() {
    let rgba = picture(5, 1, |x, _| u8::try_from(x * 40).unwrap());
    let plain = make(5, 1, &rgba, &NormalMapSettings::default());
    let inverted = make(
        5,
        1,
        &rgba,
        &NormalMapSettings {
            invert: true,
            ..NormalMapSettings::default()
        },
    );
    let (a, b) = (texel(&plain, 5, 2, 0)[0], texel(&inverted, 5, 2, 0)[0]);
    assert!(a < 128 && b > 128);
    assert!((i16::from(a) - 128).abs().abs_diff(i16::from(b) - 128) <= 1);
}

#[test]
fn alpha_is_kept_and_a_transparent_texel_is_stored_flat() {
    let mut rgba = picture(3, 1, |_, _| 200);
    rgba[3] = 0;
    rgba[7] = 120;
    let map = make(3, 1, &rgba, &NormalMapSettings::default());
    assert_eq!(texel(&map, 3, 0, 0), [128, 128, 255, 0]);
    assert_eq!(texel(&map, 3, 1, 0)[3], 120);
    assert_eq!(texel(&map, 3, 2, 0)[3], 255);
}

#[test]
fn alpha_height_bevels_a_silhouette_toward_its_outline() {
    // A bright square on transparency.
    let mut rgba = picture(5, 1, |_, _| 200);
    for x in [0, 4] {
        rgba[x * 4 + 3] = 0;
    }
    let bevelled = make(5, 1, &rgba, &NormalMapSettings::default());
    assert!(texel(&bevelled, 5, 1, 0)[0] < 128, "left edge faces left");
    assert!(texel(&bevelled, 5, 3, 0)[0] > 128, "right edge faces right");
    let flat = make(
        5,
        1,
        &rgba,
        &NormalMapSettings {
            alpha_height: false,
            ..NormalMapSettings::default()
        },
    );
    assert_eq!(texel(&flat, 5, 1, 0), FLAT_NORMAL);
}

#[test]
fn a_cell_never_reads_its_neighbor_in_the_sheet() {
    // Two 2x1 cells side by side: dark, then bright.
    let rgba = picture(4, 1, |x, _| if x < 2 { 0 } else { 255 });
    let source = Pixels::new(4, 1, &rgba).unwrap();
    let cells = [[0, 0, 2, 1], [2, 0, 2, 1]];
    let settings = NormalMapSettings::default();
    let separate = generate(source, None, &settings, &cells).unwrap();
    assert!(separate.chunks_exact(4).all(|texel| texel == FLAT_NORMAL));
    let joined = generate(
        source,
        None,
        &NormalMapSettings {
            cells: false,
            ..settings
        },
        &cells,
    )
    .unwrap();
    assert!(texel(&joined, 4, 1, 0)[0] < 128);
}

#[test]
fn wrap_reads_the_far_side_and_clamp_does_not() {
    let rgba = picture(4, 1, |x, _| if x == 3 { 255 } else { 0 });
    let clamped = make(4, 1, &rgba, &NormalMapSettings::default());
    assert_eq!(texel(&clamped, 4, 0, 0), FLAT_NORMAL);
    let wrapped = make(
        4,
        1,
        &rgba,
        &NormalMapSettings {
            edges: EdgeMode::Wrap,
            ..NormalMapSettings::default()
        },
    );
    assert!(
        texel(&wrapped, 4, 0, 0)[0] > 128,
        "the raised texel is to its left"
    );
}

#[test]
fn blur_spreads_relief_and_full_detail_puts_it_back() {
    let rgba = picture(9, 1, |x, _| if x == 4 { 255 } else { 0 });
    let sharp = make(9, 1, &rgba, &NormalMapSettings::default());
    assert_eq!(texel(&sharp, 9, 1, 0), FLAT_NORMAL);
    let soft = NormalMapSettings {
        blur: 3.0,
        detail: 0.0,
        ..NormalMapSettings::default()
    };
    assert!(texel(&make(9, 1, &rgba, &soft), 9, 2, 0)[0] < 128);
    let detailed = NormalMapSettings {
        detail: 1.0,
        ..soft
    };
    assert_eq!(make(9, 1, &rgba, &detailed), sharp);
}

#[test]
fn a_height_map_is_read_instead_of_the_picture() {
    let rgba = picture(5, 1, |_, _| 128);
    let heights = picture(5, 1, |x, _| u8::try_from(x * 60).unwrap());
    let source = Pixels::new(5, 1, &rgba).unwrap();
    let settings = NormalMapSettings {
        method: HeightSource::HeightMap,
        ..NormalMapSettings::default()
    };
    let map = generate(
        source,
        Some(Pixels::new(5, 1, &heights).unwrap()),
        &settings,
        &[],
    )
    .unwrap();
    assert!(texel(&map, 5, 2, 0)[0] < 128);
    assert_eq!(
        generate(source, None, &settings, &[]),
        Err(NormalMapError::MissingHeightMap)
    );
    let small = picture(4, 1, |_, _| 0);
    assert!(matches!(
        generate(
            source,
            Some(Pixels::new(4, 1, &small).unwrap()),
            &settings,
            &[]
        ),
        Err(NormalMapError::HeightMapSize { .. })
    ));
}

#[test]
fn a_picture_that_is_not_its_size_is_refused() {
    assert_eq!(Pixels::new(0, 4, &[]).unwrap_err(), NormalMapError::Empty);
    assert_eq!(
        Pixels::new(2, 2, &[0; 15]).unwrap_err(),
        NormalMapError::WrongLength {
            expected: 16,
            actual: 15
        }
    );
}

#[test]
fn settings_out_of_range_are_brought_back() {
    let wild = NormalMapSettings {
        strength: f32::NAN,
        blur: 400.0,
        detail: -3.0,
        ..NormalMapSettings::default()
    }
    .sanitized();
    assert!((wild.strength - 2.0).abs() < f32::EPSILON);
    assert!((wild.blur - 16.0).abs() < f32::EPSILON);
    assert!(wild.detail.abs() < f32::EPSILON);
}

/// The same input makes the same bytes, on every host: a fixed noisy picture
/// through blur, detail, cells and wrap, checked against a recorded FNV hash.
#[test]
fn the_same_input_makes_the_same_bytes() {
    let mut seed = 0x2545_f491_u32;
    let rgba: Vec<u8> = (0..32 * 16 * 4)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed.to_le_bytes()[0]
        })
        .collect();
    let source = Pixels::new(32, 16, &rgba).unwrap();
    let settings = NormalMapSettings {
        blur: 2.5,
        detail: 0.4,
        edges: EdgeMode::Wrap,
        strength: 3.0,
        ..NormalMapSettings::default()
    };
    let map = generate(source, None, &settings, &[[0, 0, 16, 16], [16, 0, 16, 16]]).unwrap();
    let hash = map.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    });
    assert_eq!(hash, 8_669_400_626_857_984_929, "recorded hash");
}

#[test]
fn presets_keep_the_files_choices_and_set_the_rest() {
    let current = NormalMapSettings {
        method: HeightSource::HeightMap,
        green: GreenAxis::Down,
        invert: true,
        ..NormalMapSettings::default()
    };
    for preset in NormalMapPreset::ALL {
        let applied = preset.apply(&current);
        assert_eq!(applied.method, HeightSource::HeightMap);
        assert_eq!(applied.green, GreenAxis::Down);
        assert!(!applied.invert);
        assert_eq!(applied, applied.sanitized(), "{}", preset.label());
    }
    assert_eq!(
        NormalMapPreset::General.apply(&NormalMapSettings::default()),
        NormalMapSettings::default()
    );
}

#[test]
fn a_document_round_trips_and_fills_what_it_does_not_say() {
    let document = NormalMapDocument::new(
        "floor.png",
        NormalMapPreset::Stone.apply(&NormalMapSettings::default()),
    );
    assert_eq!(
        NormalMapDocument::from_json(&document.to_json()).unwrap(),
        document
    );
    let sparse = NormalMapDocument::from_json(
        r#"{"format_version":1,"source":"a.png","settings":{"strength":5}}"#,
    )
    .unwrap();
    assert!((sparse.settings.strength - 5.0).abs() < f32::EPSILON);
    assert!(sparse.settings.alpha_height);
    assert!(matches!(
        NormalMapDocument::from_json(r#"{"format_version":9,"source":"a.png"}"#),
        Err(NormalMapDocumentError::Version(9))
    ));
}

#[test]
fn a_map_and_its_settings_sit_beside_the_picture() {
    let (map, settings) = normal_map_paths(Path::new("assets/textures/floor.png"));
    assert_eq!(map, Path::new("assets/textures/floor_n.png"));
    assert_eq!(settings, Path::new("assets/textures/floor_n.normalmap"));
}

#[test]
fn a_sheets_grid_becomes_pixel_cells() {
    let sheet = SpriteSheetDocument::from_grid(2, 2);
    let mut cells = cells_of_sheet(&sheet, 32, 16);
    cells.sort_unstable();
    assert_eq!(
        cells,
        vec![[0, 0, 16, 8], [0, 8, 16, 8], [16, 0, 16, 8], [16, 8, 16, 8]]
    );
}

#[test]
fn a_map_encodes_as_a_png_that_decodes_to_itself() {
    let rgba = picture(3, 2, |x, y| u8::try_from(x * 70 + y * 20).unwrap());
    let map = make(3, 2, &rgba, &NormalMapSettings::default());
    let png = encode_png(3, 2, &map).unwrap();
    let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(decoded.into_raw(), map);
}

#[test]
fn a_settings_file_makes_its_map_again_from_the_art_as_it_is_now() {
    let folder = tempfile::tempdir().unwrap();
    let ramp = picture(8, 4, |x, _| u8::try_from(x * 30).unwrap());
    std::fs::write(
        folder.path().join("floor.png"),
        encode_png(8, 4, &ramp).unwrap(),
    )
    .unwrap();
    let settings = NormalMapSettings {
        strength: 3.0,
        ..NormalMapSettings::default()
    };
    let document = NormalMapDocument {
        format_version: NORMAL_MAP_FORMAT_VERSION,
        source: "floor.png".to_owned(),
        height_map: None,
        settings: settings.clone(),
    };
    let settings_path = folder.path().join("floor_n.normalmap");
    std::fs::write(&settings_path, document.to_json()).unwrap();
    let remade = remake(&settings_path).unwrap();
    assert_eq!(remade.map_path, folder.path().join("floor_n.png"));
    let expected = make(8, 4, &ramp, &settings);
    assert_eq!(remade.png, encode_png(8, 4, &expected).unwrap());
    assert_eq!(
        Picture::read(&folder.path().join("floor.png"))
            .unwrap()
            .rgba,
        ramp
    );

    let missing = folder.path().join("gone_n.normalmap");
    assert!(matches!(remake(&missing), Err(RemakeError::Read { .. })));
}
