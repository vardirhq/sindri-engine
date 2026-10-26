use sindri_scene::{EmissionShape, ParticlesComponent};

pub const NAMES: [&str; 6] = ["Dust", "Smoke", "Sparks", "Bubbles", "Rain", "Magic"];

pub fn preset(index: usize) -> ParticlesComponent {
    let mut p = ParticlesComponent {
        texture: String::new(),
        ..ParticlesComponent::default()
    };
    match index {
        0 => {
            p.rate = 24.0;
            p.speed = 0.8;
            p.spread = 120.0;
            p.lifetime = 0.8;
            p.size = 0.16;
            p.size_end = 0.38;
            p.gravity = [0.0, -0.4];
            p.colors = vec![[0.68, 0.53, 0.34, 0.7], [0.6, 0.48, 0.35, 0.0]];
        }
        1 => {
            p.rate = 18.0;
            p.speed = 0.65;
            p.spread = 25.0;
            p.lifetime = 2.5;
            p.size = 0.2;
            p.size_end = 0.8;
            p.colors = vec![[0.45, 0.45, 0.48, 0.7], [0.35, 0.35, 0.38, 0.0]];
        }
        2 => {
            p.rate = 30.0;
            p.speed = 4.0;
            p.spread = 100.0;
            p.lifetime = 0.7;
            p.gravity = [0.0, -6.0];
            p.size = 0.11;
            p.size_end = 0.02;
            p.colors = vec![[1.0, 0.85, 0.3, 1.0], [1.0, 0.2, 0.02, 0.0]];
        }
        3 => {
            p.rate = 12.0;
            p.speed = 0.8;
            p.spread = 18.0;
            p.lifetime = 2.5;
            p.size = 0.08;
            p.size_end = 0.2;
            p.colors = vec![[0.5, 0.8, 1.0, 0.65], [0.7, 0.9, 1.0, 0.0]];
        }
        4 => {
            p.rate = 80.0;
            p.speed = 4.0;
            p.direction = 270.0;
            p.spread = 8.0;
            p.shape = EmissionShape::Box;
            p.shape_size = [6.0, 0.1];
            p.offset = [0.0, 2.0];
            p.lifetime = 1.0;
            p.size = 0.07;
            p.colors = vec![[0.6, 0.75, 1.0, 0.8], [0.6, 0.75, 1.0, 0.0]];
        }
        _ => {
            p.rate = 20.0;
            p.speed = 0.6;
            p.spread = 360.0;
            p.shape = EmissionShape::Ring;
            p.shape_size = [1.0, 1.0];
            p.lifetime = 1.6;
            p.size = 0.1;
            p.size_end = 0.02;
            p.colors = vec![[0.5, 0.3, 1.0, 1.0], [0.1, 0.9, 1.0, 0.0]];
        }
    }
    p
}
