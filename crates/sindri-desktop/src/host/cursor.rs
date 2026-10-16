//! Window-owned cursor capture and actual-state feedback.

use sindri_platform::InputEvent;
#[cfg(not(target_arch = "wasm32"))]
use winit::window::CursorGrabMode;
use winit::window::Window;

use super::DesktopApp;

#[derive(Default)]
pub(super) struct CursorCapture {
    locked: bool,
    wanted: bool,
}

impl CursorCapture {
    pub(super) fn request<A: DesktopApp>(&mut self, window: &Window, app: &mut A, lock: bool) {
        if lock && !window.has_focus() {
            return;
        }
        self.wanted = lock;
        if lock && self.locked {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let result = {
            let mode = if lock {
                CursorGrabMode::Locked
            } else {
                CursorGrabMode::None
            };
            let result = window.set_cursor_grab(mode);
            // X11 offers confinement. Raw device motion continues at edges.
            if lock {
                result.or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
            } else {
                result
            }
        };
        #[cfg(target_arch = "wasm32")]
        let result = request_browser_capture(window, lock);
        if let Err(error) = result {
            log::warn!("cursor capture request failed: {error:?}");
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.report(window, app, lock);
        #[cfg(target_arch = "wasm32")]
        self.sync(window, app);
    }

    pub(super) fn sync<A: DesktopApp>(&mut self, window: &Window, app: &mut A) {
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;

            let actual = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.pointer_lock_element())
                .zip(window.canvas())
                .is_some_and(|(element, canvas)| element.is_same_node(Some(&canvas)));
            // An asynchronous grant may arrive after an explicit release.
            if actual && !self.wanted {
                let _ = request_browser_capture(window, false);
                return;
            }
            if self.locked && !actual {
                self.wanted = false;
            }
            self.report(window, app, actual);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (self, window, app);
    }

    fn report<A: DesktopApp>(&mut self, window: &Window, app: &mut A, locked: bool) {
        if self.locked != locked {
            self.locked = locked;
            window.set_cursor_visible(!locked);
            app.input(InputEvent::PointerLockChanged(locked));
        }
    }

    pub(super) fn motion(&self, delta: (f64, f64)) -> Option<InputEvent> {
        if !self.locked {
            return None;
        }
        // Platform input rejects nonfinite narrowed values and overflow. Raw
        // native counts and browser physical-pixel motion fit f32 in practice.
        #[allow(clippy::cast_possible_truncation)]
        Some(InputEvent::PointerMotion {
            x: delta.0 as f32,
            y: delta.1 as f32,
        })
    }
}

impl<A: DesktopApp> super::Host<A> {
    pub(super) fn release_cursor(&mut self) {
        if let Some(window) = &self.window
            && let super::State::Running(running) = &mut self.state
        {
            self.cursor.request(window, &mut running.app, false);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn request_browser_capture(window: &Window, lock: bool) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};
    use winit::platform::web::WindowExtWebSys;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("the browser document is unavailable"))?;
    if !lock {
        document.exit_pointer_lock();
        return Ok(());
    }
    let canvas = window
        .canvas()
        .ok_or_else(|| JsValue::from_str("the game canvas is unavailable"))?;
    // web-sys/winit's void binding cannot catch synchronous SecurityError
    // exceptions. Function.call catches those, and modern browsers may also
    // return a Promise whose asynchronous rejection must be consumed.
    let method = js_sys::Reflect::get(canvas.as_ref(), &JsValue::from_str("requestPointerLock"))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str("this browser does not support cursor capture"))?;
    let result = method.call0(canvas.as_ref())?;
    if let Ok(promise) = result.dyn_into::<js_sys::Promise>() {
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) = wasm_bindgen_futures::JsFuture::from(promise).await {
                log::warn!("browser cursor capture was denied: {error:?}");
            }
        });
    }
    Ok(())
}
