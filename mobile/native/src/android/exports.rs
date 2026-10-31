//! The functions `dev.sindri.mobile.NativeBridge` declares `external`.
//!
//! Each is exported unmangled so the JVM can find it by name, which is the one
//! unsafe thing about it: an exported symbol could clash with another of the
//! same name. The names are JNI's mangling of a class in this app's own
//! package, which nothing else in the process defines.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use jni::JNIEnv;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jint, jstring};

use super::Host;
use super::window::AndroidWindow;

thread_local! {
    /// Java calls in on the UI thread only, so the host lives there. It holds
    /// a surface whose rebuild closure is not `Send`, which a thread-local
    /// does not need it to be.
    static HOST: RefCell<Host> = RefCell::new(Host::default());
}

/// Runs `call` against the host, turning its error or a panic into the
/// message Java receives, and success into `null`.
fn guarded(
    env: &mut JNIEnv<'_>,
    call: impl FnOnce(&mut JNIEnv<'_>, &mut Host) -> Result<(), String>,
) -> jstring {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        HOST.with(|host| call(env, &mut host.borrow_mut()))
    }));
    let message = match outcome {
        Ok(Ok(())) => return std::ptr::null_mut(),
        Ok(Err(message)) => message,
        Err(panic) => panic
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .map_or_else(
                || "the native host panicked".to_owned(),
                |text| format!("the native host panicked: {text}"),
            ),
    };
    env.new_string(message)
        .map_or(std::ptr::null_mut(), JString::into_raw)
}

fn string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Result<String, String> {
    env.get_string(value)
        .map(String::from)
        .map_err(|error| error.to_string())
}

/// A pixel size from Java, which has no unsigned integers.
fn pixels(value: jint) -> u32 {
    u32::try_from(value).unwrap_or(0).max(1)
}

/// `fun open(project: String, scene: String): String?`
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_open<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    project: JString<'local>,
    scene: JString<'local>,
) -> jstring {
    guarded(&mut env, |env, host| {
        let project = PathBuf::from(string(env, &project)?);
        let scene = string(env, &scene)?;
        host.open(project, scene)
    })
}

/// `fun attach(surface: Surface, width: Int, height: Int): String?`
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_attach<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    surface: JObject<'local>,
    width: jint,
    height: jint,
) -> jstring {
    guarded(&mut env, |env, host| {
        let window =
            AndroidWindow::from_surface(env, &surface).ok_or("the Surface has no native window")?;
        host.attach(window, pixels(width), pixels(height))
    })
}

/// `fun resize(width: Int, height: Int): String?`
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_resize<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    width: jint,
    height: jint,
) -> jstring {
    guarded(&mut env, |_, host| {
        host.resize(pixels(width), pixels(height));
        Ok(())
    })
}

/// `fun detach(): String?`
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_detach<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    guarded(&mut env, |_, host| {
        host.detach();
        Ok(())
    })
}

/// `fun draw(): String?`
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_draw<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    guarded(&mut env, |_, host| host.draw())
}

/// `fun status(): String`
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_status<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    let status = catch_unwind(|| HOST.with(|host| host.borrow().status()))
        .unwrap_or_else(|_| "the native host panicked".to_owned());
    env.new_string(status)
        .map_or(std::ptr::null_mut(), JString::into_raw)
}
