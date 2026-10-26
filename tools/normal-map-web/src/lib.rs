//! Browser adapter for the editor's normal-map generator.
use sindri_assets::normal_map::{
    NormalMapSettings, Pixels, cells_of_sheet, generate, height_field,
};
use sindri_core::SpriteSheetDocument;

pub fn make(
    source: &[u8],
    height: &[u8],
    width: u32,
    height_px: u32,
    settings: &str,
    sheet: &str,
) -> Result<Vec<u8>, String> {
    let settings: NormalMapSettings =
        serde_json::from_str(settings).map_err(|error| error.to_string())?;
    let source = Pixels::new(width, height_px, source).map_err(|error| error.to_string())?;
    let height_map = if height.is_empty() {
        None
    } else {
        Some(Pixels::new(width, height_px, height).map_err(|error| error.to_string())?)
    };
    let cells = if sheet.trim().is_empty() {
        Vec::new()
    } else {
        let doc = SpriteSheetDocument::from_json(sheet).map_err(|error| error.to_string())?;
        doc.rects().map_err(|error| error.to_string())?;
        cells_of_sheet(&doc, width, height_px)
    };
    generate(source, height_map, &settings, &cells).map_err(|error| error.to_string())
}

/// Generates the processed height field as grayscale bytes for inspection.
// Each value is clamped to 0..1 before converting to an eight-bit channel.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn height_view(
    source: &[u8],
    height: &[u8],
    width: u32,
    height_px: u32,
    settings: &str,
    sheet: &str,
) -> Result<Vec<u8>, String> {
    let settings: NormalMapSettings =
        serde_json::from_str(settings).map_err(|error| error.to_string())?;
    let source = Pixels::new(width, height_px, source).map_err(|error| error.to_string())?;
    let height_map = if height.is_empty() {
        None
    } else {
        Some(Pixels::new(width, height_px, height).map_err(|error| error.to_string())?)
    };
    let cells = if sheet.trim().is_empty() {
        Vec::new()
    } else {
        let doc = SpriteSheetDocument::from_json(sheet).map_err(|error| error.to_string())?;
        cells_of_sheet(&doc, width, height_px)
    };
    let field =
        height_field(source, height_map, &settings, &cells).map_err(|error| error.to_string())?;
    // Height values are bounded to 0..1 by the generator before conversion.
    Ok(field
        .into_iter()
        .map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn generate_height(
    source: &[u8],
    height: &[u8],
    width: u32,
    height_px: u32,
    settings: &str,
    sheet: &str,
) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    height_view(source, height, width, height_px, settings, sheet)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn generate_map(
    source: &[u8],
    height: &[u8],
    width: u32,
    height_px: u32,
    settings: &str,
    sheet: &str,
) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    make(source, height, width, height_px, settings, sheet)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn preset(index: usize, settings: &str) -> Result<String, wasm_bindgen::JsValue> {
    let current: NormalMapSettings = serde_json::from_str(settings)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))?;
    let choice = sindri_assets::normal_map::NormalMapPreset::ALL
        .get(index)
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("Unknown preset"))?;
    serde_json::to_string(&choice.apply(&current))
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_adapter_uses_the_same_generator_and_preserves_alpha() {
        let pixels = [
            255, 255, 255, 255, 0, 0, 0, 40, 80, 80, 80, 90, 120, 120, 120, 150,
        ];
        let settings = NormalMapSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let result = make(&pixels, &[], 2, 2, &json, "").unwrap();
        let expected = generate(Pixels::new(2, 2, &pixels).unwrap(), None, &settings, &[]).unwrap();
        assert_eq!(result, expected);
        assert_eq!(
            result
                .iter()
                .skip(3)
                .step_by(4)
                .copied()
                .collect::<Vec<_>>(),
            vec![255, 40, 90, 150]
        );
    }
    #[test]
    fn malformed_height_and_sheet_are_reported() {
        let settings = NormalMapSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(make(&[0; 16], &[0; 4], 2, 2, &json, "").is_err());
        assert!(make(&[0; 16], &[], 2, 2, &json, "bad").is_err());
    }
    #[test]
    fn sheet_cells_prevent_neighboring_tiles_from_changing_their_edges() {
        use sindri_core::SheetGrid;
        let source = [
            0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        ];
        let sheet = SpriteSheetDocument {
            format_version: sindri_core::SHEET_FORMAT_VERSION,
            grid: Some(SheetGrid::edge_to_edge(2, 1)),
            ..SpriteSheetDocument::default()
        };
        let json = serde_json::to_string(&sheet).unwrap();
        let settings = serde_json::to_string(&NormalMapSettings::default()).unwrap();
        let separate = make(&source, &[], 4, 1, &settings, &json).unwrap();
        let together = make(&source, &[], 4, 1, &settings, "").unwrap();
        assert_ne!(separate, together);
        assert_eq!(separate[4], 128);
        assert_eq!(separate[8], 128);
    }
    #[test]
    fn height_preview_uses_the_processed_source_and_alpha() {
        let source = [255, 255, 255, 255, 255, 255, 255, 0];
        let settings = serde_json::to_string(&NormalMapSettings::default()).unwrap();
        assert_eq!(
            height_view(&source, &[], 2, 1, &settings, "").unwrap(),
            vec![255, 0]
        );
    }
}
