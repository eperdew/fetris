pub const GRAVITY_DELAY: u32 = 30; // ticks per gravity step
pub const LOCK_DELAY: u32 = 30;    // lock delay: LOCK_DELAY+1 ticks (counts down N→0, fires on N+1)
pub const SPAWN_DELAY: u32 = 30;   // ARE: SPAWN_DELAY+1 ticks (counts down N→0, fires on N+1)
pub const DAS_CHARGE: u32 = 16;    // ticks before auto-repeat activates
pub const DAS_REPEAT: u32 = 6;     // ticks between auto-repeat steps
