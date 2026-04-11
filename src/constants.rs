use crate::menu::GameMode;

pub const LOCK_DELAY: u32 = 29; // N+1 countdown → 30 actual frames (TGM1)
pub const SPAWN_DELAY_NORMAL: u32 = 29; // N+1 → 30 frames: ARE (TGM1)
pub const LINE_CLEAR_DELAY: u32 = 40; // N+1 → 41 frames: line clear display phase before ARE (TGM1)
/// Frames at the start (and end) of ARE where DAS is frozen.
/// DAS charges during ARE frames 5–29 (ticks_left 25–1 inclusive).
pub const ARE_DAS_FROZEN_FRAMES: u32 = 4;
pub const DAS_CHARGE: u32 = 16; // unchanged (matches TGM1)
pub const DAS_REPEAT: u32 = 1; // TGM1: auto-shift fires every frame once charged

/// (min_level, G_value) pairs in ascending order. G is in units of G/256 per tick.
/// Source: TGM1 wiki. Notable: gravity resets to 4 at level 200, then ramps
/// rapidly to 20G at level 500 with a brief ease-up at 420/450.
pub const MASTER_GRAVITY_TABLE: &[(u32, u32)] = &[
    (0, 4),
    (30, 6),
    (35, 8),
    (40, 10),
    (50, 12),
    (60, 16),
    (70, 32),
    (80, 48),
    (90, 64),
    (100, 80),
    (120, 96),
    (140, 112),
    (160, 128),
    (170, 144),
    (200, 4), // resets at section 2
    (220, 32),
    (230, 64),
    (233, 96),
    (236, 128),
    (239, 160),
    (243, 192),
    (247, 224),
    (251, 256),  // 1G
    (300, 512),  // 2G
    (330, 768),  // 3G
    (360, 1024), // 4G
    (400, 1280), // 5G
    (420, 1024), // 4G — intentional ease before 20G
    (450, 768),  // 3G — intentional ease before 20G
    (500, 5120), // 20G
];

/// Initial speed of line-clear particles, in pixels per frame. The direction is
/// determined by the cell's position: bottom-center points straight down; cells
/// further from center or higher up have increasingly horizontal velocity vectors.
pub const PARTICLE_INITIAL_SPEED: f32 = 1.0;

/// Downward acceleration of line-clear particles, in pixels per frame².
pub const PARTICLE_GRAVITY: f32 = 0.4;

pub fn gravity_g(game_mode: GameMode, level: u32) -> u32 {
    match game_mode {
        GameMode::Master => MASTER_GRAVITY_TABLE
            .iter()
            .rev()
            .find(|(threshold, _)| level >= *threshold)
            .map(|(_, g)| *g)
            .unwrap_or(4),
        GameMode::TwentyG => 20 * 256,
    }
}
