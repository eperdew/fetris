use crate::data::{GameMode, HiScoreEntry, Kind};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameConfigRes {
    pub game_mode: GameMode,
    pub rotation: Kind,
}

/// 4 slots: (Master,Ars), (Master,Srs), (TwentyG,Ars), (TwentyG,Srs).
#[derive(Resource, Default)]
pub struct HiScoresRes(pub [Vec<HiScoreEntry>; 4]);

#[derive(Resource, Default)]
pub struct MutedRes(pub bool);

pub fn slot_index(mode: GameMode, kind: Kind) -> usize {
    match (mode, kind) {
        (GameMode::Master, Kind::Ars) => 0,
        (GameMode::Master, Kind::Srs) => 1,
        (GameMode::TwentyG, Kind::Ars) => 2,
        (GameMode::TwentyG, Kind::Srs) => 3,
    }
}
