//! Pure data types shared across the game. No ECS-specific types here.

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// PieceKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

#[cfg(test)]
impl PieceKind {
    pub fn all() -> [Self; 7] {
        [
            Self::I,
            Self::O,
            Self::T,
            Self::S,
            Self::Z,
            Self::J,
            Self::L,
        ]
    }
}

impl PieceKind {
    /// Picks one of the 7 kinds uniformly using the supplied RNG.
    pub fn random<R: rand::Rng>(rng: &mut R) -> Self {
        match rng.gen_range(0..7) {
            0 => Self::I,
            1 => Self::O,
            2 => Self::T,
            3 => Self::S,
            4 => Self::Z,
            5 => Self::J,
            _ => Self::L,
        }
    }
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

pub const BOARD_COLS: usize = 10;
pub const BOARD_ROWS: usize = 20;

/// None = empty, Some(kind) = locked cell color.
pub type BoardGrid = [[Option<PieceKind>; BOARD_COLS]; BOARD_ROWS];

// ---------------------------------------------------------------------------
// Direction enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizDir {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDirection {
    Clockwise,
    Counterclockwise,
}

// ---------------------------------------------------------------------------
// PiecePhase
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiecePhase {
    Falling,
    Locking { ticks_left: u32 },
    LineClearDelay { ticks_left: u32 },
    Spawning { ticks_left: u32 },
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    Left,
    Right,
    RotateCw,
    RotateCcw,
    SoftDrop,
    SonicDrop,
}

#[derive(Debug, Default, Clone)]
pub struct InputSnapshot {
    pub held: HashSet<GameKey>,
    pub just_pressed: HashSet<GameKey>,
}

impl InputSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// Modes / Kinds
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GameMode {
    #[default]
    Master,
    TwentyG,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    #[default]
    Ars,
    Srs,
}

impl Kind {
    pub fn create(self) -> Box<dyn crate::rotation_system::RotationSystem> {
        match self {
            Kind::Ars => Box::new(crate::rotation_system::Ars),
            Kind::Srs => Box::new(crate::rotation_system::Srs),
        }
    }
}

// ---------------------------------------------------------------------------
// Grade + Score thresholds (TGM)
// ---------------------------------------------------------------------------

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Grade {
    Nine,
    Eight,
    Seven,
    Six,
    Five,
    Four,
    Three,
    Two,
    One,
    SOne,
    STwo,
    SThree,
    SFour,
    SFive,
    SSix,
    SSeven,
    SEight,
    SNine,
}

impl Grade {
    const SCORE_TABLE: &[(u32, Grade)] = &[
        (0, Grade::Nine),
        (400, Grade::Eight),
        (800, Grade::Seven),
        (1400, Grade::Six),
        (2000, Grade::Five),
        (3500, Grade::Four),
        (5500, Grade::Three),
        (8000, Grade::Two),
        (12000, Grade::One),
        (16000, Grade::SOne),
        (22000, Grade::STwo),
        (30000, Grade::SThree),
        (40000, Grade::SFour),
        (52000, Grade::SFive),
        (66000, Grade::SSix),
        (82000, Grade::SSeven),
        (100000, Grade::SEight),
        (120000, Grade::SNine),
    ];

    pub fn of_score(score: u32) -> Self {
        Self::SCORE_TABLE
            .iter()
            .rev()
            .find(|(t, _)| score >= *t)
            .map(|(_, g)| *g)
            .unwrap_or(Grade::Nine)
    }

    pub fn index(self) -> usize {
        Self::SCORE_TABLE
            .iter()
            .position(|(_, g)| *g == self)
            .unwrap_or(0)
    }

    pub fn grade_progress(score: u32) -> (u32, Option<u32>) {
        let idx = Self::SCORE_TABLE
            .iter()
            .rposition(|(t, _)| score >= *t)
            .unwrap_or(0);
        let prev = Self::SCORE_TABLE[idx].0;
        let next = Self::SCORE_TABLE.get(idx + 1).map(|(t, _)| *t);
        (prev, next)
    }
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Nine => "9",
            Self::Eight => "8",
            Self::Seven => "7",
            Self::Six => "6",
            Self::Five => "5",
            Self::Four => "4",
            Self::Three => "3",
            Self::Two => "2",
            Self::One => "1",
            Self::SOne => "S1",
            Self::STwo => "S2",
            Self::SThree => "S3",
            Self::SFour => "S4",
            Self::SFive => "S5",
            Self::SSix => "S6",
            Self::SSeven => "S7",
            Self::SEight => "S8",
            Self::SNine => "S9",
        };
        write!(f, "{:>2}", s)
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy::prelude::Event, bevy::prelude::Message)]
pub enum JudgeEvent {
    LockedWithoutClear,
    ClearedLines {
        level: u32,
        cleared_playfield: bool,
        num_lines: u32,
        frames_soft_drop_held: u32,
        sonic_drop_rows: u32,
        ticks_elapsed: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy::prelude::Event, bevy::prelude::Message)]
pub enum GameEvent {
    LineClear { count: u32 },
    PieceBeganLocking,
    GameEnded,
    GradeAdvanced(Grade),
}

// ---------------------------------------------------------------------------
// Hi scores (data only; storage is in Plan 3)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HiScoreEntry {
    pub grade: Grade,
    pub ticks: u64,
}

// ---------------------------------------------------------------------------
// Menu / Config (deferred to Plan 2; types defined for serde compat)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GameConfig {
    pub game_mode: GameMode,
    pub rotation: Kind,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            game_mode: GameMode::Master,
            rotation: Kind::Ars,
        }
    }
}
