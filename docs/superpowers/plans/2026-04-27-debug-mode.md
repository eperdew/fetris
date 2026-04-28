# Debug Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a hidden DEBUG menu entry (revealed by pressing D on the main menu) that opens a visual test bench for inspecting line-clear particles, line-clear text overlays, state-text overlays (READY / GAME OVER / LEVEL 999), and HUD presets, driven entirely by synthetic inputs into the live render pipeline.

**Architecture:** A new `AppState::Debug` peer to `Playing` / `Ready` / `GameOver`. Render systems already gated to `Playing`/`Ready`/`GameOver` are broadened to include `Debug`. Game-logic systems stay strictly gameplay-only. Effects are triggered by emitting the same `GameEvent::LineClear` events the live game emits (plus, for state-text, a debug-only override branch in `render_state_text`). A `DebugSceneState` resource holds tick countdowns and the active HUD preset.

**Tech Stack:** Rust, Bevy 0.18, bevy_egui, insta (inline snapshots).

---

## File Structure

**Created:**
- `src/menu/debug.rs` — debug screen system, `DebugSceneState` resource, enter/exit/input systems.
- `src/tests/debug_tests.rs` — headless test for the debug screen.

**Modified:**
- `src/app_state.rs` — add `AppState::Debug` variant.
- `src/data.rs` — add `MenuScreen::Debug` variant.
- `src/menu/state.rs` — add `debug_unlocked: bool` to `MenuState`.
- `src/menu/main_screen.rs` — D-key unlock + DEBUG row + transition to `AppState::Debug`.
- `src/menu/mod.rs` — register `debug` submodule + register debug systems.
- `src/render/mod.rs` — broaden `run_if` gates on `render_active_piece`, `render_next_preview`, `render_hud`, `spawn_particles_on_line_clear`, `spawn_line_clear_overlay`, `render_state_text` to also include `AppState::Debug`.
- `src/render/overlays.rs` — `render_state_text` gains a debug-overlay branch.
- `src/main.rs` — register Debug-state OnEnter/OnExit handlers.
- `src/tests/mod.rs` — register `debug_tests`.

**Untouched:** `src/systems/*` (game-logic systems stay gameplay-only).

---

## Task 1: Add `AppState::Debug` variant

**Files:**
- Modify: `src/app_state.rs`

- [ ] **Step 1: Add the variant**

```rust
// src/app_state.rs
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Ready,
    Playing,
    GameOver,
    Debug,
}
```

- [ ] **Step 2: Verify the project still builds**

Run: `cargo build`
Expected: success, no warnings.

- [ ] **Step 3: Commit**

```bash
git add src/app_state.rs
git commit -m "feat(debug): add AppState::Debug variant"
```

---

## Task 2: Add `MenuScreen::Debug` and `MenuState::debug_unlocked`

**Files:**
- Modify: `src/data.rs`
- Modify: `src/menu/state.rs`

- [ ] **Step 1: Add `MenuScreen::Debug`**

In `src/data.rs` find the `MenuScreen` enum. (It currently lives in `src/menu/state.rs` — confirm location with `grep -rn "enum MenuScreen" src/`.) Wherever it lives, add the variant:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuScreen {
    Main,
    HiScores,
    Controls,
    Debug,
}
```

- [ ] **Step 2: Add `debug_unlocked` to `MenuState`**

In `src/menu/state.rs`:

```rust
#[derive(Resource)]
pub struct MenuState {
    pub screen: MenuScreen,
    pub cursor: usize,
    pub game_mode: GameMode,
    pub rotation: Kind,
    pub hi_scores_tab: usize,
    pub debug_unlocked: bool,
}
```

Update both `MenuState::new` and `Default for MenuState` to initialize `debug_unlocked: false`.

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git add src/data.rs src/menu/state.rs
git commit -m "feat(debug): add MenuScreen::Debug and MenuState::debug_unlocked"
```

---

## Task 3: D-key unlock + DEBUG row on main menu

**Files:**
- Modify: `src/menu/main_screen.rs`

- [ ] **Step 1: Add `D` to `MenuInput`**

