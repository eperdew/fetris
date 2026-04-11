use crate::game::{Game, RotationDirection};
use crate::piece::PieceKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationSystem {
    Ars,
    Srs,
}

impl RotationSystem {
    fn try_rotate_ars(game: &mut Game, direction: RotationDirection) {
        let offset = match direction {
            RotationDirection::Clockwise => 1,
            RotationDirection::Counterclockwise => 3,
        };
        let new_rot = (game.active.rotation + offset) % 4;

        // 1. Basic rotation.
        if game.fits(game.active.col, game.active.row, new_rot) {
            game.active.rotation = new_rot;
            return;
        }

        // I-piece never kicks.
        if game.active.kind == PieceKind::I {
            return;
        }

        // L/J/T center-column rule: from a 3-wide orientation (rot 0 or 2),
        // if the first destination-rotation cell that collides with the board
        // (scanning left-to-right, top-to-bottom) is in the center column,
        // suppress kicks for this direction.
        if matches!(game.active.kind, PieceKind::L | PieceKind::J | PieceKind::T)
            && game.active.rotation % 2 == 0
            && Self::center_column_blocked_first(game, new_rot)
        {
            return;
        }

        // 2. Kick right, then left.
        for dcol in [1i32, -1] {
            if game.fits(game.active.col + dcol, game.active.row, new_rot) {
                game.active.col += dcol;
                game.active.rotation = new_rot;
                return;
            }
        }
    }

    /// Scans the destination rotation's cells left-to-right, top-to-bottom.
    /// Returns true if the first destination cell that collides with the board
    /// is in the center column (dc == 1), meaning a kick would not escape the obstacle.
    fn center_column_blocked_first(game: &Game, new_rot: usize) -> bool {
        let dest_cells = crate::piece::cells(game.active.kind, new_rot);
        for dr in 0..3i32 {
            for dc in 0..3i32 {
                if dest_cells.iter().any(|&(ddc, ddr)| ddc == dc && ddr == dr)
                    && !game.unoccupied(game.active.col + dc, game.active.row + dr)
                {
                    return dc == 1;
                }
            }
        }
        false
    }

    fn try_rotate_srs(game: &mut Game, direction: RotationDirection) {
        // TODO: Implement SRS rotation system. For now we use ARS rotation to avoid crashing.
        Self::try_rotate_ars(game, direction);
    }

    pub fn try_rotate(self, game: &mut Game, direction: RotationDirection) {
        match self {
            Self::Ars => Self::try_rotate_ars(game, direction),
            Self::Srs => Self::try_rotate_srs(game, direction),
        }
    }
}
