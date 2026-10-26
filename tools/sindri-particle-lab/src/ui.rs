use crate::{presets, preview::Preview};
use eframe::egui::{self, Color32, Sense, Vec2};
use sindri_scene::{EmissionShape, ParticleSpace, ParticlesComponent};
use std::time::Duration;
use web_time::Instant;

/// An authoring session. `show` returns a validated component on Apply.
pub struct ParticleLab {
    pub component: ParticlesComponent,
    preview: Option<Preview>,
    last: Option<Instant>,
    status: String,
    import: String,
    texture: Option<egui::TextureHandle>,
    pending_texture: Option<Vec<u8>>,
    background: usize,
    burst_count: u32,
    preset: usize,
}
impl Default for ParticleLab {
    fn default() -> Self {
        let component = presets::preset(0);
        let preview = Preview::new(&component).ok();
        Self {
            component,
            preview,
            last: None,
            status: String::new(),
            import: String::new(),
            texture: None,
            pending_texture: None,
            background: 0,
            burst_count: 30,
            preset: 0,
        }
    }
}
impl ParticleLab {
    /// Queues a local image for the preview; it does not change the asset path.
    pub fn queue_preview_texture(&mut self, bytes: Vec<u8>) {
        self.pending_texture = Some(bytes);
    }
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<ParticlesComponent> {
        self.show_inner(ui, true)
    }
    fn show_inner(&mut self, ui: &mut egui::Ui, editor: bool) -> Option<ParticlesComponent> {
        ui.heading("Particle Lab");
        ui.label("Author a Sindri emitter and preview its particle simulation.");
        let mut applied = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("particle-preset").selected_text(presets::NAMES[self.preset]).show_ui(ui, |ui| {
                    for (index, name) in presets::NAMES.iter().enumerate() {
                        if ui.selectable_label(index == self.preset, *name).clicked() {
                            self.preset = index;
                            let texture = self.component.texture.clone();
                            self.component = presets::preset(index);
                            self.component.texture = texture;
                            self.preview = Preview::new(&self.component).ok();
                        }
                    }
                });
                ui.checkbox(&mut self.component.playing, "Play");
                if ui.button("Burst").clicked()
                    && let Some(preview) = self.preview.as_mut()
                    && let Err(error) = preview.burst(&self.component, self.burst_count)
                {
                    self.status = error;
                }
                ui.add(egui::DragValue::new(&mut self.burst_count).range(1..=1000).prefix("Count "));
                if ui.button("Clear").clicked() {
                    self.preview = Preview::new(&self.component).ok();
                }
            });
            self.load_dropped_texture(ui);
            self.draw_preview(ui);
            ui.horizontal(|ui| {
                ui.label("Background");
                for (index, name) in ["Dark", "Light", "Forest", "Industrial"].into_iter().enumerate() {
                    ui.selectable_value(&mut self.background, index, name);
                }
            });
            ui.small("Drop an image on the preview, or use the browser image picker. Scene lighting is not shown.");
            ui.horizontal(|ui| {
                ui.label("Texture asset");
                ui.text_edit_singleline(&mut self.component.texture);
            });
            ui.small("Use a texture path from your Sindri project before applying or exporting.");
            self.controls(ui);
            ui.collapsing("Import emitter JSON", |ui| {
                ui.add(egui::TextEdit::multiline(&mut self.import).desired_rows(4).hint_text("Paste emitter JSON"));
                if ui.button("Load emitter").clicked() {
                    match serde_json::from_str::<ParticlesComponent>(&self.import) {
                        Ok(component) => match component.validate() {
                            Ok(()) => {
                                self.component = component;
                                self.preview = Preview::new(&self.component).ok();
                                self.status = "Emitter loaded.".into();
                            }
                            Err(error) => self.status = error.to_string(),
                        },
                        Err(error) => self.status = error.to_string(),
                    }
                }
            });
            if ui.button("Copy emitter JSON").clicked() {
                match self.json() {
                    Ok(json) => { ui.ctx().copy_text(json); self.status = "Emitter JSON copied.".into(); }
                    Err(error) => self.status = error,
                }
            }
            #[cfg(target_arch = "wasm32")]
            if !editor && ui.button("Download emitter JSON").clicked() {
                match self.json().and_then(|json| crate::download(&json)) {
                    Ok(()) => self.status = "Downloaded emitter JSON.".into(),
                    Err(error) => self.status = error,
                }
            }
            if editor && ui.button("Apply to selected emitter").clicked() {
                match self.json() {
                    Ok(_) => applied = Some(self.component.clone()),
                    Err(error) => self.status = error,
                }
            }
            if !self.status.is_empty() { ui.label(&self.status); }
        });
        applied
    }
    pub fn json(&self) -> Result<String, String> {
        self.component.validate().map_err(|e| e.to_string())?;
        if self.component.texture.trim().is_empty() {
            return Err("Enter a texture asset path before exporting or applying.".into());
        }
        serde_json::to_string_pretty(&self.component).map_err(|e| e.to_string())
    }
    fn draw_preview(&mut self, ui: &mut egui::Ui) {
        let now = Instant::now();
        let delta = self.last.map_or(Duration::ZERO, |last| {
            (now - last).min(Duration::from_millis(100))
        });
        self.last = Some(now);
        if let Some(preview) = self.preview.as_mut()
            && let Err(error) = preview.step(&self.component, delta)
        {
            self.status = error;
        }
        let width = ui.available_width().max(100.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 260.0), Sense::hover());
        let bg = [
            Color32::from_rgb(14, 20, 28),
            Color32::from_gray(225),
            Color32::from_rgb(35, 55, 42),
            Color32::from_rgb(65, 63, 61),
        ][self.background];
        ui.painter().rect_filled(rect, 5.0, bg);
        let painter = ui.painter().with_clip_rect(rect);
        if let Some(preview) = self.preview.as_ref() {
            if let Some(texture) = self.texture.as_ref() {
                let mut mesh = egui::Mesh::with_texture(texture.id());
                for fleck in preview.effects.flecks() {
                    let tint = preview.effects.color_of(fleck);
                    let color = Color32::from(egui::Rgba::from_rgba_unmultiplied(
                        tint[0], tint[1], tint[2], tint[3],
                    ));
                    let at = rect.center()
                        + Vec2::new(fleck.position[0] * 50.0, -fleck.position[1] * 50.0);
                    let half = fleck.drawn_size() * 25.0;
                    let (sin, cos) = fleck.rotation.sin_cos();
                    let base =
                        u32::try_from(mesh.vertices.len()).expect("particle count bounded by pool");
                    for (x, y, u, v) in [
                        (-1.0, -1.0, 0.0, 0.0),
                        (1.0, -1.0, 1.0, 0.0),
                        (1.0, 1.0, 1.0, 1.0),
                        (-1.0, 1.0, 0.0, 1.0),
                    ] {
                        let rotated =
                            Vec2::new((x * cos + y * sin) * half, (-x * sin + y * cos) * half);
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: at + rotated,
                            uv: egui::pos2(u, v),
                            color,
                        });
                    }
                    mesh.indices.extend_from_slice(&[
                        base,
                        base + 1,
                        base + 2,
                        base,
                        base + 2,
                        base + 3,
                    ]);
                }
                painter.add(egui::Shape::mesh(mesh));
            } else {
                for fleck in preview.effects.flecks() {
                    let tint = preview.effects.color_of(fleck);
                    let rgba =
                        egui::Rgba::from_rgba_unmultiplied(tint[0], tint[1], tint[2], tint[3]);
                    let at = rect.center()
                        + Vec2::new(fleck.position[0] * 50.0, -fleck.position[1] * 50.0);
                    painter.circle_filled(
                        at,
                        (fleck.drawn_size() * 25.0).max(0.5),
                        Color32::from(rgba),
                    );
                }
            }
        }
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
    fn load_dropped_texture(&mut self, ui: &egui::Ui) {
        if let Some(bytes) = self.pending_texture.take() {
            self.load_texture(ui, &bytes);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(bytes) = crate::PREVIEW_IMAGE.with(|slot| slot.borrow_mut().take()) {
            self.load_texture(ui, &bytes);
        }
        let files = ui.input(|input| input.raw.dropped_files.clone());
        for file in files {
            #[cfg(not(target_arch = "wasm32"))]
            match file.bytes() {
                Ok(bytes) => self.load_texture(ui, &bytes),
                Err(error) => self.status = error,
            }
            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(bytes) = file.bytes_async().await {
                    crate::set_preview_image(bytes);
                }
            });
        }
    }
    fn load_texture(&mut self, ui: &egui::Ui, bytes: &[u8]) {
        match image::load_from_memory(bytes) {
            Ok(image) => {
                let image = image.to_rgba8();
                let size = [
                    usize::try_from(image.width()).expect("image width"),
                    usize::try_from(image.height()).expect("image height"),
                ];
                let image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                self.texture = Some(ui.ctx().load_texture(
                    "particle-preview",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
                self.status = "Preview texture loaded; set the project texture path below.".into();
            }
            Err(error) => self.status = format!("Could not load preview texture: {error}"),
        }
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        let p = &mut self.component;
        ui.collapsing("Emission", |ui| {
            ui.add(egui::Slider::new(&mut p.rate, 0.0..=500.0).text("Rate / second"));
            ui.add(egui::Slider::new(&mut p.max, 0..=2000).text("Maximum"));
            ui.add(egui::Slider::new(&mut p.lifetime, 0.05..=10.0).text("Lifetime"));
            ui.add(egui::Slider::new(&mut p.lifetime_spread, 0.0..=1.0).text("Lifetime variation"));
            ui.add(egui::Slider::new(&mut p.speed, 0.0..=20.0).text("Speed"));
            ui.add(egui::Slider::new(&mut p.speed_spread, 0.0..=1.0).text("Speed variation"));
            ui.add(egui::Slider::new(&mut p.direction, -180.0..=360.0).text("Direction"));
            ui.add(egui::Slider::new(&mut p.spread, 0.0..=360.0).text("Spread"));
            egui::ComboBox::from_id_salt("particle-shape")
                .selected_text(format!("{:?}", p.shape))
                .show_ui(ui, |ui| {
                    for (shape, name) in [
                        (EmissionShape::Point, "Point"),
                        (EmissionShape::Circle, "Circle"),
                        (EmissionShape::Box, "Box"),
                        (EmissionShape::Ring, "Ring"),
                    ] {
                        ui.selectable_value(&mut p.shape, shape, name);
                    }
                });
            ui.horizontal(|ui| {
                ui.label("Shape size");
                for value in &mut p.shape_size {
                    ui.add(egui::DragValue::new(value).speed(0.05).range(0.0..=100.0));
                }
            });
            ui.horizontal(|ui| {
                ui.label("Offset");
                for value in &mut p.offset {
                    ui.add(egui::DragValue::new(value).speed(0.05));
                }
            });
        });
        ui.collapsing("Motion & appearance", |ui| {
            ui.horizontal(|ui| {
                ui.label("Gravity");
                for value in &mut p.gravity {
                    ui.add(egui::DragValue::new(value).speed(0.1));
                }
            });
            ui.add(egui::Slider::new(&mut p.drag, 0.0..=10.0).text("Drag"));
            ui.add(egui::Slider::new(&mut p.size, 0.0..=2.0).text("Start size"));
            ui.add(egui::Slider::new(&mut p.size_end, 0.0..=2.0).text("End size"));
            ui.checkbox(&mut p.random_rotation, "Random rotation");
            ui.add(egui::Slider::new(&mut p.spin, -720.0..=720.0).text("Spin"));
            ui.horizontal(|ui| {
                ui.label("Space");
                ui.selectable_value(&mut p.space, ParticleSpace::World, "World");
                ui.selectable_value(&mut p.space, ParticleSpace::Local, "Local");
            });
            ui.horizontal(|ui| {
                ui.label("Layer");
                ui.add(egui::DragValue::new(&mut p.layer));
            });
            for (index, color) in p.colors.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Color {}", index + 1));
                    ui.color_edit_button_rgba_unmultiplied(color);
                });
            }
            ui.horizontal(|ui| {
                if ui.button("Add color").clicked() && p.colors.len() < 8 {
                    p.colors.push([1.0; 4]);
                }
                if ui.button("Remove color").clicked() && p.colors.len() > 1 {
                    p.colors.pop();
                }
            });
        });
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub struct WebApp {
    lab: ParticleLab,
}
#[cfg(target_arch = "wasm32")]
impl eframe::App for WebApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            if self.lab.show_inner(ui, false).is_some() {
                ui.label(
                    egui::RichText::new("Copy the emitter JSON into a Sindri scene component.")
                        .italics(),
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_requires_a_texture_and_preserves_the_emitter() {
        let mut lab = ParticleLab::default();
        assert!(lab.json().unwrap_err().contains("texture"));
        lab.component.texture = "textures/dust.png".into();
        let json = lab.json().unwrap();
        let restored: ParticlesComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, lab.component);
        lab.component.lifetime = 0.0;
        assert!(lab.json().is_err());
    }
}