```rust
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub back: bool,
    pub unlock_debug: bool,
}

pub fn read_input(keys: &ButtonInput<KeyCode>) -> MenuInput {
    MenuInput {
        up: keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyK),
        down: keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyJ),
        left: keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyH),
        right: keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyL),
        confirm: keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter),
        back: keys.just_pressed(KeyCode::Backspace),
        unlock_debug: keys.just_pressed(KeyCode::KeyD),
    }
}
```

- [ ] **Step 2: Wire D to set `debug_unlocked`**

In `main_menu_system`, just after `let input = read_input(&keys);`:

```rust
    if input.unlock_debug {
        menu.debug_unlocked = true;
    }
```

- [ ] **Step 3: Adjust cursor cap**

Change `menu.cursor = (menu.cursor + 1).min(4);` to:

```rust
    let cursor_max = if menu.debug_unlocked { 5 } else { 4 };
    if input.down {
        menu.cursor = (menu.cursor + 1).min(cursor_max);
    }
```

(Make sure to remove the original `if input.down { ... }` line — the snippet replaces it.)

- [ ] **Step 4: Update the match on cursor for confirm**

Replace the `match menu.cursor { ... }` block with a version that handles cursor 4 conditionally and adds 5 = START:

```rust
    let mut start_game = false;
    let mut enter_debug = false;
    match menu.cursor {
        0 if input.left || input.right => {
            menu.game_mode = match menu.game_mode {
                GameMode::Master => GameMode::TwentyG,
                GameMode::TwentyG => GameMode::Master,
            };
        }
        1 if input.left || input.right => {
            menu.rotation = match menu.rotation {
                Kind::Ars => Kind::Srs,
                Kind::Srs => Kind::Ars,
            };
        }
        2 if input.confirm => {
            menu.hi_scores_tab = match (menu.game_mode, menu.rotation) {
                (GameMode::Master, Kind::Ars) => 0,
                (GameMode::Master, Kind::Srs) => 1,
                (GameMode::TwentyG, Kind::Ars) => 2,
                (GameMode::TwentyG, Kind::Srs) => 3,
            };
            menu.screen = MenuScreen::HiScores;
        }
        3 if input.confirm => {
            menu.screen = MenuScreen::Controls;
        }
        4 if input.confirm => {
            if menu.debug_unlocked {
                enter_debug = true;
            } else {
                start_game = true;
            }
        }
        5 if input.confirm => {
            start_game = true;
        }
        _ => {}
    }
```

- [ ] **Step 5: Render the DEBUG row when unlocked**

Inside the egui layout block, just before the existing `ui.label(make_bracketed("START", menu.cursor == 4, 24.0));` line:

- Replace `menu.cursor == 4` (the START selector) with: `menu.cursor == if menu.debug_unlocked { 5 } else { 4 }`.
- Insert a DEBUG row immediately above START, only when unlocked:

```rust
                if menu.debug_unlocked {
                    ui.label(make_bracketed("DEBUG", menu.cursor == 4, 24.0));
                }
                ui.add_space(20.0);
                let start_idx = if menu.debug_unlocked { 5 } else { 4 };
                ui.label(make_bracketed("START", menu.cursor == start_idx, 24.0));
```

(Apply this in place of the existing `ui.add_space(20.0); ui.label(make_bracketed("START", ...));` pair.)

- [ ] **Step 6: Wire `enter_debug` to the state transition**

After the existing `if start_game { ... }` block at the bottom of `main_menu_system`, add:

```rust
    if enter_debug {
        next_state.set(crate::app_state::AppState::Debug);
    }
```

- [ ] **Step 7: Build**

Run: `cargo build`
Expected: success.

- [ ] **Step 8: Commit**

```bash
git add src/menu/main_screen.rs
git commit -m "feat(debug): hide DEBUG menu row behind D-key unlock"
```

---

## Task 4: Create the debug module skeleton + register

**Files:**
- Create: `src/menu/debug.rs`
- Modify: `src/menu/mod.rs`

- [ ] **Step 1: Create `src/menu/debug.rs` with the resource and stub systems**

