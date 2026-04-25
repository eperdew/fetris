# Bevy Migration Plan 2: Rendering & Menu

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the bevy port playable end-to-end on native by adding rendering and the menu UI.

**Architecture:** Switch from `MinimalPlugins` to `DefaultPlugins` (+ `bevy_egui`). Render systems read snapshot data from the `Board` resource and active-piece entity components, gated by `AppState`. Menu uses `bevy_egui` immediate-mode UI driven by an egui-side `Menu` resource. Particles become entities with `Particle` + `Lifetime` components. Stub resources stand in for hi-scores and config persistence — Plan 3 wires those to `bevy_pkv`.

**Tech Stack:** bevy (`Sprite`, `Text2d`, `bevy::color::Color`), `bevy_egui`, the existing assets in `assets/font/`.

**Reads spec:** [docs/superpowers/specs/2026-04-25-bevy-migration-design.md](../specs/2026-04-25-bevy-migration-design.md). This plan covers spec phases 4–5 ("Renderer" + "Menu").

**Builds on:** [Plan 1](2026-04-25-bevy-migration-1-logic.md). All Plan 1 tests must remain green throughout.

---

## Module-path translation table

Plan 1 splits types across `data.rs`, `components.rs`, `resources.rs`, `judge.rs`. The code blocks in this plan reference items by their Plan-1 home. Use this table when editing:

| Plan 1 module | Items used in Plan 2 |
|---|---|
| `crate::data` | `PieceKind`, `BOARD_COLS`, `BOARD_ROWS`, `BoardGrid`, `Grade`, `GameMode`, `Kind`, `HiScoreEntry`, `GameConfig`, `GameEvent`, `JudgeEvent`, `MenuScreen` |
| `crate::components` | `ActivePiece`, `PieceKindComp`, `PiecePosition`, `PieceRotation` |
| `crate::resources` | `Board` (newtype: `Board(pub BoardGrid)` — index as `board.0[r][c]`), `CurrentPhase`, `NextPiece`, `GameProgress`, `PendingCompaction`, `RotationSystemRes`, `GameModeRes`, `RotationKind`, `InputState`, `DasState`, `RotationBuffer`, `DropTracking` |
| `crate::judge` | `Judge` |
| `crate::randomizer` | `Randomizer` |

**Event note:** Plan 1 emits `GameEvent::LineClear { count: u32 }` — there is no `GameEvent::LineClear` and the event does **not** carry row indices. When Plan 2 needs row indices (particles, board renderer skip-list), read them from `Res<PendingCompaction>` (`pending.0: Vec<usize>`), which `lock_piece` populates and `line_clear_delay` consumes. Verify timing: rows must still be in `PendingCompaction` at the moment particles spawn (i.e., particle spawner must run before `line_clear_delay` clears them, or copy out the indices on event arrival).

If during execution you discover the event needs to carry rows after all (e.g., you can't get the timing right via `PendingCompaction`), add `rows: Vec<usize>` to `GameEvent::LineClear` in `data.rs` and adjust `lock_piece` to populate it. That's a one-line widening; not worth a separate task.

---

## Pre-flight

**Worktree:** continue working in `.worktrees/bevy-migration` from Plan 1.

**Branch:** continue on the branch created in Plan 1.

**Verify Plan 1 status before starting:**

```bash
cd .worktrees/bevy-migration
cargo test
cargo build
```

Both must succeed. Tests must all pass. If not, finish Plan 1 first.

**Out of scope for this plan:**
- WASM build (Plan 3)
- Persistent storage / bevy_pkv (Plan 3) — use in-memory stub resources
- Audio playback (deferred entirely; no audio in the bevy port. The spec deletes `audio_player.rs`. Audio events are still emitted by game systems but no system consumes them.)
- The CRT-scanline shader on the line-clear overlay. Plan 2 renders the overlay as plain text with the same lifetime/opacity semantics. The shader can be revisited later if missed; the gameplay-critical part (the label text + hue cycling + frame-parity scanlines) is not load-bearing for correctness.

**Visual reference:** the existing renderer at [src/renderer.rs](../../../src/renderer.rs) is the source of truth for layout numbers, colors, and behavior. When porting any visual element, open `src/renderer.rs` from `master` and copy the pixel offsets / color values verbatim. Do not invent new layout.

**Color note:** `macroquad::Color` is `Color { r, g, b, a: f32 }` in 0.0–1.0. In bevy use `bevy::color::Color::srgb(r, g, b)` / `Color::srgba(r, g, b, a)` directly. `Color::from_rgba(220, 50, 50, 255)` in macroquad becomes `Color::srgba_u8(220, 50, 50, 255)` in bevy.

**Coordinate system:** bevy 2D uses centered, Y-up coordinates by default. The macroquad code uses top-left-origin Y-down with the window at 560×780. To minimize translation cost, **use a custom `Camera2d` configured to top-left origin Y-down** (set the projection's viewport origin) so the existing `BOARD_X`, `BOARD_Y`, `CELL`, etc. constants port verbatim. Set this up once in Task 1.

---

## File Structure (post-Plan-2)

```
src/
├── main.rs                   # App setup; DefaultPlugins; bevy_egui; AppState transitions
├── constants.rs              # unchanged
├── types.rs                  # unchanged from Plan 1
├── rotation_system.rs        # unchanged from Plan 1
├── randomizer.rs             # unchanged from Plan 1
├── judge.rs                  # unchanged from Plan 1
├── systems/                  # unchanged from Plan 1
│   ├── mod.rs
│   ├── input.rs
│   ├── gravity.rs
│   ├── lock.rs
│   ├── line_clear.rs
│   ├── spawn.rs
│   ├── judge.rs
│   └── game_over_check.rs
├── render/
│   ├── mod.rs                # plugin registration; shared layout consts; cell-sprite helper
│   ├── board.rs              # board background + locked cells
│   ├── piece.rs              # active piece + ghost + next preview + ready-screen preview
│   ├── particles.rs          # particle entity spawn from GameEvent::LineClear + particle update
│   ├── overlays.rs           # line-clear label overlay; game-over / game-won text; READY text
│   ├── hud.rs                # sidebar (level/lines/time/score/grade/next) + grade bar + bg color
│   └── assets.rs             # font + cell-texture handle resource
├── menu/
│   ├── mod.rs                # plugin; egui contexts; AppState gating
│   ├── state.rs              # `MenuState` resource (cursor, screen, selection, hi-scores tab)
│   ├── main_screen.rs        # main menu egui system
│   ├── hi_scores.rs          # hi-scores egui system
│   └── controls.rs           # controls egui system
├── stub_storage.rs           # in-memory resources standing in for hi-scores + config + mute. Plan 3 deletes this.
└── tests.rs                  # unchanged from Plan 1
```

---

## Task 1: Switch to DefaultPlugins and configure window

**Files:**
- Modify: `src/main.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add bevy_egui to Cargo.toml**

Open `Cargo.toml` and add to `[dependencies]`:

```toml
bevy_egui = "0.36"
```

(Pin to whatever version is current at start of work. Do not bump during the migration.)

- [ ] **Step 2: Run cargo build to fetch the dependency**

```bash
cargo build
```

Expected: build succeeds (will recompile bevy with extra features, may take a minute).

- [ ] **Step 3: Switch from MinimalPlugins to DefaultPlugins; configure window**

Open `src/main.rs`. Replace the `MinimalPlugins` registration (added in Plan 1) with `DefaultPlugins` configured for the fetris window:

```rust
use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin, WindowResolution};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "fetris".into(),
                resolution: WindowResolution::new(560.0, 780.0),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy_egui::EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        // ... existing Plan 1 plugins (game_logic, states, fixed timestep, etc.)
        .run();
}
```

Keep the `Time::<Fixed>::from_hz(60.0)` configuration from Plan 1.

- [ ] **Step 4: Spawn a Camera2d with top-left-origin Y-down**

Add a startup system in `src/main.rs`:

```rust
fn setup_camera(mut commands: Commands) {
    use bevy::render::camera::{OrthographicProjection, ScalingMode};
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed { width: 560.0, height: 780.0 };
    projection.viewport_origin = Vec2::new(0.0, 1.0); // top-left origin
    commands.spawn((
        Camera2d,
        Projection::Orthographic(projection),
        // Flip Y by negating scale, so Y grows downward like macroquad.
        Transform::from_scale(Vec3::new(1.0, -1.0, 1.0)),
    ));
}
```

Register: `.add_systems(Startup, setup_camera)`.

- [ ] **Step 5: Run native build**

```bash
cargo run
```

Expected: a 560×780 window titled "fetris" opens with a black background. Close it with Cmd+Q. (Game logic from Plan 1 ticks but nothing visible yet.)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "feat(render): switch to DefaultPlugins; add window + camera"
```

