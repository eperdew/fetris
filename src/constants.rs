pub const LOCK_DELAY: u32 = 29;    // N+1 countdown → 30 actual frames (TGM1)
pub const SPAWN_DELAY_NORMAL: u32 = 29;      // N+1 → 30 frames: ARE without line clear (TGM1)
pub const SPAWN_DELAY_LINE_CLEAR: u32 = 40;  // N+1 → 41 frames: ARE after line clear (TGM1)
pub const DAS_CHARGE: u32 = 16;    // unchanged (matches TGM1)
pub const DAS_REPEAT: u32 = 1;     // TGM1: auto-shift fires every frame once charged

/// (min_level, G_value) pairs in ascending order. G is in units of G/256 per tick.
/// Source: TGM1 wiki. Notable: gravity resets to 4 at level 200, then ramps
/// rapidly to 20G at level 500 with a brief ease-up at 420/450.
pub const GRAVITY_TABLE: &[(u32, u32)] = &[
    (0,   4),
    (30,  6),
    (35,  8),
    (40,  10),
    (50,  12),
    (60,  16),
    (70,  32),
    (80,  48),
    (90,  64),
    (100, 80),
    (120, 96),
    (140, 112),
    (160, 128),
    (170, 144),
    (200, 4),    // resets at section 2
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

pub fn gravity_g(level: u32) -> u32 {
    GRAVITY_TABLE
        .iter()
        .rev()
        .find(|(threshold, _)| level >= *threshold)
        .map(|(_, g)| *g)
        .unwrap_or(4)
}