```rust
//! Debug visual test bench.
//!
//! Reuses the live render pipeline by populating the same resources gameplay
//! uses, then driving effects via `GameEvent::LineClear` events and a
//! debug-only `state_overlay` override on `render_state_text`.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::components::ActivePieceBundle;
use crate::data::{GameMode, Kind, PieceKind, PiecePhase};
use crate::judge::Judge;
use crate::resources::{
    Board, CurrentPhase, GameModeRes, GameProgress, NextPiece, PendingCompaction, RotationKind,
    RotationSystemRes,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum DebugStateOverlay {
    #[default]
    None,
    Ready,
    GameOver,
    Won,
}

#[derive(Resource, Default)]
pub struct DebugSceneState {
    pub hud_preset: usize,
    pub state_overlay: DebugStateOverlay,
    pub state_overlay_ticks_left: u32,
    pub line_clear_cleanup_ticks_left: u32,
}

/// Six presets covering Grade 9 → S9 with matching score/level.
/// Score values land at the lower bound of each grade band.
pub(crate) const HUD_PRESETS: &[(u32, u32)] = &[
    (0, 0),        // Grade 9
    (1700, 250),   // Grade 6
    (8500, 500),   // Grade 2
    (16000, 700),  // Grade S1
    (52000, 850),  // Grade S5
    (120000, 999), // Grade S9
];

pub(crate) fn apply_hud_preset(judge: &mut Judge, progress: &mut GameProgress, idx: usize) {
    let (score, level) = HUD_PRESETS[idx % HUD_PRESETS.len()];
    *judge = Judge::new();
    judge.set_score_for_debug(score);
    progress.level = level;
    progress.lines = level / 10;
    progress.ticks_elapsed = (level as u64) * 60;
    progress.initial_delay_ticks = 0;
    progress.game_over = false;
    progress.game_won = false;
}

pub fn on_enter_debug(world: &mut World) {
    // Insert resources gameplay normally inserts via `start_game`.
    world.insert_resource(RotationSystemRes(Kind::Ars.create()));
    world.insert_resource(GameModeRes(GameMode::Master));
    world.insert_resource(RotationKind(Kind::Ars));
    world.insert_resource(NextPiece(PieceKind::T));
    world.insert_resource(CurrentPhase(PiecePhase::Falling));
    world.insert_resource(DebugSceneState::default());

    // Despawn any prior ActivePiece, then spawn a static T at mid-board.
    let prior: Vec<Entity> = world
        .query::<(Entity, &crate::components::ActivePiece)>()
        .iter(world)
        .map(|(e, _)| e)
        .collect();
    for e in prior {
        world.despawn(e);
    }
    let mut bundle = ActivePieceBundle::new(PieceKind::T);
    bundle.position.row = 8;
    bundle.position.col = 4;
    world.spawn(bundle);

    // Apply preset 0 to HUD. resource_scope releases the borrow on Judge so
    // we can mutably grab GameProgress in the same call.
    world.resource_scope::<Judge, _>(|world, mut judge| {
        let mut progress = world.resource_mut::<GameProgress>();
        apply_hud_preset(&mut judge, &mut progress, 0);
    });
}

pub fn debug_input_system(
    _keys: Res<ButtonInput<KeyCode>>,
    _scene: ResMut<DebugSceneState>,
    _board: ResMut<Board>,
    _pending: ResMut<PendingCompaction>,
    _next_state: ResMut<NextState<AppState>>,
) {
    // Filled in by Tasks 6, 7, 8.
}

pub fn debug_tick_system(_scene: ResMut<DebugSceneState>) {
    // Filled in by Tasks 7, 9 (cleanup countdowns).
}
```

NOTE: `Judge::set_score_for_debug` is added in the next step.

- [ ] **Step 2: Add `Judge::set_score_for_debug` helper**

Inside `impl Judge` in `src/judge.rs` (it has fields `combo`, `score`, `best_grade`, `grade_ticks`):

