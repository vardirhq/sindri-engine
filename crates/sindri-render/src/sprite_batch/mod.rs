//! Drawing sprites in as few passes as the ordering allows.

mod binding;
mod instance;
mod mask;
mod pipelines;
mod stats;

#[cfg(test)]
mod tests;

pub use instance::{SpriteDepth, SpriteInstance};
pub use mask::SpriteMask;
pub use stats::SpriteBatchStats;

use binding::{BindingKey, create_bind_group_layout};
use instance::BatchUniform;
use pipelines::PipelineSet;

use std::collections::HashMap;

use glam::Mat4;
use thiserror::Error;
use wgpu::util::DeviceExt;

use crate::light2d::Lights2dGpu;
use crate::{
    DepthTarget, Lighting2d, MeshBuffers, RegistryIdentity, SpriteBlendMode, SpriteEnvironment,
    SpriteShader, Texture2D, TextureId, TextureRegistry, TexturedVertex,
};

const DEFAULT_CAPACITY: u32 = 64;

const VERTICES: [TexturedVertex; 4] = [
    TexturedVertex::new([-0.5, -0.5, 0.0], [0.0, 1.0]),
    TexturedVertex::new([0.5, -0.5, 0.0], [1.0, 1.0]),
    TexturedVertex::new([0.5, 0.5, 0.0], [1.0, 0.0]),
    TexturedVertex::new([-0.5, 0.5, 0.0], [0.0, 0.0]),
];

const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

/// One batch's own GPU resources.
///
/// Per batch rather than per renderer, and that is the whole point.
/// `queue.write_buffer` stages a write that lands *before* the command buffer
/// executes, so a renderer that wrote one uniform buffer once per batch gave
/// every pass in the frame whatever the last batch had put there. A frame with
/// a single sprite batch never notices; a scene with a world and an overlay
/// draws the world through the overlay's camera.
#[derive(Debug)]
struct Batch {
    uniform: wgpu::Buffer,
    instances: wgpu::Buffer,
    capacity: u32,
    /// One bind group per texture used in this slot. Keyed per slot because a
    /// bind group names the uniform buffer it reads, and each slot has its own.
    bind_groups: std::collections::HashMap<BindingKey, wgpu::BindGroup>,
    /// The registry `bind_groups` were made from. Handles are only unique
    /// within one, so a different registry means none of them apply.
    registry: RegistryIdentity,
}

#[derive(Debug)]
pub struct SpriteBatchRenderer {
    /// The built-in look, which a batch with no shader draws with.
    built_in: PipelineSet,
    /// Pipelines for project shaders, by asset path, with the revision they
    /// were built from. A reload has a new revision and replaces its entry
    /// rather than adding one, so editing a shader fifty times holds one set.
    shaded: HashMap<String, (u64, PipelineSet)>,
    layout: wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    bind_group_layout: wgpu::BindGroupLayout,
    mesh: MeshBuffers,
    /// Grown as a frame needs them and reused every frame after.
    batches: Vec<Batch>,
    /// Which slot the next batch of this frame takes.
    next: usize,
    clip: Option<[u32; 4]>,
    draw_stats: crate::BatchDrawStats,
    time: f32,
    /// The frame's 2D lights, uploaded on the frame's first batch.
    lighting: Option<Lighting2d>,
    lights: Lights2dGpu,
    /// What a batch with no normal map binds in its place, made on the first
    /// draw because making a texture needs the queue.
    flat_normal: Option<Texture2D>,
    blend_mode: SpriteBlendMode,
    stats: SpriteBatchStats,
}

impl SpriteBatchRenderer {
    /// Cumulative physical draw/write payloads, including every nonempty batch.
    pub const fn draw_stats(&self) -> crate::BatchDrawStats {
        self.draw_stats
    }