---

## Task 2: Stub storage resources

**Files:**
- Create: `src/stub_storage.rs`
- Modify: `src/main.rs`

These resources stand in for the eventual `bevy_pkv` integration. Plan 3 deletes this file and wires the same data to a real store.

- [ ] **Step 1: Create stub storage module**

Create `src/stub_storage.rs`:

```rust
use bevy::prelude::*;
use crate::data::{GameMode, HiScoreEntry, Kind};

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
```

If `GameMode` and `Kind` don't already implement `Default`, add `#[derive(Default)]` and pick `Master` / `Ars` as the default variants (`#[default]` attribute).

- [ ] **Step 2: Register in main.rs**

In `src/main.rs`, add `mod stub_storage;` and:

```rust
.init_resource::<stub_storage::GameConfigRes>()
.init_resource::<stub_storage::HiScoresRes>()
.init_resource::<stub_storage::MutedRes>()
```

- [ ] **Step 3: Build**

```bash
cargo build
```

Expected: succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/stub_storage.rs src/main.rs src/types.rs
git commit -m "feat(render): stub in-memory resources for hi-scores and config"
```

---

## Task 3: Render module scaffold + cell sprite helper

**Files:**
- Create: `src/render/mod.rs`
- Create: `src/render/assets.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create render/assets.rs (font + cell texture handle)**

Create `src/render/assets.rs`:

```rust
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameAssets {
    pub font: Handle<Font>,
    pub cell_texture: Handle<Image>,
}

pub fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let font = asset_server.load("font/Oxanium-Regular.ttf");
    let cell_texture = images.add(make_cell_image());
    commands.insert_resource(GameAssets { font, cell_texture });
}

/// Mirrors `make_cell_texture` in src/renderer.rs:786-808.
fn make_cell_image() -> Image {
    use bevy::render::render_asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    const SIZE: u32 = 32;
    let mut pixels = vec![255u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let fy = y as f32 / (SIZE - 1) as f32;
            let raw = if x == 0 || y == 0 { 1.0 } else { 1.0 - 0.4 * fy };
            let quantized = (raw * 16.0).floor() / 16.0;
            let v = (quantized * 255.0) as u8;
            let i = ((y * SIZE + x) * 4) as usize;
            pixels[i] = v;
            pixels[i + 1] = v;
            pixels[i + 2] = v;
            // alpha already 255
        }
    }
    Image::new(
        Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
```

**Asset path note:** bevy's `AssetServer` reads from the `assets/` directory at the workspace root by default. The font already lives at `assets/font/Oxanium-Regular.ttf`. If the path resolves to nothing, verify with `ls assets/font/`.

- [ ] **Step 2: Create render/mod.rs with shared constants and plugin shell**

Create `src/render/mod.rs`:

```rust
use bevy::prelude::*;

pub mod assets;
pub mod board;
pub mod piece;
pub mod particles;
pub mod overlays;
pub mod hud;

// Layout constants — copied verbatim from src/renderer.rs:117-130.
pub const CELL: f32 = 32.0;
pub const INSET: f32 = 2.0;
pub const PAD: f32 = 20.0;
pub const BOARD_X: f32 = PAD;
pub const BOARD_Y: f32 = 2.0 * CELL + 2.0 * PAD;
pub const BAR_WIDTH: f32 = 24.0;
pub const BAR_LEFT_GAP: f32 = 24.0;
pub const BAR_RIGHT_GAP: f32 = 14.0;
pub const BAR_X: f32 = BOARD_X + crate::data::BOARD_COLS as f32 * CELL + BAR_LEFT_GAP;
pub const SIDEBAR_X: f32 = BAR_X + BAR_WIDTH + BAR_RIGHT_GAP;
pub const DIVIDER_X: f32 = BOARD_X + crate::data::BOARD_COLS as f32 * CELL + BAR_LEFT_GAP / 2.0;
pub const WINDOW_W: f32 = 560.0;
pub const WINDOW_H: f32 = 780.0;
pub const BOARD_BG: Color = Color::srgba(0.06, 0.06, 0.10, 1.0);

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, assets::load_assets);
        // Subsequent tasks add Update systems here.
    }
}

/// Color for a piece kind. Mirrors src/renderer.rs:888-898.
pub fn piece_color(kind: crate::data::PieceKind) -> Color {
    use crate::data::PieceKind;
    match kind {
        PieceKind::I => Color::srgba_u8(200, 50, 50, 255),
        PieceKind::O => Color::srgba_u8(220, 200, 0, 255),
        PieceKind::T => Color::srgba_u8(0, 200, 200, 255),
        PieceKind::S => Color::srgba_u8(200, 0, 200, 255),
        PieceKind::Z => Color::srgba_u8(0, 160, 0, 255),
        PieceKind::J => Color::srgba_u8(50, 100, 220, 255),
        PieceKind::L => Color::srgba_u8(255, 150, 100, 255),
    }
}

/// Spawn a sprite for a single CELL×CELL block at top-left pixel (x, y).
/// Y is screen-down; sprite anchor is top-left so this matches macroquad's draw_cell_at.
pub fn cell_sprite(x: f32, y: f32, color: Color, texture: Handle<Image>, z: f32) -> impl Bundle {
    (
        Sprite {
            image: texture,
            color,
            custom_size: Some(Vec2::new(CELL - INSET * 2.0, CELL - INSET * 2.0)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(x + INSET, y + INSET, z),
    )
}
```

- [ ] **Step 3: Empty stub files for downstream tasks**

Create empty stub files so `mod` declarations compile:

```bash
for f in board piece particles overlays hud; do
  echo "use bevy::prelude::*;" > src/render/$f.rs
done
```

- [ ] **Step 4: Register the plugin**

In `src/main.rs`:

```rust
mod render;
// ...
.add_plugins(render::RenderPlugin)
```

Remove the Plan 1 `setup_camera` system if it lives in main; or keep it and skip — either way the camera spawn happens once. Keep it in main.rs for now.

- [ ] **Step 5: Build**

```bash
cargo run
```

Expected: window still opens, still empty. Asset loads succeed (no panic).

- [ ] **Step 6: Commit**

```bash
git add src/render/ src/main.rs
git commit -m "feat(render): scaffold render module + cell sprite helper"
```

---

## Task 4: Render board background and locked cells

**Files:**
- Modify: `src/render/board.rs`
- Modify: `src/render/mod.rs`

Board cells are despawned-and-respawned each frame for simplicity (tens of cells; trivial). A marker component tags them for cleanup.

- [ ] **Step 1: Replace src/render/board.rs**

