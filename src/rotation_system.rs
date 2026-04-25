use crate::data::{BoardGrid, PieceKind, RotationDirection};

/// Lightweight value used by RotationSystem::try_rotate. Mirrors the active
/// piece's spatial state. Created from ECS components by callers.
#[derive(Debug, Clone, Copy)]
pub struct PieceState {
    pub kind: PieceKind,
    pub rotation: usize,
    pub col: i32,
    pub row: i32,
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

// ---------------------------------------------------------------------------
// SRS shape tables (computed at compile time)
// ---------------------------------------------------------------------------

const SRS_I: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | ..O. | .... | .O..
    OOOO | ..O. | .... | .O..
    .... | ..O. | OOOO | .O..
    .... | ..O. | .... | .O..
",
);
const SRS_O: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .... | .... | .... | ....
    .OO. | .OO. | .OO. | .OO.
    .OO. | .OO. | .OO. | .OO.
    .... | .... | .... | ....
",
);
const SRS_T: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .O.. | .O.. | .... | .O..
    OOO. | .OO. | OOO. | OO..
    .... | .O.. | .O.. | .O..
    .... | .... | .... | ....
",
);
const SRS_S: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    .OO. | .O.. | .... | O...
    OO.. | .OO. | .OO. | OO..
    .... | ..O. | OO.. | .O..
    .... | .... | .... | ....
",
);
const SRS_Z: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    OO.. | ..O. | .... | .O..
    .OO. | .OO. | OO.. | OO..
    .... | .O.. | .OO. | O...
    .... | .... | .... | ....
",
);
const SRS_J: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    O... | .OO. | .... | .O..
    OOO. | .O.. | OOO. | .O..
    .... | .O.. | ..O. | OO..
    .... | .... | .... | ....
",
);
const SRS_L: [[(i32, i32); 4]; 4] = parse_rotations(
    "
    ..O. | .O.. | .... | OO..
    OOO. | .O.. | OOO. | .O..
    .... | .OO. | O... | .O..
    .... | .... | .... | ....
",
);

fn srs_cells(kind: PieceKind, rotation: usize) -> [(i32, i32); 4] {
    let table = match kind {
        PieceKind::I => &SRS_I,
        PieceKind::O => &SRS_O,
        PieceKind::T => &SRS_T,
        PieceKind::S => &SRS_S,
        PieceKind::Z => &SRS_Z,
        PieceKind::J => &SRS_J,
        PieceKind::L => &SRS_L,
    };
    table[rotation % 4]
}

// ---------------------------------------------------------------------------
// SRS kick tables
// Offsets are (dcol, drow) in our coordinate system (positive drow = down).
// Converted from wiki (x, y) with y-up: dcol = x, drow = -y.
// 8 entries indexed by kick_index(from_rotation, cw).
// ---------------------------------------------------------------------------

/// Maps (from_rotation, clockwise) to a kick table index.
const fn kick_index(from_rot: usize, cw: bool) -> usize {
    match (from_rot, cw) {
        (0, true) => 0,  // 0→1 CW
        (1, false) => 1, // 1→0 CCW
        (1, true) => 2,  // 1→2 CW
        (2, false) => 3, // 2→1 CCW
        (2, true) => 4,  // 2→3 CW
        (3, false) => 5, // 3→2 CCW
        (3, true) => 6,  // 3→0 CW
        (0, false) => 7, // 0→3 CCW
        _ => unreachable!(),
    }
}

