use crate::game::{Game, RotationDirection};
use crate::piece::PieceKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationSystem {
    Ars,
    Srs,
}

/// Parses a diagram of 4 rotations laid out side by side with `|` column separators,
/// at compile time. Each rotation must have exactly 4 filled cells (`O`).
/// Rows are indexed top-to-bottom, columns left-to-right within each segment.
const fn parse_rotations(diagram: &str) -> [[(i32, i32); 4]; 4] {
    let bytes = diagram.as_bytes();
    let len = bytes.len();
    let mut cells = [[(0i32, 0i32); 4]; 4];
    let mut counts = [0usize; 4];
    let mut i = 0usize;
    let mut data_row = 0i32;

    while i < len {
        // Skip line terminators.
        while i < len && (bytes[i] == b'\n' || bytes[i] == b'\r') {
            i += 1;
        }
        if i >= len {
            break;
        }

        // Find end of current line.
        let line_start = i;
        while i < len && bytes[i] != b'\n' && bytes[i] != b'\r' {
            i += 1;
        }
        let line_end = i;

        // Trim leading spaces.
        let mut ls = line_start;
        while ls < line_end && bytes[ls] == b' ' {
            ls += 1;
        }
        // Trim trailing spaces.
        let mut le = line_end;
        while le > ls && bytes[le - 1] == b' ' {
            le -= 1;
        }

        if ls >= le {
            continue; // blank line
        }

        // Parse segments separated by '|'.
        let mut rot = 0usize;
        let mut seg_start = ls;
        let mut j = ls;
        while j <= le {
            if j == le || bytes[j] == b'|' {
                // Trim segment.
                let mut ss = seg_start;
                while ss < j && bytes[ss] == b' ' {
                    ss += 1;
                }
                let mut se = j;
                while se > ss && bytes[se - 1] == b' ' {
                    se -= 1;
                }
                // Scan for filled cells.
                let mut k = ss;
                let mut col = 0i32;
                while k < se {
                    if bytes[k] == b'O' {
                        assert!(rot < 4, "too many segments in diagram row");
                        assert!(counts[rot] < 4, "too many filled cells in rotation");
                        cells[rot][counts[rot]] = (col, data_row);
                        counts[rot] += 1;
                    }
                    col += 1;
                    k += 1;
                }
                rot += 1;
                seg_start = j + 1;
            }
            j += 1;
        }
        data_row += 1;
    }

    assert!(
        counts[0] == 4 && counts[1] == 4 && counts[2] == 4 && counts[3] == 4,
        "each rotation must have exactly 4 filled cells"
    );
    cells
}

// ---------------------------------------------------------------------------
// ARS shape tables (computed at compile time)
// ---------------------------------------------------------------------------

const ARS_I: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | ..O. | .... | ..O.
    OOOO | ..O. | OOOO | ..O.
    .... | ..O. | .... | ..O.
    .... | ..O. | .... | ..O.
",
);
const ARS_O: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | .... | .... | ....
    .OO. | .OO. | .OO. | .OO.
    .OO. | .OO. | .OO. | .OO.
    .... | .... | .... | ....
",
);
const ARS_T: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | .O.. | .... | .O..
    OOO. | OO.. | .O.. | .OO.
    .O.. | .O.. | OOO. | .O..
    .... | .... | .... | ....
",
);
const ARS_S: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | O... | .... | O...
    .OO. | OO.. | .OO. | OO..
    OO.. | .O.. | OO.. | .O..
    .... | .... | .... | ....
",
);
const ARS_Z: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | ..O. | .... | ..O.
    OO.. | .OO. | OO.. | .OO.
    .OO. | .O.. | .OO. | .O..
    .... | .... | .... | ....
",
);
const ARS_J: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | .O.. | .... | .OO.
    OOO. | .O.. | O... | .O..
    ..O. | OO.. | OOO. | .O..
    .... | .... | .... | ....
",
);
const ARS_L: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | OO.. | .... | .O..
    OOO. | .O.. | ..O. | .O..
    O... | .O.. | OOO. | .OO.
    .... | .... | .... | ....
",
);

fn ars_cells(kind: PieceKind, rotation: usize) -> [(i32, i32); 4] {
    let table = match kind {
        PieceKind::I => &ARS_I,
        PieceKind::O => &ARS_O,
        PieceKind::T => &ARS_T,
        PieceKind::S => &ARS_S,
        PieceKind::Z => &ARS_Z,
        PieceKind::J => &ARS_J,
        PieceKind::L => &ARS_L,
    };
    table[rotation % 4]
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

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn parse_rotations_i_piece_ars() {
        // rot 0: horizontal bar in row 1
        // rot 1: vertical bar in col 2
        let shape = parse_rotations(
            "
            .... | ..O. | .... | ..O.
            OOOO | ..O. | OOOO | ..O.
            .... | ..O. | .... | ..O.
            .... | ..O. | .... | ..O.
        ",
        );
        assert_eq!(shape[0], [(0, 1), (1, 1), (2, 1), (3, 1)]);
        assert_eq!(shape[1], [(2, 0), (2, 1), (2, 2), (2, 3)]);
        assert_eq!(shape[2], [(0, 1), (1, 1), (2, 1), (3, 1)]); // same as rot 0 in ARS
        assert_eq!(shape[3], [(2, 0), (2, 1), (2, 2), (2, 3)]); // same as rot 1 in ARS
    }
}