```rust
use bevy::prelude::*;
use crate::render::{
    BOARD_BG, BOARD_X, BOARD_Y, CELL, INSET, cell_sprite, piece_color,
};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_COLS, BOARD_ROWS, PieceKind};
use crate::resources::{Board, PendingCompaction};

#[derive(Component)]
pub struct BoardSprite;

pub fn render_board(
    mut commands: Commands,
    existing: Query<Entity, With<BoardSprite>>,
    board: Res<Board>,
    pending: Res<PendingCompaction>,
    assets: Res<GameAssets>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }

    // Background.
    commands.spawn((
        BoardSprite,
        Sprite {
            color: BOARD_BG,
            custom_size: Some(Vec2::new(BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(BOARD_X, BOARD_Y, 0.0),
    ));

    // Locked cells (skip rows pending compaction — those become particles in Task 7).
    for r in 0..BOARD_ROWS {
        if pending.0.contains(&r) {
            continue;
        }
        for c in 0..BOARD_COLS {
            // Board is `pub struct Board(pub BoardGrid)` — index via `board.0`.
            if let Some(kind) = board.0[r][c] {
                let left = c == 0 || board.0[r][c - 1].is_none();
                let top = r == 0 || board.0[r - 1][c].is_none();
                let right = c == BOARD_COLS - 1 || board.0[r][c + 1].is_none();
                let bottom = r == BOARD_ROWS - 1 || board.0[r + 1][c].is_none();
                spawn_bordered_cell(
                    &mut commands, &assets, c as i32, r as i32, piece_color(kind),
                    left, top, right, bottom,
                );
            }
        }
    }

    // Top dim overlay.
    commands.spawn((
        BoardSprite,
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.1),
            custom_size: Some(Vec2::new(BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(BOARD_X, BOARD_Y, 100.0),
    ));
}

fn spawn_bordered_cell(
    commands: &mut Commands,
    assets: &GameAssets,
    col: i32,
    row: i32,
    color: Color,
    left: bool,
    top: bool,
    right: bool,
    bottom: bool,
) {
    const BORDER: Color = Color::srgba(0.70, 0.70, 0.70, 1.0);
    let x = BOARD_X + col as f32 * CELL;
    let y = BOARD_Y + row as f32 * CELL;
    let mk_strip = |x: f32, y: f32, w: f32, h: f32| -> (BoardSprite, Sprite, Transform) {
        (
            BoardSprite,
            Sprite {
                color: BORDER,
                custom_size: Some(Vec2::new(w, h)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            Transform::from_xyz(x, y, 1.0),
        )
    };
    if left {  commands.spawn(mk_strip(x, y, INSET, CELL)); }
    if top {   commands.spawn(mk_strip(x, y, CELL, INSET)); }
    if right { commands.spawn(mk_strip(x + CELL - INSET, y, INSET, CELL)); }
    if bottom { commands.spawn(mk_strip(x, y + CELL - INSET, CELL, INSET)); }

    let (sprite, transform) = match cell_sprite(x, y, color, assets.cell_texture.clone(), 2.0) {
        bundle => {
            // Decompose: cell_sprite returns (Sprite, Transform). Re-add BoardSprite marker.
            // bevy 0.18: just spawn the bundle plus the marker.
            (bundle, ())
        }
    };
    let _ = (sprite, transform);
    commands.spawn((BoardSprite, cell_sprite(x, y, color, assets.cell_texture.clone(), 2.0)));
}
```

**Compile note:** the awkward `let (sprite, transform) = match ... { bundle => ... }` block above is a copy-paste artifact — delete it. The actual spawn is the final line: `commands.spawn((BoardSprite, cell_sprite(...)))`.

- [ ] **Step 2: Verify `PendingCompaction` resource exists from Plan 1**

Plan 1 introduces `pub struct PendingCompaction(pub Vec<usize>)` in `src/resources.rs`, populated by `lock_piece` and consumed by `line_clear_delay`. The board renderer reads it to skip rows mid-clear (those become particles). No new types needed here — just confirm the resource exists.

- [ ] **Step 3: Register the system**

In `src/render/mod.rs::RenderPlugin::build`:

```rust
app.add_systems(
    Update,
    board::render_board.run_if(in_state(crate::AppState::Playing)
        .or(in_state(crate::AppState::GameOver))
        .or(in_state(crate::AppState::Ready))),
);
```

- [ ] **Step 4: Run**

```bash
cargo run
```

