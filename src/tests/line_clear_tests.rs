use crate::constants::{LINE_CLEAR_DELAY, SPAWN_DELAY_NORMAL};
use crate::data::*;
use crate::resources::PendingCompaction;
use crate::tests::harness::*;
use bevy::prelude::*;

/// Place a vertical I piece (rotation 1) in the right well (col 9, piece col 7)
/// with its top at row 16.
fn place_vertical_i_right_well(app: &mut App) {
    set_active_rot_col(app, 1, 7);
    set_active_position(app, 7, 16);
}

/// Lock the active piece (sonic drop to floor, then soft drop to lock immediately),
/// then tick through LineClearDelay into ARE so the next piece has spawned at the top.
fn lock_and_snap(mut app: App) -> String {
    press(&mut app, GameKey::SonicDrop); // drop to floor, enter Locking phase
                                         // SoftDrop while Locking → lock + line clear → LineClearDelay
    let soft_input = InputSnapshot {
        held: [GameKey::SoftDrop].iter().copied().collect(),
        just_pressed: std::collections::HashSet::new(),
    };
    tick_with(&mut app, soft_input);
    idle(&mut app, LINE_CLEAR_DELAY + 1 + SPAWN_DELAY_NORMAL + 1); // tick through LineClearDelay and ARE
    board_lines(&mut app, &[]).join("\n")
}

#[test]
fn line_clear_increments_level() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    app.world_mut()
        .resource_mut::<crate::resources::GameProgress>()
        .level = 50;
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1); // fire lock → 1 line cleared
    assert_eq!(level(&app), 51, "1 line clear should increment level 50→51");
}

#[test]
fn line_clear_passes_section_stop() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    app.world_mut()
        .resource_mut::<crate::resources::GameProgress>()
        .level = 99;
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1);
    assert_eq!(
        level(&app),
        100,
        "line clear should pass section stop 99→100"
    );
}

#[test]
fn level_clamped_to_999() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    app.world_mut()
        .resource_mut::<crate::resources::GameProgress>()
        .level = 998;
    setup_line_clear(&mut app, 4); // tetris: +4 would be 1002, clamped to 999
    idle(&mut app, 1);
    assert_eq!(level(&app), 999, "level should clamp to 999");
}

#[test]
fn line_clear_enters_line_clear_delay() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1); // fire lock + 1 line clear
                       // In the bevy port, line_clear_delay_system runs in the same update frame as
                       // active_phase_system (which fires the lock). So the observable ticks_left is
                       // LINE_CLEAR_DELAY - 1 = 39, not 40. This is expected behavior.
    assert!(
        matches!(
            piece_phase(&mut app),
            PiecePhase::LineClearDelay { ticks_left } if ticks_left == LINE_CLEAR_DELAY - 1
        ),
        "expected LineClearDelay{{ ticks_left: LINE_CLEAR_DELAY-1={} }}, got {:?}",
        LINE_CLEAR_DELAY - 1,
        piece_phase(&mut app)
    );
}

#[test]
fn line_clear_delay_transitions_to_are() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1); // fire lock → LineClearDelay
    idle(&mut app, LINE_CLEAR_DELAY + 1); // exhaust line clear delay → Spawning
                                          // In the bevy port, both line_clear_delay_system and spawning_system run in the
                                          // same update frame. When line_clear_delay fires the transition, spawning_system
                                          // also runs and decrements twice (once on transition tick, once on next).
                                          // Observable value is SPAWN_DELAY_NORMAL - 2 = 27.
    assert!(
        matches!(
            piece_phase(&mut app),
            PiecePhase::Spawning { ticks_left } if ticks_left == SPAWN_DELAY_NORMAL - 2
        ),
        "expected Spawning{{ ticks_left: SPAWN_DELAY_NORMAL-2={} }}, got {:?}",
        SPAWN_DELAY_NORMAL - 2,
        piece_phase(&mut app)
    );
}

#[test]
fn rows_pending_compaction_populated_during_delay() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1); // fire lock → LineClearDelay
    let pending = app.world().resource::<PendingCompaction>().0.clone();
    assert_eq!(
        pending,
        vec![BOARD_ROWS - 1],
        "cleared row index should be in PendingCompaction during LineClearDelay"
    );
}

#[test]
fn board_not_compacted_during_delay() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1); // fire lock → LineClearDelay
    let b = board(&mut app);
    assert!(
        b[BOARD_ROWS - 1].iter().all(|c| c.is_some()),
        "cleared row should still be present in board during LineClearDelay"
    );
}

#[test]
fn board_compacted_and_pending_cleared_after_delay() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1); // fire lock → LineClearDelay
    idle(&mut app, LINE_CLEAR_DELAY + 1); // exhaust delay → compaction → Spawning
    let pending = app.world().resource::<PendingCompaction>().0.clone();
    assert!(
        pending.is_empty(),
        "PendingCompaction should be empty after compaction"
    );
    let b = board(&mut app);
    assert!(
        !b.iter().any(|row| row.iter().all(|c| c.is_some())),
        "no row should be fully filled after compaction"
    );
}

#[test]
fn i_right_well_clears_4() {
    // All 4 rows filled left 9 cols; I piece fills col 9 on all 4 → tetris, board empty
    let mut app = make_app(PieceKind::I);
    set_board(
        &mut app,
        board_from_ascii(
            "
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
    ",
        ),
    );
    place_vertical_i_right_well(&mut app);
    insta::assert_snapshot!(lock_and_snap(app), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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
      │                    │
      │                    │
      │                    │
      │                    │
    20└────────────────────┘
    ");
}

#[test]
fn i_right_well_clears_top_3() {
    // Top 3 rows filled; bottom row empty → top 3 clear, stub at bottom
    let mut app = make_app(PieceKind::I);
    set_board(
        &mut app,
        board_from_ascii(
            "
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOO.OOOOO.
    ",
        ),
    );
    place_vertical_i_right_well(&mut app);
    insta::assert_snapshot!(lock_and_snap(app), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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
      │                    │
      │                    │
      │                    │
      │######  ############│
    20└────────────────────┘
    ");
}

#[test]
fn i_right_well_clears_bottom_3() {
    // Top row empty; bottom 3 rows filled → bottom 3 clear, stub at top
    let mut app = make_app(PieceKind::I);
    set_board(
        &mut app,
        board_from_ascii(
            "
        .OOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
    ",
        ),
    );
    place_vertical_i_right_well(&mut app);
    insta::assert_snapshot!(lock_and_snap(app), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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
      │                    │
      │                    │
      │                    │
      │  ##################│
    20└────────────────────┘
    ");
}

#[test]
fn i_right_well_clears_middle_2() {
    // Middle 2 rows filled; top and bottom empty → middle 2 clear, stubs at top and bottom
    let mut app = make_app(PieceKind::I);
    set_board(
        &mut app,
        board_from_ascii(
            "
        .OOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOO.OOOOO.
    ",
        ),
    );
    place_vertical_i_right_well(&mut app);
    insta::assert_snapshot!(lock_and_snap(app), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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
      │                    │
      │                    │
      │  ##################│
      │######  ############│
    20└────────────────────┘
    ");
}
