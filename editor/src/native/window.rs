//! Opening the editor's window.

use eframe::egui;

use super::EditorApp;
use crate::benchmark::BenchmarkPlan;

pub fn run() -> eframe::Result {
    let benchmark = match benchmark_plan() {
        Ok(plan) => plan,
        Err(problem) => {
            eprintln!("sindri-editor: {problem}");
            std::process::exit(2);
        }
    };
    let mut options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Sindri Editor")
            .with_inner_size([1_440.0, 1_024.0])
            .with_min_inner_size([1_100.0, 720.0])
            // Hidden until there is something to show. A launch that opens the
            // welcome window would otherwise flash an empty editor up behind
            // it, and the first frame is where the editor learns which of the
            // two this launch is: the preferences it decides from live in
            // eframe's storage, which does not exist until the app is built.
            //
            // eframe paints a hidden window directly, ten times a second, so
            // that a `Visible` command still reaches it. That is what makes
            // this safe rather than a window that can never be shown again.
            .with_visible(false),
        ..Default::default()
    };
    if benchmark.is_some() {
        // A frame's cost rather than the display's rate: with vsync a fast
        // frame and a slow one both report the refresh interval.
        options.wgpu_options.surface.present_mode = eframe::wgpu::PresentMode::AutoNoVsync;
        // Measured as a fresh install would see it, and leaving the person's
        // own settings as they were.
        options.persist_window = false;
        options.persistence_path = Some(std::env::temp_dir().join("sindri-editor-benchmark"));
    }
    eframe::run_native(
        "Sindri Editor",
        options,
        Box::new(|context| Ok(Box::new(EditorApp::new(context, benchmark)))),
    )
}

/// The benchmark the command line asks for, if any: everything after the
/// path being opened.
fn benchmark_plan() -> Result<Option<BenchmarkPlan>, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((first, _)) if first.starts_with("--") => Err(format!(
            "{first} comes after the scene or project to open, not before it"
        )),
        Some((_, rest)) => BenchmarkPlan::from_args(rest),
        None => Ok(None),
    }
}
