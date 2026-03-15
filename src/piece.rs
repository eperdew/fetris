use rand::Rng;

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
        match rand::rng().random_range(0..7) {
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
pub fn cells(kind: PieceKind, rotation: usize) -> [(i32, i32); 4] {
    // Rotations defined as (col, row) offsets from spawn top-left corner.
    // 0 = spawn orientation.
    match kind {
        PieceKind::I => [
            [(0, 1), (1, 1), (2, 1), (3, 1)],
            [(2, 0), (2, 1), (2, 2), (2, 3)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(1, 0), (1, 1), (1, 2), (1, 3)],
        ][rotation % 4],
        PieceKind::O => [
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
        ][rotation % 4],
        PieceKind::T => [
            [(1, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (1, 2)],
            [(1, 0), (0, 1), (1, 1), (1, 2)],
        ][rotation % 4],
        PieceKind::S => [
            [(1, 0), (2, 0), (0, 1), (1, 1)],
            [(1, 0), (1, 1), (2, 1), (2, 2)],
            [(1, 1), (2, 1), (0, 2), (1, 2)],
            [(0, 0), (0, 1), (1, 1), (1, 2)],
        ][rotation % 4],
        PieceKind::Z => [
            [(0, 0), (1, 0), (1, 1), (2, 1)],
            [(2, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 2), (2, 2)],
            [(1, 0), (0, 1), (1, 1), (0, 2)],
        ][rotation % 4],
        PieceKind::J => [
            [(0, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 2)],
            [(1, 0), (1, 1), (0, 2), (1, 2)],
        ][rotation % 4],
        PieceKind::L => [
            [(2, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 2)],
            [(0, 1), (1, 1), (2, 1), (0, 2)],
            [(0, 0), (1, 0), (1, 1), (1, 2)],
        ][rotation % 4],
    }
}