```rust
    /// Debug helper: directly set the score and re-derive the best grade.
    /// `score()` already derives the live grade from `score`, but `best_grade`
    /// drives the grade-bar color in the HUD, so set it consistently.
    pub fn set_score_for_debug(&mut self, score: u32) {
        self.score = score;
        self.best_grade = crate::data::Grade::of_score(score);
    }
```

- [ ] **Step 3: Register the debug submodule**

In `src/menu/mod.rs`:

```rust
pub mod controls;
pub mod debug;
pub mod hi_scores;
pub mod main_screen;
pub mod state;
```

- [ ] **Step 4: Wire OnEnter into `main.rs`**

Add in `main()`, near the other `OnEnter` handlers:

```rust
        .add_systems(OnEnter(AppState::Debug), menu::debug::on_enter_debug)
```

(`OnExit` is not needed: leaving Debug routes through `AppState::Menu`, and `reset_game_on_enter_menu` already despawns all render entities. Verify: when the user hits Backspace in Debug, the next state will be `Menu`.)

- [ ] **Step 5: Wire input + tick systems into `main.rs`**

```rust
        .add_systems(
            Update,
            menu::debug::debug_input_system.run_if(in_state(AppState::Debug)),
        )
        .add_systems(
            FixedUpdate,
            menu::debug::debug_tick_system.run_if(in_state(AppState::Debug)),
        )
```

- [ ] **Step 6: Build**

Run: `cargo build && cargo test --lib`
Expected: build succeeds, no test regressions.

- [ ] **Step 7: Commit**

```bash
git add src/menu/debug.rs src/menu/mod.rs src/main.rs src/judge.rs
git commit -m "feat(debug): add DebugSceneState resource and OnEnter scaffold"
```

---

## Task 5: Broaden render-system gates to include `AppState::Debug`

**Files:**
- Modify: `src/render/mod.rs`

- [ ] **Step 1: Update each affected `run_if`**

In `RenderPlugin::build`, change the gates as follows. (Each shown as a before/after snippet.)

`render_board` — already covers `Playing|GameOver|Ready`, just add `Debug`:

```rust
        app.add_systems(
            Update,
            board::render_board.run_if(
                in_state(AppState::Playing)
                    .or(in_state(AppState::GameOver))
                    .or(in_state(AppState::Ready))
                    .or(in_state(AppState::Debug)),
            ),
        );
```

`piece::render_active_piece` + `piece::render_next_preview`:

```rust
        app.add_systems(
            Update,
            (piece::render_active_piece, piece::render_next_preview).run_if(
                in_state(AppState::Playing)
                    .or(in_state(AppState::GameOver))
                    .or(in_state(AppState::Debug)),
            ),
        );
```

`hud::render_hud`:

```rust
        app.add_systems(
            Update,
            hud::render_hud.run_if(
                in_state(AppState::Playing)
                    .or(in_state(AppState::GameOver))
                    .or(in_state(AppState::Debug)),
            ),
        );
```

`particles::spawn_particles_on_line_clear`:

```rust
        app.add_systems(
            FixedUpdate,
            particles::spawn_particles_on_line_clear
                .after(active_phase_system)
                .before(line_clear_delay_system)
                .run_if(in_state(AppState::Playing).or(in_state(AppState::Debug))),
        );
```

`overlays::spawn_line_clear_overlay` + `overlays::render_state_text`:

```rust
        app.add_systems(
            Update,
            (
                overlays::spawn_line_clear_overlay,
                overlays::render_state_text,
            )
                .run_if(
                    in_state(AppState::Playing)
                        .or(in_state(AppState::Ready))
                        .or(in_state(AppState::GameOver))
                        .or(in_state(AppState::Debug)),
                ),
        );
```

(`update_particles` and `tick_line_clear_overlay` are already ungated — leave them alone.)

- [ ] **Step 2: Build**

