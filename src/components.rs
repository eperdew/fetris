use crate::data::PieceKind;
use bevy::prelude::*;

/// Marker for the single active piece entity.
#[derive(Component, Debug)]
pub struct ActivePiece;

#[derive(Component, Debug, Clone, Copy)]
pub struct PieceKindComp(pub PieceKind);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PiecePosition {
    pub col: i32,
    pub row: i32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceRotation(pub usize);

/// All four components needed for the active piece, bundled for spawn convenience.
#[derive(Bundle)]
pub struct ActivePieceBundle {
    pub marker: ActivePiece,
    pub kind: PieceKindComp,
    pub position: PiecePosition,
    pub rotation: PieceRotation,
}

impl ActivePieceBundle {
    pub fn new(kind: PieceKind) -> Self {
        Self {
            marker: ActivePiece,
            kind: PieceKindComp(kind),
            position: PiecePosition { col: 3, row: 0 },
            rotation: PieceRotation(0),
        }
    }
}

/// Convert ECS components into the value type used by RotationSystem.
impl PiecePosition {
    pub fn to_state(self, kind: PieceKind, rotation: usize) -> crate::rotation_system::PieceState {
        crate::rotation_system::PieceState {
            kind,
            rotation,
            col: self.col,
            row: self.row,
        }
    }
}
