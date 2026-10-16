//! Noticing that a browser tab went away, so the frame clock does not
//! come back with a delta of several minutes -- or that the game is still
//! under the page's loading screen, so it does not start playing unseen.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};
#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoopProxy;

#[cfg(target_arch = "wasm32")]
use super::startup::Startup;

/// The page's loading screen, while it is up, covers the game: a game that
/// played under it would spend its first seconds where nobody can see them.
#[cfg(target_arch = "wasm32")]
const LOADING_SCREEN: &str = "sindri-loading";

/// What the page dispatches on `window` once its loading screen has gone.
#[cfg(target_arch = "wasm32")]
const REVEALED_EVENT: &str = "sindri:revealed";

/// Whether a player can see the game: the tab is showing, and no loading
/// screen is over it. A page without one is revealed from the start.
#[cfg(target_arch = "wasm32")]
fn seen(document: &web_sys::Document) -> bool {
    !document.hidden() && document.get_element_by_id(LOADING_SCREEN).is_none()
}

#[cfg(target_arch = "wasm32")]
pub(super) struct VisibilityListener {
    window: web_sys::Window,
    document: web_sys::Document,
    callback: Closure<dyn FnMut()>,
}

#[cfg(target_arch = "wasm32")]
impl VisibilityListener {
    pub(super) fn new(proxy: EventLoopProxy<Startup>) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let observed = document.clone();
        let callback = Closure::wrap(Box::new(move || {
            let visible = seen(&observed);
            if proxy
                .send_event(Startup::VisibilityChanged(visible))
                .is_err()
            {
                log::debug!("visibility changed after the event loop closed");
            }
        }) as Box<dyn FnMut()>);
        document.set_onvisibilitychange(Some(callback.as_ref().unchecked_ref()));
        let _ = window
            .add_event_listener_with_callback(REVEALED_EVENT, callback.as_ref().unchecked_ref());
        Some(Self {
            window,
            document,
            callback,
        })
    }

    pub(super) fn visible(&self) -> bool {
        seen(&self.document)
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for VisibilityListener {
    fn drop(&mut self) {
        self.document.set_onvisibilitychange(None);
        let _ = self.window.remove_event_listener_with_callback(
            REVEALED_EVENT,
            self.callback.as_ref().unchecked_ref(),
        );
    }
}
