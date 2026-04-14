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
