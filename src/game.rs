use crate::input::GameAction;
use crate::piece::{Piece, PieceKind};

pub const BOARD_COLS: usize = 10;
pub const BOARD_ROWS: usize = 20;

/// None = empty, Some(kind) = locked cell color
pub type Board = [[Option<PieceKind>; BOARD_COLS]; BOARD_ROWS];

pub struct Game {
    pub board: Board,
    pub active: Piece,
    pub next: Piece,
    pub score: u32,
    pub level: u32,
    pub lines: u32,
    pub game_over: bool,
}

pub enum RotationDirection {
    Clockwise,
    Counterclockwise,
}

impl Game {
    pub fn new() -> Self {
        Self {
            board: [[None; BOARD_COLS]; BOARD_ROWS],
            active: Piece::new(PieceKind::random()),
            next: Piece::new(PieceKind::random()),
            score: 0,
            level: 1,
            lines: 0,
            game_over: false,
        }
    }

    /// Called on every gravity tick.
    pub fn tick(&mut self) {
        if self.game_over {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock_piece();
        }
    }

    pub fn handle_action(&mut self, action: GameAction) {
        if self.game_over {
            return;
        }
        match action {
            GameAction::MoveLeft => {
                self.try_move(-1, 0);
            }
            GameAction::MoveRight => {
                self.try_move(1, 0);
            }
            GameAction::MoveDown => {
                if !self.try_move(0, 1) {
                    self.lock_piece();
                }
            }
            GameAction::RotateCw => self.try_rotate(RotationDirection::Clockwise),
            GameAction::RotateCcw => self.try_rotate(RotationDirection::Counterclockwise),
            GameAction::HardDrop => self.hard_drop(),
        }
    }

    /// Attempts to move the active piece by (dcol, drow). Returns true on success.
    fn try_move(&mut self, dcol: i32, drow: i32) -> bool {
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
        if self.fits(self.active.col, self.active.row, new_rot) {
            self.active.rotation = new_rot;
        }
    }

    fn hard_drop(&mut self) {
        while self.try_move(0, 1) {}
        self.lock_piece();
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

    fn lock_piece(&mut self) {
        for (dc, dr) in self.active.cells() {
            let c = (self.active.col + dc) as usize;
            let r = (self.active.row + dr) as usize;
            if r < BOARD_ROWS {
                self.board[r][c] = Some(self.active.kind);
            }
        }
        self.clear_lines();
        // Advance to next piece
        let next_kind = PieceKind::random();
        self.active = std::mem::replace(&mut self.next, Piece::new(next_kind));
        self.active.col = 3;
        self.active.row = 0;
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
        self.score += match count {
            1 => 100,
            2 => 300,
            3 => 500,
            _ => 800,
        } * self.level;
        self.level = 1 + self.lines / 10;
    }
}
