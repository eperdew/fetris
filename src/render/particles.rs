use bevy::prelude::*;
use rand::Rng;
use crate::constants::{PARTICLE_BASE_LIFETIME, PARTICLE_BASE_SPEED, PARTICLE_GRAVITY};
use crate::render::{BOARD_X, BOARD_Y, CELL, INSET, piece_color};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_COLS, GameEvent};
use crate::resources::{Board, PendingCompaction};

#[derive(Component)]
pub struct Particle {
    pub vx: f32,
    pub vy: f32,
    pub age: u32,
    pub lifetime: u32,
    pub base_color: Color,
}

pub fn spawn_particles_on_line_clear(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    board: Res<Board>,
    pending: Res<PendingCompaction>,
    assets: Res<GameAssets>,
) {
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        let GameEvent::LineClear { count } = *ev else { continue; };
        let rows: Vec<usize> = pending.0.clone();
        let particles_per_cell: u32 = if count >= 4 { 3 } else { 1 };
        let speed_scale = match count {
            1 => 1.0f32,
            2 => 1.4,
            3 => 1.8,
            _ => 2.5,
        };

        for &r in &rows {
            for c in 0..BOARD_COLS {
                let Some(kind) = board.0[r][c] else { continue; };
                for _ in 0..particles_per_cell {
                    let dist = c as f32 - (BOARD_COLS as f32 - 1.0) / 2.0;
                    let base_angle = dist.atan2(-1.5_f32);
                    let spread = (rng.gen::<f32>() - 0.5) * std::f32::consts::FRAC_PI_3;
                    let angle = base_angle + spread;
                    let speed = PARTICLE_BASE_SPEED * speed_scale * (0.6 + 0.8 * rng.gen::<f32>());
                    let lifetime = PARTICLE_BASE_LIFETIME + (rng.gen::<f32>() * 25.0) as u32;
                    let x = BOARD_X + c as f32 * CELL + CELL * 0.5;
                    let y = BOARD_Y + r as f32 * CELL + CELL * 0.5;
                    let color = piece_color(kind);

                    commands.spawn((
                        Particle {
                            vx: angle.sin() * speed,
                            vy: -angle.cos().abs() * speed,
                            age: 0,
                            lifetime,
                            base_color: color,
                        },
                        Sprite {
                            image: assets.cell_texture.clone(),
                            color,
                            custom_size: Some(Vec2::new(CELL - INSET * 2.0, CELL - INSET * 2.0)),
                            ..default()
                        },
                        bevy::sprite::Anchor::CENTER,
                        Transform::from_xyz(x, y, 50.0),
                    ));
                }
            }
        }
    }
}

pub fn update_particles(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    for (entity, mut particle, mut transform, mut sprite) in &mut q {
        transform.translation.x += particle.vx;
        transform.translation.y += particle.vy;
        particle.vy += PARTICLE_GRAVITY;
        particle.age += 1;
        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
        } else {
            let alpha = 1.0 - particle.age as f32 / particle.lifetime as f32;
            let s = particle.base_color.to_srgba();
            sprite.color = Color::srgba(s.red, s.green, s.blue, alpha);
        }
    }
}
