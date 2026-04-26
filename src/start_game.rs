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
    // Draw only the first piece upfront; spawning_system draws subsequent pieces.
    // Starting in Spawning{0} means spawning_system fires on the first live tick,
    // which applies IRS and initial gravity exactly like every later spawn.
    let first_kind = randomizer.next();

    world.insert_resource(RotationSystemRes(opts.rotation.create()));
    world.insert_resource(GameModeRes(opts.mode));
    world.insert_resource(RotationKind(opts.rotation));
    world.insert_resource(NextPiece(first_kind));
    world.insert_resource(randomizer);
    world.insert_resource(Board::default());
    world.insert_resource(CurrentPhase(PiecePhase::Spawning { ticks_left: 0 }));
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

    // Entity kind is a placeholder; spawning_system overwrites it on the first live tick.
    world.spawn(ActivePieceBundle::new(first_kind));

    // Transition into Playing.
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
}
