use crate::data::*;
use crate::resources::PendingCompaction;
use crate::tests::harness::*;

/// Helper: fill all cells in a row on the given board.
fn fill_row(board: &mut BoardGrid, row: usize) {
    for c in 0..BOARD_COLS {
        board[row][c] = Some(PieceKind::O);
    }
}

#[test]
fn drain_events_no_clear_is_empty() {
    let mut app = make_app(PieceKind::T);
    // Drop a T-piece into an empty board — no line clear.
    // Run until not Falling anymore.
    while matches!(piece_phase(&mut app), PiecePhase::Falling) {
        idle(&mut app, 1);
    }
    let events = collect_line_clear_events(&app);
    assert!(events.is_empty(), "no line clear should produce no events");
}

#[test]
fn drain_events_single_clear() {
    let mut app = make_app(PieceKind::I);
    // Pre-fill the bottom row with gaps only where the I-piece will land.
    let mut b = board(&mut app);
    fill_row(&mut b, BOARD_ROWS - 1);
    b[BOARD_ROWS - 1][3] = None;
    b[BOARD_ROWS - 1][4] = None;
    b[BOARD_ROWS - 1][5] = None;
    b[BOARD_ROWS - 1][6] = None;
    set_board(&mut app, b);
    // Place active I-piece at the bottom row in horizontal orientation.
    set_active_position(&mut app, 3, BOARD_ROWS as i32 - 2);
    // Drop to floor
    drop_to_floor(&mut app);
    idle(&mut app, 1); // enter Locking
                       // Lock immediately with soft drop.
    press(&mut app, GameKey::SoftDrop);
    let counts = collect_line_clear_events(&app);
    assert_eq!(counts, vec![1]);
}

#[test]
fn drain_events_clears_after_drain() {
    let mut app = make_app(PieceKind::I);
    let mut b = board(&mut app);
    fill_row(&mut b, BOARD_ROWS - 1);
    b[BOARD_ROWS - 1][3] = None;
    b[BOARD_ROWS - 1][4] = None;
    b[BOARD_ROWS - 1][5] = None;
    b[BOARD_ROWS - 1][6] = None;
    set_board(&mut app, b);
    set_active_position(&mut app, 3, BOARD_ROWS as i32 - 2);
    drop_to_floor(&mut app);
    idle(&mut app, 1); // enter Locking
    press(&mut app, GameKey::SoftDrop);
    // First drain
    let _first = collect_line_clear_events(&app);
    // Advance one more tick so messages are cleared
    idle(&mut app, 1);
    // Second drain should be empty (no line clear this tick)
    let events2 = collect_line_clear_events(&app);
    assert!(
        events2.is_empty(),
        "drain_events should clear the buffer after one frame"
    );
}

#[test]
fn snapshot_active_hidden_during_spawning() {
    let mut app = make_app(PieceKind::T);
    // Force the Spawning phase.
    app.world_mut()
        .resource_mut::<crate::resources::CurrentPhase>()
        .0 = PiecePhase::Spawning { ticks_left: 5 };
    let snap = snapshot(&mut app);
    assert!(
        snap.active_kind.is_none(),
        "active should be hidden during Spawning"
    );
    assert!(snap.active_cells.is_none());
    assert!(snap.ghost_cells.is_none());
}

#[test]
fn snapshot_active_hidden_during_line_clear_delay() {
    let mut app = make_app(PieceKind::T);
    app.world_mut()
        .resource_mut::<crate::resources::CurrentPhase>()
        .0 = PiecePhase::LineClearDelay { ticks_left: 10 };
    let snap = snapshot(&mut app);
    assert!(snap.active_kind.is_none());
}

#[test]
fn snapshot_active_visible_during_falling() {
    let mut app = make_app(PieceKind::T);
    // Default piece_phase is Falling.
    let snap = snapshot(&mut app);
    assert_eq!(snap.active_kind, Some(PieceKind::T));
    assert!(snap.active_cells.is_some());
}

#[test]
fn snapshot_ghost_none_when_piece_on_floor() {
    let mut app = make_app(PieceKind::O);
    // Move O piece to the bottom row (rows 18-19 for ARS O in rotation 0).
    let col = active_position(&mut app).col;
    set_active_position(&mut app, col, 18);
    let snap = snapshot(&mut app);
    // Ghost row == active row → ghost_cells should be None.
    assert!(
        snap.ghost_cells.is_none(),
        "ghost should be None when piece is already on floor"
    );
}

#[test]
fn snapshot_ghost_present_above_floor() {
    let mut app = make_app(PieceKind::O);
    let col = active_position(&mut app).col;
    set_active_position(&mut app, col, 0); // piece near top, lots of room to fall
    let snap = snapshot(&mut app);
    assert!(
        snap.ghost_cells.is_some(),
        "ghost should be Some when piece can still fall"
    );
}

#[test]
fn lock_and_move_regression_test() {
    // Make sure that locking a piece on the same frame you move it does something sensible.
    let mut app = headless_app();
    start_with(&mut app, GameMode::TwentyG, Kind::Ars, PieceKind::O);
    set_board(
        &mut app,
        board_from_ascii(
            "
        ....O.....
        OOOOOOOOOO
    ",
        ),
    );
    idle(&mut app, 1);
    insta::assert_snapshot!(board_lines(&mut app, &[]).join("\n"), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
     5│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    10│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │        [][]        │
      │        [][]        │
      │        ##          │
      │####################│
    20└────────────────────┘
    ");
    // Press right and down on the same frame.
    let mut input = InputSnapshot::empty();
    input.held.insert(GameKey::Right);
    input.held.insert(GameKey::SoftDrop);
    input.just_pressed.insert(GameKey::Right);
    input.just_pressed.insert(GameKey::SoftDrop);
    tick_with(&mut app, input);
    // Here, locking is applied first before horizontal movement, so locking wins.
    insta::assert_snapshot!(board_lines(&mut app, &[]).join("\n"), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
     5│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    10│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │        ####        │
      │        ####        │
      │        ##          │
      │####################│
    20└────────────────────┘
    ");
}