Expected: window opens; game logic ticks (you won't see the active piece yet because Task 5 adds that), but if you hold Space to sonic-drop, eventually pieces lock to the board and you'll see locked cells appear at the bottom. (The menu doesn't exist yet either — when Task 13 lands you'll start in the menu. For now the game starts directly in `Playing`. Verify by temporarily setting `AppState`'s default to `Playing` if needed.)

- [ ] **Step 5: Commit**

```bash
git add src/render/ src/main.rs
git commit -m "feat(render): board background and locked cells"
```

---

## Task 5: Render active piece, ghost piece, and next preview

**Files:**
- Modify: `src/render/piece.rs`
- Modify: `src/render/mod.rs`

- [ ] **Step 1: Replace src/render/piece.rs**

```rust
use bevy::prelude::*;
use crate::render::{BOARD_X, BOARD_Y, CELL, PAD, cell_sprite, piece_color};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_COLS, BOARD_ROWS, PieceKind};
use crate::resources::{Board, NextPiece, RotationSystemRes};
use crate::components::{ActivePiece, PieceKindComp, PiecePosition, PieceRotation};

#[derive(Component)]
pub struct PieceSprite;

pub fn render_active_piece(
    mut commands: Commands,
    existing: Query<Entity, With<PieceSprite>>,
    active: Query<(&PieceKindComp, &PiecePosition, &PieceRotation), With<ActivePiece>>,
    rotation_system: Res<RotationSystemRes>,
    assets: Res<GameAssets>,
    board: Res<Board>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }

    let Ok((kind_comp, pos, rot)) = active.single() else { return; };
    let kind = kind_comp.0;
    // RotationSystemRes is `pub struct RotationSystemRes(pub Box<dyn RotationSystem>)`.
    // Adapt the method name (`shape`/`cells`/`offsets`) to whatever Plan 1's trait actually exposes.
    let cells = rotation_system.0.shape(kind, rot.0);

    // Ghost: drop the piece as far as it goes given current board occupancy.
    let mut ghost_row = pos.row;
    while can_place(&board.0, &cells, pos.col, ghost_row + 1) {
        ghost_row += 1;
    }
    if ghost_row != pos.row {
        let base = piece_color(kind);
        let ghost_color = Color::srgba(base.to_srgba().red, base.to_srgba().green, base.to_srgba().blue, 0.25);
        for &(dc, dr) in cells {
            let c = pos.col + dc;
            let r = ghost_row + dr;
            if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                spawn_cell_sprite(&mut commands, &assets, c, r, ghost_color, 3.0);
            }
        }
    }

    // Active piece on top.
    let color = piece_color(kind);
    for &(dc, dr) in cells {
        let c = pos.col + dc;
        let r = pos.row + dr;
        if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
            spawn_cell_sprite(&mut commands, &assets, c, r, color, 4.0);
        }
    }
}

pub fn render_next_preview(
    mut commands: Commands,
    existing: Query<Entity, With<NextPreviewSprite>>,
    next: Res<NextPiece>,
    rotation_system: Res<RotationSystemRes>,
    assets: Res<GameAssets>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }
    let kind = next.0;
    let cells = rotation_system.0.shape(kind, 0);
    // Y offset to vertically center the preview above the board (mirrors src/renderer.rs:317-335).
    let preview_y_offset = next_preview_y_offset(kind);
    let color = piece_color(kind);
    for &(dc, dr) in cells {
        let c = 3 + dc;
        let r = -3 + dr + preview_y_offset;
        let x = BOARD_X + c as f32 * CELL;
        let y = (BOARD_Y - PAD) + r as f32 * CELL;
        commands.spawn((
            NextPreviewSprite,
            cell_sprite(x, y, color, assets.cell_texture.clone(), 5.0),
        ));
    }
}

#[derive(Component)]
pub struct NextPreviewSprite;

fn spawn_cell_sprite(commands: &mut Commands, assets: &GameAssets, col: i32, row: i32, color: Color, z: f32) {
    let x = BOARD_X + col as f32 * CELL;
    let y = BOARD_Y + row as f32 * CELL;
    commands.spawn((PieceSprite, cell_sprite(x, y, color, assets.cell_texture.clone(), z)));
}

fn can_place(board: &crate::data::BoardGrid, cells: &[(i32, i32); 4], col: i32, row: i32) -> bool {
    for &(dc, dr) in cells {
        let c = col + dc;
        let r = row + dr;
        if c < 0 || c >= BOARD_COLS as i32 { return false; }
        if r >= BOARD_ROWS as i32 { return false; }
        if r < 0 { continue; }
        if board[r as usize][c as usize].is_some() { return false; }
    }
    true
}

fn next_preview_y_offset(kind: PieceKind) -> i32 {
    // Mirrors current `next_preview_y_offset` logic — copy from src/types.rs or src/game.rs in master.
    // Most pieces: 0; I-piece: 1 (or whatever master has). VERIFY against master before committing.
    match kind {
        PieceKind::I => 1,
        _ => 0,
    }
}
```

**Verification:** open `src/types.rs` and `src/game.rs` from master (`git show master:src/types.rs`) and search for `next_preview` to copy the exact y-offset and shape-offset logic. Do not guess.

**Resource names:** the resource names `RotationSystemRes` and `NextPiece` and the component name `PieceKindComp` are placeholders from Plan 1. Use whatever names Plan 1 actually picked. If Plan 1 stored the next piece inside a `Randomizer` resource, query `Res<Randomizer>` and call its peek method. **Read your own Plan 1 code first; do not assume.**

- [ ] **Step 2: Register systems**

In `src/render/mod.rs::RenderPlugin::build` add to the `Update` group already gated by Playing/Ready/GameOver:

```rust
piece::render_active_piece,
piece::render_next_preview,
```

Both run after `board::render_board` so they draw on top — but z-ordering already handles layer; system order is fine in any order.

- [ ] **Step 3: Run**

```bash
cargo run
```

Expected: at game start, a piece spawns at the top of the board, falls under gravity, can be moved with arrow keys, and a faint ghost appears below where it will land. Next-piece preview shows above the board.

- [ ] **Step 4: Commit**

```bash
git add src/render/ 
git commit -m "feat(render): active piece, ghost, and next preview"
```

---

## Task 6: HUD sidebar and grade bar

**Files:**
- Modify: `src/render/hud.rs`
- Modify: `src/render/mod.rs`

The HUD uses `Text2d` (or 2D text via `bevy_egui` — but we're keeping egui for menus only). Use `Text2d` with the loaded font.

- [ ] **Step 1: Replace src/render/hud.rs**

```rust
use bevy::prelude::*;
use crate::render::{
    BAR_WIDTH, BAR_X, BOARD_BG, BOARD_X, BOARD_Y, CELL, DIVIDER_X, SIDEBAR_X,
};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_COLS, BOARD_ROWS, Grade};

#[derive(Component)]
pub struct HudNode;

pub fn render_hud(
    mut commands: Commands,
    existing: Query<Entity, With<HudNode>>,
    judge: Res<crate::judge::Judge>,
    progress: Res<crate::resources::GameProgress>,
    assets: Res<GameAssets>,
    mut clear_color: ResMut<ClearColor>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }

    // Background tint by grade index.
    *clear_color = ClearColor(grade_bg_color(judge.grade().index()));

    // Grade bar.
    spawn_grade_bar(&mut commands, judge.score(), judge.grade());

    // Sidebar text.
    let dim = Color::srgba(0.5, 0.5, 0.5, 1.0);
    const FONT_LG: f32 = 26.0;
    const FONT_SM: f32 = 18.0;
    const LH: f32 = 30.0;

    let x = SIDEBAR_X;
    let mut y = BOARD_Y + 22.0;

    let push = |commands: &mut Commands, text: String, x: f32, y: f32, size: f32, color: Color| {
        commands.spawn((
            HudNode,
            Text2d::new(text),
            TextFont { font: assets.font.clone(), font_size: size, ..default() },
            TextColor(color),
            bevy::sprite::Anchor::TopLeft,
            Transform::from_xyz(x, y, 10.0),
        ));
    };

    push(&mut commands, "LEVEL".into(), x, y, FONT_SM, dim); y += LH;
    push(&mut commands, format!("{:03}", progress.level), x, y, FONT_LG, Color::WHITE);
    y += 6.0;
    // Divider line under current level.
    commands.spawn((
        HudNode,
        Sprite {
            color: dim,
            custom_size: Some(Vec2::new(48.0, 2.0)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(x, y, 10.0),
    ));
    y += 24.0;
    push(&mut commands, format!("{}", next_level_barrier(progress.level)), x, y, FONT_LG, Color::WHITE);
    y += LH + 8.0;

    push(&mut commands, "LINES".into(), x, y, FONT_SM, dim); y += LH;
    push(&mut commands, format!("{}", progress.lines), x, y, FONT_LG, Color::WHITE);
    y += LH + 8.0;

    push(&mut commands, "TIME".into(), x, y, FONT_SM, dim); y += LH;
    push(&mut commands, format_time(progress.ticks_elapsed), x, y, FONT_LG, Color::WHITE);
    y += LH + 8.0;

    push(&mut commands, "SCORE".into(), x, y, FONT_SM, dim); y += LH;
    push(&mut commands, format!("{}", judge.score()), x, y, FONT_LG, Color::WHITE);
    y += LH + 8.0;

    push(&mut commands, "GRADE".into(), x, y, FONT_SM, dim); y += LH;
    push(&mut commands, format!("{}", judge.grade()), x, y, FONT_LG, Color::WHITE);
    y += LH + 8.0;

    push(&mut commands, "NEXT".into(), x, y, FONT_SM, dim); y += LH;
    let (_, next_opt) = Grade::grade_progress(judge.score());
    let next_str = match next_opt {
        Some(n) => format!("{}", n),
        None => "??????".to_string(),
    };
    push(&mut commands, next_str, x, y, FONT_LG, Color::WHITE);
}

fn spawn_grade_bar(commands: &mut Commands, score: u32, grade: Grade) {
    let (prev, next_opt) = Grade::grade_progress(score);
    let progress: f32 = match next_opt {
        None => 1.0,
        Some(next) => (score - prev) as f32 / (next - prev) as f32,
    };

    let bar_h = BOARD_ROWS as f32 * CELL;
    const SHADOW_PAD: f32 = 2.0;
    let inner_h = bar_h - SHADOW_PAD * 2.0;
    let fill_h = inner_h * progress;

    // Shadow.
    commands.spawn((
        HudNode,
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.55),
            custom_size: Some(Vec2::new(BAR_WIDTH + SHADOW_PAD * 2.0, bar_h)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(BAR_X - SHADOW_PAD, BOARD_Y, 5.0),
    ));

    // Divider line.
    commands.spawn((
        HudNode,
        Sprite {
            color: Color::srgba(0.25, 0.25, 0.35, 1.0),
            custom_size: Some(Vec2::new(1.5, bar_h)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(DIVIDER_X, BOARD_Y, 5.0),
    ));

    // Bar background.
    commands.spawn((
        HudNode,
        Sprite {
            color: BOARD_BG,
            custom_size: Some(Vec2::new(BAR_WIDTH, inner_h)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(BAR_X, BOARD_Y + SHADOW_PAD, 6.0),
    ));

    // Bar fill (rises from bottom).
    commands.spawn((
        HudNode,
        Sprite {
            color: grade_bar_color(grade.index()),
            custom_size: Some(Vec2::new(BAR_WIDTH, fill_h)),
            anchor: bevy::sprite::Anchor::TopLeft,
            ..default()
        },
        Transform::from_xyz(BAR_X, BOARD_Y + SHADOW_PAD + inner_h - fill_h, 7.0),
    ));
}

fn grade_bar_color(idx: usize) -> Color {
    match idx % 7 {
        0 => Color::srgba_u8(220, 50, 50, 200),
        1 => Color::srgba_u8(230, 130, 0, 200),
        2 => Color::srgba_u8(220, 210, 0, 200),
        3 => Color::srgba_u8(50, 180, 50, 200),
        4 => Color::srgba_u8(50, 100, 220, 200),
        5 => Color::srgba_u8(80, 0, 200, 200),
        _ => Color::srgba_u8(150, 0, 220, 200),
    }
}

fn grade_bg_color(idx: usize) -> Color {
    let tint = grade_bar_color(idx).to_srgba();
    Color::srgba(0.04 + tint.red * 0.14, 0.04 + tint.green * 0.14, 0.07 + tint.blue * 0.14, 1.0)
}

fn next_level_barrier(level: u32) -> u32 {
    let round_up = (level + 1).next_multiple_of(100);
    if round_up == 1000 { 999 } else { round_up }
}

pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}
```

**Resource names again:** `Judge`, `GameProgress`, `RotationSystemRes` are placeholders. Adjust to match what Plan 1 actually defined.

- [ ] **Step 2: Register the system**

In `src/render/mod.rs`:

```rust
hud::render_hud,
```

Also register `ClearColor` as a resource at startup (or set it via `insert_resource` in main):

```rust
.insert_resource(ClearColor(Color::srgba(0.04, 0.04, 0.07, 1.0)))
```

- [ ] **Step 3: Run**

```bash
cargo run
```

Expected: sidebar shows LEVEL/LINES/TIME/SCORE/GRADE/NEXT updating each frame, grade bar fills as score grows, background subtly tints by grade.

- [ ] **Step 4: Commit**

```bash
git add src/render/
git commit -m "feat(render): HUD sidebar, grade bar, and grade-tinted background"
```

---

## Task 7: Particle system from line clears

**Files:**
- Modify: `src/render/particles.rs`
- Modify: `src/render/mod.rs`

- [ ] **Step 1: Replace src/render/particles.rs**

```rust
use bevy::prelude::*;
use rand::Rng;
use crate::constants::{PARTICLE_BASE_LIFETIME, PARTICLE_BASE_SPEED, PARTICLE_GRAVITY};
use crate::render::{BOARD_X, BOARD_Y, CELL, INSET, cell_sprite, piece_color};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_COLS, GameEvent};
use crate::resources::{Board, PendingCompaction};

#[derive(Component)]
pub struct Particle {
    pub vx: f32,
    pub vy: f32,
    pub age: u32,
    pub lifetime: u32,
    pub base_color: Color,
}

/// Spawn particles in response to GameEvent::LineClear. Mirrors src/renderer.rs:743-784.
pub fn spawn_particles_on_line_clear(
    mut commands: Commands,
    mut events: EventReader<GameEvent>,
    board: Res<Board>,
    pending: Res<PendingCompaction>,
    assets: Res<GameAssets>,
) {
    let mut rng = rand::thread_rng();
    for ev in events.read() {
        let GameEvent::LineClear { count } = *ev else { continue; };
        // System ordering ensures this runs in the same FixedUpdate tick the event was
        // emitted (during `lock_piece`), before `line_clear_delay` clears `pending`.
        // Snapshot the rows now so later motion of `pending` can't affect this burst.
        let rows: Vec<usize> = pending.0.clone();
        let particles_per_cell: u32 = if count >= 4 { 3 } else { 1 };
        let speed_scale = match count {
            1 => 1.0,
            2 => 1.4,
            3 => 1.8,
            _ => 2.5,
        };

        for &r in &rows {
            for c in 0..BOARD_COLS {
                let Some(kind) = board.0[r][c] else { continue; };
                for _ in 0..particles_per_cell {
                    let dist = c as f32 - (BOARD_COLS as f32 - 1.0) / 2.0;
                    let base_angle = dist.atan2(-1.5_f32);
                    let spread = (rng.r#gen::<f32>() - 0.5) * std::f32::consts::FRAC_PI_3;
                    let angle = base_angle + spread;
                    let speed = PARTICLE_BASE_SPEED * speed_scale * (0.6 + 0.8 * rng.r#gen::<f32>());

                    let lifetime = PARTICLE_BASE_LIFETIME + (rng.r#gen::<f32>() * 25.0) as u32;
                    let x = BOARD_X + c as f32 * CELL + CELL * 0.5;
                    let y = BOARD_Y + r as f32 * CELL + CELL * 0.5;
                    let color = piece_color(kind);

                    commands.spawn((
                        Particle {
                            vx: angle.sin() * speed,
                            vy: -angle.cos().abs() * speed,
                            age: 0,
                            lifetime,
                            base_color: color,
                        },
                        Sprite {
                            image: assets.cell_texture.clone(),
                            color,
                            custom_size: Some(Vec2::new(CELL - INSET * 2.0, CELL - INSET * 2.0)),
                            anchor: bevy::sprite::Anchor::Center,
                            ..default()
                        },
                        Transform::from_xyz(x, y, 50.0),
                    ));
                }
            }
        }
    }
}

/// Update particle positions and despawn expired. Runs in FixedUpdate (60 Hz) to match
/// the original tick-based animation cadence. (See ba34489: animation speeds decoupled
/// from framerate.)
pub fn update_particles(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    for (entity, mut particle, mut transform, mut sprite) in &mut q {
        transform.translation.x += particle.vx;
        transform.translation.y += particle.vy;
        particle.vy += PARTICLE_GRAVITY;
        particle.age += 1;
        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
        } else {
            let alpha = 1.0 - particle.age as f32 / particle.lifetime as f32;
            let s = particle.base_color.to_srgba();
            sprite.color = Color::srgba(s.red, s.green, s.blue, alpha);
        }
    }
}
```

**System ordering note:** `spawn_particles_on_line_clear` reads `pending.0` to know which rows just cleared. That snapshot only works if this system runs in the same `FixedUpdate` tick as the event — and *before* `line_clear_delay` mutates `pending` further. Order it explicitly:

```rust
.add_systems(FixedUpdate,
    particles::spawn_particles_on_line_clear
        .after(crate::systems::lock_piece::lock_piece_system)
        .before(crate::systems::line_clear_delay::line_clear_delay_system),
)
```

(Adapt system function names to match Plan 1.) If this is too brittle, fall back: widen `GameEvent::LineClear` to `LineClear { count: u32, rows: Vec<usize> }` in `data.rs` and have `lock_piece` populate it. That eliminates the ordering dependency entirely.

- [ ] **Step 2: Add rand to Cargo.toml**

If not already present from Plan 1's randomizer port, add to `[dependencies]`:

```toml
rand = "0.8"
```

- [ ] **Step 3: Register systems**

In `src/render/mod.rs::RenderPlugin::build`:

```rust
.add_systems(Update, particles::spawn_particles_on_line_clear
    .run_if(in_state(crate::AppState::Playing)))
.add_systems(FixedUpdate, particles::update_particles)
```

- [ ] **Step 4: Run**

```bash
cargo run
```

Expected: clearing a line produces a burst of particles flying outward and falling under gravity. Multi-line clears produce more, faster particles.

- [ ] **Step 5: Commit**

```bash
git add src/render/ Cargo.toml Cargo.lock
git commit -m "feat(render): particle entities for line clears"
```

---

## Task 8: Line clear overlay text and game-over / ready overlays

**Files:**
- Modify: `src/render/overlays.rs`
- Modify: `src/render/mod.rs`

This is the simplified version (no scanline shader). The label appears for ~45 ticks, then despawns.

- [ ] **Step 1: Replace src/render/overlays.rs**

```rust
use bevy::prelude::*;
use crate::render::{BOARD_X, BOARD_Y, CELL};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_COLS, BOARD_ROWS, GameEvent};

const OVERLAY_LIFETIME: u32 = 45;

#[derive(Component)]
pub struct LineClearOverlay {
    pub ticks_left: u32,
    pub kind: OverlayKind,
}

#[derive(Clone, Copy)]
pub enum OverlayKind {
    Double,
    Triple,
    Fetris,
}

pub fn spawn_line_clear_overlay(
    mut commands: Commands,
    mut events: EventReader<GameEvent>,
    existing: Query<Entity, With<LineClearOverlay>>,
    assets: Res<GameAssets>,
) {
    for ev in events.read() {
        let GameEvent::LineClear { count } = *ev else { continue; };
        let (label, kind) = match count {
            2 => ("DOUBLE", OverlayKind::Double),
            3 => ("TRIPLE", OverlayKind::Triple),
            4 => ("FETRIS", OverlayKind::Fetris),
            _ => continue,
        };

        // Replace any existing overlay.
        for e in &existing {
            commands.entity(e).despawn();
        }

        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;

        commands.spawn((
            LineClearOverlay { ticks_left: OVERLAY_LIFETIME, kind },
            Text2d::new(label),
            TextFont { font: assets.font.clone(), font_size: 40.0, ..default() },
            TextColor(Color::WHITE),
            bevy::sprite::Anchor::Center,
            Transform::from_xyz(cx, cy, 200.0),
        ));
    }
}

pub fn tick_line_clear_overlay(
    mut commands: Commands,
    mut q: Query<(Entity, &mut LineClearOverlay)>,
) {
    for (entity, mut o) in &mut q {
        if o.ticks_left == 0 {
            commands.entity(entity).despawn();
        } else {
            o.ticks_left -= 1;
        }
    }
}

#[derive(Component)]
pub struct StateText;

/// Spawned/despawned by AppState transitions. Shows "READY" / "GAME OVER" / "LEVEL 999" + time.
pub fn render_state_text(
    mut commands: Commands,
    existing: Query<Entity, With<StateText>>,
    state: Res<State<crate::AppState>>,
    progress: Res<crate::resources::GameProgress>,
    assets: Res<GameAssets>,
    // Plan 1 stores game-won as a flag inside GameProgress (or as part of an
    // AppState distinction). Adapt the predicate below to whatever Plan 1 actually
    // uses to distinguish "level 999 reached" from a generic top-out.
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
            TextFont { font: assets.font.clone(), font_size: size, ..default() },
            TextColor(color),
            bevy::sprite::Anchor::Center,
            Transform::from_xyz(cx, cy + dy, 150.0),
        ));
    };

    match state.get() {
        crate::AppState::Ready => {
            mk(&mut commands, "READY".into(), 0.0, 28.0, Color::WHITE);
        }
        crate::AppState::GameOver if progress.game_won => {
            mk(&mut commands, "LEVEL 999".into(), -16.0, 28.0, Color::WHITE);
            mk(&mut commands,
                crate::render::hud::format_time(progress.ticks_elapsed),
                20.0, 22.0, Color::srgba(0.83, 0.83, 0.83, 1.0));
        }
        crate::AppState::GameOver => {
            mk(&mut commands, "GAME OVER".into(), 0.0, 28.0, Color::WHITE);
        }
        _ => {}
    }
}
```

If `GameProgress` doesn't yet have a `game_won: bool` field, add it: defaults to `false`, set to `true` by Plan 1's `game_over` system when level reaches 999. The same field is what `submit_score_on_game_over` (Plan 3 Task 3) reads to decide between regular and "Level 999" hi-score handling.

- [ ] **Step 2: Register systems**

In `src/render/mod.rs::RenderPlugin::build`:

```rust
.add_systems(Update, (
    overlays::spawn_line_clear_overlay,
    overlays::render_state_text,
).run_if(state_active()))
.add_systems(FixedUpdate, overlays::tick_line_clear_overlay)
```

Where `state_active()` is `in_state(AppState::Playing).or(in_state(AppState::Ready)).or(in_state(AppState::GameOver))` — extract this into a helper `fn state_active() -> impl Condition<...>` in `src/render/mod.rs` if you reuse it many times, or just inline it.

- [ ] **Step 3: Run**

```bash
cargo run
```

Expected: clearing 2/3/4 lines pops up DOUBLE/TRIPLE/FETRIS text in the center of the board for ~3/4 second. Game over shows "GAME OVER".

- [ ] **Step 4: Commit**

```bash
git add src/render/ src/types.rs src/systems/
git commit -m "feat(render): line-clear, ready, and game-over text overlays"
```

---

## Task 9: bevy_egui menu — main screen

**Files:**
- Create: `src/menu/mod.rs`
- Create: `src/menu/state.rs`
- Create: `src/menu/main_screen.rs`
- Modify: `src/main.rs`

The original menu.rs uses arrow-key navigation. We'll preserve that exact UX in egui — rendering selectable items but driving them with `ButtonInput<KeyCode>` rather than mouse clicks. (egui mouse interaction also works as a bonus.)

- [ ] **Step 1: Delete the old src/menu.rs (it's keyboard-coupled to macroquad)**

```bash
git rm src/menu.rs
```

If Plan 1 already removed/ignored `menu.rs`, skip.

- [ ] **Step 2: Create src/menu/state.rs**

```rust
use bevy::prelude::*;
use crate::data::{GameMode, Kind};

#[derive(Resource)]
pub struct MenuState {
    pub screen: MenuScreen,
    pub cursor: usize,
    pub game_mode: GameMode,
    pub rotation: Kind,
    pub hi_scores_tab: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuScreen { Main, HiScores, Controls }

impl Default for MenuState {
    fn default() -> Self {
        Self {
            screen: MenuScreen::Main,
            cursor: 0,
            game_mode: GameMode::Master,
            rotation: Kind::Ars,
            hi_scores_tab: 0,
        }
    }
}
```

- [ ] **Step 3: Create src/menu/main_screen.rs**

```rust
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::menu::state::{MenuScreen, MenuState};
use crate::data::{GameMode, Kind};

/// Reads the same key bindings as src/menu.rs / src/main.rs:62-71.
struct MenuInput {
    up: bool, down: bool, left: bool, right: bool,
    confirm: bool, back: bool,
}

fn read_input(keys: &ButtonInput<KeyCode>) -> MenuInput {
    MenuInput {
        up: keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyK),
        down: keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyJ),
        left: keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyH),
        right: keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyL),
        confirm: keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter),
        back: keys.just_pressed(KeyCode::Backspace),
    }
}

pub fn main_menu_system(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<crate::AppState>>,
    mut config: ResMut<crate::stub_storage::GameConfigRes>,
    hi_scores: Res<crate::stub_storage::HiScoresRes>,
    muted: Res<crate::stub_storage::MutedRes>,
) {
    if menu.screen != MenuScreen::Main { return; }
    let input = read_input(&keys);

    if input.up { menu.cursor = menu.cursor.saturating_sub(1); }
    if input.down { menu.cursor = (menu.cursor + 1).min(4); }

    let mut start_game = false;
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
            menu.hi_scores_tab = crate::stub_storage::slot_index(menu.game_mode, menu.rotation);
            menu.screen = MenuScreen::HiScores;
        }
        3 if input.confirm => { menu.screen = MenuScreen::Controls; }
        4 if input.confirm => { start_game = true; }
        _ => {}
    }

    let _ = (hi_scores,); // hi_scores read elsewhere; kept for parity with original menu.rs flow

    let ctx = contexts.ctx_mut();
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 10, 18)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(120.0);
                let mode_str = match menu.game_mode { GameMode::Master => "MASTER", GameMode::TwentyG => "20G" };
                let rot_str = match menu.rotation { Kind::Ars => "ARS", Kind::Srs => "SRS" };

                let row = |ui: &mut egui::Ui, label: &str, color: egui::Color32, size: f32| {
                    ui.label(egui::RichText::new(label).color(color).size(size));
                };
                let bracket = |s: &str, active: bool| -> String {
                    if active { format!("< {} >", s) } else { format!("  {}  ", s) }
                };

                row(ui, "GAME MODE", egui::Color32::GRAY, 18.0);
                row(ui, &bracket(mode_str, menu.cursor == 0), egui::Color32::WHITE, 24.0);
                ui.add_space(20.0);
                row(ui, "ROTATION", egui::Color32::GRAY, 18.0);
                row(ui, &bracket(rot_str, menu.cursor == 1), egui::Color32::WHITE, 24.0);
                ui.add_space(20.0);
                row(ui, &bracket("HI SCORES", menu.cursor == 2), egui::Color32::WHITE, 24.0);
                row(ui, &bracket("CONTROLS", menu.cursor == 3), egui::Color32::WHITE, 24.0);
                ui.add_space(20.0);
                row(ui, &bracket("START", menu.cursor == 4), egui::Color32::WHITE, 24.0);

                ui.add_space(60.0);
                let (label, color) = if muted.0 {
                    ("[M]  MUTED", egui::Color32::from_rgb(204, 102, 102))
                } else {
                    ("[M]  SOUND ON", egui::Color32::GRAY)
                };
                row(ui, label, color, 14.0);
            });
        });

    if start_game {
        config.game_mode = menu.game_mode;
        config.rotation = menu.rotation;
        next_state.set(crate::AppState::Ready);
    }
}
```

- [ ] **Step 4: Create src/menu/mod.rs**

```rust
use bevy::prelude::*;

pub mod state;
pub mod main_screen;
pub mod hi_scores;
pub mod controls;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<state::MenuState>()
            .add_systems(Update, (
                main_screen::main_menu_system,
                hi_scores::hi_scores_system,
                controls::controls_system,
            ).run_if(in_state(crate::AppState::Menu)));
    }
}
```

- [ ] **Step 5: Stub the hi_scores and controls modules**

```bash
cat > src/menu/hi_scores.rs <<'EOF'
use bevy::prelude::*;
use bevy_egui::EguiContexts;

pub fn hi_scores_system(_: EguiContexts) {}
EOF

cat > src/menu/controls.rs <<'EOF'
use bevy::prelude::*;
use bevy_egui::EguiContexts;

pub fn controls_system(_: EguiContexts) {}
EOF
```

- [ ] **Step 6: Register MenuPlugin in main.rs and ensure AppState::Menu is the initial state**

```rust
mod menu;
// ...
.add_plugins(menu::MenuPlugin)
// (init_state added in Plan 1; ensure default is Menu)
.init_state::<AppState>() // if AppState::Menu is the Default variant
```

If `AppState` doesn't yet have a `Menu` variant (Plan 1 may have started in `Playing`), add it now:

```rust
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default] Menu,
    Ready,
    Playing,
    GameOver,
}
```

- [ ] **Step 7: Run**

```bash
cargo run
```

Expected: window opens at the main menu. Arrow keys / hjkl move the `< X >` indicator. Left/right toggles GAME MODE and ROTATION. Pressing Space on START transitions to `Ready` (and you'll see the READY text from Task 8 + the board).

- [ ] **Step 8: Commit**

```bash
git add src/menu/ src/main.rs
git commit -m "feat(menu): bevy_egui main menu with keyboard navigation"
```

---

## Task 10: Hi-scores screen (egui, reads stub resource)

**Files:**
- Modify: `src/menu/hi_scores.rs`

- [ ] **Step 1: Replace src/menu/hi_scores.rs**

```rust
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::menu::state::{MenuScreen, MenuState};
use crate::menu::main_screen::read_input; // export from main_screen if not already pub

pub fn hi_scores_system(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
    hi_scores: Res<crate::stub_storage::HiScoresRes>,
) {
    if menu.screen != MenuScreen::HiScores { return; }
    let input = read_input(&keys);
    if input.back { menu.screen = MenuScreen::Main; return; }
    if input.left { menu.hi_scores_tab = menu.hi_scores_tab.saturating_sub(1); }
    if input.right { menu.hi_scores_tab = (menu.hi_scores_tab + 1).min(3); }

    let tab_names = ["MASTER / ARS", "MASTER / SRS", "20G / ARS", "20G / SRS"];
    let tab = menu.hi_scores_tab;
    let entries = &hi_scores.0[tab];

    let ctx = contexts.ctx_mut();
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 10, 18)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(120.0);
                ui.label(egui::RichText::new(format!("< {} >", tab_names[tab]))
                    .color(egui::Color32::WHITE).size(26.0));
                ui.add_space(40.0);

                egui::Grid::new("hi_scores_grid").num_columns(3).spacing([60.0, 12.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("#").color(egui::Color32::GRAY).size(15.0));
                    ui.label(egui::RichText::new("GRADE").color(egui::Color32::GRAY).size(15.0));
                    ui.label(egui::RichText::new("TIME").color(egui::Color32::GRAY).size(15.0));
                    ui.end_row();

                    for i in 0..5 {
                        let color = if i == 0 { egui::Color32::WHITE } else { egui::Color32::LIGHT_GRAY };
                        ui.label(egui::RichText::new(format!("{}", i + 1)).color(color).size(20.0));
                        if let Some(e) = entries.get(i) {
                            ui.label(egui::RichText::new(format!("{}", e.grade)).color(color).size(20.0));
                            ui.label(egui::RichText::new(crate::render::hud::format_time(e.ticks))
                                .color(color).size(20.0));
                        } else {
                            ui.label(egui::RichText::new("---").color(egui::Color32::DARK_GRAY).size(20.0));
                            ui.label(egui::RichText::new("---").color(egui::Color32::DARK_GRAY).size(20.0));
                        }
                        ui.end_row();
                    }
                });

                ui.add_space(40.0);
                ui.label(egui::RichText::new("BKSP to go back").color(egui::Color32::GRAY).size(14.0));
            });
        });
}
```

In `src/menu/main_screen.rs`, change `fn read_input` and `struct MenuInput` to `pub` so this file can use them. (Or move them to `src/menu/state.rs` — cleaner.)

- [ ] **Step 2: Run**

```bash
cargo run
```

Expected: from main menu, navigate to HI SCORES and press Space — see the hi-scores screen with 5 empty rows ("---"). Left/right cycles tabs. Backspace returns to main.

- [ ] **Step 3: Commit**

```bash
git add src/menu/
git commit -m "feat(menu): hi-scores screen reading stub resource"
```

---

## Task 11: Controls screen

**Files:**
- Modify: `src/menu/controls.rs`

- [ ] **Step 1: Replace src/menu/controls.rs**

```rust
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::menu::state::{MenuScreen, MenuState};
use crate::menu::main_screen::read_input;

pub fn controls_system(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if menu.screen != MenuScreen::Controls { return; }
    if read_input(&keys).back { menu.screen = MenuScreen::Main; return; }

    let ctx = contexts.ctx_mut();
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(10, 10, 18)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(100.0);
                ui.label(egui::RichText::new("CONTROLS").color(egui::Color32::WHITE).size(26.0));
                ui.add_space(40.0);

                let rows: &[(&str, &str)] = &[
                    ("Left / H", "Move left"),
                    ("Right / L", "Move right"),
                    ("Down / J", "Soft drop"),
                    ("Space", "Sonic drop"),
                    ("X", "Rotate CW"),
                    ("Z", "Rotate CCW"),
                    ("Backspace", "Back / quit"),
                ];

                egui::Grid::new("controls_grid").num_columns(2).spacing([40.0, 12.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("KEY").color(egui::Color32::GRAY).size(15.0));
                    ui.label(egui::RichText::new("ACTION").color(egui::Color32::GRAY).size(15.0));
                    ui.end_row();
                    for (k, a) in rows {
                        ui.label(egui::RichText::new(*k).color(egui::Color32::LIGHT_GRAY).size(20.0));
                        ui.label(egui::RichText::new(*a).color(egui::Color32::LIGHT_GRAY).size(20.0));
                        ui.end_row();
                    }
                });

                ui.add_space(40.0);
                ui.label(egui::RichText::new("BKSP to go back").color(egui::Color32::GRAY).size(14.0));
            });
        });
}
```

- [ ] **Step 2: Run + verify**

```bash
cargo run
```

Expected: navigate to CONTROLS and Space → see the key/action table. Backspace returns.

- [ ] **Step 3: Commit**

```bash
git add src/menu/
git commit -m "feat(menu): controls screen"
```

---

## Task 12: Mute toggle (M key) and escape-to-quit / back

**Files:**
- Create: `src/systems/global_input.rs`
- Modify: `src/systems/mod.rs`
- Modify: `src/main.rs`

The original main.rs handles two global keys: `M` toggles mute, `Escape` quits (or backs out from menu sub-screens). Audio is out of scope for Plan 2, but keep the `MutedRes` toggle so the menu mute label updates.

- [ ] **Step 1: Create src/systems/global_input.rs**

```rust
use bevy::prelude::*;
use bevy::app::AppExit;
use crate::menu::state::{MenuScreen, MenuState};

pub fn handle_global_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut muted: ResMut<crate::stub_storage::MutedRes>,
    mut menu: ResMut<MenuState>,
    state: Res<State<crate::AppState>>,
    mut next_state: ResMut<NextState<crate::AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        muted.0 = !muted.0;
    }
    if keys.just_pressed(KeyCode::Escape) {
        match state.get() {
            crate::AppState::Menu => {
                if menu.screen == MenuScreen::Main {
                    exit.send(AppExit::Success);
                } else {
                    menu.screen = MenuScreen::Main;
                }
            }
            // Esc also quits during gameplay, mirroring src/main.rs:96.
            _ => { exit.send(AppExit::Success); }
        }
    }
}
```

- [ ] **Step 2: Register**

In `src/systems/mod.rs` add `pub mod global_input;` and in `main.rs`:

```rust
.add_systems(Update, systems::global_input::handle_global_input)
```

- [ ] **Step 3: Run + test**

```bash
cargo run
```

Expected: on the menu, pressing M flips between `[M] MUTED` (red) and `[M] SOUND ON` (gray). Esc on a sub-screen returns to main menu; Esc on main quits. Esc during gameplay quits.

- [ ] **Step 4: Commit**

```bash
git add src/systems/ src/main.rs
git commit -m "feat: global mute toggle and escape handling"
```

---

## Task 13: Game-over → Space → return to menu

**Files:**
- Create: `src/systems/post_game.rs`
- Modify: `src/systems/mod.rs`
- Modify: `src/main.rs`

Mirrors src/main.rs:175-177. After game over, Space restarts the cycle by going back to the menu.

- [ ] **Step 1: Create src/systems/post_game.rs**

```rust
use bevy::prelude::*;

pub fn return_to_menu_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<crate::AppState>>,
    mut next_state: ResMut<NextState<crate::AppState>>,
) {
    if state.get() == &crate::AppState::GameOver && keys.just_pressed(KeyCode::Space) {
        next_state.set(crate::AppState::Menu);
    }
}
```

- [ ] **Step 2: Register**

In `main.rs`:

```rust
.add_systems(Update, systems::post_game::return_to_menu_on_space)
```

- [ ] **Step 3: Reset game state on entering Menu**

When transitioning back to the menu, the board, judge, and active piece must be reset. Add an `OnEnter(AppState::Menu)` system:

```rust
fn reset_game_on_enter_menu(
    mut commands: Commands,
    mut board: ResMut<crate::resources::Board>,
    mut judge: ResMut<crate::judge::Judge>,
    mut progress: ResMut<crate::resources::GameProgress>,
    active: Query<Entity, With<crate::components::ActivePiece>>,
    particles: Query<Entity, With<crate::render::particles::Particle>>,
) {
    *board = Default::default();
    *judge = Default::default();
    *progress = Default::default();
    for e in &active { commands.entity(e).despawn(); }
    for e in &particles { commands.entity(e).despawn(); }
    // Reset other game-only resources Plan 1 introduced (Randomizer, DasState, etc.) —
    // the reader should audit Plan 1's resources and reset each one.
}

// In main.rs:
.add_systems(OnEnter(AppState::Menu), reset_game_on_enter_menu)
```

Also add an `OnEnter(AppState::Ready)` system that initializes a fresh game with the chosen `GameConfigRes` — this is essentially what Plan 1's startup did, but now triggered on state entry rather than `Startup`. The `Randomizer`, the active piece spawn, and the `Ready` countdown all happen here. Refactor Plan 1's startup to be triggered `OnEnter(AppState::Ready)` instead of `Startup`.

- [ ] **Step 4: Run end-to-end**

```bash
cargo run
```

Manual test: menu → start → ready countdown → play → die → see "GAME OVER" → press Space → back to menu → can start a new game cleanly.

- [ ] **Step 5: Commit**

```bash
git add src/systems/ src/main.rs
git commit -m "feat: game-over to menu transition; reset on menu entry"
```

---

## Task 14: Sanity pass — all Plan 1 tests still green

**Files:** none (verification)

- [ ] **Step 1: Run the full test suite**

```bash
cargo test
```

Expected: all tests from Plan 1 still pass. The render and menu code is `Update`-scheduled so it doesn't run in headless `App`s; the data resources still tick correctly under `MinimalPlugins`.

- [ ] **Step 2: If anything broke**

Likely culprits:
- `GameEvent::LineClear` shape was widened (now carries `rows`) — update Plan 1 tests that assert on event shape.
- A resource was renamed or restructured — update tests.
- `Default` impls added to `GameMode`/`Kind` may collide with explicit `#[derive]` — fix.

Fix what's broken. Do not skip tests.

- [ ] **Step 3: Commit if fixes were needed**

```bash
git commit -am "fix(tests): adapt to render/menu integration changes"
```

---

## Task 15: Final native acceptance run

**Files:** none

This is the spec's checkpoint after Phase 5: "native game playable end-to-end."

- [ ] **Step 1: Build and play**

```bash
cargo run --release
```

- [ ] **Step 2: Manual checklist**

Verify each of these works in the running game:

- [ ] Window opens at 560×780 titled "fetris"
- [ ] Main menu shows GAME MODE / ROTATION / HI SCORES / CONTROLS / START
- [ ] Arrow keys / hjkl navigate menu cursor
- [ ] Left/right toggles GAME MODE between MASTER and 20G
- [ ] Left/right toggles ROTATION between ARS and SRS
- [ ] HI SCORES screen renders 5 empty rows; left/right cycles tabs
- [ ] CONTROLS screen renders the key/action table
- [ ] Backspace returns from sub-screens to main; Esc on main quits
- [ ] M toggles `[M] MUTED` ↔ `[M] SOUND ON`
- [ ] Pressing Space on START → READY countdown → playable
- [ ] Active piece falls; arrow keys move; X/Z rotate; J soft-drops; Space sonic-drops
- [ ] Ghost piece appears below active
- [ ] Next-piece preview shows above board
- [ ] Locked cells render with grey borders against unfilled neighbors
- [ ] Sidebar shows LEVEL/LINES/TIME/SCORE/GRADE/NEXT, all updating
- [ ] Grade bar fills as score increases; tints background
- [ ] Clearing 1 line: small particle burst
- [ ] Clearing 2/3/4 lines: larger burst + DOUBLE/TRIPLE/FETRIS overlay text
- [ ] Game over: GAME OVER text appears; Space returns to menu
- [ ] Reaching level 999: LEVEL 999 + final time appears
- [ ] After returning to menu, starting a new game works (no leaked particles, score reset)
- [ ] 20G mode renders correctly (piece appears at bottom immediately)
- [ ] SRS rotation system selectable (kicks behave differently from ARS)

Anything broken: file a follow-up task or fix inline before claiming Plan 2 complete.

- [ ] **Step 3: If everything works, no commit needed.**

Plan 2 done. Move to Plan 3 (storage + WASM).