Run: `cargo build`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add src/render/mod.rs
git commit -m "feat(debug): broaden render-system gates to AppState::Debug"
```

---

## Task 6: Backspace exits Debug back to Menu

**Files:**
- Modify: `src/menu/debug.rs`

- [ ] **Step 1: Wire Backspace in `debug_input_system`**

Replace the placeholder body:

```rust
pub fn debug_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut next_state: ResMut<NextState<AppState>>,
    mut menu: ResMut<crate::menu::state::MenuState>,
) {
    if keys.just_pressed(KeyCode::Backspace) {
        menu.screen = crate::menu::state::MenuScreen::Main;
        next_state.set(AppState::Menu);
        return;
    }
    let _ = (scene, board, pending);
}
```

(Keep the unused-bindings line; it suppresses warnings until Tasks 7/8/9 wire them in.)

- [ ] **Step 2: Manual smoke test**

Run: `cargo run --release`
Steps:
1. Press D on the main menu — DEBUG row appears.
2. Arrow-down to DEBUG, press Enter — board appears with a static T-piece, HUD around it.
3. Press Backspace — back to main menu.

Expected: each step works; no crashes; the T-piece is visibly in the middle of the board.

- [ ] **Step 3: Commit**

```bash
git add src/menu/debug.rs
git commit -m "feat(debug): backspace exits debug screen"
```

---

## Task 7: Line-clear trigger keys (1/2/3/4)

**Files:**
- Modify: `src/menu/debug.rs`

- [ ] **Step 1: Populate synthetic rows + emit `LineClear`**

Extend `debug_input_system` (replace its body, keeping the Backspace handler):

```rust
pub fn debug_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut events: bevy::ecs::message::MessageWriter<crate::data::GameEvent>,
    mut next_state: ResMut<NextState<AppState>>,
    mut menu: ResMut<crate::menu::state::MenuState>,
) {
    if keys.just_pressed(KeyCode::Backspace) {
        menu.screen = crate::menu::state::MenuScreen::Main;
        next_state.set(AppState::Menu);
        return;
    }

    let count = if keys.just_pressed(KeyCode::Digit1) {
        Some(1u32)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(3)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(4)
    } else {
        None
    };

    if let Some(count) = count {
        // Don't stack triggers while a previous burst is still cleaning up.
        if scene.line_clear_cleanup_ticks_left > 0 {
            return;
        }
        let n = count as usize;
        let rows: Vec<usize> = (crate::data::BOARD_ROWS - n..crate::data::BOARD_ROWS).collect();
        for &r in &rows {
            for c in 0..crate::data::BOARD_COLS {
                board.0[r][c] = Some(PieceKind::T);
            }
        }
        pending.0 = rows;
        events.write(crate::data::GameEvent::LineClear { count });
        // Hold synthetic state long enough for both particle spawn (FixedUpdate)
        // and overlay spawn (Update) to read it. 3 ticks = 50ms is ample.
        scene.line_clear_cleanup_ticks_left = 3;
    }
}
```

- [ ] **Step 2: Wire cleanup into `debug_tick_system`**

```rust
pub fn debug_tick_system(
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
) {
    if scene.line_clear_cleanup_ticks_left > 0 {
        scene.line_clear_cleanup_ticks_left -= 1;
        if scene.line_clear_cleanup_ticks_left == 0 {
            // Clear the synthetic rows so the next press starts fresh.
            for r in &pending.0 {
                for c in 0..crate::data::BOARD_COLS {
                    board.0[*r][c] = None;
                }
            }
            pending.0.clear();
        }
    }
}
```

- [ ] **Step 3: Manual smoke test**

Run: `cargo run --release`
Steps:
1. Open the debug screen.
2. Press 1, 2, 3, 4 in sequence (wait ~1s between each).

Expected: each press fills the bottom N rows with T-color cells, plays the corresponding particle burst, and (for 2/3/4) shows DOUBLE/TRIPLE/FETRIS overlay text. After ~3 ticks the synthetic rows disappear so the next press starts on a clean board.

- [ ] **Step 4: Commit**

```bash
git add src/menu/debug.rs
git commit -m "feat(debug): trigger line-clear bursts via 1/2/3/4 keys"
```

---

## Task 8: HUD preset cycling (Up/Down)

**Files:**
- Modify: `src/menu/debug.rs`

`debug_input_system` is a regular system, so it can hold `ResMut<Judge>` and `ResMut<GameProgress>` simultaneously and apply the preset directly — no exclusive system needed.

- [ ] **Step 1: Add Judge + GameProgress to `debug_input_system`'s signature and apply on arrow press**

Update the signature to include `mut judge: ResMut<Judge>, mut progress: ResMut<GameProgress>`:

```rust
pub fn debug_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut events: bevy::ecs::message::MessageWriter<crate::data::GameEvent>,
    mut judge: ResMut<Judge>,
    mut progress: ResMut<GameProgress>,
    mut next_state: ResMut<NextState<AppState>>,
    mut menu: ResMut<crate::menu::state::MenuState>,
) {
```

After the `Backspace` block and before the `count` block, add:

```rust
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowUp) {
        let delta: i32 = if keys.just_pressed(KeyCode::ArrowDown) { 1 } else { -1 };
        let len = HUD_PRESETS.len() as i32;
        let new_idx = (scene.hud_preset as i32 + delta).rem_euclid(len) as usize;
        scene.hud_preset = new_idx;
        apply_hud_preset(&mut judge, &mut progress, new_idx);
    }
