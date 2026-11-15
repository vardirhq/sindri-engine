//! Bloom: the light a bright thing throws into the dark around it.
//!
//! The one effect that decides whether neon strokes on black read as light or as
//! clip art. Without it a bright ring is a colored line; with it the ring is a
//! source and the black around it is lit, which is the whole difference between
//! a vector drawing and a game that looks like it is glowing.
//!
//! It is opt in and sits *outside* [`crate::encode_prepared_frame`] rather than
//! inside it, because it changes where a frame is drawn: the scene goes to a
//! target this owns, and this then writes the scene plus its glow to wherever
//! the host actually wanted it. A host that does not want bloom keeps drawing
//! straight to its surface and pays nothing.
//!
//! ```no_run
//! # use sindri_render::{Bloom, PostProcessSettings};
//! # fn wrap(device: &wgpu::Device, queue: &wgpu::Queue,
//! #         encoder: &mut wgpu::CommandEncoder, surface: &wgpu::TextureView) {
//! let mut bloom = Bloom::new(device, wgpu::TextureFormat::Rgba8UnormSrgb);
//! bloom.resize(device, 1920, 1080);
//! // Draw the frame into `bloom.scene_view()` instead of the surface, then:
//! bloom.resolve(device, queue, encoder, surface, PostProcessSettings::default());
//! # }
//! ```

mod chain;
mod settings;

use std::borrow::Cow;

use chain::Chain;
pub use settings::{
    BloomSettings, DepthOfFieldSettings, NightVisionSettings, PostProcessSettings, ToneMapping,
};

/// How much smaller the blur chain is than the screen.
///
/// A blur is the one effect that is better for being cheap. The downsample is
/// itself a blur, and the taps that follow reach four times as far across the
/// picture for the same cost — so a quarter-resolution chain is both faster and
/// *wider* than a full-resolution one.
const DOWNSCALE: u32 = 4;

/// The scene, its glow, and the passes between.
pub struct Bloom {
    pipelines: Pipelines,
    sampler: wgpu::Sampler,
    chain: Option<Chain>,
    output_format: wgpu::TextureFormat,
}

struct Pipelines {
    bright: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    sample_layout: wgpu::BindGroupLayout,
    glow_layout: wgpu::BindGroupLayout,
}

impl Bloom {
    /// The format the scene is drawn in when the output is not itself an sRGB
    /// picture.
    ///
    /// sRGB, like every other color target here: the scene is a picture, and a
    /// picture is stored encoded. The chain's own targets are not — see
    /// [`chain::Chain::FORMAT`].
    pub const SCENE_FORMAT: wgpu::TextureFormat = crate::COLOR_TARGET_FORMAT;

    /// The format the scene is drawn in for this output, and so the format the
    /// frame's renderers must be built for.
    ///
    /// The output's own, when that is sRGB. A host builds one set of renderers
    /// for its surface and draws the world into the scene and the interface
    /// onto the surface with them, so the two have to agree: a browser canvas
    /// negotiates `Bgra8UnormSrgb`, and a scene fixed at `Rgba8UnormSrgb` had
    /// every world pipeline rejected there.
    #[must_use]
    pub fn scene_format_for(output_format: wgpu::TextureFormat) -> wgpu::TextureFormat {
        if output_format.is_srgb() {
            output_format
        } else {
            Self::SCENE_FORMAT
        }
    }

    /// `output_format` is the format of the target [`Self::resolve`] will write
    /// to — a surface's negotiated format, usually.
    #[must_use]
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sindri bloom shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("bloom.wgsl"))),
        });
        let sample_layout = sample_layout(device);
        let glow_layout = glow_layout(device);
        let build = |label: &str,
                     entry: &str,
                     format: wgpu::TextureFormat,
                     layouts: &[Option<&wgpu::BindGroupLayout>]| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: layouts,
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_fullscreen"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                // No depth anywhere in the chain: these are picture-to-picture
                // passes over a full-screen triangle, and there is nothing in
                // front of anything.
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            pipelines: Pipelines {
                bright: build(
                    "Sindri bloom bright pass",
                    "fs_bright",
                    Chain::FORMAT,
                    &[Some(&sample_layout)],
                ),
                blur: build(
                    "Sindri bloom blur pass",
                    "fs_blur",
                    Chain::FORMAT,
                    &[Some(&sample_layout)],
                ),
                composite: build(
                    "Sindri bloom composite",
                    "fs_composite",
                    output_format,
                    &[Some(&sample_layout), Some(&glow_layout)],
                ),
                sample_layout,
                glow_layout,
            },
            // Linear and clamped: the chain is read between sizes, so the taps
            // have to interpolate, and a repeating edge would wrap one side of
            // the screen's light onto the other.
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Sindri bloom sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..wgpu::SamplerDescriptor::default()
            }),
            chain: None,
            output_format,
        }
    }

    /// The format [`Self::resolve`] writes.
    #[must_use]
    pub const fn output_format(&self) -> wgpu::TextureFormat {
        self.output_format
    }

    /// The format [`Self::scene_view`] is drawn in.
    #[must_use]
    pub fn scene_format(&self) -> wgpu::TextureFormat {
        Self::scene_format_for(self.output_format)
    }

    /// Sizes the scene target and the chain to this viewport, rebuilding them
    /// only when the size actually changed.
    ///
    /// A zero on either axis leaves everything alone: a viewport with no area
    /// draws nothing, and a texture of no width is an error rather than an
    /// empty picture.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self
            .chain
            .as_ref()
            .is_some_and(|chain| chain.matches(width, height))
        {
            return;
        }
        self.chain = Some(Chain::new(
            device,
            width,
            height,
            DOWNSCALE,
            self.scene_format(),
        ));
    }

    /// Where a frame should be drawn so that it can glow.
    ///
    /// `None` before the first [`Self::resize`], which is the one state a host
    /// can reach by drawing before it has said how big anything is.
    #[must_use]
    pub fn scene_view(&self) -> Option<&wgpu::TextureView> {
        self.chain.as_ref().map(Chain::scene_view)
    }

    /// Reads the scene, and writes it plus its glow to `target`.
    ///
    /// Does nothing without a scene to read — see [`Self::scene_view`].
    pub fn resolve(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        settings: PostProcessSettings,
    ) {
        let Some(chain) = self.chain.as_ref() else {
            return;
        };
        let settings = settings.sane();

        if settings.bloom.enabled {
            // Bright pass: grading is applied before thresholding, so bloom
            // responds to the same world color that reaches the player.
            chain.run(
                device,
                queue,
                encoder,
                &self.pipelines,
                &self.sampler,
                chain::Step::Bright(settings),
            );
        }
        for pass in 0..if settings.bloom.enabled {
            settings.bloom.passes
        } else {
            0
        } {
            // Each sweep reaches twice as far as the last, so a few passes cover
            // a wide glow without a kernel wide enough to need one tap per texel
            // of it.
            #[allow(clippy::cast_precision_loss)]
            let reach = (1 << pass) as f32;
            chain.run(
                device,
                queue,
                encoder,
                &self.pipelines,
                &self.sampler,
                chain::Step::Blur {
                    direction: [reach, 0.0],
                },
            );
            chain.run(
                device,
                queue,
                encoder,
                &self.pipelines,
                &self.sampler,
                chain::Step::Blur {
                    direction: [0.0, reach],
                },
            );
        }
        chain.run(
            device,
            queue,
            encoder,
            &self.pipelines,
            &self.sampler,
            chain::Step::Composite { target, settings },
        );
    }
}

fn sample_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Sindri bloom sample layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn glow_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Sindri bloom glow layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    })
}
