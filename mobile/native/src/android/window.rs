//! An Android `Surface`, as something `wgpu` can present to.

use jni::JNIEnv;
use jni::objects::JObject;
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};

/// The native window behind a `Surface`. It holds its own reference, released
/// when it is dropped, so the window stays valid for as long as `wgpu` holds
/// a surface made from it.
pub(super) struct AndroidWindow(NativeWindow);

impl AndroidWindow {
    /// The native window behind `surface`, an `android.view.Surface`.
    pub(super) fn from_surface(env: &JNIEnv<'_>, surface: &JObject<'_>) -> Option<Self> {
        // SAFETY: `env` is the live JNI environment of the thread Java called
        // in on, and `surface` is a local reference to an `android.view.Surface`
        // that Java passed into that same call, so both are valid for its whole
        // duration. `ANativeWindow_fromSurface` acquires its own reference to
        // the window, which `NativeWindow` releases on drop; nothing is kept of
        // either pointer once this returns.
        #[allow(unsafe_code)]
        let window = unsafe { NativeWindow::from_surface(env.get_raw(), surface.as_raw()) };
        window.map(Self)
    }
}

impl HasWindowHandle for AndroidWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for AndroidWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}

/// Android's display, which carries no data: the one display every native
/// window is on. What `wgpu` is told when its instance is made.
#[derive(Debug)]
pub(super) struct AndroidDisplay;

impl HasDisplayHandle for AndroidDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}