```

- [ ] **Step 2: Build and run smoke test**

Run: `cargo build`
Expected: success.

Run: `cargo run --release`. Open Debug screen, press Down/Up. Expected: HUD score, level, grade, grade-bar fill, and background tint cycle through six visibly distinct presets.

- [ ] **Step 3: Commit**

```bash
git add src/menu/debug.rs
git commit -m "feat(debug): cycle HUD presets with up/down arrows"
```

---

## Task 9: State-text overlay override (READY / GAME OVER / LEVEL 999)

**Files:**
- Modify: `src/render/overlays.rs`
- Modify: `src/menu/debug.rs`

- [ ] **Step 1: Add the debug branch to `render_state_text`**

In `src/render/overlays.rs`, modify `render_state_text` to take an additional optional resource and short-circuit on Debug:

```rust
pub fn render_state_text(
    mut commands: Commands,
    existing: Query<Entity, With<StateText>>,
    state: Res<State<AppState>>,
    progress: Res<crate::resources::GameProgress>,
    assets: Res<GameAssets>,
    debug_scene: Option<Res<crate::menu::debug::DebugSceneState>>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }
    let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
    let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;

    let mk = |commands: &mut Commands, text: String, dy: f32, size: f32, color: Color| {
        commands.spawn((
            StateText,
            Text2d::new(text),
            TextFont {
                font: assets.font.clone(),
                font_size: size,
                ..default()
            },
            TextColor(color),
            bevy::sprite::Anchor::CENTER,
            Transform::from_xyz(cx, cy + dy, 150.0).with_scale(Vec3::new(1.0, -1.0, 1.0)),
        ));
    };

    // In Debug, the override flag drives the overlay; ignore AppState.
    if matches!(state.get(), AppState::Debug) {
        use crate::menu::debug::DebugStateOverlay;
        if let Some(scene) = debug_scene {
            match scene.state_overlay {
                DebugStateOverlay::Ready => {
                    mk(&mut commands, "READY".into(), 0.0, 28.0, Color::WHITE);
                }
                DebugStateOverlay::GameOver => {
                    mk(&mut commands, "GAME OVER".into(), 0.0, 28.0, Color::WHITE);
                }
                DebugStateOverlay::Won => {
                    mk(&mut commands, "LEVEL 999".into(), -16.0, 28.0, Color::WHITE);
                    mk(
                        &mut commands,
                        crate::render::hud::format_time(progress.ticks_elapsed),
                        20.0,
                        22.0,
                        Color::srgba(0.83, 0.83, 0.83, 1.0),
                    );
                }
                DebugStateOverlay::None => {}
            }
        }
        return;
    }

    match state.get() {
        AppState::Ready => {
            mk(&mut commands, "READY".into(), 0.0, 28.0, Color::WHITE);
        }
        AppState::Playing if progress.initial_delay_ticks > 0 => {
            mk(&mut commands, "READY".into(), 0.0, 28.0, Color::WHITE);
        }
        AppState::GameOver if progress.game_won => {
            mk(&mut commands, "LEVEL 999".into(), -16.0, 28.0, Color::WHITE);
            mk(
                &mut commands,
                crate::render::hud::format_time(progress.ticks_elapsed),
                20.0,
                22.0,
                Color::srgba(0.83, 0.83, 0.83, 1.0),
            );
        }
        AppState::GameOver => {
            mk(&mut commands, "GAME OVER".into(), 0.0, 28.0, Color::WHITE);
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Wire Q / W / R triggers + countdown in `debug.rs`**

In `debug_input_system`, after the arrow-key block and before the `count` block, add:

```rust
    if keys.just_pressed(KeyCode::KeyQ) {
        scene.state_overlay = DebugStateOverlay::Ready;
        scene.state_overlay_ticks_left = 90;
    } else if keys.just_pressed(KeyCode::KeyW) {
        scene.state_overlay = DebugStateOverlay::GameOver;
        scene.state_overlay_ticks_left = 90;
    } else if keys.just_pressed(KeyCode::KeyR) {
        scene.state_overlay = DebugStateOverlay::Won;
        scene.state_overlay_ticks_left = 90;
    }
```

- [ ] **Step 3: Tick countdown in `debug_tick_system`**

```rust
    if scene.state_overlay_ticks_left > 0 {
        scene.state_overlay_ticks_left -= 1;
        if scene.state_overlay_ticks_left == 0 {
            scene.state_overlay = DebugStateOverlay::None;
        }
    }
```

- [ ] **Step 4: Manual smoke test**

Run: `cargo run --release`. Open debug screen. Press Q → "READY" appears for 1.5s, then disappears. Press W → "GAME OVER" appears, disappears. Press R → "LEVEL 999" + time, disappears.

- [ ] **Step 5: Commit**

```bash
git add src/render/overlays.rs src/menu/debug.rs
git commit -m "feat(debug): trigger state-text overlays via Q/W/R"
```

---

## Task 10: Keymap label panel (egui)

**Files:**
- Modify: `src/menu/debug.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add the egui panel system**

```rust
use bevy_egui::{egui, EguiContexts};

pub fn debug_keymap_panel(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::SidePanel::right("debug_keymap")
        .resizable(false)
        .default_width(220.0)
        .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(10, 10, 18, 220)))
        .show(ctx, |ui| {
            ui.add_space(12.0);
            ui.label(egui::RichText::new("DEBUG").color(egui::Color32::WHITE).size(20.0));
            ui.add_space(8.0);
            let row = |ui: &mut egui::Ui, k: &str, what: &str| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(k).color(egui::Color32::from_rgb(180, 180, 220)).size(14.0));
                    ui.label(egui::RichText::new(what).color(egui::Color32::GRAY).size(14.0));
                });
            };
            row(ui, "1 / 2 / 3 / 4", "line-clear bursts");
            row(ui, "Q", "READY");
            row(ui, "W", "GAME OVER");
            row(ui, "R", "LEVEL 999 win");
            row(ui, "↑ / ↓", "cycle HUD preset");
            row(ui, "Backspace", "back to menu");
        });
}
```

- [ ] **Step 2: Register in `main.rs`**

```rust
        .add_systems(
            bevy_egui::EguiPrimaryContextPass,
            menu::debug::debug_keymap_panel.run_if(in_state(AppState::Debug)),
        )