    pub fn set_clip(&mut self, clip: Option<[u32; 4]>) {
        self.clip = clip;
    }

    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        Self::with_blend_mode(device, target_format, SpriteBlendMode::Alpha)
    }

    pub fn with_blend_mode(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        blend_mode: SpriteBlendMode,
    ) -> Self {
        let bind_group_layout = create_bind_group_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sindri sprite batch pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let built_in = PipelineSet::new(
            device,
            &layout,
            target_format,
            blend_mode,
            &crate::sprite_shader::compose(crate::sprite_shader::BUILT_IN),
        );

        Self {
            built_in,
            shaded: HashMap::new(),
            layout,
            target_format,
            bind_group_layout,
            mesh: MeshBuffers::new(device, "Sindri sprite batch quad", &VERTICES, &INDICES),
            batches: Vec::new(),
            next: 0,
            clip: None,
            draw_stats: crate::BatchDrawStats::default(),
            time: 0.0,
            lighting: None,
            lights: Lights2dGpu::new(device),
            flat_normal: None,
            blend_mode,
            stats: SpriteBatchStats::default(),
        }
    }

    /// What every sprite in the frame being encoded shares.
    ///
    /// Set once per frame by whoever encodes it, the way the clip is: the
    /// whole frame shares one clock.
    pub fn set_environment(&mut self, environment: &SpriteEnvironment) {
        self.time = if environment.time.is_finite() {
            environment.time
        } else {
            0.0
        };
        self.lighting.clone_from(&environment.lighting);
    }

    /// Starts a submission, so the next batch takes the first slot again.
    ///
    /// Without this a long-running host would allocate a slot per batch per
    /// frame forever. Slots are reused rather than freed, so a frame costs
    /// nothing after the first one of its shape.
    ///
    /// A *submission* and not a frame, and the difference is the whole reason
    /// the slots exist: `queue.write_buffer` stages a write that lands when
    /// the queue is next submitted, so reusing a slot is only safe once the
    /// commands that read it have been submitted. Two frames encoded into one
    /// submission — an editor drawing a scene view and a game view, say — must
    /// either submit between them or keep drawing into fresh slots. The editor
    /// submits between them.
    pub fn begin_submission(&mut self) {
        self.next = 0;
        self.stats = SpriteBatchStats::default();
    }

    /// How many batch slots the renderer is holding.
    ///
    /// The high-water mark of one frame's batches, which is what a host would
    /// look at to find out whether a scene is asking for a great many.
    #[must_use]
    pub fn batch_slots(&self) -> usize {
        self.batches.len()
    }

    /// Draws one batch, with its own camera and its own instances.
    ///
    /// The batch's data goes in the slot [`Self::begin_submission`] handed
    /// out, so nothing another batch in the same submission writes can reach
    /// it.
    ///
    /// Preparing and encoding are one call because they were never separable:
    /// a batch's uniform and instance data are only correct for the pass drawn
    /// immediately after they are written, and splitting them left an API whose
    /// only safe use was to call the two in a pair. Now the pair is the call,
    /// and the slot it writes belongs to it.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_target: &DepthTarget,
        registry: &TextureRegistry,
        texture: TextureId,
        view_projection: Mat4,
        depth: SpriteDepth,
        instances: &[SpriteInstance],
    ) -> Result<SpriteBatchStats, SpriteBatchError> {
        self.draw_shaded(
            device,
            queue,
            encoder,
            SpriteBatchTarget {
                color: target,
                depth: depth_target,
            },
            registry,
            SpriteBatchDraw {
                texture,
                normal_map: None,
                mask: None,
                shader: None,
                view_projection,
                depth,
                instances,
            },
        )
    }

    /// [`Self::draw`], with the batch drawn through a project shader when it
    /// names one.
    pub fn draw_shaded(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: SpriteBatchTarget<'_>,
        registry: &TextureRegistry,
        batch: SpriteBatchDraw<'_>,
    ) -> Result<SpriteBatchStats, SpriteBatchError> {
        let SpriteBatchDraw {
            texture,
            normal_map,
            mask,
            shader,
            view_projection,
            depth,
            instances,
        } = batch;
        let SpriteBatchTarget {
            color: target,
            depth: depth_target,
        } = target;
        if let Some(shader) = shader {
            self.prepare_shader(device, shader);
        }
        let instance_count =
            u32::try_from(instances.len()).map_err(|_| SpriteBatchError::TooManyInstances)?;
        let stats = SpriteBatchStats::for_sprite_count(instance_count);
        if instance_count == 0 {
            return Ok(stats);
        }

        self.lights.write(queue, self.lighting.as_ref());
        let slot = self.reserve(device, instance_count)?;
        {
            let batch = &self.batches[slot];
            queue.write_buffer(
                &batch.uniform,
                0,
                bytemuck::bytes_of(&BatchUniform {
                    view_projection: view_projection.to_cols_array_2d(),
                    time: [
                        self.time,
                        f32::from(u8::from(normal_map.is_some())),
                        SpriteMask::mode(mask.as_ref()),
                        0.0,
                    ],
                    mask_from_world: mask
                        .map_or(Mat4::IDENTITY, |mask| mask.from_world)
                        .to_cols_array_2d(),
                    mask_uv: mask.map_or([0.0, 0.0, 1.0, 1.0], |mask| mask.uv.to_array()),
                }),
            );
            queue.write_buffer(&batch.instances, 0, bytemuck::cast_slice(instances));
        }
        let key = BindingKey {
            texture,
            normal_map,
            mask: mask.map(|mask| mask.texture),
        };
        self.bind_texture(device, queue, registry, slot, key);
        self.stats = self.stats.and(stats);

        let batch = &self.batches[slot];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sindri sprite batch pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_target.view(),
                // A blended batch reads the buffer and never writes it, and
                // saying so here makes that a rule rather than a pipeline
                // setting someone could change alone. A solid batch keeps what
                // it writes, and loads rather than clears, because the surfaces
                // drawn before it in this frame are what it has to sort against.
                depth_ops: depth.writes().then_some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some([x, y, w, h]) = self.clip {
            pass.set_scissor_rect(x, y, w, h);
        }
        let pipelines = shader
            .and_then(|shader| self.shaded.get(shader.name()))
            .map_or(&self.built_in, |(_, pipelines)| pipelines);
        pass.set_pipeline(pipelines.for_depth(depth));
        pass.set_bind_group(
            0,
            batch
                .bind_groups
                .get(&key)
                .expect("the bind group was just created"),
            &[],
        );
        pass.set_vertex_buffer(1, batch.instances.slice(..));
        self.mesh.draw_instances(&mut pass, 0..instance_count);
        self.draw_stats.record_draw(
            instance_count,
            std::mem::size_of::<BatchUniform>(),
            std::mem::size_of_val(instances),
        );
        Ok(stats)
    }

    /// Builds the pipelines for `shader` unless the ones for its revision exist.
    fn prepare_shader(&mut self, device: &wgpu::Device, shader: &SpriteShader) {
        if self
            .shaded
            .get(shader.name())
            .is_some_and(|(revision, _)| *revision == shader.revision())
        {
            return;
        }
        let pipelines = PipelineSet::new(
            device,
            &self.layout,
            self.target_format,
            self.blend_mode,
            shader.composed(),
        );
        self.shaded
            .insert(shader.name().to_owned(), (shader.revision(), pipelines));
    }

    /// How many project shaders this renderer has pipelines for.
    #[must_use]
    pub fn shader_count(&self) -> usize {
        self.shaded.len()
    }

    /// The slot this frame's next batch draws from, grown to hold `capacity`.
    fn reserve(&mut self, device: &wgpu::Device, capacity: u32) -> Result<usize, SpriteBatchError> {
        let slot = self.next;
        self.next += 1;
        if slot == self.batches.len() {
            let wanted = DEFAULT_CAPACITY.max(capacity);
            self.batches.push(Batch {
                uniform: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Sindri sprite batch uniform"),
                    contents: bytemuck::bytes_of(&BatchUniform {
                        view_projection: Mat4::IDENTITY.to_cols_array_2d(),
                        time: [0.0; 4],
                        mask_from_world: Mat4::IDENTITY.to_cols_array_2d(),
                        mask_uv: [0.0, 0.0, 1.0, 1.0],
                    }),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }),
                instances: create_instance_buffer(device, wanted)?,
                capacity: wanted,
                bind_groups: std::collections::HashMap::new(),
                registry: RegistryIdentity::default(),
            });
        }
        let batch = &mut self.batches[slot];
        if capacity > batch.capacity {
            let grown = capacity
                .checked_next_power_of_two()
                .ok_or(SpriteBatchError::TooManyInstances)?;
            batch.instances = create_instance_buffer(device, grown)?;
            batch.capacity = grown;
            // The bind groups name the uniform, not the instances, so they
            // survive the instance buffer being replaced.
        }
        Ok(slot)
    }

    /// Builds this slot's bind group for `key` on first use.
    ///
    /// Keyed per slot rather than per renderer because a bind group names the
    /// uniform buffer it reads, and each slot has its own.
    fn bind_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        registry: &TextureRegistry,
        slot: usize,
        key: BindingKey,
    ) {
        let batch = &mut self.batches[slot];
        if batch.registry != registry.identity() {
            batch.bind_groups.clear();
            batch.registry = registry.identity();
        }
        if batch.bind_groups.contains_key(&key) {
            return;
        }
        let flat_normal = self
            .flat_normal
            .get_or_insert_with(|| binding::flat_normal(device, queue));
        let bind_group = binding::create_bind_group(
            device,
            &self.bind_group_layout,
            &self.batches[slot].uniform,
            &self.lights,
            registry,
            key,
            flat_normal,
        );
        self.batches[slot].bind_groups.insert(key, bind_group);
    }

    /// What this frame has drawn since [`Self::begin_submission`], across every
    /// batch. [`Self::draw`] hands back one batch's share of it.
    pub const fn stats(&self) -> SpriteBatchStats {
        self.stats
    }

    pub const fn blend_mode(&self) -> SpriteBlendMode {
        self.blend_mode
    }
}

