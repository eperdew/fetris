use crate::audio_player::AudioPlayer;
use crate::constants::{
    ARE_DAS_FROZEN_FRAMES, DAS_CHARGE, DAS_REPEAT, LINE_CLEAR_DELAY, LOCK_DELAY,
    SPAWN_DELAY_NORMAL, gravity_g,
};
use crate::judge::Judge;
use crate::rotation_system;
use crate::types::{
    BOARD_COLS, BOARD_ROWS, Board, GameKey, GameMode, Grade, HorizDir, InputState, JudgeEvent,
    Kind, Piece, PieceKind, PiecePhase, RotationDirection,
};
use std::sync::Arc;

/// TGM-style randomizer. Keeps a 4-piece history (initialized to [Z; 4]) and
/// makes up to 4 attempts to produce a piece not in that history.
/// The first piece is never S, Z, or O.
struct Randomizer {
    history: [PieceKind; 4],
    is_first: bool,
}

impl Randomizer {
    fn new() -> Self {
        Self {
            history: [PieceKind::Z; 4],
            is_first: true,
        }
    }

    fn next(&mut self) -> PieceKind {
        let mut piece = self.candidate();
        for _ in 1..4 {
            if !self.history.contains(&piece) {
                break;
            }
            piece = self.candidate();
        }
        self.history.rotate_left(1);
        self.history[3] = piece;
        self.is_first = false;
        piece
    }

    fn candidate(&self) -> PieceKind {
        if self.is_first {
            // Avoid S, Z, O on the first piece to prevent forced overhangs.
            match (macroquad::rand::rand() % 4) as u8 {
                0 => PieceKind::I,
                1 => PieceKind::T,
                2 => PieceKind::J,
                _ => PieceKind::L,
            }
        } else {
            PieceKind::random()
        }
    }
}

pub(crate) struct Game {
    pub board: Board,
    pub active: Piece,
    pub rotation_system: Box<dyn rotation_system::RotationSystem>,
    pub game_mode: GameMode,
    pub judge: Judge,
    pub next: Piece,
    pub level: u32,
    pub lines: u32,
    pub game_over: bool,
    pub game_won: bool,
    pub ticks_elapsed: u64,
    randomizer: Randomizer,
    pub piece_phase: PiecePhase,
    pub gravity_accumulator: u32,
    pub das_direction: Option<HorizDir>,
    pub das_counter: u32,
    pub rotation_buffer: Option<RotationDirection>,
    pub rows_pending_compaction: Vec<usize>,
    pub soft_drop_frames: u32,
    pub sonic_drop_rows: u32,
    pub rotation_kind: Kind,
    pub score_submitted: bool,
    pub audio: Arc<dyn AudioPlayer>,
}

impl Game {
    pub fn new(
        game_mode: GameMode,
        rotation_kind: Kind,
        rotation_system: Box<dyn rotation_system::RotationSystem>,
        audio: Arc<dyn AudioPlayer>,
    ) -> Self {
        let mut randomizer = Randomizer::new();
        let active = Piece::new(randomizer.next());
        let next = Piece::new(randomizer.next());
        Self {
            board: [[None; BOARD_COLS]; BOARD_ROWS],
            active,
            rotation_system,
            game_mode,
            judge: Judge::new(),
            next,
            level: 0,
            lines: 0,
            game_over: false,
            game_won: false,
            ticks_elapsed: 0,
            randomizer,
            piece_phase: PiecePhase::Falling,
            gravity_accumulator: 0,
            das_direction: None,
            das_counter: 0,
            rotation_buffer: None,
            rows_pending_compaction: Vec::new(),
            soft_drop_frames: 0,
            sonic_drop_rows: 0,
            rotation_kind,
            score_submitted: false,
            audio,
        }
    }

