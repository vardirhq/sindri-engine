//! Widget authoring uses the same schema inspector and checked edit path.
use super::SceneExtractError;
use crate::{
    UiDropdownComponent, UiOptionComponent, UiScrollComponent, UiTextInputComponent,
    UiToggleComponent,
};
use sindri_core::ComponentSchemaRegistry;

pub(super) fn register(components: &mut ComponentSchemaRegistry) -> Result<(), SceneExtractError> {
    components.register_with_default::<UiToggleComponent>(
        "UI Toggle",
        serde_json::json!({"label":"", "checked":false, "disabled":false, "group":"", "autofocus":false}),
    )?;
    components.register_with_default::<UiTextInputComponent>("UI Text Input", serde_json::json!({"label":"", "value":"", "placeholder":"", "max_length":128, "disabled":false, "autofocus":false}))?;
    components.register_with_default::<UiScrollComponent>(
        "UI Scroll",
        serde_json::json!({"label":"", "content_height":2.0, "offset":0.0, "disabled":false}),
    )?;
    components.register_with_default::<UiDropdownComponent>(
        "UI Dropdown",
        serde_json::json!({"label":"", "selected":0, "open":false, "disabled":false, "autofocus":false}),
    )?;
    components.register_with_default::<UiOptionComponent>(
        "UI Option",
        serde_json::json!({"label":"", "checked":false, "disabled":false}),
    )?;
    Ok(())
}
