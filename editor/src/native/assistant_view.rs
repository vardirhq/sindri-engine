//! The Assistant panel: set up a local model without leaving the editor.
//!
//! One button starts everything, and the panel then shows each step as it
//! happens — what it is doing, how much has arrived and how long is left — so
//! nobody is left wondering whether anything is happening. It decides nothing
//! itself: `crate::assistant::setup` says which state the machine is in, the
//! install pipeline does the work on a worker, and this draws both.

mod cards;

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::assistant::install::{self, Failure, Finished, Shared};
use crate::assistant::managed::Home;
use crate::assistant::server::{Runner, Server};
use crate::assistant::{Machine, Setup, Step, setup};
use crate::ui::icons;
use crate::ui::widgets::button::{self, Intent};

use super::EditorApp;

/// How often a settled machine is looked at again while the panel is open.
const SETTLED: Duration = Duration::from_secs(30);

/// Everything the editor knows about the local assistant.
#[derive(Default)]
pub(crate) struct AssistantState {
    machine: Option<Machine>,
    looking: Option<Receiver<Machine>>,
    looked: Option<Instant>,
    job: Option<Job>,
    failure: Option<Shown>,
    /// The running model, shared with whatever is asking it something.
    /// Dropping the last handle stops it.
    server: Arc<Mutex<Option<Server>>>,
    confirm_remove: bool,
    /// A previewed script's compiler errors, and any fix proposed for them.
    pub(super) repair: super::repair_view::RepairState,
}

/// A setup running on a worker.
struct Job {
    shared: Arc<Shared>,
    done: Receiver<Result<Finished, Failure>>,
    speed: Speed,
}

/// A failure as the panel shows it.
struct Shown {
    step: Step,
    message: String,
    details: Option<String>,
}

/// Download speed, smoothed so the estimate does not jump every frame.
#[derive(Default)]
struct Speed {
    last: Option<(Instant, u64)>,
    rate: f64,
}

impl Speed {
    fn sample(&mut self, arrived: u64) {
        let now = Instant::now();
        match self.last {
            Some((then, before)) if now.duration_since(then) >= Duration::from_millis(500) => {
                let seconds = now.duration_since(then).as_secs_f64();
                #[allow(clippy::cast_precision_loss)] // bytes per second, shown to two figures
                let instant = arrived.saturating_sub(before) as f64 / seconds;
                self.rate = if self.rate == 0.0 {
                    instant
                } else {
                    self.rate * 0.8 + instant * 0.2
                };
                self.last = Some((now, arrived));
            }
            None => self.last = Some((now, arrived)),
            Some(_) => {}
        }
    }
}

/// How a worker reaches the running model, starting it first if it is not.
///
/// Cloned into whatever thread needs it, so a repair asked for while the model
/// is stopped starts it rather than failing.
#[derive(Clone)]
pub(crate) struct Connector {
    home: Home,
    machine: Machine,
    server: Arc<Mutex<Option<Server>>>,
}

impl Connector {
    pub(crate) fn connect(&self, shared: &Shared) -> Result<Runner, String> {
        let (Some(runtime), Some(model)) = (&self.machine.runtime, &self.machine.model) else {
            return Err("the assistant is not set up".to_owned());
        };
        let mut running = self
            .server
            .lock()
            .map_err(|_| "the assistant is busy".to_owned())?;
        if !running.as_mut().is_some_and(Server::alive) {
            *running = Some(
                install::start(&self.home, runtime, model, shared)
                    .map_err(|failure| failure.friendly())?,
            );
        }
        running
            .as_ref()
            .map(Server::model)
            .ok_or_else(|| "the assistant stopped".to_owned())
    }
}

impl AssistantState {
    /// The model's name and a way to reach it, once it has proved it can
    /// repair Decay on this machine.
    pub(crate) fn repairer(&self) -> Option<(String, Connector)> {
        let machine = self.machine.clone()?;
        if !matches!(setup(&machine), Setup::Ready { .. }) {
            return None;
        }
        let name = machine.model.as_ref()?.display_name.clone();
        Some((
            name,
            Connector {
                home: Home::standard()?,
                machine,
                server: Arc::clone(&self.server),
            },
        ))
    }

