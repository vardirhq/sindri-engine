//! Counters follow physical batches, empty guards and persistent slot reuse.

use glam::Mat4;
use sindri_gpu::{GpuContext, GpuRequestOptions};
use sindri_render::{
    BatchDrawStats, ClearOperations, DepthTarget, DrawContext, FrameTarget, GlyphInstance,
    GlyphRenderer, OffscreenTarget, Shape, ShapeBlend, ShapeInstance, ShapeRenderer,
    SpriteBatchRenderer, SpriteDepth, SpriteInstance, Texture2D, TextureRegistry, UvRect,
    encode_clear,
};

struct Batches {
    sprites: SpriteBatchRenderer,
    shapes: ShapeRenderer,
    glyphs: GlyphRenderer,
}

impl Batches {
    fn snapshot(&self) -> BatchDrawStats {
        self.sprites
            .draw_stats()
            .combined(self.shapes.draw_stats())
            .combined(self.glyphs.draw_stats())
    }

    fn draw(
        &mut self,
        context: DrawContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        target: FrameTarget<'_>,
        atlas: &Texture2D,
    ) {
        self.sprites.begin_submission();
        self.shapes.begin_submission();
        self.glyphs.begin_submission();
        let before = self.snapshot();
        for empty in [true, false] {
            let sprites = [SpriteInstance::new(Mat4::IDENTITY, [1.0; 4]); 2];
            let shapes = [ShapeInstance::filled(Mat4::IDENTITY, Shape::Rect, [1.0; 4]); 3];
            let glyphs = [GlyphInstance::new(Mat4::IDENTITY, UvRect::FULL, [1.0; 4]); 4];
            self.sprites
                .draw(
                    context.device,
                    context.queue,
                    encoder,
                    target.color,
                    target.depth,
                    context.textures,
                    context.texture,
                    Mat4::IDENTITY,
                    SpriteDepth::Ignore,
                    if empty { &[] } else { &sprites },
                )
                .unwrap();
            self.shapes
                .draw(
                    context.device,
                    context.queue,
                    encoder,
                    target.color,
                    target.depth,
                    ShapeBlend::Over,
                    Mat4::IDENTITY,
                    if empty { &[] } else { &shapes },
                )
                .unwrap();
            self.glyphs
                .draw(
                    context.device,
                    context.queue,
                    encoder,
                    target.color,
                    target.depth,
                    atlas,
                    1,
                    Mat4::IDENTITY,
                    if empty { &[] } else { &glyphs },
                )
                .unwrap();
            if empty {
                assert_eq!(self.snapshot(), before, "empty batches issue no work");
            }
        }
    }
}

#[test]
fn physical_counts_survive_submission_reuse_and_ignore_empty_batches() {
    let gpu = match pollster::block_on(GpuContext::request(
        &wgpu::Instance::default(),
        None,
        &GpuRequestOptions::default(),
    )) {
        Ok(gpu) => gpu,
        Err(error) => {
            assert!(
                std::env::var_os("SINDRI_REQUIRE_GPU").is_none(),
                "required GPU unavailable: {error}"
            );
            return;
        }
    };
    let target = OffscreenTarget::new(&gpu.device, 64, 64).unwrap();
    let depth = DepthTarget::new(&gpu.device, 64, 64);
    let mut textures = TextureRegistry::new(&gpu.device, &gpu.queue);
    let texture = textures.insert(
        Texture2D::from_rgba8(&gpu.device, &gpu.queue, "batch sprite", 1, 1, &[255; 4]).unwrap(),
    );
    let atlas =
        Texture2D::from_rgba8(&gpu.device, &gpu.queue, "batch atlas", 1, 1, &[255; 4]).unwrap();
    let mut batches = Batches {
        sprites: SpriteBatchRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
        shapes: ShapeRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
        glyphs: GlyphRenderer::new(&gpu.device, OffscreenTarget::FORMAT),
    };
    let expected_bytes = 2 * std::mem::size_of::<SpriteInstance>()
        + 3 * std::mem::size_of::<ShapeInstance>()
        + 4 * std::mem::size_of::<GlyphInstance>();
    for frame in 1..=2 {
        let before = batches.snapshot();
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encode_clear(
            &mut encoder,
            target.view(),
            &depth,
            ClearOperations::default(),
        );
        batches.draw(
            DrawContext {
                device: &gpu.device,
                queue: &gpu.queue,
                textures: &textures,
                texture,
            },
            &mut encoder,
            FrameTarget {
                color: target.view(),
                depth: &depth,
            },
            &atlas,
        );
        let readback = target.copy_to_buffer(&gpu.device, &mut encoder).unwrap();
        gpu.queue.submit([encoder.finish()]);
        assert_eq!(readback.read_rgba8(&gpu.device).unwrap().len(), 64 * 64 * 4);
        let delta = batches.snapshot().since(before);
        assert_eq!(delta.draw_calls, 3);
        assert_eq!(delta.render_passes, 3);
        assert_eq!(delta.instances, 9);
        assert_eq!(delta.buffer_writes, 6);
        assert_eq!(delta.uniform_bytes, 288);
        assert_eq!(delta.instance_bytes, u64::try_from(expected_bytes).unwrap());
        assert_eq!(batches.snapshot().draw_calls, frame * 3);
    }
}