```

- [ ] **Step 3: Manual smoke test**

Run: `cargo run --release`. Open debug screen → keymap panel visible on the right side, showing all 6 rows. Buttons still work as before.

- [ ] **Step 4: Commit**

```bash
git add src/menu/debug.rs src/main.rs
git commit -m "feat(debug): add keymap side panel"
```

---

## Task 11: Headless test for debug entry + line-clear trigger

**Files:**
- Create: `src/tests/debug_tests.rs`
- Modify: `src/tests/mod.rs`

- [ ] **Step 1: Add the test file**

```rust
// src/tests/debug_tests.rs
use crate::app_state::AppState;
use crate::components::{ActivePiece, PieceKindComp};
use crate::data::{GameEvent, PieceKind};
use crate::menu::debug::DebugSceneState;
use crate::resources::{Board, NextPiece};
use bevy::ecs::message::Messages;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

fn debug_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_message::<GameEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::resources::Board>()
        .init_resource::<crate::resources::CurrentPhase>()
        .init_resource::<crate::resources::GameProgress>()
        .init_resource::<crate::resources::PendingCompaction>()
        .init_resource::<crate::judge::Judge>()
        .init_resource::<crate::menu::state::MenuState>()
        .add_systems(OnEnter(AppState::Debug), crate::menu::debug::on_enter_debug)
        .add_systems(
            Update,
            crate::menu::debug::debug_input_system.run_if(in_state(AppState::Debug)),
        )
        .add_systems(
            FixedUpdate,
            crate::menu::debug::debug_tick_system.run_if(in_state(AppState::Debug)),
        );
    app
}

