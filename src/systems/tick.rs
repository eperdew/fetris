use crate::resources::{CurrentPhase, GameProgress, TickStartPhase};
use bevy::prelude::*;

/// Increments ticks_elapsed once per FixedUpdate. Skipped if game ended.
/// Also captures the start-of-tick phase into `TickStartPhase` so that
/// downstream phase systems gate on the phase that was active at the
/// *start* of this tick (matching master's "one phase per tick" semantics).
pub fn tick_counter(
    mut progress: ResMut<GameProgress>,
    phase: Res<CurrentPhase>,
    mut start: ResMut<TickStartPhase>,
) {
    if progress.game_over || progress.game_won {
        start.0 = None;
        return;
    }
    start.0 = Some(phase.0);
    progress.ticks_elapsed += 1;
}
