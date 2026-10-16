//! The browser host: Web Audio for sound, downloads and file inputs for files.
use super::{Opened, Purpose, SaveAs};
use crate::lab::{RATE, live::LiveSource};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    AudioBufferSourceNode, AudioContext, AudioProcessingEvent, AudioScheduledSourceNode, Blob,
    BlobPropertyBag, HtmlAnchorElement, HtmlCanvasElement, HtmlInputElement, ScriptProcessorNode,
    Url,
};

thread_local! {
    /// One context per page. Browsers start it suspended until a gesture.
    static CONTEXT: RefCell<Option<AudioContext>> = const { RefCell::new(None) };
}

/// Samples the processor asks for at a time: about 90 ms, so a busy frame in
/// the interface does not starve it.
const BLOCK: u32 = 4096;

/// The song as the processor plays it, resampled from the Lab's 44.1 kHz to
/// whatever rate the device runs at.
struct Live {
    source: LiveSource,
    paused: bool,
    finished: bool,
    /// Lab samples per device sample, and how far between `a` and `b` we are.
    step: f64,
    phase: f64,
    a: f32,
    b: f32,
}

impl Live {
    // Display-quality linear interpolation between two adjacent samples.
    #[allow(clippy::cast_possible_truncation)]
    fn fill(&mut self, out: &mut [f32]) {
        for sample in out {
            if self.paused || self.finished {
                *sample = 0.0;
                continue;
            }
            while self.phase >= 1.0 {
                self.phase -= 1.0;
                self.a = self.b;
                if let Some(next) = self.source.next() {
                    self.b = next;
                } else {
                    self.finished = true;
                    break;
                }
            }
            *sample = self.a + (self.b - self.a) * self.phase as f32;
            self.phase += self.step;
        }
    }
}

type Process = Closure<dyn FnMut(AudioProcessingEvent)>;

pub(in crate::lab::gui) struct Speaker {
    context: AudioContext,
    live: Option<(ScriptProcessorNode, Process, Rc<RefCell<Live>>)>,
    once: Option<AudioBufferSourceNode>,
}

fn text(error: &JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

impl Speaker {
    pub(in crate::lab::gui) fn open() -> Result<Self, String> {
        let context = CONTEXT
            .with(|slot| slot.borrow().clone())
            .ok_or("Web Audio is not available in this browser")?;
        // A gesture already reached the page if anything is being played.
        let _ = context.resume();
        Ok(Self {
            context,
            live: None,
            once: None,
        })
    }

    pub(in crate::lab::gui) fn live(&mut self, source: LiveSource) {
        self.stop_live();
        let Ok(node) = self
            .context
            .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
                BLOCK, 0, 1,
            )
        else {
            return;
        };
        let state = Rc::new(RefCell::new(Live {
            source,
            paused: false,
            finished: false,
            step: f64::from(RATE) / f64::from(self.context.sample_rate()),
            phase: 1.0,
            a: 0.0,
            b: 0.0,
        }));
        let shared = Rc::clone(&state);
        let process: Process = Closure::new(move |event: AudioProcessingEvent| {
            let Ok(buffer) = event.output_buffer() else {
                return;
            };
            let mut out = vec![0.0; buffer.length() as usize];
            shared.borrow_mut().fill(&mut out);
            let _ = buffer.copy_to_channel(&out, 0);
        });
        node.set_onaudioprocess(Some(process.as_ref().unchecked_ref()));
        let _ = node.connect_with_audio_node(&self.context.destination());
        self.live = Some((node, process, state));
    }

    pub(in crate::lab::gui) fn set_paused(&self, paused: bool) {
        if let Some((_, _, state)) = &self.live {
            state.borrow_mut().paused = paused;
        }
    }

    pub(in crate::lab::gui) fn stop_live(&mut self) {
        if let Some((node, _process, _)) = self.live.take() {
            node.set_onaudioprocess(None);
            let _ = node.disconnect();
        }
    }

    pub(in crate::lab::gui) fn live_finished(&self) -> bool {
        self.live
            .as_ref()
            .is_some_and(|(_, _, state)| state.borrow().finished)
    }

    // An effect is at most a few seconds, far inside u32 samples.
    #[allow(clippy::cast_precision_loss)]
    pub(in crate::lab::gui) fn once(&mut self, samples: &[f32]) {
        if let Some(previous) = self.once.take() {
            let _ = previous.unchecked_ref::<AudioScheduledSourceNode>().stop();
        }
        let Ok(length) = u32::try_from(samples.len()) else {
            return;
        };
        let played = (|| -> Result<AudioBufferSourceNode, JsValue> {
            let buffer = self.context.create_buffer(1, length.max(1), RATE as f32)?;
            buffer.copy_to_channel(samples, 0)?;
            let node = self.context.create_buffer_source()?;
            node.set_buffer(Some(&buffer));
            node.connect_with_audio_node(&self.context.destination())?;
            node.unchecked_ref::<AudioScheduledSourceNode>().start()?;
            Ok(node)
        })();
        self.once = played.ok();
    }
}