/// JLSTZ wall kick offsets (dcol, drow), 5 tests per transition.
/// Test 1 is always (0,0) — the basic rotation.
const JLSTZ_KICKS: [[(i32, i32); 5]; 8] = [
    // 0→1 CW
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // 1→0 CCW
    [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
    // 1→2 CW
    [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
    // 2→1 CCW
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // 2→3 CW
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    // 3→2 CCW
    [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
    // 3→0 CW
    [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
    // 0→3 CCW
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
];

/// I-piece wall kick offsets (dcol, drow), 5 tests per transition.
const I_KICKS: [[(i32, i32); 5]; 8] = [
    // 0→1 CW
    [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
    // 1→0 CCW
    [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
    // 1→2 CW
    [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
    // 2→1 CCW
    [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
    // 2→3 CW
    [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
    // 3→2 CCW
    [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
    // 3→0 CW
    [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
    // 0→3 CCW
    [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
];

pub trait RotationSystem: Send + Sync + 'static {
    fn cells(&self, kind: PieceKind, rotation: usize) -> [(i32, i32); 4];

    fn preview_y_offset(&self, kind: PieceKind) -> i32;

    /// Returns true if the piece at (col, row) with the given rotation fits on the board
    /// (all cells in bounds and unoccupied).
    fn fits(
        &self,
        board: &BoardGrid,
        kind: PieceKind,
        col: i32,
        row: i32,
        rotation: usize,
    ) -> bool {
        self.cells(kind, rotation).iter().all(|(dc, dr)| {
            board
                .get((row + dr) as usize)
                .and_then(|r| r.get((col + dc) as usize))
                .map(|cell| cell.is_none())
                .unwrap_or(false)
        })
    }

    /// Attempt to rotate `piece` in `direction` on `board`.
    /// Returns `Some(new_piece)` with updated `col` and `rotation` on success, `None` if no
    /// kick position fits.
    fn try_rotate(
        &self,
        piece: &PieceState,
        direction: RotationDirection,
        board: &BoardGrid,
    ) -> Option<PieceState>;
}

// ---------------------------------------------------------------------------
// ARS
// ---------------------------------------------------------------------------

pub struct Ars;

impl Ars {
    /// Scans the destination rotation's cells left-to-right, top-to-bottom.
    /// Returns true if the first destination cell that collides with the board
    /// is in the center column (dc == 1), meaning a kick would not escape the obstacle.
    fn center_column_blocked_first(
        board: &BoardGrid,
        piece: &PieceState,
        new_rot: usize,
    ) -> bool {
        let dest_cells = ars_cells(piece.kind, new_rot);
        for dr in 0..3i32 {
            for dc in 0..3i32 {
                if dest_cells.iter().any(|&(ddc, ddr)| ddc == dc && ddr == dr) {
                    let col = piece.col + dc;
                    let row = piece.row + dr;
                    let occupied = board
                        .get(row as usize)
                        .and_then(|r| r.get(col as usize))
                        .map(|cell| cell.is_some())
                        .unwrap_or(true); // out-of-bounds → occupied
                    if occupied {
                        return dc == 1;
                    }
                }
            }
        }
        false
    }
}

impl RotationSystem for Ars {
    fn cells(&self, kind: PieceKind, rotation: usize) -> [(i32, i32); 4] {
        ars_cells(kind, rotation)
    }

    fn preview_y_offset(&self, _: PieceKind) -> i32 {
        0
    }

    fn try_rotate(
        &self,
        piece: &PieceState,
        direction: RotationDirection,
        board: &BoardGrid,
    ) -> Option<PieceState> {
        let offset = match direction {
            RotationDirection::Clockwise => 1,
            RotationDirection::Counterclockwise => 3,
        };
        let new_rot = (piece.rotation + offset) % 4;

        // 1. Basic rotation.
        if self.fits(board, piece.kind, piece.col, piece.row, new_rot) {
            return Some(PieceState {
                rotation: new_rot,
                ..*piece
            });
        }

        // I-piece never kicks.
        if piece.kind == PieceKind::I {
            return None;
        }

        // L/J/T center-column rule: from a 3-wide orientation (rot 0 or 2),
        // if the first destination-rotation cell that collides with the board
        // (scanning left-to-right, top-to-bottom) is in the center column,
        // suppress kicks for this direction.
        if matches!(piece.kind, PieceKind::L | PieceKind::J | PieceKind::T)
            && piece.rotation % 2 == 0
            && Self::center_column_blocked_first(board, piece, new_rot)
        {
            return None;
        }

        // 2. Kick right, then left.
        for dcol in [1i32, -1] {
            if self.fits(board, piece.kind, piece.col + dcol, piece.row, new_rot) {
                return Some(PieceState {
                    col: piece.col + dcol,
                    rotation: new_rot,
                    ..*piece
                });
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// SRS
// ---------------------------------------------------------------------------

pub struct Srs;

impl RotationSystem for Srs {
    fn cells(&self, kind: PieceKind, rotation: usize) -> [(i32, i32); 4] {
        srs_cells(kind, rotation)
    }

    fn preview_y_offset(&self, kind: PieceKind) -> i32 {
        use PieceKind::*;
        match kind {
            I | O => 0,
            T | S | Z | L | J => 1,
        }
    }

    fn try_rotate(
        &self,
        piece: &PieceState,
        direction: RotationDirection,
        board: &BoardGrid,
    ) -> Option<PieceState> {
        let cw = matches!(direction, RotationDirection::Clockwise);
        let offset = if cw { 1 } else { 3 };
        let new_rot = (piece.rotation + offset) % 4;

        // O-piece: basic rotation only (symmetric — always fits or always doesn't).
        if piece.kind == PieceKind::O {
            return if self.fits(board, piece.kind, piece.col, piece.row, new_rot) {
                Some(PieceState {
                    rotation: new_rot,
                    ..*piece
                })
            } else {
                None
            };
        }

        let kicks = if piece.kind == PieceKind::I {
            &I_KICKS
        } else {
            &JLSTZ_KICKS
        };
        let idx = kick_index(piece.rotation, cw);

        for &(dcol, drow) in &kicks[idx] {
            let new_col = piece.col + dcol;
            let new_row = piece.row + drow;
            if self.fits(board, piece.kind, new_col, new_row, new_rot) {
                return Some(PieceState {
                    col: new_col,
                    row: new_row,
                    rotation: new_rot,
                    ..*piece
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use crate::data::{BOARD_COLS, BOARD_ROWS};

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

    #[test]
    fn ars_cells_matches_const_table() {
        let ars = Ars;
        // I-piece rot 0: horizontal bar at row 1
        assert_eq!(ars.cells(PieceKind::I, 0), [(0, 1), (1, 1), (2, 1), (3, 1)]);
        // T-piece rot 1: column shape
        assert_eq!(ars.cells(PieceKind::T, 1), [(1, 0), (0, 1), (1, 1), (1, 2)]);
    }

    #[test]
    fn srs_cells_i_piece() {
        let srs = Srs;
        // SRS I rot 0: bar at row 1
        assert_eq!(srs.cells(PieceKind::I, 0), [(0, 1), (1, 1), (2, 1), (3, 1)]);
        // SRS I rot 1: bar at col 2, rows 0-3
        assert_eq!(srs.cells(PieceKind::I, 1), [(2, 0), (2, 1), (2, 2), (2, 3)]);
        // SRS I rot 2: bar at row 2
        assert_eq!(srs.cells(PieceKind::I, 2), [(0, 2), (1, 2), (2, 2), (3, 2)]);
        // SRS I rot 3: bar at col 1, rows 0-3
        assert_eq!(srs.cells(PieceKind::I, 3), [(1, 0), (1, 1), (1, 2), (1, 3)]);
    }

    #[test]
    fn srs_cells_t_piece_spawn() {
        let srs = Srs;
        // SRS T rot 0: bump at top
        assert_eq!(srs.cells(PieceKind::T, 0), [(1, 0), (0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn srs_t_basic_rotation_empty_board() {
        let board = [[None; BOARD_COLS]; BOARD_ROWS];
        let piece = PieceState {
            kind: PieceKind::T,
            rotation: 0,
            col: 3,
            row: 8,
        };
        let srs = Srs;
        let result = srs.try_rotate(&piece, RotationDirection::Clockwise, &board);
        assert!(
            result.is_some(),
            "basic rotation on empty board must succeed"
        );
        let new_piece = result.unwrap();
        assert_eq!(new_piece.rotation, 1);
        assert_eq!(new_piece.col, piece.col);
        assert_eq!(new_piece.row, piece.row);
    }
}