fn enter_debug(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Debug);
    app.update(); // run state transition + OnEnter
}

#[test]
fn entering_debug_spawns_t_piece() {
    let mut app = debug_app();
    enter_debug(&mut app);
    let world = app.world_mut();
    let mut q = world.query_filtered::<&PieceKindComp, With<ActivePiece>>();
    let kind = q.single(world).expect("ActivePiece").0;
    assert_eq!(kind, PieceKind::T);
    assert_eq!(world.resource::<NextPiece>().0, PieceKind::T);
    assert!(world.contains_resource::<DebugSceneState>());
}

#[test]
fn digit4_emits_fetris_line_clear() {
    let mut app = debug_app();
    enter_debug(&mut app);
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Digit4);
    }
    app.update(); // run debug_input_system
    let world = app.world_mut();
    let messages = world.resource::<Messages<GameEvent>>();
    let mut reader = messages.get_cursor();
    let evs: Vec<_> = reader.read(messages).copied().collect();
    assert_eq!(evs, vec![GameEvent::LineClear { count: 4 }]);
    // Synthetic rows are populated for the cleanup window.
    let board = &world.resource::<Board>().0;
    let bottom = crate::data::BOARD_ROWS - 1;
    assert!(board[bottom][0].is_some());
}

#[test]
fn backspace_returns_to_menu() {
    let mut app = debug_app();
    enter_debug(&mut app);
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Backspace);
    }
    app.update();
    app.update(); // let NextState resolve
    assert_eq!(*app.world().resource::<State<AppState>>().get(), AppState::Menu);
}
```

- [ ] **Step 2: Register the module**

In `src/tests/mod.rs`:

```rust
mod debug_tests;
```

(Place it alphabetically with the other `mod ...` lines.)

- [ ] **Step 3: Run the tests**

Run: `cargo test --lib debug_tests`
Expected: 3 tests pass.

- [ ] **Step 4: Run the full test suite + WASM build to check for regressions**

Run: `cargo test`
Expected: all tests pass, no warnings.

Run: `cargo build --target wasm32-unknown-unknown`
Expected: success, no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/tests/debug_tests.rs src/tests/mod.rs
git commit -m "test(debug): cover entry, digit4 trigger, backspace exit"
```

---

## Task 12: Update `CLAUDE.md` source-layout map

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add `src/menu/debug.rs` to the file map**

In the source-layout table (under "Source layout"), add a row to the `src/menu/` entry mentioning `debug` alongside `main_screen`, `hi_scores`, `controls`, `state`. Update the `AppState` line under "Architecture" to mention `Debug` as a peer.

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(claude-md): note AppState::Debug + menu/debug.rs"
```

---

## Final verification

- [ ] `cargo build` — clean
- [ ] `cargo build --target wasm32-unknown-unknown` — clean
- [ ] `cargo test` — all green
- [ ] Manual: D unlocks DEBUG row, DEBUG menu opens, T-piece visible, 1/2/3/4 trigger particles + (for 2/3/4) DOUBLE/TRIPLE/FETRIS overlays, Q/W/R show state-text overlays, ↑/↓ cycle HUD presets, Backspace returns to main menu, leaving and re-entering Menu does not crash, restarting the game re-locks DEBUG.
