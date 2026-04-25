use crate::data::*;
use crate::tests::harness::*;

#[test]
fn left_press_moves_one_column() {
    let mut app = make_app(PieceKind::T);
    let col_before = active_position(&mut app).col;
    press(&mut app, GameKey::Left);
    assert_eq!(active_position(&mut app).col, col_before - 1);
}
