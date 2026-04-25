use crate::constants::{LOCK_DELAY, SPAWN_DELAY_NORMAL};
use crate::data::*;
use crate::tests::harness::*;

#[test]
fn lock_delay_prevents_immediate_lock() {
    let mut app = make_app(PieceKind::T);
    // Drop piece to floor manually
    drop_to_floor(&mut app);
    // Tick once — transitions from Falling to Locking { ticks_left: LOCK_DELAY }
    idle(&mut app, 1);
    assert!(
        matches!(piece_phase(&mut app), PiecePhase::Locking { .. }),
        "expected Locking, got {:?}",
        piece_phase(&mut app)
    );
    // LOCK_DELAY ticks decrement ticks_left to 0; one more tick fires the lock.
    idle(&mut app, LOCK_DELAY + 1);
    assert!(
        matches!(piece_phase(&mut app), PiecePhase::Spawning { .. }),
        "expected Spawning, got {:?}",
        piece_phase(&mut app)
    );
}

#[test]
fn sonic_drop_enters_lock_delay() {
    let mut app = make_app(PieceKind::T);
    press(&mut app, GameKey::SonicDrop);
    assert!(
        matches!(piece_phase(&mut app), PiecePhase::Locking { .. }),
        "expected Locking after sonic drop, got {:?}",
        piece_phase(&mut app)
    );
}

#[test]
fn rotation_buffer_applied_on_spawn() {
    let mut app = make_app(PieceKind::T);
    // Move piece to floor
    drop_to_floor(&mut app);
    idle(&mut app, 1); // enter Locking { ticks_left: LOCK_DELAY }
    idle(&mut app, LOCK_DELAY + 1); // lock → Spawning
    assert!(matches!(piece_phase(&mut app), PiecePhase::Spawning { .. }));
    // Hold rotate through all of ARE — IRS only fires if held at spawn.
    hold(&mut app, &[GameKey::RotateCw], SPAWN_DELAY_NORMAL + 1);
    assert_eq!(
        active_rotation(&mut app),
        1,
        "spawned piece should be rotated CW"
    );
}

#[test]
fn rotation_released_during_are_does_not_rotate() {
    let mut app = make_app(PieceKind::T);
    drop_to_floor(&mut app);
    idle(&mut app, 1);
    idle(&mut app, LOCK_DELAY + 1); // lock → Spawning
                                    // Tap rotate then release — should NOT trigger IRS.
    press(&mut app, GameKey::RotateCw);
    idle(&mut app, SPAWN_DELAY_NORMAL);
    assert_eq!(
        active_rotation(&mut app),
        0,
        "released key should not trigger IRS"
    );
}
