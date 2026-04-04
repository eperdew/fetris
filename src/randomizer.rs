use crate::piece::PieceKind;

/// TGM-style randomizer. Keeps a 4-piece history (initialized to [Z; 4]) and
/// makes up to 4 attempts to produce a piece not in that history.
/// The first piece is never S, Z, or O.
pub struct Randomizer {
    history: [PieceKind; 4],
    is_first: bool,
}

impl Randomizer {
    pub fn new() -> Self {
        Self {
            history: [PieceKind::Z; 4],
            is_first: true,
        }
    }

    pub fn next(&mut self) -> PieceKind {
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
