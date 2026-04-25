use bevy::prelude::*;
use crate::resources::GameProgress;

/// Increments ticks_elapsed once per FixedUpdate. Skipped if game ended.
pub fn tick_counter(mut progress: ResMut<GameProgress>) {
    if progress.game_over || progress.game_won {
        return;
    }
    progress.ticks_elapsed += 1;
}
