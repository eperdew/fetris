use crate::data::*;
use crate::tests::harness::*;

#[test]
fn left_press_moves_one_column() {
    let mut app = make_app(PieceKind::T);
    let col_before = active_position(&mut app).col;
    press(&mut app, GameKey::Left);
    assert_eq!(active_position(&mut app).col, col_before - 1);
}

#[test]
fn o_piece_move_left() {
    insta::assert_snapshot!(movement_snap(PieceKind::O, GameKey::Left), @"
               1                          2                          3                          4            
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │      [][]'.        │     │    [][]'.          │     │  [][]'.            │     │[][]'.              │
    10│- - - [][]'.- - - - │   10│- - [][]'.- - - - - │   10│- [][]'.- - - - - - │   10│[][]'.- - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │   15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘   20└────────────────────┘   20└────────────────────┘
    ");
}

#[test]
fn o_piece_move_right() {
    insta::assert_snapshot!(movement_snap(PieceKind::O, GameKey::Right), @"
               1                          2                          3                          4            
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │        '.[][]      │     │          '.[][]    │     │            '.[][]  │     │              '.[][]│
    10│- - - - '.[][]- - - │   10│- - - - - '.[][]- - │   10│- - - - - - '.[][]- │   10│- - - - - - - '.[][]│
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │   15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘   20└────────────────────┘   20└────────────────────┘
    ");
}

#[test]
fn das_activates_after_charge() {
    use crate::constants::DAS_CHARGE;
    let mut app = make_app(PieceKind::T);
    let start_col = active_position(&mut app).col;
    // First press moves immediately
    press(&mut app, GameKey::Left);
    assert_eq!(
        active_position(&mut app).col,
        start_col - 1,
        "expected immediate move on press"
    );
    // Hold for DAS_CHARGE - 1 ticks: no additional movement (counter not yet at charge)
    hold(&mut app, &[GameKey::Left], DAS_CHARGE - 1);
    assert_eq!(
        active_position(&mut app).col,
        start_col - 1,
        "no movement before DAS charge"
    );
    // One more tick triggers first auto-repeat
    hold(&mut app, &[GameKey::Left], 1);
    assert_eq!(
        active_position(&mut app).col,
        start_col - 2,
        "first auto-repeat after DAS charge"
    );
}

#[test]
fn das_repeats_every_tick_after_charge() {
    use crate::constants::DAS_CHARGE;
    let mut app = make_app(PieceKind::T);
    // Start further right so we can move 5 columns left
    let row = active_position(&mut app).row;
    set_active_position(&mut app, 8, row);
    let start_col = active_position(&mut app).col;
    press(&mut app, GameKey::Left); // immediate: start_col - 1
    hold(&mut app, &[GameKey::Left], DAS_CHARGE); // first auto-repeat at charge: start_col - 2
    hold(&mut app, &[GameKey::Left], 3); // 3 more repeats (DAS_REPEAT=1): start_col - 5
    assert_eq!(
        active_position(&mut app).col,
        start_col - 5,
        "DAS should repeat every tick after charge"
    );
}
