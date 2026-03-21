use std::collections::HashSet;

/// Renderer-agnostic held-trackable key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    Left,
    Right,
    RotateCw,
    RotateCcw,
    SoftDrop,
    SonicDrop,
}

/// Snapshot of input state for one tick.
/// `held`: keys currently held down.
/// `just_pressed`: keys that transitioned to pressed this tick (subset of held).
/// Both are HashSets — ordering within a 16ms tick is not meaningful.
pub struct InputState {
    pub held: HashSet<GameKey>,
    pub just_pressed: HashSet<GameKey>,
}

impl InputState {
    pub fn empty() -> Self {
        Self {
            held: HashSet::new(),
            just_pressed: HashSet::new(),
        }
    }
}