    pub fn tick(&mut self, input: &InputState) {
        if self.game_over || self.game_won {
            return;
        }
        self.ticks_elapsed += 1;

        // Phase 1a: Line clear delay — DAS frozen, buffer rotation, count down, then enter ARE.
        if let PiecePhase::LineClearDelay { ticks_left } = &mut self.piece_phase {
            if input.held.contains(&GameKey::RotateCw) {
                self.rotation_buffer = Some(RotationDirection::Clockwise);
            } else if input.held.contains(&GameKey::RotateCcw) {
                self.rotation_buffer = Some(RotationDirection::Counterclockwise);
            }
            if *ticks_left == 0 {
                self.compact_pending_rows();
                self.piece_phase = PiecePhase::Spawning {
                    ticks_left: SPAWN_DELAY_NORMAL,
                };
            } else {
                *ticks_left -= 1;
            }
            return;
        }

        // Phase 1b: ARE (spawn delay) — buffer rotation, DAS charges during middle frames.
        if let PiecePhase::Spawning { ticks_left } = &mut self.piece_phase {
            if input.held.contains(&GameKey::RotateCw) {
                self.rotation_buffer = Some(RotationDirection::Clockwise);
            } else if input.held.contains(&GameKey::RotateCcw) {
                self.rotation_buffer = Some(RotationDirection::Counterclockwise);
            } else {
                self.rotation_buffer = None;
            }
            let tl = *ticks_left;
            if tl == 0 {
                self.spawn_piece();
            } else {
                *ticks_left -= 1;
                // DAS charges during ARE frames 5–29 (tl in 1..=SPAWN_DELAY_NORMAL-ARE_DAS_FROZEN_FRAMES).
                // First 4 frames (tl > SPAWN_DELAY_NORMAL-ARE_DAS_FROZEN_FRAMES) and spawn frame (tl==0) are frozen.
                if tl <= SPAWN_DELAY_NORMAL - ARE_DAS_FROZEN_FRAMES {
                    let horiz = if input.held.contains(&GameKey::Left) {
                        Some(HorizDir::Left)
                    } else if input.held.contains(&GameKey::Right) {
                        Some(HorizDir::Right)
                    } else {
                        None
                    };
                    match horiz {
                        None => {
                            self.das_direction = None;
                            self.das_counter = 0;
                        }
                        Some(dir) => {
                            if self.das_direction != Some(dir) {
                                self.das_direction = Some(dir);
                                self.das_counter = 0;
                            } else {
                                self.das_counter += 1;
                            }
                        }
                    }
                }
            }
            return;
        }

        // Phase 2: Rotation (instant, not held).
        if input.just_pressed.contains(&GameKey::RotateCw) {
            self.try_rotate(RotationDirection::Clockwise);
        } else if input.just_pressed.contains(&GameKey::RotateCcw) {
            self.try_rotate(RotationDirection::Counterclockwise);
        }

        // Phase 3: Sonic drop (Space) — drop to floor, enter lock delay.
        if input.just_pressed.contains(&GameKey::SonicDrop) {
            let row_before = self.active.row;
            while self.try_move(0, 1) {}
            self.sonic_drop_rows += (self.active.row - row_before) as u32;
            self.piece_phase = PiecePhase::Locking {
                ticks_left: LOCK_DELAY,
            };
            return;
        }

        // Phase 4: Soft drop (Down) — bypass lock delay or advance gravity.
        if input.held.contains(&GameKey::SoftDrop) {
            self.soft_drop_frames += 1;
            match self.piece_phase {
                PiecePhase::Locking { .. } => {
                    self.lock_piece(input);
                    return;
                }
                _ => {
                    self.try_move(0, 1);
                    self.gravity_accumulator = 0;
                }
            }
        }

        // Phase 5: Horizontal DAS.
        let horiz = if input.held.contains(&GameKey::Left) {
            Some(HorizDir::Left)
        } else if input.held.contains(&GameKey::Right) {
            Some(HorizDir::Right)
        } else {
            None
        };

        match horiz {
            None => {
                self.das_direction = None;
                self.das_counter = 0;
            }
            Some(dir) => {
                if self.das_direction != Some(dir) {
                    // Direction changed or newly pressed: move immediately, reset counter.
                    self.das_direction = Some(dir);
                    self.das_counter = 0;
                    let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                    self.try_move(dcol, 0);
                } else {
                    self.das_counter += 1;
                    if self.das_counter >= DAS_CHARGE
                        && (self.das_counter - DAS_CHARGE) % DAS_REPEAT == 0
                    {
                        let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                        self.try_move(dcol, 0);
                    }
                }
            }
        }

        // Phase 6: Gravity (G/256 accumulator).
        let row_before = self.active.row;
        self.apply_gravity();
        let moved_down = self.active.row > row_before;

        // Phase 7: Lock state transitions.
        let on_floor = !self.fits(self.active.col, self.active.row + 1, self.active.rotation);
        match self.piece_phase {
            PiecePhase::Falling => {
                if on_floor {
                    self.piece_phase = PiecePhase::Locking {
                        ticks_left: LOCK_DELAY,
                    };
                    self.audio.piece_begin_locking();
                }
            }
            PiecePhase::Locking { ref mut ticks_left } => {
                if !on_floor {
                    // Piece moved off its resting surface.
                    self.piece_phase = PiecePhase::Falling;
                } else if moved_down {
                    // Piece dropped a row and re-landed: reset the lock timer.
                    *ticks_left = LOCK_DELAY;
                } else if *ticks_left == 0 {
                    self.lock_piece(input);
                } else {
                    *ticks_left -= 1;
                }
            }
            PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. } => unreachable!(),
        }
    }

    fn apply_gravity(&mut self) {
        self.gravity_accumulator += gravity_g(self.game_mode, self.level);
        let drops = self.gravity_accumulator / 256;
        self.gravity_accumulator %= 256;
        for _ in 0..drops {
            if !self.try_move(0, 1) {
                break;
            }
        }
    }

    /// Attempts to move the active piece by (dcol, drow). Returns true on success.
    pub(crate) fn try_move(&mut self, dcol: i32, drow: i32) -> bool {
        let new_col = self.active.col + dcol;
        let new_row = self.active.row + drow;
        if self.fits(new_col, new_row, self.active.rotation) {
            self.active.col = new_col;
            self.active.row = new_row;
            true
        } else {
            false
        }
    }

    fn try_rotate(&mut self, direction: RotationDirection) {
        if let Some(new_piece) =
            self.rotation_system
                .try_rotate(&self.active, direction, &self.board)
        {
            self.active = new_piece;
        }
    }

    pub fn fits(&self, col: i32, row: i32, rotation: usize) -> bool {
        self.rotation_system
            .fits(&self.board, self.active.kind, col, row, rotation)
    }

    fn board_is_empty(&self) -> bool {
        self.board.iter().all(|row| row.iter().all(Option::is_none))
    }

    fn lock_piece(&mut self, input: &InputState) {
        for (dc, dr) in self
            .rotation_system
            .cells(self.active.kind, self.active.rotation)
        {
            let c = (self.active.col + dc) as usize;
            let r = (self.active.row + dr) as usize;
            if r < BOARD_ROWS {
                self.board[r][c] = Some(self.active.kind);
            }
        }
        self.audio.piece_locked();
        let lines_cleared = self.clear_lines();
        if lines_cleared > 0 {
            self.audio.lines_cleared(lines_cleared);
        }
        // Buffer any held rotation key so it applies when the next piece spawns.
        if input.held.contains(&GameKey::RotateCw) {
            self.rotation_buffer = Some(RotationDirection::Clockwise);
        } else if input.held.contains(&GameKey::RotateCcw) {
            self.rotation_buffer = Some(RotationDirection::Counterclockwise);
        }
        // DAS charge carries over to the next piece (DAS buffering).
        // das_direction and das_counter are intentionally NOT reset here.
        self.piece_phase = if lines_cleared > 0 {
            PiecePhase::LineClearDelay {
                ticks_left: LINE_CLEAR_DELAY,
            }
        } else {
            PiecePhase::Spawning {
                ticks_left: SPAWN_DELAY_NORMAL,
            }
        };

        // Update the judge
        let judge_event = if lines_cleared > 0 {
            JudgeEvent::ClearedLines {
                level: self.level,
                cleared_playfield: self.board_is_empty(),
                num_lines: lines_cleared,
                frames_soft_drop_held: self.soft_drop_frames,
                sonic_drop_rows: self.sonic_drop_rows,
                ticks_elapsed: self.ticks_elapsed,
            }
        } else {
            JudgeEvent::LockedWithoutClear
        };
        self.judge.on_event(&judge_event);
    }

    fn spawn_piece(&mut self) {
        if can_piece_increment(self.level) {
            self.level += 1;
        }
        let next_kind = self.randomizer.next();
        self.active = std::mem::replace(&mut self.next, Piece::new(next_kind));
        self.active.col = 3;
        self.active.row = 0;
        self.gravity_accumulator = 0;
        self.piece_phase = PiecePhase::Falling;
        self.soft_drop_frames = 0;
        self.sonic_drop_rows = 0;
        // Apply buffered rotation if any.
        if let Some(dir) = self.rotation_buffer.take() {
            self.try_rotate(dir);
        }
        if !self.fits(self.active.col, self.active.row, self.active.rotation) {
            self.game_over = true;
        }

        // Gravity applies immediately upon spawning. This is necessary so that we spawn
        // on the ground in 20G.
        self.apply_gravity();
    }

    fn clear_lines(&mut self) -> u32 {
        let cleared: Vec<usize> = (0..BOARD_ROWS)
            .filter(|&r| self.board[r].iter().all(|c| c.is_some()))
            .collect();
        let count = cleared.len() as u32;
        if count == 0 {
            return 0;
        }
        self.rows_pending_compaction = cleared;
        self.lines += count;
        self.level = (self.level + count).min(999);
        if self.level == 999 {
            self.game_won = true;
        }
        count
    }

    fn compact_pending_rows(&mut self) {
        if self.rows_pending_compaction.is_empty() {
            return;
        }
        let mut new_board: Board = [[None; BOARD_COLS]; BOARD_ROWS];
        let kept: Vec<[Option<PieceKind>; BOARD_COLS]> = self
            .board
            .iter()
            .enumerate()
            .filter(|(r, _)| !self.rows_pending_compaction.contains(r))
            .map(|(_, row)| *row)
            .collect();
        let offset = BOARD_ROWS - kept.len();
        for (i, row) in kept.into_iter().enumerate() {
            new_board[offset + i] = row;
        }
        self.board = new_board;
        self.rows_pending_compaction.clear();
    }

    pub fn tick_ready(&mut self, input: &InputState) {
        // IRS: buffer rotation key (cleared if neither held, matching Spawning phase behavior)
        if input.held.contains(&GameKey::RotateCw) {
            self.rotation_buffer = Some(RotationDirection::Clockwise);
        } else if input.held.contains(&GameKey::RotateCcw) {
            self.rotation_buffer = Some(RotationDirection::Counterclockwise);
        } else {
            self.rotation_buffer = None;
        }

        // DAS charging
        let horiz = if input.held.contains(&GameKey::Left) {
            Some(HorizDir::Left)
        } else if input.held.contains(&GameKey::Right) {
            Some(HorizDir::Right)
        } else {
            None
        };
        match horiz {
            None => {
                self.das_direction = None;
                self.das_counter = 0;
            }
            Some(dir) => {
                if self.das_direction != Some(dir) {
                    self.das_direction = Some(dir);
                    self.das_counter = 0;
                } else {
                    self.das_counter += 1;
                }
            }
        }
    }

    pub fn apply_irs(&mut self) {
        if let Some(dir) = self.rotation_buffer.take() {
            self.try_rotate(dir);
        }
    }

    pub fn score(&self) -> u32 {
        self.judge.score()
    }

    pub fn grade(&self) -> Grade {
        self.judge.grade()
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn next_level_barrier(&self) -> u32 {
        let round_up = (self.level + 1).next_multiple_of(100);
        if round_up == 1000 { 999 } else { round_up }
    }
}

pub(crate) fn can_piece_increment(level: u32) -> bool {
    level % 100 != 99 && level != 998
}
