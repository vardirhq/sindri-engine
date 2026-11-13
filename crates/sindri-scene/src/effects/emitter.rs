//! Emitters throwing particles into the pool, step by step.
//!
//! Each step every playing `sindri.particles` emitter adds `rate` times the
//! step's length to what it owes and throws the whole particles of it, so a
//! rate of 2.5 a second at 60 steps a second is two or three a second and
//! never a rounding error's worth more. What an emitter owes, how many of its
//! particles are alive and where it last stood are kept here between steps,
//! because none of them is anything a scene authors.

use std::f32::consts::TAU;
use std::time::Duration;

use sindri_core::{
    ComponentRegistryError, ComponentSchemaRegistry, EntityId, SceneComponent, World,
};

use super::pool::{Effects2d, Fleck};
use super::{EmissionShape, ParticleSpace, ParticlesComponent};

/// Which emitter threw a particle: its entity, and which of that entity's
/// particle parts. An entity's unnamed `sindri.particles` and a named
/// `sindri.particles#glow` are two emitters with state of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmitterId {
    pub entity: EntityId,
    /// 0 for the unnamed part, otherwise one more than its name's place in
    /// the pool's list of part names.
    part: u16,
}

impl EmitterId {
    /// The entity's own, unnamed emitter.
    #[must_use]
    pub const fn of(entity: EntityId) -> Self {
        Self { entity, part: 0 }
    }
}

/// What an emitter carries between steps.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EmitterState {
    /// Particles owed but not yet whole.
    carry: f32,
    /// Its particles alive now.
    pub(super) live: u32,
    /// Where it stood last step, so local particles can follow it.
    last: Option<[f32; 2]>,
}

impl Effects2d {
    /// Runs every emitter in `world` for one step of `delta`.
    ///
    /// Returns a line for each emitter that cannot be used, which then throws
    /// nothing rather than stopping the step.
    ///
    /// # Errors
    /// If the particle component is not registered.
    pub fn emit(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
        delta: Duration,
    ) -> Result<Vec<String>, ComponentRegistryError> {
        self.emit_where(world, components, delta, |_| true)
    }

    /// Runs only the emitters `wanted` picks, which is how the editor
    /// previews the selected one without the rest of the scene playing.
    ///
    /// # Errors
    /// If the particle component is not registered.
    pub fn emit_where(
        &mut self,
        world: &World,
        components: &ComponentSchemaRegistry,
        delta: Duration,
        wanted: impl Fn(EntityId) -> bool,
    ) -> Result<Vec<String>, ComponentRegistryError> {
        let seconds = delta.as_secs_f32();
        let mut problems = Vec::new();
        let mut parts = components.query_parts::<ParticlesComponent>(world)?;
        parts.retain(|(entity, _, _)| wanted(*entity));
        let emitters: Vec<(EmitterId, ParticlesComponent)> = parts
            .into_iter()
            .map(|(entity, part, particles)| (self.emitter_id(entity, part.as_deref()), particles))
            .collect();
        for (emitter, particles) in &emitters {
            let entity = &emitter.entity;
            if let Err(error) = particles.validate() {
                problems.push(format!("{}: {error}", ParticlesComponent::TYPE_NAME));
                continue;
            }
            let at = world
                .world_transform(*entity)
                .unwrap_or_default()
                .position_2d();
            let at = [at[0], at[1]];
            let state = self.emitters.entry(*emitter).or_default();
            let moved = state.last.map(|last| [at[0] - last[0], at[1] - last[1]]);
            state.last = Some(at);
            if particles.space == ParticleSpace::Local
                && let Some(moved) = moved.filter(|moved| moved[0].abs() + moved[1].abs() > 0.0)
            {
                self.follow(*emitter, moved);
            }
            if !particles.playing || seconds <= 0.0 {
                continue;
            }
            let state = self.emitters.entry(*emitter).or_default();
            state.carry += particles.rate * seconds;
            let whole = state.carry.floor();
            state.carry -= whole;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            self.emit_from(*emitter, particles, at, whole as u32);
        }
        // An emitter that is gone, and whose particles are too, is forgotten.
        let present: Vec<EmitterId> = emitters.iter().map(|(emitter, _)| *emitter).collect();
        self.emitters
            .retain(|emitter, state| state.live > 0 || present.contains(emitter));
        Ok(problems)
    }

    /// Throws `count` of an emitter's particles at once, at `at`, within its
    /// cap. Returns how many it threw.
    pub fn emit_now(
        &mut self,
        entity: EntityId,
        particles: &ParticlesComponent,
        at: [f32; 2],
        count: u32,
    ) -> usize {
        self.emit_from(EmitterId::of(entity), particles, at, count)
    }

    /// [`Self::emit_now`] for a named emitter part, or the entity's own with
    /// `None`.
    pub fn emit_part_now(
        &mut self,
        entity: EntityId,
        part: Option<&str>,
        particles: &ParticlesComponent,
        at: [f32; 2],
        count: u32,
    ) -> usize {
        let emitter = self.emitter_id(entity, part);
        self.emit_from(emitter, particles, at, count)
    }