fn create_instance_buffer(
    device: &wgpu::Device,
    capacity: u32,
) -> Result<wgpu::Buffer, SpriteBatchError> {
    let stride = u64::try_from(std::mem::size_of::<SpriteInstance>())
        .map_err(|_| SpriteBatchError::BufferSizeOverflow)?;
    let size = u64::from(capacity)
        .checked_mul(stride)
        .ok_or(SpriteBatchError::BufferSizeOverflow)?;
    Ok(device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Sindri sprite batch instances"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    }))
}

/// Where a sprite batch draws.
#[derive(Clone, Copy)]
pub struct SpriteBatchTarget<'a> {
    pub color: &'a wgpu::TextureView,
    pub depth: &'a DepthTarget,
}

/// One batch: what it draws, how, and through which camera.
#[derive(Clone, Copy)]
pub struct SpriteBatchDraw<'a> {
    pub texture: TextureId,
    /// The texture's normal map, which 2D lights read to light the art as a
    /// surface rather than a flat picture.
    pub normal_map: Option<TextureId>,
    /// Where the batch may draw, when it is masked.
    pub mask: Option<SpriteMask>,
    /// The project shader the batch is drawn through, or the built-in look.
    pub shader: Option<&'a SpriteShader>,
    pub view_projection: Mat4,
    pub depth: SpriteDepth,
    pub instances: &'a [SpriteInstance],
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum SpriteBatchError {
    #[error("sprite batch contains more instances than the renderer can address")]
    TooManyInstances,
    #[error("sprite batch instance buffer size overflowed")]
    BufferSizeOverflow,
}
