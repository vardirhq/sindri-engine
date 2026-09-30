//! Widget authoring uses the same schema inspector and checked edit path.
use sindri_core::ComponentSchemaRegistry;
use crate::{UiScrollComponent, UiTextInputComponent, UiToggleComponent};
use super::SceneExtractError;

pub(super) fn register(components: &mut ComponentSchemaRegistry) -> Result<(), SceneExtractError> {
    components.register_with_default::<UiToggleComponent>("UI Toggle", serde_json::json!({"label":"", "checked":false, "disabled":false}))?;
    components.register_with_default::<UiTextInputComponent>("UI Text Input", serde_json::json!({"label":"", "value":"", "placeholder":"", "max_length":128, "disabled":false}))?;
    components.register_with_default::<UiScrollComponent>("UI Scroll", serde_json::json!({"label":"", "content_height":2.0, "offset":0.0, "disabled":false}))?;
    Ok(())
}
