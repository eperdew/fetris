use crate::data::{
    BoardGrid, GameMode, HorizDir, InputSnapshot, Kind, PieceKind, PiecePhase, RotationDirection,
    BOARD_COLS, BOARD_ROWS,
};
use crate::rotation_system::RotationSystem;
use bevy::prelude::*;

#[derive(Resource)]
pub struct Board(pub BoardGrid);

impl Default for Board {
    fn default() -> Self {
        Board([[None; BOARD_COLS]; BOARD_ROWS])
    }
}

#[derive(Resource)]
pub struct CurrentPhase(pub PiecePhase);

impl Default for CurrentPhase {
    fn default() -> Self {
        CurrentPhase(PiecePhase::Falling)
    }
}

#[derive(Resource)]
pub struct NextPiece(pub PieceKind);

#[derive(Resource)]
pub struct GameProgress {
    pub level: u32,
    pub lines: u32,
    pub ticks_elapsed: u64,
    pub game_over: bool,
    pub game_won: bool,
    pub score_submitted: bool,
}

impl Default for GameProgress {
    fn default() -> Self {
        Self {
            level: 0,
            lines: 0,
            ticks_elapsed: 0,
            game_over: false,
            game_won: false,
            score_submitted: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct DasState {
    pub direction: Option<HorizDir>,
    pub counter: u32,
}

#[derive(Resource, Default)]
pub struct RotationBuffer(pub Option<RotationDirection>);

#[derive(Resource, Default)]
pub struct PendingCompaction(pub Vec<usize>);

/// Per-piece state that resets on spawn.
#[derive(Resource, Default)]
pub struct DropTracking {
    pub gravity_accumulator: u32,
    pub soft_drop_frames: u32,
    pub sonic_drop_rows: u32,
}

#[derive(Resource)]
pub struct InputState(pub InputSnapshot);

impl Default for InputState {
    fn default() -> Self {
        InputState(InputSnapshot::empty())
    }
}

#[derive(Resource)]
pub struct RotationSystemRes(pub Box<dyn RotationSystem>);

#[derive(Resource)]
pub struct GameModeRes(pub GameMode);

#[derive(Resource)]
pub struct RotationKind(pub Kind);

/// Captures the piece phase at the very start of each tick.
///
/// Phase-specific systems gate on this value instead of the live `CurrentPhase`
/// so that at most one phase system runs per tick — matching master's
/// "one phase per tick" semantics. `None` means no tick is in progress
/// (game not started, game over, or game won).
#[derive(Resource, Default, Clone, Copy)]
pub struct TickStartPhase(pub Option<crate::data::PiecePhase>);
