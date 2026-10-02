//! The last frame drawn of each scene, for the board's cards.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::{egui, wgpu};
use sindri_render::{ViewportTarget, sampled_format};

/// How often the open scene's picture is taken from a view that is drawing
/// it. A picture is a copy on the GPU, cheap but not free.
const PICTURE_EVERY: Duration = Duration::from_millis(750);

/// The last frame drawn of each scene, kept on the GPU.
///
/// Taken from the views rather than rendered for the board, so a card shows
/// what the scene looked like the last time anybody looked — with its
/// stylesheets, its lighting and its camera — and not a second renderer's
/// idea of it. A scene not opened since the editor started has none, and its
/// card says what it holds instead.
#[derive(Default)]
pub(in crate::native) struct ScenePictures {
    taken: BTreeMap<PathBuf, Picture>,
    /// When the Game view last gave a picture. The Game view is the player's
    /// framing, so the Scene view only gives one while the Game view is not
    /// being drawn.
    from_game: Option<Instant>,
}

struct Picture {
    texture: wgpu::Texture,
    id: egui::TextureId,
    size: [u32; 2],
    at: Instant,
}

impl ScenePictures {
    /// Takes a picture of `scene` from a target a view just drew into, unless
    /// one was taken recently, or a better view is giving them.
    pub(in crate::native) fn take(
        &mut self,
        state: &eframe::egui_wgpu::RenderState,
        target: &ViewportTarget,
        scene: &Path,
        game: bool,
    ) {
        let now = Instant::now();
        if !game
            && self
                .from_game
                .is_some_and(|at| now.duration_since(at) < PICTURE_EVERY * 3)
        {
            return;
        }
        if let Some(picture) = self.taken.get(scene)
            && now.duration_since(picture.at) < PICTURE_EVERY
        {
            return;
        }
        if game {
            self.from_game = Some(now);
        }
        let size = [target.width(), target.height()];
        let reuse = self
            .taken
            .get(scene)
            .is_some_and(|picture| picture.size == size);
        if !reuse {
            let texture = state.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Sindri editor scene picture"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: ViewportTarget::FORMAT,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[sampled_format(ViewportTarget::FORMAT)],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(sampled_format(ViewportTarget::FORMAT)),
                ..Default::default()
            });
            let mut renderer = state.renderer.write();
            let id = match self.taken.remove(scene) {
                Some(old) => {
                    renderer.update_egui_texture_from_wgpu_texture(
                        &state.device,
                        &view,
                        wgpu::FilterMode::Linear,
                        old.id,
                    );
                    old.id
                }
                None => {
                    renderer.register_native_texture(&state.device, &view, wgpu::FilterMode::Linear)
                }
            };
            self.taken.insert(
                scene.to_path_buf(),
                Picture {
                    texture,
                    id,
                    size,
                    at: now,
                },
            );
        }
        let Some(picture) = self.taken.get_mut(scene) else {
            return;
        };
        picture.at = now;
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sindri editor scene picture"),
            });
        encoder.copy_texture_to_texture(
            target.color().as_image_copy(),
            picture.texture.as_image_copy(),
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
        state.queue.submit([encoder.finish()]);
    }

    pub(super) fn get(&self, scene: &Path) -> Option<(egui::TextureId, [u32; 2])> {
        self.taken
            .get(scene)
            .map(|picture| (picture.id, picture.size))
    }
}