#[derive(Default)]
pub(in crate::lab::gui) struct Files {
    opened: Rc<RefCell<Vec<Opened>>>,
}

impl Files {
    /// Shows the browser's file picker; the file arrives through `poll`.
    pub(in crate::lab::gui) fn open(
        &mut self,
        context: &eframe::egui::Context,
        purpose: Purpose,
        _label: &str,
        extensions: &[&str],
    ) {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Ok(input) = document
            .create_element("input")
            .map(JsCast::unchecked_into::<HtmlInputElement>)
        else {
            return;
        };
        input.set_type("file");
        let accept: Vec<String> = extensions.iter().map(|e| format!(".{e}")).collect();
        input.set_accept(&accept.join(","));
        let opened = Rc::clone(&self.opened);
        let context = context.clone();
        let picked = input.clone();
        let change = Closure::once_into_js(move || {
            let Some(file) = picked.files().and_then(|list| list.get(0)) else {
                return;
            };
            wasm_bindgen_futures::spawn_local(async move {
                let Ok(buffer) = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await
                else {
                    return;
                };
                opened.borrow_mut().push(Opened {
                    purpose,
                    name: file.name(),
                    bytes: js_sys::Uint8Array::new(&buffer).to_vec(),
                });
                context.request_repaint();
            });
        });
        input.set_onchange(Some(change.unchecked_ref()));
        input.click();
    }

    pub(in crate::lab::gui) fn poll(&mut self) -> Option<Opened> {
        self.opened.borrow_mut().pop()
    }

    /// Downloads `bytes` under the save's file name.
    // One signature for both hosts; neither keeps state for a save.
    #[allow(clippy::unused_self)]
    pub(in crate::lab::gui) fn save(
        &mut self,
        save: &SaveAs<'_>,
        bytes: &[u8],
    ) -> Result<Option<String>, String> {
        download(&save.file_name(), save.mime(), bytes).map_err(|e| text(&e))?;
        Ok(Some(save.file_name()))
    }
}

fn download(name: &str, mime: &str, bytes: &[u8]) -> Result<(), JsValue> {
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let options = BlobPropertyBag::new();
    options.set_type(mime);
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let anchor: HtmlAnchorElement = document.create_element("a")?.unchecked_into();
    anchor.set_href(&url);
    anchor.set_download(name);
    anchor.click();
    // Released after the browser has taken the download, not before.
    let release = Closure::once_into_js(move || {
        let _ = Url::revoke_object_url(&url);
    });
    if let Some(window) = web_sys::window() {
        let _ = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(release.unchecked_ref(), 10_000);
    }
    Ok(())
}

/// The page address's query string, `?mode=sfx` and the like.
pub(in crate::lab) fn search() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default()
}

/// Starts the Lab on the page's `#sound-lab` canvas.
pub(in crate::lab) fn start(
    app: impl FnOnce(&eframe::CreationContext<'_>) -> Box<dyn eframe::App> + 'static,
) {
    console_error_panic_hook::set_once();
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Ok(context) = AudioContext::new() {
        // Browsers keep audio silent until the person interacts with the page;
        // every gesture resumes it, which is free once it is running.
        let resume = context.clone();
        let unlock = Closure::<dyn FnMut()>::new(move || {
            let _ = resume.resume();
        });
        for event in ["pointerdown", "keydown", "touchend"] {
            let _ =
                document.add_event_listener_with_callback(event, unlock.as_ref().unchecked_ref());
        }
        unlock.forget();
        CONTEXT.with(|slot| *slot.borrow_mut() = Some(context));
    }
    let Some(canvas) = document
        .get_element_by_id("sound-lab")
        .and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok())
    else {
        return;
    };
    // WebGL2 rather than WebGPU: the Lab draws a flat interface, every phone
    // has WebGL2, and Chrome on Android exposes `navigator.gpu` more widely
    // than its drivers can serve, which leaves a blank canvas.
    let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = eframe::wgpu::Backends::GL;
    let mut options = eframe::WebOptions::default();
    options.wgpu_options.wgpu_setup = eframe::egui_wgpu::WgpuSetup::CreateNew(setup);
    wasm_bindgen_futures::spawn_local(async move {
        let started = eframe::WebRunner::new()
            .start(canvas, options, Box::new(move |cc| Ok(app(cc))))
            .await;
        if let (Err(error), Some(loading)) = (
            started,
            web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("loading")),
        ) {
            loading.set_text_content(Some(&format!(
                "The Sound Lab could not start: {}",
                text(&error)
            )));
        } else if let Some(loading) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading"))
        {
            loading.remove();
        }
    });
}
