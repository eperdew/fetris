use crate::constants::{LOCK_DELAY, SPAWN_DELAY_NORMAL};
use crate::data::*;
use crate::resources::GameProgress;
use crate::systems::spawning::can_piece_increment;
use crate::tests::harness::*;

#[test]
fn can_piece_increment_section_stops() {
    assert!(!can_piece_increment(99), "99 is section stop");
    assert!(!can_piece_increment(199), "199 is section stop");
    assert!(!can_piece_increment(899), "899 is section stop");
    assert!(!can_piece_increment(998), "998 is final stop");
    assert!(can_piece_increment(0), "0 is not a stop");
    assert!(can_piece_increment(100), "100 is not a stop");
    assert!(can_piece_increment(500), "500 is not a stop");
}

#[test]
fn level_starts_at_zero() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    assert_eq!(level(&app), 0);
}

#[test]
fn level_increments_on_piece_spawn() {
    let mut app = make_app(PieceKind::T);
    app.world_mut().resource_mut::<GameProgress>().level = 50;
    drop_to_floor(&mut app);
    idle(&mut app, 1); // enter Locking{LOCK_DELAY}
    idle(&mut app, LOCK_DELAY + 1); // fire lock → Spawning{SPAWN_DELAY_NORMAL}
    idle(&mut app, SPAWN_DELAY_NORMAL + 1); // complete ARE → spawn_piece called
    assert_eq!(
        level(&app),
        51,
        "level should increment from 50 to 51 on spawn"
    );
}

#[test]
fn section_stop_blocks_piece_increment() {
    let mut app = make_app(PieceKind::T);
    app.world_mut().resource_mut::<GameProgress>().level = 99;
    drop_to_floor(&mut app);
    idle(&mut app, 1);
    idle(&mut app, LOCK_DELAY + 1);
    idle(&mut app, SPAWN_DELAY_NORMAL + 1);
    assert_eq!(
        level(&app),
        99,
        "section stop: level should remain 99 after spawn"
    );
}

#[test]
fn game_won_on_reaching_999() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    app.world_mut().resource_mut::<GameProgress>().level = 998;
    setup_line_clear(&mut app, 1); // +1 = 999
    idle(&mut app, 1);
    assert!(
        game_won(&app),
        "game_won should be set when level reaches 999"
    );
}

#[test]
fn ticks_elapsed_increments_each_tick() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    idle(&mut app, 5);
    assert_eq!(ticks_elapsed(&app), 5);
}

#[test]
fn ticks_elapsed_stops_after_win() {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, PieceKind::T);
    app.world_mut().resource_mut::<GameProgress>().level = 998;
    setup_line_clear(&mut app, 1);
    idle(&mut app, 1); // fires win
    let frozen = ticks_elapsed(&app);
    idle(&mut app, 10);
    assert_eq!(
        ticks_elapsed(&app),
        frozen,
        "ticks_elapsed should freeze after win"
    );
}

#[test]
fn normal_are_uses_spawn_delay_normal() {
    let mut app = make_app(PieceKind::T);
    drop_to_floor(&mut app);
    idle(&mut app, 1); // enter Locking
    idle(&mut app, LOCK_DELAY + 1); // fire lock (no lines cleared)
                                    // In the bevy port, spawning_system runs in the same update frame as
                                    // active_phase_system (which fires the lock). So the observable ticks_left
                                    // is SPAWN_DELAY_NORMAL - 1 = 28, not 29. This is expected behavior.
    assert!(
        matches!(
            piece_phase(&mut app),
            PiecePhase::Spawning { ticks_left } if ticks_left == SPAWN_DELAY_NORMAL - 1
        ),
        "expected Spawning{{ ticks_left: SPAWN_DELAY_NORMAL-1={} }}, got {:?}",
        SPAWN_DELAY_NORMAL - 1,
        piece_phase(&mut app)
    );
}
