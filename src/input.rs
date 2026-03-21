use std::collections::HashSet;
use crossterm::event::KeyCode;

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

/// Maps a KeyCode to a GameKey. Returns None for unrecognised keys.
pub fn map_game_key(code: KeyCode) -> Option<GameKey> {
    match code {
        KeyCode::Left | KeyCode::Char('h')  => Some(GameKey::Left),
        KeyCode::Right | KeyCode::Char('l') => Some(GameKey::Right),
        KeyCode::Down | KeyCode::Char('j')  => Some(GameKey::SoftDrop),
        KeyCode::Char(' ')                  => Some(GameKey::SonicDrop),
        KeyCode::Char('x')                  => Some(GameKey::RotateCw),
        KeyCode::Char('z')                  => Some(GameKey::RotateCcw),
        _ => None,
    }
}
