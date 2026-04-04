
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

    /// Returns the (col, row) offsets of the four occupied cells relative to (self.col, self.row).
    pub fn cells(&self) -> [(i32, i32); 4] {
        cells(self.kind, self.rotation)
    }
}

/// Returns the four (col, row) offsets for a given kind and rotation.
/// Diagrams show all 4 rotations side by side, separated by `|`
/// `O` = filled cell, `.` = empty. Rows top-to-bottom, cols left-to-right.
pub fn cells(kind: PieceKind, rotation: usize) -> [(i32, i32); 4] {
    parse_rotations(match kind {
        //         rot 0  | rot 1  | rot 2  | rot 3
        PieceKind::I => {
            "
            .... | ..O. | .... | ..O.
            OOOO | ..O. | OOOO | ..O.
            .... | ..O. | .... | ..O.
            .... | ..O. | .... | ..O.
        "
        }
        PieceKind::O => {
            "
            .... | .... | .... | ....
            .OO. | .OO. | .OO. | .OO.
            .OO. | .OO. | .OO. | .OO.
            .... | .... | .... | ....
        "
        }
        PieceKind::T => {
            "
            .... | .O.. | .... | .O..
            OOO. | OO.. | .O.. | .OO.
            .O.. | .O.. | OOO. | .O..
            .... | .... | .... | ....
        "
        }
        PieceKind::S => {
            "
            .... | O... | .... | O...
            .OO. | OO.. | .OO. | OO..
            OO.. | .O.. | OO.. | .O..
            .... | .... | .... | ....
        "
        }
        PieceKind::Z => {
            "
            .... | ..O. | .... | ..O.
            OO.. | .OO. | OO.. | .OO.
            .OO. | .O.. | .OO. | .O..
            .... | .... | .... | ....
        "
        }
        PieceKind::J => {
            "
            .... | .O.. | .... | .OO.
            OOO. | .O.. | O... | .O..
            ..O. | OO.. | OOO. | .O..
            .... | .... | .... | ....
        "
        }
        PieceKind::L => {
            "
            .... | OO.. | .... | .O..
            OOO. | .O.. | ..O. | .O..
            O... | .O.. | OOO. | .OO.
            .... | .... | .... | ....
        "
        }
    })[rotation % 4]
}

/// Parses a diagram of 4 rotations laid out side by side with `|` column separators.
/// Returns one `[(col, row); 4]` array per rotation.
fn parse_rotations(diagram: &str) -> [[(i32, i32); 4]; 4] {
    let mut cells = [[(0i32, 0i32); 4]; 4];
    let mut counts = [0usize; 4];
    for (row, line) in diagram
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        for (rot, segment) in line.split('|').enumerate() {
            for (col, ch) in segment.trim().chars().enumerate() {
                if ch == 'O' {
                    cells[rot][counts[rot]] = (col as i32, row as i32);
                    counts[rot] += 1;
                }
            }
        }
    }
    debug_assert!(
        counts.iter().all(|&n| n == 4),
        "each rotation must have exactly 4 filled cells"
    );
    cells
}
