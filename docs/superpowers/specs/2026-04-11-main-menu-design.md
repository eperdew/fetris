# Main Menu — Design Spec

Date: 2026-04-11

## Overview

Add a main menu screen that appears before a game starts and after game over. The menu lets the player choose a game mode, rotation system, view hi scores, and view controls before starting a game. Returning from a game over brings the player back to the menu.

This spec covers the menu stub — UI structure, navigation, and rendering. Actual logic for game mode, rotation system, hi scores, and controls is deferred to later work.

## Data Model (`src/menu.rs`)

```rust
pub enum GameMode { Master, TwentyG }
pub enum RotationSystem { Ars, Srs }

enum MenuScreen { Main, HiScores, Controls }

pub struct Menu {
    screen: MenuScreen,
    cursor: usize,       // index 0..=4, only used on MenuScreen::Main
    game_mode: GameMode,
    rotation: RotationSystem,
}

pub enum MenuResult {
    Stay,
    StartGame { mode: GameMode, rotation: RotationSystem },
}
```

### Main menu item indices

| Index | Label      | Behaviour                          |
|-------|------------|------------------------------------|
| 0     | GAME MODE  | Toggle: `Master` / `TwentyG`       |
| 1     | ROTATION   | Toggle: `Ars` / `Srs`              |
| 2     | HI SCORES  | Open `MenuScreen::HiScores`        |
| 3     | CONTROLS   | Open `MenuScreen::Controls`        |
| 4     | START      | Return `MenuResult::StartGame`     |

## Input & Tick (`Menu::tick`)

Uses raw `KeyCode` checks — the `GameKey` / DAS abstraction is not needed here.

### `MenuScreen::Main`

| Key(s)            | Action                                                        |
|-------------------|---------------------------------------------------------------|
| Up / K            | `cursor = cursor.saturating_sub(1)`                          |
| Down / J          | `cursor = (cursor + 1).min(4)`                               |
| Left / H          | Items 0–1: cycle toggle option backwards; others: no-op      |
| Right / L         | Items 0–1: cycle toggle option forwards; others: no-op       |
| Space / Enter     | Items 2–3: open sub-screen; item 4: return `StartGame`; others: no-op |
| Escape / Backspace| No-op                                                         |

### `MenuScreen::HiScores` / `MenuScreen::Controls`

| Key(s)             | Action                  |
|--------------------|-------------------------|
| Escape / Backspace | Return to `MenuScreen::Main` (cursor position preserved) |

All other keys: no-op.

`Menu::tick` returns `MenuResult::Stay` in all cases except when START is activated.

## Rendering (`renderer::render_menu`)

Signature: `pub fn render_menu(menu: &Menu)`  
No texture parameter — added when there's a concrete reason.

### Main screen layout

Vertically and horizontally centered. Labels are static (not cursor targets); cursor targets are the value/action lines below them. Blank lines separate groups.

```
GAME MODE
< MASTER >

ROTATION
< ARS >

HI SCORES
CONTROLS

START
```

The example above shows the cursor on GAME MODE (index 0). Rules:

- Toggle items (0–1): label on one line, current value on the next.
  - Cursor here: `< MASTER >` / `< 20G >` or `< ARS >` / `< SRS >`
  - Cursor elsewhere: `  MASTER  ` / `  20G  ` etc. (padded, no brackets)
- Action items (2–4): label only.
  - Cursor here: `< HI SCORES >` / `< CONTROLS >` / `< START >`
  - Cursor elsewhere: `  HI SCORES  ` etc.
- Groups: `{GAME MODE, ROTATION}`, `{HI SCORES, CONTROLS}`, `{START}` — blank line between each group.

### Sub-screen layout

Stub only. Centered on screen:
- Line 1: sub-screen name (`HI SCORES` or `CONTROLS`)
- Line 2: `ESC / BKSP to go back`

## Main Loop Changes (`src/main.rs`)

Add:

```rust
enum AppState {
    Menu(Menu),
    Playing(Game),
}
```

Initial state: `AppState::Menu(Menu::new())`.

**Each frame:**

- `AppState::Menu(menu)`: read raw key state, call `menu.tick()`, match result:
  - `Stay` → no transition
  - `StartGame { .. }` → transition to `AppState::Playing(Game::new())`
  - Render via `renderer::render_menu(&menu)`
- `AppState::Playing(game)`: existing 60 Hz tick loop unchanged. After ticking: if `game.game_over` and Space just pressed → transition to `AppState::Menu(Menu::new())`.
  - Render via `renderer::render(&game, &cell_texture)`

Escape exits the process from either state.

## Testing Plan

1. **Unit tests** — `Menu::tick` logic: cursor clamping, toggle cycling, sub-screen transitions, `StartGame` result on START activation.
2. **User acceptance testing (UAT)** — Run the game and visually verify:
   - Menu renders correctly on launch
   - Cursor moves and clamps at top/bottom
   - Toggle items cycle correctly with left/right
   - HI SCORES and CONTROLS open their stub sub-screens
   - ESC/Backspace returns from sub-screens
   - START transitions to gameplay
   - Game over + Space returns to menu
   - Iterate on layout and rendering before wiring up game logic
