use crate::data::PieceKind;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// TGM-style randomizer. 4-piece history (initialized to [Z; 4]); up to 4 retries
/// to avoid history collisions. First piece never S, Z, or O.
#[derive(Resource)]
pub struct Randomizer {
    history: [PieceKind; 4],
    is_first: bool,
    rng: StdRng,
}

impl Randomizer {
    pub fn new() -> Self {
        Self::with_seed(rand::random())
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            history: [PieceKind::Z; 4],
            is_first: true,
            rng: StdRng::seed_from_u64(seed),
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

    fn candidate(&mut self) -> PieceKind {
        if self.is_first {
            // First piece avoids S, Z, O.
            match rand::Rng::gen_range(&mut self.rng, 0..4) {
                0 => PieceKind::I,
                1 => PieceKind::T,
                2 => PieceKind::J,
                _ => PieceKind::L,
            }
        } else {
            PieceKind::random(&mut self.rng)
        }
    }
}

impl Default for Randomizer {
    fn default() -> Self {
        Self::new()
    }
}
