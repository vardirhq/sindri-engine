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
        let mut emitters = components.query::<ParticlesComponent>(world)?;
        emitters.retain(|(entity, _)| wanted(*entity));
        for (entity, particles) in &emitters {
            if let Err(error) = particles.validate() {
                problems.push(format!("{}: {error}", ParticlesComponent::TYPE_NAME));
                continue;
            }
            let at = world
                .world_transform(*entity)
                .unwrap_or_default()
                .position_2d();
            let at = [at[0], at[1]];
            let state = self.emitters.entry(*entity).or_default();
            let moved = state.last.map(|last| [at[0] - last[0], at[1] - last[1]]);
            state.last = Some(at);
            if particles.space == ParticleSpace::Local
                && let Some(moved) = moved.filter(|moved| moved[0].abs() + moved[1].abs() > 0.0)
            {
                self.follow(*entity, moved);
            }
            if !particles.playing || seconds <= 0.0 {
                continue;
            }
            let state = self.emitters.entry(*entity).or_default();
            state.carry += particles.rate * seconds;
            let whole = state.carry.floor();
            state.carry -= whole;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            self.emit_now(*entity, particles, at, whole as u32);
        }
        // An emitter that is gone, and whose particles are too, is forgotten.
        let present: Vec<EntityId> = emitters.iter().map(|(entity, _)| *entity).collect();
        self.emitters
            .retain(|entity, state| state.live > 0 || present.contains(entity));
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
        entity: EntityId,
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
    fn follow(&mut self, entity: EntityId, moved: [f32; 2]) {
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
        self.emitters.get(&entity).map_or(0, |state| state.live)
    }
}

#[cfg(test)]
#[path = "emitter_tests.rs"]
mod tests;
