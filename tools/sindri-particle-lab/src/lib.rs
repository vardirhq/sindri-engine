//! Shared engine-backed particle authoring for editor and web.
mod presets;
mod preview;
mod ui;

pub use ui::ParticleLab;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    use wasm_bindgen::JsCast;
    console_error_panic_hook::set_once();
    let Some(canvas) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("particle-lab"))
        .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
    else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let runner = eframe::WebRunner::new();
        let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
        setup.instance_descriptor.backends = eframe::wgpu::Backends::GL;
        let mut options = eframe::WebOptions::default();
        options.wgpu_options.wgpu_setup = eframe::egui_wgpu::WgpuSetup::CreateNew(setup);
        let result = runner
            .start(
                canvas,
                options,
                Box::new(|cc| {
                    cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
                    Ok(Box::<ui::WebApp>::default())
                }),
            )
            .await;
        if let Err(error) = result {
            web_sys::console::error_1(&error);
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn download(json: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(json));
    let blob = web_sys::Blob::new_with_str_sequence(&parts).map_err(|e| format!("{e:?}"))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|e| format!("{e:?}"))?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or("No document")?;
    let link: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e| format!("{e:?}"))?;
    link.set_href(&url);
    link.set_download("particle-emitter.json");
    link.click();
    web_sys::Url::revoke_object_url(&url).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PREVIEW_IMAGE: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

/// Receives a local preview image selected by the browser's file picker.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn set_preview_image(bytes: Vec<u8>) {
    PREVIEW_IMAGE.with(|slot| *slot.borrow_mut() = Some(bytes));
}
