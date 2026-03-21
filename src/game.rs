use crate::constants::{DAS_CHARGE, DAS_REPEAT, GRAVITY_DELAY, LOCK_DELAY, SPAWN_DELAY};
use crate::input::{GameKey, InputState};
use crate::piece::{Piece, PieceKind};
use crate::randomizer::Randomizer;

pub const BOARD_COLS: usize = 10;
pub const BOARD_ROWS: usize = 20;

/// None = empty, Some(kind) = locked cell color
pub type Board = [[Option<PieceKind>; BOARD_COLS]; BOARD_ROWS];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PiecePhase {
    Falling,
    Locking { ticks_left: u32 },
    Spawning { ticks_left: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizDir {
    Left,
    Right,
}

pub struct Game {
    pub board: Board,
    pub active: Piece,
    pub next: Piece,
    pub level: u32,
    pub lines: u32,
    pub game_over: bool,
    pub randomizer: Randomizer,
    pub piece_phase: PiecePhase,
    pub gravity_counter: u32,
    pub das_direction: Option<HorizDir>,
    pub das_counter: u32,
    pub rotation_buffer: Option<RotationDirection>,
}

pub enum RotationDirection {
    Clockwise,
    Counterclockwise,
}

impl Game {
    pub fn new() -> Self {
        let mut randomizer = Randomizer::new();
        let active = Piece::new(randomizer.next());
        let next = Piece::new(randomizer.next());
        Self {
            board: [[None; BOARD_COLS]; BOARD_ROWS],
            active,
            next,
            level: 1,
            lines: 0,
            game_over: false,
            randomizer,
            piece_phase: PiecePhase::Falling,
            gravity_counter: 0,
            das_direction: None,
            das_counter: 0,
            rotation_buffer: None,
        }
    }

    pub fn tick(&mut self, input: &InputState) {
        if self.game_over {
            return;
        }

        // Phase 1: Spawn delay — buffer rotation inputs, count down, then spawn.
        if let PiecePhase::Spawning { ticks_left } = &mut self.piece_phase {
            if input.held.contains(&GameKey::RotateCw) {
                self.rotation_buffer = Some(RotationDirection::Clockwise);
            } else if input.held.contains(&GameKey::RotateCcw) {
                self.rotation_buffer = Some(RotationDirection::Counterclockwise);
            }
            if *ticks_left == 0 {
                self.spawn_piece();
            } else {
                *ticks_left -= 1;
            }
            return; // No other input processed during spawn delay.
        }

        // Phase 2: Rotation (instant, not held).
        if input.just_pressed.contains(&GameKey::RotateCw) {
            self.try_rotate(RotationDirection::Clockwise);
        } else if input.just_pressed.contains(&GameKey::RotateCcw) {
            self.try_rotate(RotationDirection::Counterclockwise);
        }

        // Phase 3: Horizontal DAS.
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

        // Phase 4: Sonic drop (Space) — drop to floor, enter lock delay.
        if input.just_pressed.contains(&GameKey::SonicDrop) {
            while self.try_move(0, 1) {}
            self.piece_phase = PiecePhase::Locking {
                ticks_left: LOCK_DELAY,
            };
            return;
        }

        // Phase 5: Soft drop (Down) — bypass lock delay or advance gravity.
        if input.held.contains(&GameKey::SoftDrop) {
            match self.piece_phase {
                PiecePhase::Locking { .. } => {
                    self.lock_piece(input);
                    return;
                }
                _ => {
                    self.try_move(0, 1);
                    self.gravity_counter = 0; // soft drop resets gravity timer
                }
            }
        }

        // Phase 6: Gravity.
        self.gravity_counter += 1;
        if self.gravity_counter >= GRAVITY_DELAY {
            self.gravity_counter = 0;
            self.try_move(0, 1);
        }

        // Phase 7: Lock state transitions.
        let on_floor = !self.fits(self.active.col, self.active.row + 1, self.active.rotation);
        match self.piece_phase {
            PiecePhase::Falling => {
                if on_floor {
                    self.piece_phase = PiecePhase::Locking {
                        ticks_left: LOCK_DELAY,
                    };
                }
            }
            PiecePhase::Locking { ref mut ticks_left } => {
                if !on_floor {
                    // Piece moved off its resting surface.
                    self.piece_phase = PiecePhase::Falling;
                } else if *ticks_left == 0 {
                    self.lock_piece(input);
                } else {
                    *ticks_left -= 1;
                }
            }
            PiecePhase::Spawning { .. } => unreachable!(),
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
        let offset = match direction {
            RotationDirection::Clockwise => 1,
            RotationDirection::Counterclockwise => 3,
        };
        let new_rot = (self.active.rotation + offset) % 4;

        // 1. Basic rotation.
        if self.fits(self.active.col, self.active.row, new_rot) {
            self.active.rotation = new_rot;
            return;
        }

        // I-piece never kicks.
        if self.active.kind == PieceKind::I {
            return;
        }

        // L/J/T center-column rule: from a 3-wide orientation (rot 0 or 2),
        // if the first destination-rotation cell that collides with the board
        // (scanning left-to-right, top-to-bottom) is in the center column,
        // suppress kicks for this direction.
        if matches!(self.active.kind, PieceKind::L | PieceKind::J | PieceKind::T)
            && self.active.rotation % 2 == 0
            && self.center_column_blocked_first(new_rot)
        {
            return;
        }

        // 2. Kick right, then left.
        for dcol in [1i32, -1] {
            if self.fits(self.active.col + dcol, self.active.row, new_rot) {
                self.active.col += dcol;
                self.active.rotation = new_rot;
                return;
            }
        }
    }

    /// Scans the destination rotation's cells left-to-right, top-to-bottom.
    /// Returns true if the first destination cell that collides with the board
    /// is in the center column (dc == 1), meaning a kick would not escape the obstacle.
    fn center_column_blocked_first(&self, new_rot: usize) -> bool {
        let dest_cells = crate::piece::cells(self.active.kind, new_rot);
        for dr in 0..3i32 {
            for dc in 0..3i32 {
                if dest_cells.iter().any(|&(ddc, ddr)| ddc == dc && ddr == dr)
                    && !self.unoccupied(self.active.col + dc, self.active.row + dr)
                {
                    return dc == 1;
                }
            }
        }
        false
    }

    // A cell is unoccupied if
    //
    // 1. It is out of bounds, or...
    // 2. It is in bounds, but the cell is empty.
    fn unoccupied(&self, col: i32, row: i32) -> bool {
        self.board
            .get(row as usize)
            .and_then(|row| row.get(col as usize))
            .map(Option::is_none)
            .unwrap_or(false)
    }

    fn fits(&self, col: i32, row: i32, rotation: usize) -> bool {
        crate::piece::cells(self.active.kind, rotation)
            .iter()
            .all(|(dc, dr)| self.unoccupied(col + dc, row + dr))
    }

    fn lock_piece(&mut self, input: &InputState) {
        for (dc, dr) in self.active.cells() {
            let c = (self.active.col + dc) as usize;
            let r = (self.active.row + dr) as usize;
            if r < BOARD_ROWS {
                self.board[r][c] = Some(self.active.kind);
            }
        }
        self.clear_lines();
        // Buffer any held rotation key so it applies when the next piece spawns.
        if input.held.contains(&GameKey::RotateCw) {
            self.rotation_buffer = Some(RotationDirection::Clockwise);
        } else if input.held.contains(&GameKey::RotateCcw) {
            self.rotation_buffer = Some(RotationDirection::Counterclockwise);
        }
        // DAS charge carries over to the next piece (DAS buffering).
        // das_direction and das_counter are intentionally NOT reset here.
        self.piece_phase = PiecePhase::Spawning {
            ticks_left: SPAWN_DELAY,
        };
    }

    fn spawn_piece(&mut self) {
        let next_kind = self.randomizer.next();
        self.active = std::mem::replace(&mut self.next, Piece::new(next_kind));
        self.active.col = 3;
        self.active.row = 0;
        self.gravity_counter = 0;
        self.piece_phase = PiecePhase::Falling;
        // Apply buffered rotation if any.
        if let Some(dir) = self.rotation_buffer.take() {
            self.try_rotate(dir);
        }
        if !self.fits(self.active.col, self.active.row, self.active.rotation) {
            self.game_over = true;
        }
    }

    fn clear_lines(&mut self) {
        let cleared: Vec<usize> = (0..BOARD_ROWS)
            .filter(|&r| self.board[r].iter().all(|c| c.is_some()))
            .collect();
        let count = cleared.len() as u32;
        if count == 0 {
            return;
        }
        // Compact: keep non-full rows, then prepend empty rows at top
        let mut new_board: Board = [[None; BOARD_COLS]; BOARD_ROWS];
        let kept: Vec<[Option<PieceKind>; BOARD_COLS]> = self
            .board
            .iter()
            .filter(|row| row.iter().any(|c| c.is_none()))
            .copied()
            .collect();
        let offset = BOARD_ROWS - kept.len();
        for (i, row) in kept.into_iter().enumerate() {
            new_board[offset + i] = row;
        }
        self.board = new_board;
        self.lines += count;
        self.level = 1 + self.lines / 10;
    }
}
