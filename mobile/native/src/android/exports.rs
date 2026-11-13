//! The functions `dev.sindri.mobile.NativeBridge` declares `external`.
//!
//! Each is exported unmangled so the JVM can find it by name, which is the one
//! unsafe thing about it: an exported symbol could clash with another of the
//! same name. The names are JNI's mangling of a class in this app's own
//! package, which nothing else in the process defines.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use jni::objects::{JClass, JObject, JString};
use jni::sys::{jint, jstring};
use jni::{Env, EnvOutcome, EnvUnowned, Outcome};

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
fn guarded<'local>(
    env: &mut EnvUnowned<'local>,
    call: impl FnOnce(&mut Env<'local>, &mut Host) -> Result<(), String>,
) -> jstring {
    returned(env.with_env(|env| {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            HOST.with(|host| call(env, &mut host.borrow_mut()))
        }));
        match outcome {
            Ok(Ok(())) => Ok(std::ptr::null_mut()),
            Ok(Err(message)) => java_string(env, message),
            Err(panic) => java_string(env, panicked(&*panic)),
        }
    }))
}

/// What a panic said, as the message Java receives.
fn panicked(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .map_or_else(
            || "the native host panicked".to_owned(),
            |text| format!("the native host panicked: {text}"),
        )
}

/// `text` as a new Java string, raw for returning to Java.
fn java_string(env: &mut Env<'_>, text: impl AsRef<str>) -> jni::errors::Result<jstring> {
    env.new_string(text).map(JString::into_raw)
}

/// What a native method returns: its value, or `null` when making it failed
/// or panicked, which leaves Java a pending exception or nothing to show.
fn returned(outcome: EnvOutcome<'_, jstring, jni::errors::Error>) -> jstring {
    match outcome.into_outcome() {
        Outcome::Ok(value) => value,
        Outcome::Err(_) | Outcome::Panic(_) => std::ptr::null_mut(),
    }
}

fn string(env: &Env<'_>, value: &JString<'_>) -> Result<String, String> {
    value.try_to_string(env).map_err(|error| error.to_string())
}

/// A pixel size from Java, which has no unsigned integers.
fn pixels(value: jint) -> u32 {
    u32::try_from(value).unwrap_or(0).max(1)
}

/// `fun open(project: String, scene: String): String?`
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_open<'local>(
    mut env: EnvUnowned<'local>,
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
    mut env: EnvUnowned<'local>,
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
    mut env: EnvUnowned<'local>,
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
    mut env: EnvUnowned<'local>,
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
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> jstring {
    guarded(&mut env, |_, host| host.draw())
}

/// `fun call(command: String, arguments: String): String?`, where `arguments`
/// is a JSON array; see `Host::call` for the commands.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_call<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    command: JString<'local>,
    arguments: JString<'local>,
) -> jstring {
    guarded(&mut env, |env, host| {
        let command = string(env, &command)?;
        let arguments = string(env, &arguments)?;
        host.call(&command, &arguments)
    })
}

/// `fun state(): String`, a JSON object; see `Host::state`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_state<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> jstring {
    let state = catch_unwind(|| HOST.with(|host| host.borrow().state())).unwrap_or_else(|_| {
        serde_json::json!({ "status": "the native host panicked" }).to_string()
    });
    returned(env.with_env(|env| java_string(env, state)))
}

/// `fun script(): String`, a JSON object or `null`; see `Host::script`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_sindri_mobile_NativeBridge_script<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> jstring {
    let script = catch_unwind(|| HOST.with(|host| host.borrow().script()))
        .unwrap_or_else(|_| "null".to_owned());
    returned(env.with_env(|env| java_string(env, script)))
}
