//! The three pipelines one sprite shader is drawn with.

use std::borrow::Cow;

use crate::{DepthTarget, SpriteBlendMode, TexturedVertex};

use super::{SpriteDepth, SpriteInstance};

/// One pipeline per depth behavior, because the comparison is pipeline state:
/// a batch cannot choose between them at draw time otherwise.
#[derive(Debug)]
pub(super) struct PipelineSet {
    over: wgpu::RenderPipeline,
    within: wgpu::RenderPipeline,
    /// Opaque, depth-writing surfaces: the blocks a camera can go round.
    solid: wgpu::RenderPipeline,
}

impl PipelineSet {
    /// Builds the set from a composed module.
    ///
    /// The module has already been checked, by `SpriteShader::new` or because
    /// it is the engine's own, so this cannot fail the way a project's first
    /// draft can.
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        target_format: wgpu::TextureFormat,
        blend_mode: SpriteBlendMode,
        composed: &str,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sindri sprite batch shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(composed)),
        });
        let pipeline = |depth: SpriteDepth| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Sindri sprite batch pipeline"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("sindri_vertex"),
                    buffers: &[
                        Some(TexturedVertex::layout()),
                        Some(SpriteInstance::layout()),
                    ],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    // A writing batch discards what it cannot show, so the
                    // shape a cutout leaves is a hole in the geometry rather
                    // than a hole in the world behind it.
                    entry_point: Some(if depth.writes() {
                        "sindri_fragment_solid"
                    } else {
                        "sindri_fragment"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(if depth.writes() {
                            wgpu::BlendState::REPLACE
                        } else {
                            blend_mode.blend_state()
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    // Solid geometry is closed, so a face pointing away from
                    // the camera is a face on the far side of the thing it
                    // belongs to: drawing it costs fill and, where the solid is
                    // open at the edges, shows its inside. A blended sprite is
                    // not closed and often faces away on purpose, so only the
                    // writing pipeline culls.
                    cull_mode: depth.writes().then_some(wgpu::Face::Back),
                    ..wgpu::PrimitiveState::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DepthTarget::FORMAT,
                    depth_write_enabled: Some(depth.writes()),
                    depth_compare: Some(depth.compare()),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            over: pipeline(SpriteDepth::Ignore),
            within: pipeline(SpriteDepth::Test),
            solid: pipeline(SpriteDepth::Write),
        }
    }

    pub(super) const fn for_depth(&self, depth: SpriteDepth) -> &wgpu::RenderPipeline {
        match depth {
            SpriteDepth::Ignore => &self.over,
            SpriteDepth::Test => &self.within,
            SpriteDepth::Write => &self.solid,
        }
    }
}
