use crate::data::*;
use crate::tests::harness::*;

#[test]
fn gravity_g_lookup() {
    use crate::constants::gravity_g;
    let gravity_g = |level| gravity_g(GameMode::Master, level);
    assert_eq!(gravity_g(0), 4, "level 0 → 4 G/256");
    assert_eq!(gravity_g(29), 4, "level 29 → still 4 G/256");
    assert_eq!(gravity_g(30), 6, "level 30 → 6 G/256");
    assert_eq!(gravity_g(199), 144, "level 199 → 144 G/256");
    assert_eq!(gravity_g(200), 4, "level 200 → resets to 4 G/256");
    assert_eq!(gravity_g(251), 256, "level 251 → 256 G/256 (1G)");
    assert_eq!(gravity_g(500), 5120, "level 500 → 5120 G/256 (20G)");
}

#[test]
fn soft_drop_on_floor_locks_immediately() {
    let mut app = make_app(PieceKind::T);
    // Drop to floor and enter locking state
    drop_to_floor(&mut app);
    idle(&mut app, 1); // enter Locking
                       // Soft drop bypasses lock delay
    press(&mut app, GameKey::SoftDrop);
    assert!(
        matches!(piece_phase(&mut app), PiecePhase::Spawning { .. }),
        "expected Spawning after soft drop on floor, got {:?}",
        piece_phase(&mut app)
    );
}

#[test]
fn lock_timer_resets_when_gravity_drops_piece() {
    // Set up a piece one row above the floor with a partially-spent lock timer.
    // A single 20G tick should drop it, re-land it, and reset the timer.
    let mut app = make_app(PieceKind::T);
    app.world_mut()
        .resource_mut::<crate::resources::GameProgress>()
        .level = 500; // 20G
    drop_to_floor(&mut app);
    // lift 1 row so there's room to drop
    let pos = active_position(&mut app);
    set_active_position(&mut app, pos.col, pos.row - 1);
    // Set partially-spent lock timer
    app.world_mut()
        .resource_mut::<crate::resources::CurrentPhase>()
        .0 = PiecePhase::Locking { ticks_left: 10 };

    let row = active_position(&mut app).row;
    let phase = piece_phase(&mut app);
    insta::assert_snapshot!(
        format!("row={} phase={:?}", row, phase),
        @"row=16 phase=Locking { ticks_left: 10 }"
    );
    idle(&mut app, 1);
    let row = active_position(&mut app).row;
    let phase = piece_phase(&mut app);
    insta::assert_snapshot!(
        format!("row={} phase={:?}", row, phase),
        @"row=17 phase=Locking { ticks_left: 29 }"
    );
}
