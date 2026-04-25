use crate::app_state::AppState;
use crate::components::ActivePieceBundle;
use crate::data::{GameMode, Kind, PiecePhase};
use crate::judge::Judge;
use crate::randomizer::Randomizer;
use crate::resources::*;
use bevy::prelude::*;

pub struct StartGameOptions {
    pub mode: GameMode,
    pub rotation: Kind,
    pub seed: Option<u64>,
}

pub fn start_game(world: &mut World, opts: StartGameOptions) {
    let mut randomizer = match opts.seed {
        Some(s) => Randomizer::with_seed(s),
        None => Randomizer::new(),
    };
    let active_kind = randomizer.next();
    let next_kind = randomizer.next();

    world.insert_resource(RotationSystemRes(opts.rotation.create()));
    world.insert_resource(GameModeRes(opts.mode));
    world.insert_resource(RotationKind(opts.rotation));
    world.insert_resource(NextPiece(next_kind));
    world.insert_resource(randomizer);
    world.insert_resource(Board::default());
    world.insert_resource(CurrentPhase(PiecePhase::Falling));
    world.insert_resource(GameProgress::default());
    world.insert_resource(DasState::default());
    world.insert_resource(RotationBuffer::default());
    world.insert_resource(PendingCompaction::default());
    world.insert_resource(DropTracking::default());
    world.insert_resource(InputState::default());
    world.insert_resource(Judge::new());

    // Despawn any prior ActivePiece entity.
    let prior: Vec<Entity> = world
        .query::<(Entity, &crate::components::ActivePiece)>()
        .iter(world)
        .map(|(e, _)| e)
        .collect();
    for e in prior {
        world.despawn(e);
    }

    world.spawn(ActivePieceBundle::new(active_kind));

    // Transition into Playing.
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
}