    fn running(&self) -> bool {
        self.server
            .lock()
            .is_ok_and(|mut server| server.as_mut().is_some_and(Server::alive))
    }
}

impl EditorApp {
    /// The panel.
    pub(super) fn assistant_body(&mut self, ui: &mut egui::Ui) {
        self.look_when_due(ui.ctx());
        self.take_finished_job();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| self.assistant_card(ui));
    }

    /// The panel's own control: look at the machine again, now.
    pub(super) fn assistant_actions(&mut self, ui: &mut egui::Ui) {
        if button::row_icon(ui, icons::REFRESH, Intent::Quiet, "Look again").clicked() {
            self.assistant.looked = None;
        }
    }

    /// Reads the machine when the panel opens and every so often after, on a
    /// worker: asking a GPU tool how much memory it has can drop a frame.
    fn look_when_due(&mut self, context: &egui::Context) {
        if let Some(channel) = &self.assistant.looking {
            match channel.try_recv() {
                Ok(machine) => {
                    self.assistant.machine = Some(machine);
                    self.assistant.looking = None;
                }
                Err(TryRecvError::Empty) => {
                    context.request_repaint_after(Duration::from_millis(100));
                    return;
                }
                Err(TryRecvError::Disconnected) => self.assistant.looking = None,
            }
        }
        let due = self.assistant.job.is_none()
            && self
                .assistant
                .looked
                .is_none_or(|looked| looked.elapsed() >= SETTLED);
        if !due {
            return;
        }
        self.assistant.looked = Some(Instant::now());
        let Some(home) = Home::standard() else {
            return;
        };
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let _ = sender.send(Machine::look(&home));
        });
        self.assistant.looking = Some(receiver);
    }

    fn begin_setup(&mut self) {
        let (Some(machine), Some(home)) = (self.assistant.machine.clone(), Home::standard()) else {
            return;
        };
        let (Some(runtime), Some(model)) = (machine.runtime, machine.model) else {
            return;
        };
        // One model at a time: a running one would be started a second time.
        if let Ok(mut server) = self.assistant.server.lock() {
            *server = None;
        }
        let shared = Arc::new(Shared::default());
        let worker = Arc::clone(&shared);
        let (sender, done) = channel();
        std::thread::spawn(move || {
            let _ = sender.send(install::install(&home, &runtime, &model, &worker));
        });
        self.console.info("Setting up the local assistant");
        self.assistant.failure = None;
        self.assistant.job = Some(Job {
            shared,
            done,
            speed: Speed::default(),
        });
    }

    fn take_finished_job(&mut self) {
        let Some(job) = &self.assistant.job else {
            return;
        };
        let result = match job.done.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.assistant.job = None;
                return;
            }
        };
        self.assistant.job = None;
        self.assistant.looked = None;
        match result {
            Ok(finished) => {
                self.console.info(if finished.verified.is_empty() {
                    "The local assistant is installed, but the model could not fix Sindri's \
                     test scripts"
                } else {
                    "The local assistant is ready"
                });
                if let Ok(mut server) = self.assistant.server.lock() {
                    *server = Some(finished.server);
                }
            }
            Err(failure) if failure.cancelled() => {
                self.console.info("Stopped setting up the local assistant");
            }
            Err(failure) => {
                self.console
                    .info(format!("Setting up the local assistant failed: {failure}"));
                self.assistant.failure = Some(Shown {
                    step: failure.step(),
                    message: failure.friendly(),
                    details: failure.details().map(str::to_owned),
                });
            }
        }
    }

    fn stop_model(&mut self) {
        if let Ok(mut server) = self.assistant.server.lock() {
            *server = None;
        }
        self.console.info("Stopped the local model");
    }

    fn remove_assistant(&mut self) {
        self.stop_model();
        self.assistant.confirm_remove = false;
        let Some(home) = Home::standard() else {
            return;
        };
        match home.remove() {
            Ok(()) => self.console.info("Removed the local assistant"),
            Err(error) => self.report(format!("Could not remove the local assistant: {error}")),
        }
        self.assistant.machine = None;
        self.assistant.looked = None;
    }
}
