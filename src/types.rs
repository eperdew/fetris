use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Piece
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceKind {
    #[cfg(test)]
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

    pub fn random() -> Self {
        match macroquad::rand::rand() % 7 {
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

/// A tetromino's shape as a 4x4 bitmask of occupied cells, in (col, row) pairs.
/// Rotation is stored as an index 0–3.
#[derive(Debug, Clone)]
pub struct Piece {
    pub kind: PieceKind,
    pub rotation: usize,
    /// Board position of the top-left corner of the bounding box
    pub col: i32,
    pub row: i32,
}

impl Piece {
    pub fn new(kind: PieceKind) -> Self {
        Self {
            kind,
            rotation: 0,
            col: 3,
            row: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

pub const BOARD_COLS: usize = 10;
pub const BOARD_ROWS: usize = 20;

/// None = empty, Some(kind) = locked cell color
pub type Board = [[Option<PieceKind>; BOARD_COLS]; BOARD_ROWS];

// ---------------------------------------------------------------------------
// Game state enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PiecePhase {
    Falling,
    Locking {
        ticks_left: u32,
    },
    /// Line clear display phase (41 frames). DAS is frozen throughout.
    /// Transitions to Spawning{SPAWN_DELAY_NORMAL} when complete.
    LineClearDelay {
        ticks_left: u32,
    },
    /// ARE: piece spawn delay (30 frames). DAS charges during middle frames.
    Spawning {
        ticks_left: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizDir {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationDirection {
    Clockwise,
    Counterclockwise,
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Renderer-agnostic held-trackable key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    Left,
    Right,
    RotateCw,
    RotateCcw,
    SoftDrop,
    SonicDrop,
}

/// Snapshot of input state for one tick.
/// `held`: keys currently held down.
/// `just_pressed`: keys that transitioned to pressed this tick (subset of held).
/// Both are HashSets — ordering within a 16ms tick is not meaningful.
pub struct InputState {
    pub held: HashSet<GameKey>,
    pub just_pressed: HashSet<GameKey>,
}

impl InputState {
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            held: HashSet::new(),
            just_pressed: HashSet::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Rotation system kind
// ---------------------------------------------------------------------------

/// Menu-facing enum for selecting which rotation system to use.
/// Call `.create()` to obtain a `Box<dyn RotationSystem>`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
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
// Menu
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GameMode {
    Master,
    TwentyG,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuScreen {
    Main,
    HiScores,
    Controls,
}

#[derive(Default)]
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub back: bool,
}

pub enum MenuResult {
    Stay,
    StartGame { mode: GameMode, rotation: Kind },
}

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

// ---------------------------------------------------------------------------
// Judge / scoring
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
            .find(|(threshold, _)| score >= *threshold)
            .map(|(_, g)| *g)
            .unwrap_or(Grade::Nine)
    }

    /// Index of this grade in SCORE_TABLE (Nine=0, Eight=1, ..., SNine=17).
    pub fn index(self) -> usize {
        Self::SCORE_TABLE
            .iter()
            .position(|(_, g)| *g == self)
            .unwrap_or(0)
    }

    /// Returns (prev_threshold, Some(next_threshold)) for progress within the current grade,
    /// or (prev_threshold, None) at the max grade.
    pub fn grade_progress(score: u32) -> (u32, Option<u32>) {
        let idx = Self::SCORE_TABLE
            .iter()
            .rposition(|(threshold, _)| score >= *threshold)
            .unwrap_or(0);
        let prev = Self::SCORE_TABLE[idx].0;
        let next = Self::SCORE_TABLE.get(idx + 1).map(|(t, _)| *t);
        (prev, next)
    }
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_string = match self {
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
        write!(f, "{:>2}", as_string)
    }
}

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

// ---------------------------------------------------------------------------
// Hi scores
// ---------------------------------------------------------------------------

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct HiScoreEntry {
    pub grade: Grade,
    pub ticks: u64,
}