    /// [`Self::emit_now`] for any emitter, a named part's included.
    fn emit_from(
        &mut self,
        entity: EmitterId,
        particles: &ParticlesComponent,
        at: [f32; 2],
        count: u32,
    ) -> usize {
        let texture = self.texture_index(&particles.texture);
        let ramp = self.ramp_index(&particles.colors);
        let mut made = 0;
        for _ in 0..count {
            let live = self.emitters.entry(entity).or_default().live;
            if live >= particles.max || !self.make_room() {
                break;
            }
            let fleck = self.particle(particles, at, texture, ramp, entity);
            self.push(fleck);
            self.emitters.entry(entity).or_default().live += 1;
            made += 1;
        }
        made
    }

    /// One particle, drawn from the pool's own stream.
    fn particle(
        &mut self,
        particles: &ParticlesComponent,
        at: [f32; 2],
        texture: usize,
        ramp: usize,
        entity: EmitterId,
    ) -> Fleck {
        let offset = self.offset(particles.shape, particles.shape_size);
        let angle = particles
            .spread
            .mul_add(self.rng.next_f32() - 0.5, particles.direction)
            .to_radians();
        let speed = particles.speed * self.varied(particles.speed_spread);
        let lifetime = (particles.lifetime * self.varied(particles.lifetime_spread)).max(0.01);
        let rotation = if particles.random_rotation {
            self.rng.next_f32() * TAU
        } else {
            0.0
        };
        Fleck {
            position: [
                at[0] + particles.offset[0] + offset[0],
                at[1] + particles.offset[1] + offset[1],
            ],
            velocity: [angle.cos() * speed, angle.sin() * speed],
            remaining: lifetime,
            lifetime,
            size: particles.size,
            tint: [1.0; 4],
            fade: false,
            drag: particles.drag,
            layer: particles.layer,
            texture,
            rotation,
            spin: particles.spin.to_radians(),
            size_end: particles.size_end,
            gravity: particles.gravity,
            ramp: Some(ramp),
            emitter: Some(entity),
        }
    }

    /// `1 ± spread`, never below zero.
    fn varied(&mut self, spread: f32) -> f32 {
        spread
            .mul_add(self.rng.next_f32().mul_add(2.0, -1.0), 1.0)
            .max(0.0)
    }

    /// Where in the emission shape a particle starts, from its centre.
    fn offset(&mut self, shape: EmissionShape, size: [f32; 2]) -> [f32; 2] {
        let radius = size[0] / 2.0;
        match shape {
            EmissionShape::Point => [0.0, 0.0],
            EmissionShape::Box => [
                (self.rng.next_f32() - 0.5) * size[0],
                (self.rng.next_f32() - 0.5) * size[1],
            ],
            EmissionShape::Circle | EmissionShape::Ring => {
                let angle = self.rng.next_f32() * TAU;
                // The square root spreads a disc evenly rather than bunching
                // particles at its centre.
                let reach = if shape == EmissionShape::Ring {
                    radius
                } else {
                    radius * self.rng.next_f32().sqrt()
                };
                [angle.cos() * reach, angle.sin() * reach]
            }
        }
    }

    /// Moves an emitter's particles by as far as it moved.
    fn follow(&mut self, entity: EmitterId, moved: [f32; 2]) {
        for fleck in self.flecks_mut() {
            if fleck.emitter == Some(entity) {
                fleck.position[0] += moved[0];
                fleck.position[1] += moved[1];
            }
        }
    }

    fn ramp_index(&mut self, colors: &[[f32; 4]]) -> usize {
        if let Some(found) = self.ramps.iter().position(|known| known == colors) {
            return found;
        }
        self.ramps.push(colors.to_vec());
        self.ramps.len() - 1
    }

    /// How many of an emitter's particles are alive.
    #[must_use]
    pub fn emitter_live(&self, entity: EntityId) -> u32 {
        self.emitters
            .get(&EmitterId::of(entity))
            .map_or(0, |state| state.live)
    }

    /// How many of a named emitter part's particles are alive.
    #[must_use]
    pub fn part_live(&self, entity: EntityId, part: &str) -> u32 {
        self.part_names
            .iter()
            .position(|name| name == part)
            .and_then(|index| u16::try_from(index + 1).ok())
            .and_then(|part| self.emitters.get(&EmitterId { entity, part }))
            .map_or(0, |state| state.live)
    }

    /// The emitter a part of `entity` is, its name learned the first time.
    fn emitter_id(&mut self, entity: EntityId, part: Option<&str>) -> EmitterId {
        let Some(name) = part else {
            return EmitterId::of(entity);
        };
        let index = self
            .part_names
            .iter()
            .position(|known| known == name)
            .unwrap_or_else(|| {
                self.part_names.push(name.to_owned());
                self.part_names.len() - 1
            });
        // A scene with more than sixty-five thousand differently named
        // emitter parts shares the last number among the rest.
        let part = u16::try_from(index + 1).unwrap_or(u16::MAX);
        EmitterId { entity, part }
    }
}

#[cfg(test)]
#[path = "emitter_tests.rs"]
mod tests;
