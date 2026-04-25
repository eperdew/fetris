# Bevy Migration Plan 1: Scaffold + Logic + Tests

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a headless bevy app on a worktree branch that ports all fetris game logic onto bevy ECS and passes the full snapshot test suite. No rendering, no menu, no audio, no UI.

**Architecture:** Bevy `App` with `MinimalPlugins`, `FixedUpdate` at 60 Hz. Game logic lives in 7 systems gated by an `AppState` enum. The active piece is a single ECS entity with `ActivePiece`/`PieceKindComp`/`PiecePosition`/`PieceRotation` components. The board, phase, judge, randomizer, and tick-local state live in resources. `JudgeEvent` and `GameEvent` are bevy `Event`s. Snapshot tests build a headless `App`, drive `Resource<InputState>`, call `app.update()`, and snapshot via `GameSnapshot::from_world(&World)`.

**Tech Stack:** Rust, bevy (latest stable), `rand` (replaces `macroquad::rand`), `serde`/`serde_json` (retained for HiScoreEntry serde derives), `insta` (tests).

**Spec:** [docs/superpowers/specs/2026-04-25-bevy-migration-design.md](../specs/2026-04-25-bevy-migration-design.md)

---

## Pre-flight notes

- All work happens on `bevy-migration` branch in `.worktrees/bevy-migration/`. Task 1 creates this.
- "Headless" = no window, no rendering, no audio. Bevy's `MinimalPlugins` plus the game-logic systems registered manually.
- `Time<Fixed>` runs at 60 Hz. Each `app.update()` call from a test pumps exactly one fixed tick *iff* the test seeds enough virtual time. Easier path used here: configure `Time<Fixed>` and call a small helper that advances the fixed schedule by one tick deterministically.
- Audio is **out of scope** for Plan 1. The `AudioPlayer` trait and all `audio.*()` calls in the existing `game.rs` are dropped on the floor. Sound triggers will be re-introduced as `EventReader<GameEvent>` listeners in a later plan.
- Hi-scores logic is **out of scope** for Plan 1 (lives in Plan 3 with `bevy_pkv`). The existing `hiscores.rs` and `storage.rs` files are deleted in this plan; `Judge::grade_entry()` is retained because tests use it.
- `tests.rs` is 2461 lines. Tasks 19–22 port it incrementally: helpers first, one snapshot test as proof, then the bulk port follows a documented rule.

---

## File Structure

After this plan completes:

```
src/
  main.rs                    # bevy App setup, plugin/system registration
  app_state.rs               # AppState enum (States derive)
  data.rs                    # Pure data types: PieceKind, Kind, GameMode, Grade, JudgeEvent, GameEvent, HorizDir, RotationDirection, HiScoreEntry, GameConfig, MenuScreen
  components.rs              # ECS components: ActivePiece, PieceKindComp, PiecePosition, PieceRotation
  resources.rs               # Resources: Board, PiecePhase, NextPiece, GameProgress, DasState, RotationBuffer, PendingCompaction, DropTracking, RotationSystemRes, RotationKind, GameModeRes
  randomizer.rs              # Randomizer resource (TGM history bag)
  rotation_system.rs         # RotationSystem trait + Ars + Srs (verbatim port + Send+Sync bound)
  judge.rs                   # Judge resource + judge_system (consumes JudgeEvent)
  constants.rs               # Verbatim port
  input.rs                   # InputState resource + (Plan 2 will add the input gather system)
  systems/
    mod.rs
    tick.rs                  # tick_counter (increments ticks_elapsed each FixedUpdate)
    active.rs                # Falling+Locking phase logic (rotation/sonic/soft/DAS/gravity/lock-transitions)
    line_clear_delay.rs      # countdown and compact rows
    spawning.rs              # countdown, IRS, spawn next piece
    lock_piece.rs            # helper: write piece to board, detect line clears, emit events, transition phase
    game_over.rs             # AppState transition on game_over/won
  snapshot.rs                # GameSnapshot + from_world(&World)
  tests/
    mod.rs                   # test module wiring
    harness.rs               # headless_app + press/hold/idle/board_from_ascii + accessors
    judge_tests.rs           # ported judge tests
    rotation_tests.rs        # ported rotation/wall-kick tests
    movement_tests.rs        # ported left/right/DAS/soft-drop tests
    lock_tests.rs            # ported lock-delay tests
    line_clear_tests.rs      # ported line-clear/compaction tests
    spawn_tests.rs           # ported spawn/IRS tests
    gravity_tests.rs         # ported 20G + gravity-table tests
    snapshot_tests.rs        # ported snapshot-shape tests
Cargo.toml
.cargo/config.toml           # removed (no longer needed; bevy supports wasm-bindgen properly)
```

Files **deleted** in this plan: `audio_player.rs`, `storage.rs`, `hiscores.rs`, `menu.rs`, `renderer.rs` (latter two return in Plan 2). Old `game.rs` and `types.rs` are deleted after their contents are split into the files above.

---

## Task 1: Create the migration worktree

**Files:**
- Create: `.worktrees/bevy-migration/` (new worktree)
- Create: branch `bevy-migration` (off `master`)

- [ ] **Step 1: Create the worktree**

```bash
git worktree add .worktrees/bevy-migration -b bevy-migration
```

Expected: `Preparing worktree (new branch 'bevy-migration')`. Branch tracks no remote.

- [ ] **Step 2: Switch into the worktree for the rest of this plan**

```bash
cd .worktrees/bevy-migration
```

All file paths in subsequent tasks are relative to this directory. **Every later task that says "Run: cargo …" runs from this worktree.** Verify with `pwd` — should end in `.worktrees/bevy-migration`.

- [ ] **Step 3: Confirm clean baseline**

```bash
cargo build && cargo test
```

Expected: clean build, all existing tests pass. This is the pre-migration baseline.

- [ ] **Step 4: Commit nothing yet** — no changes made.

---

## Task 2: Replace Cargo.toml with bevy dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Rewrite Cargo.toml**

```toml
[package]
name = "fetris"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = { version = "0.18", default-features = false, features = ["bevy_state"] }
rand = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
insta = "1"
```

Notes:
- `default-features = false` keeps the binary lean for headless plan 1; rendering features are added in Plan 2.
- `bevy_state` is needed for the `States` derive used in `app_state.rs`.
- If bevy 0.18 is not the latest at execution time, bump to whatever `cargo search bevy` reports as the latest stable. Update this file to that version. Pin exactly — do not use `^` ranges.

- [ ] **Step 2: Remove `.cargo/config.toml` if it contains macroquad-specific config**

```bash
cat .cargo/config.toml
```

If the file exists and contains the `getrandom`/`wasm-bindgen` workaround comment from CLAUDE.md, delete the file:

```bash
rm .cargo/config.toml
```

Otherwise leave alone.

- [ ] **Step 3: Verify Cargo resolves**

```bash
cargo fetch
```

Expected: bevy and dependencies download without error.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml .cargo/config.toml 2>/dev/null
git rm .cargo/config.toml 2>/dev/null
git commit -m "build: replace macroquad with bevy in Cargo.toml"
```

---

## Task 3: Replace src/ with a hello-world bevy app

**Files:**
- Delete: every file in `src/` except none — all current src files are stale for the new architecture
- Create: `src/main.rs`

- [ ] **Step 1: Wipe the old src/ tree**

```bash
git rm -r src/
```

Expected: every `.rs` file under `src/` removed from the index.

- [ ] **Step 2: Create the new src/main.rs**

```bash
mkdir -p src
```

Write `src/main.rs`:

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, hello_world)
        .run();
}

fn hello_world() {
    info!("fetris bevy scaffold");
}
```

- [ ] **Step 3: Verify native build**

Run: `cargo build`
Expected: success, single warning at most.

- [ ] **Step 4: Verify wasm build**

Run: `cargo build --target wasm32-unknown-unknown`
Expected: success. (Trunk integration is a Plan 3 concern; here we just confirm the target compiles.)

If the wasm target is not installed: `rustup target add wasm32-unknown-unknown`.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: bevy scaffold with hello-world MinimalPlugins app"
```

---

## Task 4: Port constants.rs verbatim

**Files:**
- Create: `src/constants.rs`

- [ ] **Step 1: Copy the file**

Copy the contents of the master branch's `src/constants.rs` into `src/constants.rs` in the worktree.

```bash
git show master:src/constants.rs > src/constants.rs
```

The file references `crate::types::GameMode`. Update the import after Task 5 lands. For now, comment out the import and `gravity_g` function temporarily to keep the file standalone:

Add at the top:
```rust
// gravity_g and GameMode-using helpers are restored in Task 5 once `data` exists.
```

Then comment out the `use` line and the `gravity_g` fn body. The constants themselves should remain uncommented.

- [ ] **Step 2: Wire into main.rs**

Edit `src/main.rs`, add at the top:
```rust
mod constants;
```

- [ ] **Step 3: Verify**

Run: `cargo build`
Expected: builds cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/constants.rs src/main.rs
git commit -m "feat: port constants module"
```

---

## Task 5: Create data.rs with pure data types

**Files:**
- Create: `src/data.rs`
- Modify: `src/constants.rs` (restore `gravity_g`)
- Modify: `src/main.rs` (declare `mod data`)

- [ ] **Step 1: Create src/data.rs**

This file holds every type from old `types.rs` that does **not** become an ECS component or resource. Components live in `components.rs` (Task 7), resources live in `resources.rs` (Task 9).

```rust
//! Pure data types shared across the game. No ECS-specific types here.

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// PieceKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    I, O, T, S, Z, J, L,
}

impl PieceKind {
    pub fn all() -> [Self; 7] {
        [Self::I, Self::O, Self::T, Self::S, Self::Z, Self::J, Self::L]
    }

    /// Picks one of the 7 kinds uniformly using the supplied RNG.
    pub fn random<R: rand::Rng>(rng: &mut R) -> Self {
        match rng.gen_range(0..7) {
            0 => Self::I, 1 => Self::O, 2 => Self::T,
            3 => Self::S, 4 => Self::Z, 5 => Self::J,
            _ => Self::L,
        }
    }
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

pub const BOARD_COLS: usize = 10;
pub const BOARD_ROWS: usize = 20;

/// None = empty, Some(kind) = locked cell color.
pub type BoardGrid = [[Option<PieceKind>; BOARD_COLS]; BOARD_ROWS];

// ---------------------------------------------------------------------------
// Direction enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizDir { Left, Right }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDirection { Clockwise, Counterclockwise }

// ---------------------------------------------------------------------------
// PiecePhase
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiecePhase {
    Falling,
    Locking { ticks_left: u32 },
    LineClearDelay { ticks_left: u32 },
    Spawning { ticks_left: u32 },
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    Left, Right, RotateCw, RotateCcw, SoftDrop, SonicDrop,
}

#[derive(Debug, Default, Clone)]
pub struct InputSnapshot {
    pub held: HashSet<GameKey>,
    pub just_pressed: HashSet<GameKey>,
}

impl InputSnapshot {
    pub fn empty() -> Self { Self::default() }
}

// ---------------------------------------------------------------------------
// Modes / Kinds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GameMode { Master, TwentyG }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind { Ars, Srs }

// `Kind::create()` returns a Box<dyn RotationSystem>; defined in rotation_system.rs (Task 6)
// to avoid a forward-reference here.

// ---------------------------------------------------------------------------
// Grade + Score thresholds (TGM)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Grade {
    Nine, Eight, Seven, Six, Five, Four, Three, Two, One,
    SOne, STwo, SThree, SFour, SFive, SSix, SSeven, SEight, SNine,
}

impl Grade {
    const SCORE_TABLE: &[(u32, Grade)] = &[
        (0, Grade::Nine), (400, Grade::Eight), (800, Grade::Seven),
        (1400, Grade::Six), (2000, Grade::Five), (3500, Grade::Four),
        (5500, Grade::Three), (8000, Grade::Two), (12000, Grade::One),
        (16000, Grade::SOne), (22000, Grade::STwo), (30000, Grade::SThree),
        (40000, Grade::SFour), (52000, Grade::SFive), (66000, Grade::SSix),
        (82000, Grade::SSeven), (100000, Grade::SEight), (120000, Grade::SNine),
    ];

    pub fn of_score(score: u32) -> Self {
        Self::SCORE_TABLE.iter().rev()
            .find(|(t, _)| score >= *t).map(|(_, g)| *g)
            .unwrap_or(Grade::Nine)
    }

    pub fn index(self) -> usize {
        Self::SCORE_TABLE.iter().position(|(_, g)| *g == self).unwrap_or(0)
    }

    pub fn grade_progress(score: u32) -> (u32, Option<u32>) {
        let idx = Self::SCORE_TABLE.iter().rposition(|(t, _)| score >= *t).unwrap_or(0);
        let prev = Self::SCORE_TABLE[idx].0;
        let next = Self::SCORE_TABLE.get(idx + 1).map(|(t, _)| *t);
        (prev, next)
    }
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Nine => "9", Self::Eight => "8", Self::Seven => "7",
            Self::Six => "6", Self::Five => "5", Self::Four => "4",
            Self::Three => "3", Self::Two => "2", Self::One => "1",
            Self::SOne => "S1", Self::STwo => "S2", Self::SThree => "S3",
            Self::SFour => "S4", Self::SFive => "S5", Self::SSix => "S6",
            Self::SSeven => "S7", Self::SEight => "S8", Self::SNine => "S9",
        };
        write!(f, "{:>2}", s)
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy::prelude::Event)]
pub enum JudgeEvent {
    LockedWithoutClear,
    ClearedLines {
        level: u32,
        cleared_playfield: bool,
        num_lines: u32,
        frames_soft_drop_held: u32,
        sonic_drop_rows: u32,
        ticks_elapsed: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy::prelude::Event)]
pub enum GameEvent {
    LineClear { count: u32 },
    PieceBeganLocking,
    GameEnded,
    GradeAdvanced(Grade),
}

// ---------------------------------------------------------------------------
// Hi scores (data only; storage is in Plan 3)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HiScoreEntry {
    pub grade: Grade,
    pub ticks: u64,
}

// ---------------------------------------------------------------------------
// Menu / Config (deferred to Plan 2; types defined for serde compat)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuScreen { Main, HiScores, Controls }

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GameConfig {
    pub game_mode: GameMode,
    pub rotation: Kind,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self { game_mode: GameMode::Master, rotation: Kind::Ars }
    }
}
```

Notes:
- `InputSnapshot` (renamed from `InputState`) is the per-tick frozen view. The bevy `Resource` wrapping a current snapshot is named `InputState` and lives in `resources.rs`.
- `BoardGrid` is the raw `[[Option<PieceKind>; ...]; ...]`. The `Board` newtype Resource wraps it.
- `JudgeEvent`/`GameEvent` derive bevy's `Event` so they can flow through `EventReader`/`EventWriter`.
- The `random()` method now takes an RNG instead of using a global. Callers (the `Randomizer` in Task 8) pass their owned RNG.

- [ ] **Step 2: Restore constants::gravity_g**

Edit `src/constants.rs`:

Replace the commented-out import with:
```rust
use crate::data::GameMode;
```

Uncomment the body of `gravity_g`. Verify it compiles (next step).

- [ ] **Step 3: Wire into main.rs**

Edit `src/main.rs`, add:
```rust
mod data;
```
(beside `mod constants;`)

- [ ] **Step 4: Verify**

Run: `cargo build`
Expected: builds cleanly. There may be unused-import or dead-code warnings — acceptable at this stage.

- [ ] **Step 5: Commit**

```bash
git add src/data.rs src/constants.rs src/main.rs
git commit -m "feat: port data types (PieceKind, Grade, events, etc.)"
```

---

## Task 6: Port rotation_system.rs (verbatim + Send/Sync)

**Files:**
- Create: `src/rotation_system.rs`
- Modify: `src/data.rs` (add `Kind::create`)
- Modify: `src/main.rs`

- [ ] **Step 1: Copy and adjust**

```bash
git show master:src/rotation_system.rs > src/rotation_system.rs
```

Then edit the new `src/rotation_system.rs`:

a. Replace the import line `use crate::types::{...}` with:
```rust
use crate::data::{BoardGrid, PieceKind, RotationDirection};
```

b. Replace every reference to `Board` (the type alias) with `BoardGrid`. There are several throughout the file.

c. Add `Send + Sync` to the trait bound:

```rust
pub trait RotationSystem: Send + Sync + 'static {
    fn cells(&self, kind: PieceKind, rotation: usize) -> [(i32, i32); 4];
    fn preview_y_offset(&self, kind: PieceKind) -> i32;
    fn fits(&self, board: &BoardGrid, kind: PieceKind, col: i32, row: i32, rotation: usize) -> bool {
        // body unchanged
    }
    fn try_rotate(&self, piece: &PieceState, direction: RotationDirection, board: &BoardGrid) -> Option<PieceState>;
}
```

d. The original code uses `Piece` (the data struct from the old `types.rs`). Since we are not making `Piece` a top-level type any more (the active piece becomes ECS components in Task 7), define a small `PieceState` value type *inside* `rotation_system.rs` for the trait's signature:

```rust
/// Lightweight value used by RotationSystem::try_rotate. Mirrors the active
/// piece's spatial state. Created from ECS components by callers.
#[derive(Debug, Clone, Copy)]
pub struct PieceState {
    pub kind: PieceKind,
    pub rotation: usize,
    pub col: i32,
    pub row: i32,
}
```

e. Replace every reference to `Piece` inside this file with `PieceState`. The `Ars::center_column_blocked_first` helper signature changes accordingly. Constructors (`Piece { ... }`) become `PieceState { ... }`.

f. Update the `parse_tests` module at the bottom: it imports `crate::types::Piece` — change to use `PieceState` directly:

```rust
let piece = PieceState { kind: PieceKind::T, rotation: 0, col: 3, row: 8 };
```

And change `crate::types::{BOARD_COLS, BOARD_ROWS}` to `crate::data::{BOARD_COLS, BOARD_ROWS}`.

- [ ] **Step 2: Add Kind::create in data.rs**

Edit `src/data.rs`, append:
```rust
impl Kind {
    pub fn create(self) -> Box<dyn crate::rotation_system::RotationSystem> {
        match self {
            Kind::Ars => Box::new(crate::rotation_system::Ars),
            Kind::Srs => Box::new(crate::rotation_system::Srs),
        }
    }
}
```

- [ ] **Step 3: Wire into main.rs**

Add `mod rotation_system;`.

- [ ] **Step 4: Verify build + run rotation tests**

```bash
cargo build
cargo test --lib parse_tests
```

Expected: build succeeds; all 6 `parse_tests` (parse_rotations_i_piece_ars, ars_cells_matches_const_table, srs_cells_i_piece, srs_cells_t_piece_spawn, srs_t_basic_rotation_empty_board, plus const-eval tests) pass.

- [ ] **Step 5: Commit**

```bash
git add src/rotation_system.rs src/data.rs src/main.rs
git commit -m "feat: port rotation systems with Send+Sync trait bound"
```

---

## Task 7: Define ECS components (active piece)

**Files:**
- Create: `src/components.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/components.rs**

```rust
use bevy::prelude::*;
use crate::data::PieceKind;

/// Marker for the single active piece entity.
#[derive(Component, Debug)]
pub struct ActivePiece;

#[derive(Component, Debug, Clone, Copy)]
pub struct PieceKindComp(pub PieceKind);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PiecePosition {
    pub col: i32,
    pub row: i32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceRotation(pub usize);

/// All four components needed for the active piece, bundled for spawn convenience.
#[derive(Bundle)]
pub struct ActivePieceBundle {
    pub marker: ActivePiece,
    pub kind: PieceKindComp,
    pub position: PiecePosition,
    pub rotation: PieceRotation,
}

impl ActivePieceBundle {
    pub fn new(kind: PieceKind) -> Self {
        Self {
            marker: ActivePiece,
            kind: PieceKindComp(kind),
            position: PiecePosition { col: 3, row: 0 },
            rotation: PieceRotation(0),
        }
    }
}

/// Convert ECS components into the value type used by RotationSystem.
impl PiecePosition {
    pub fn to_state(self, kind: PieceKind, rotation: usize) -> crate::rotation_system::PieceState {
        crate::rotation_system::PieceState {
            kind, rotation, col: self.col, row: self.row,
        }
    }
}
```

- [ ] **Step 2: Wire into main.rs**

Add `mod components;`.

- [ ] **Step 3: Verify**

Run: `cargo build`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/components.rs src/main.rs
git commit -m "feat: define active-piece ECS components"
```

---

## Task 8: Define the Randomizer resource

**Files:**
- Create: `src/randomizer.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/randomizer.rs**

```rust
use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use crate::data::PieceKind;

/// TGM-style randomizer. 4-piece history (initialized to [Z; 4]); up to 4 retries
/// to avoid history collisions. First piece never S, Z, or O.
#[derive(Resource)]
pub struct Randomizer {
    history: [PieceKind; 4],
    is_first: bool,
    rng: StdRng,
}

impl Randomizer {
    pub fn new() -> Self {
        Self::with_seed(rand::random())
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            history: [PieceKind::Z; 4],
            is_first: true,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn next(&mut self) -> PieceKind {
        let mut piece = self.candidate();
        for _ in 1..4 {
            if !self.history.contains(&piece) { break; }
            piece = self.candidate();
        }
        self.history.rotate_left(1);
        self.history[3] = piece;
        self.is_first = false;
        piece
    }

    fn candidate(&mut self) -> PieceKind {
        if self.is_first {
            // First piece avoids S, Z, O.
            match rand::Rng::gen_range(&mut self.rng, 0..4) {
                0 => PieceKind::I,
                1 => PieceKind::T,
                2 => PieceKind::J,
                _ => PieceKind::L,
            }
        } else {
            PieceKind::random(&mut self.rng)
        }
    }
}

impl Default for Randomizer {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 2: Wire into main.rs**

Add `mod randomizer;`.

- [ ] **Step 3: Verify**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/randomizer.rs src/main.rs
git commit -m "feat: TGM Randomizer as bevy Resource with seedable RNG"
```

---

## Task 9: Define remaining resources

**Files:**
- Create: `src/resources.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/resources.rs**

```rust
use bevy::prelude::*;
use crate::data::{
    BoardGrid, GameMode, HorizDir, InputSnapshot, Kind, PieceKind, PiecePhase, RotationDirection,
    BOARD_COLS, BOARD_ROWS,
};
use crate::rotation_system::RotationSystem;

#[derive(Resource)]
pub struct Board(pub BoardGrid);

impl Default for Board {
    fn default() -> Self { Board([[None; BOARD_COLS]; BOARD_ROWS]) }
}

#[derive(Resource)]
pub struct CurrentPhase(pub PiecePhase);

impl Default for CurrentPhase {
    fn default() -> Self { CurrentPhase(PiecePhase::Falling) }
}

#[derive(Resource)]
pub struct NextPiece(pub PieceKind);

#[derive(Resource)]
pub struct GameProgress {
    pub level: u32,
    pub lines: u32,
    pub ticks_elapsed: u64,
    pub game_over: bool,
    pub game_won: bool,
    pub score_submitted: bool,
}

impl Default for GameProgress {
    fn default() -> Self {
        Self { level: 0, lines: 0, ticks_elapsed: 0, game_over: false, game_won: false, score_submitted: false }
    }
}

#[derive(Resource, Default)]
pub struct DasState {
    pub direction: Option<HorizDir>,
    pub counter: u32,
}

#[derive(Resource, Default)]
pub struct RotationBuffer(pub Option<RotationDirection>);

#[derive(Resource, Default)]
pub struct PendingCompaction(pub Vec<usize>);

/// Per-piece state that resets on spawn.
#[derive(Resource, Default)]
pub struct DropTracking {
    pub gravity_accumulator: u32,
    pub soft_drop_frames: u32,
    pub sonic_drop_rows: u32,
}

#[derive(Resource)]
pub struct InputState(pub InputSnapshot);

impl Default for InputState {
    fn default() -> Self { InputState(InputSnapshot::empty()) }
}

#[derive(Resource)]
pub struct RotationSystemRes(pub Box<dyn RotationSystem>);

#[derive(Resource)]
pub struct GameModeRes(pub GameMode);

#[derive(Resource)]
pub struct RotationKind(pub Kind);
```

- [ ] **Step 2: Wire into main.rs**

Add `mod resources;`.

- [ ] **Step 3: Verify**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/resources.rs src/main.rs
git commit -m "feat: define ECS resources for board, phase, progress, DAS, input"
```

---

## Task 10: Port judge.rs as a Resource

**Files:**
- Create: `src/judge.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/judge.rs**

```rust
use bevy::prelude::*;
use crate::data::{Grade, HiScoreEntry, JudgeEvent};

#[derive(Resource)]
pub struct Judge {
    combo: u32,
    score: u32,
    best_grade: Grade,
    grade_ticks: u64,
}

impl Judge {
    pub fn new() -> Self {
        Self { combo: 1, score: 0, best_grade: Grade::Nine, grade_ticks: 0 }
    }

    pub fn on_event(&mut self, event: &JudgeEvent) {
        match *event {
            JudgeEvent::LockedWithoutClear => self.combo = 1,
            JudgeEvent::ClearedLines { level, cleared_playfield, num_lines,
                frames_soft_drop_held, sonic_drop_rows, ticks_elapsed } => {
                self.combo += 2 * num_lines - 2;
                let bravo = if cleared_playfield { 4 } else { 1 };
                self.score += ((level + 3) / 4 + frames_soft_drop_held + 2 * sonic_drop_rows)
                    * num_lines * self.combo * bravo;
                let new_grade = Grade::of_score(self.score);
                if new_grade > self.best_grade {
                    self.best_grade = new_grade;
                    self.grade_ticks = ticks_elapsed;
                }
            }
        }
    }

    pub fn score(&self) -> u32 { self.score }
    pub fn grade(&self) -> Grade { Grade::of_score(self.score) }
    pub fn grade_entry(&self) -> HiScoreEntry {
        HiScoreEntry { grade: self.best_grade, ticks: self.grade_ticks }
    }
}

impl Default for Judge {
    fn default() -> Self { Self::new() }
}

/// Bevy system: drains JudgeEvents and feeds them into the Judge resource.
pub fn judge_system(
    mut judge: ResMut<Judge>,
    mut events: EventReader<JudgeEvent>,
) {
    for event in events.read() {
        judge.on_event(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_event(level: u32, num_lines: u32, ticks_elapsed: u64) -> JudgeEvent {
        JudgeEvent::ClearedLines {
            level, cleared_playfield: false, num_lines,
            frames_soft_drop_held: 0, sonic_drop_rows: 0, ticks_elapsed,
        }
    }

    #[test]
    fn grade_entry_records_first_crossing() {
        let mut j = Judge::new();
        j.on_event(&clear_event(100, 4, 1000));
        let entry = j.grade_entry();
        assert!(entry.grade > Grade::Nine);
        assert_eq!(entry.ticks, 1000);
    }

    #[test]
    fn grade_entry_ticks_not_updated_on_same_grade() {
        let mut j = Judge::new();
        j.on_event(&clear_event(100, 4, 500));
        let g1 = j.grade_entry().grade;
        j.on_event(&JudgeEvent::LockedWithoutClear);
        j.on_event(&clear_event(100, 1, 999));
        let entry = j.grade_entry();
        assert_eq!(entry.grade, g1);
        assert_eq!(entry.ticks, 500);
    }

    #[test]
    fn grade_entry_initial_state() {
        let j = Judge::new();
        let entry = j.grade_entry();
        assert!(matches!(entry.grade, Grade::Nine));
        assert_eq!(entry.ticks, 0);
    }
}
```

- [ ] **Step 2: Wire into main.rs**

Add `mod judge;`.

- [ ] **Step 3: Run the unit tests**

```bash
cargo test --lib judge::
```

Expected: 3 tests pass (grade_entry_records_first_crossing, grade_entry_ticks_not_updated_on_same_grade, grade_entry_initial_state).

- [ ] **Step 4: Commit**

```bash
git add src/judge.rs src/main.rs
git commit -m "feat: port Judge as a bevy Resource with EventReader system"
```

---

## Task 11: Define AppState

**Files:**
- Create: `src/app_state.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/app_state.rs**

```rust
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Menu,
    Ready,
    Playing,
    GameOver,
}
```

- [ ] **Step 2: Wire into main.rs**

```rust
mod app_state;
use app_state::AppState;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .init_state::<AppState>()
        .run();
}
```

Remove the old `add_systems(Startup, hello_world)` and the `hello_world` fn.

- [ ] **Step 3: Verify**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/app_state.rs src/main.rs
git commit -m "feat: AppState states machine (Menu/Ready/Playing/GameOver)"
```

---

## Task 12: Tick counter system

**Files:**
- Create: `src/systems/mod.rs`
- Create: `src/systems/tick.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create the systems module**

`src/systems/mod.rs`:
```rust
pub mod tick;
```

`src/systems/tick.rs`:
```rust
use bevy::prelude::*;
use crate::resources::GameProgress;

/// Increments ticks_elapsed once per FixedUpdate. Skipped if game ended.
pub fn tick_counter(mut progress: ResMut<GameProgress>) {
    if progress.game_over || progress.game_won { return; }
    progress.ticks_elapsed += 1;
}
```

- [ ] **Step 2: Wire into main.rs**

```rust
mod systems;
use crate::app_state::AppState;
use crate::resources::*;
use crate::systems::tick::tick_counter;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_systems(FixedUpdate, tick_counter.run_if(in_state(AppState::Playing)))
        .run();
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: succeeds. Resources will be uninitialized — that's fine for now since `Playing` state isn't entered.

- [ ] **Step 4: Commit**

```bash
git add src/systems src/main.rs
git commit -m "feat: tick_counter system in FixedUpdate at 60Hz"
```

---

## Task 13: lock_piece helper function

**Files:**
- Create: `src/systems/lock_piece.rs`
- Modify: `src/systems/mod.rs`

This is a *helper function*, not a scheduled system. It is invoked from the active-phase system (Task 14) when a lock should occur. Splitting it out keeps the active-phase system focused.

- [ ] **Step 1: Create src/systems/lock_piece.rs**

```rust
use bevy::prelude::*;
use crate::components::*;
use crate::constants::{LINE_CLEAR_DELAY, SPAWN_DELAY_NORMAL};
use crate::data::{
    BOARD_COLS, BOARD_ROWS, GameEvent, GameKey, JudgeEvent, PiecePhase, RotationDirection,
};
use crate::resources::*;
use crate::rotation_system::RotationSystem;

/// Writes the active piece into the board, detects line clears, queues compaction,
/// emits events, and transitions PiecePhase.
///
/// Mirrors `Game::lock_piece` from the original game.rs.
#[allow(clippy::too_many_arguments)]
pub fn lock_piece(
    board: &mut Board,
    progress: &mut GameProgress,
    phase: &mut CurrentPhase,
    pending: &mut PendingCompaction,
    rotation_buffer: &mut RotationBuffer,
    drop_tracking: &DropTracking,
    rot_sys: &dyn RotationSystem,
    piece_kind: PieceKind,
    piece_pos: PiecePosition,
    piece_rot: PieceRotation,
    input: &InputSnapshot,
    judge_events: &mut EventWriter<JudgeEvent>,
    game_events: &mut EventWriter<GameEvent>,
) {
    use crate::data::{InputSnapshot, PieceKind};

    // 1. Write piece cells into the board.
    for (dc, dr) in rot_sys.cells(piece_kind, piece_rot.0) {
        let c = (piece_pos.col + dc) as usize;
        let r = (piece_pos.row + dr) as usize;
        if r < BOARD_ROWS {
            board.0[r][c] = Some(piece_kind);
        }
    }

    // 2. Detect cleared lines.
    let cleared: Vec<usize> = (0..BOARD_ROWS)
        .filter(|&r| board.0[r].iter().all(|c| c.is_some()))
        .collect();
    let count = cleared.len() as u32;

    if count > 0 {
        pending.0 = cleared;
        progress.lines += count;
        progress.level = (progress.level + count).min(999);
        if progress.level == 999 {
            progress.game_won = true;
            game_events.send(GameEvent::GameEnded);
        }
        game_events.send(GameEvent::LineClear { count });
    }

    // 3. Buffer held rotation for next piece.
    if input.held.contains(&GameKey::RotateCw) {
        rotation_buffer.0 = Some(RotationDirection::Clockwise);
    } else if input.held.contains(&GameKey::RotateCcw) {
        rotation_buffer.0 = Some(RotationDirection::Counterclockwise);
    }

    // 4. Phase transition: LineClearDelay or Spawning.
    phase.0 = if count > 0 {
        PiecePhase::LineClearDelay { ticks_left: LINE_CLEAR_DELAY }
    } else {
        PiecePhase::Spawning { ticks_left: SPAWN_DELAY_NORMAL }
    };

    // 5. Emit JudgeEvent.
    let judge_event = if count > 0 {
        let cleared_playfield = board.0.iter().all(|row| row.iter().all(|c| {
            // After the lines are cleared (compacted later), the playfield is "clear" iff
            // every remaining cell is either part of the cleared rows or empty.
            // Mirroring the original: it checks board_is_empty AFTER writing piece, BEFORE compaction.
            // The cleared rows are still full at this point, so we need to check rows NOT in `pending.0`.
            c.is_none()
        }));
        // Match original behavior precisely: board_is_empty is called between line-detection
        // and compaction, so cells in cleared rows are still Some(_). Result: only true if
        // the entire board is the cleared rows (i.e. there were no pre-existing cells outside
        // the clearing rows).
        let cleared_playfield = board.0.iter().enumerate().all(|(r, row)| {
            pending.0.contains(&r) || row.iter().all(|c| c.is_none())
        });
        JudgeEvent::ClearedLines {
            level: progress.level - count, // level was already incremented above; revert for the event
            cleared_playfield,
            num_lines: count,
            frames_soft_drop_held: drop_tracking.soft_drop_frames,
            sonic_drop_rows: drop_tracking.sonic_drop_rows,
            ticks_elapsed: progress.ticks_elapsed,
        }
    } else {
        JudgeEvent::LockedWithoutClear
    };
    judge_events.send(judge_event);
}
```

> **Important:** The original `Game::lock_piece` calls `board_is_empty()` **after** writing the piece but **before** compacting; cells in cleared rows are still `Some` at that moment. The original `cleared_playfield` flag therefore means "only cells on the board are the ones about to be cleared." The implementation above replicates that semantics precisely. The `level - count` arithmetic mirrors the fact that the original called `judge.on_event` *after* `clear_lines` ran, but `clear_lines` had already mutated `self.level`. We pass the pre-clear level to the judge so the score formula matches.

> **Wait — review the original carefully.** Looking at `Game::lock_piece` again: `clear_lines` mutates `self.level` to `(level + count).min(999)`, then `judge.on_event` runs with `level: self.level` — the *post-increment* level. Verify against `src/game.rs:444-455` on master before implementing. If the original passed the post-increment level, do the same here (delete the `- count` subtraction).

- [ ] **Step 2: Verify the level-passing semantics**

```bash
git show master:src/game.rs | sed -n '440,460p'
```

Read the output. Decide: post-increment level (delete the `- count` adjustment) or pre-increment (keep it). Apply the correct version. The original passes `level: self.level` AFTER `self.lines += count; self.level = (self.level + count).min(999);` — so the post-increment level is correct. **Delete the `- count` adjustment.**

Also delete the duplicate `cleared_playfield` calculation; keep only the second (correct) one.

Final cleaned snippet for the JudgeEvent block:
```rust
let judge_event = if count > 0 {
    let cleared_playfield = board.0.iter().enumerate().all(|(r, row)| {
        pending.0.contains(&r) || row.iter().all(|c| c.is_none())
    });
    JudgeEvent::ClearedLines {
        level: progress.level,
        cleared_playfield,
        num_lines: count,
        frames_soft_drop_held: drop_tracking.soft_drop_frames,
        sonic_drop_rows: drop_tracking.sonic_drop_rows,
        ticks_elapsed: progress.ticks_elapsed,
    }
} else {
    JudgeEvent::LockedWithoutClear
};
```

- [ ] **Step 3: Wire into systems/mod.rs**

```rust
pub mod tick;
pub mod lock_piece;
```

- [ ] **Step 4: Verify build**

Run: `cargo build`
Expected: succeeds. There will be unused warnings — fine for now.

- [ ] **Step 5: Commit**

```bash
git add src/systems
git commit -m "feat: lock_piece helper (board write + line detection + events)"
```

---

## Task 14: Active phase system (Falling + Locking)

**Files:**
- Create: `src/systems/active.rs`
- Modify: `src/systems/mod.rs`
- Modify: `src/main.rs`

This system runs in `FixedUpdate` when the current `PiecePhase` is `Falling` or `Locking`. It is the largest system in the project and corresponds to phases 2–7 of the original `Game::tick`.

- [ ] **Step 1: Create src/systems/active.rs**

```rust
use bevy::prelude::*;
use crate::components::*;
use crate::constants::{DAS_CHARGE, DAS_REPEAT, LOCK_DELAY, gravity_g};
use crate::data::{
    GameEvent, GameKey, HorizDir, JudgeEvent, PiecePhase, RotationDirection,
};
use crate::resources::*;
use crate::rotation_system::{PieceState, RotationSystem};
use crate::systems::lock_piece::lock_piece;

#[allow(clippy::too_many_arguments)]
pub fn active_phase_system(
    mut piece: Query<(&PieceKindComp, &mut PiecePosition, &mut PieceRotation), With<ActivePiece>>,
    mut board: ResMut<Board>,
    mut phase: ResMut<CurrentPhase>,
    mut progress: ResMut<GameProgress>,
    mut das: ResMut<DasState>,
    mut rotation_buffer: ResMut<RotationBuffer>,
    mut pending: ResMut<PendingCompaction>,
    mut drop_tracking: ResMut<DropTracking>,
    rot_sys: Res<RotationSystemRes>,
    mode: Res<GameModeRes>,
    input: Res<InputState>,
    mut judge_events: EventWriter<JudgeEvent>,
    mut game_events: EventWriter<GameEvent>,
) {
    if progress.game_over || progress.game_won { return; }
    if !matches!(phase.0, PiecePhase::Falling | PiecePhase::Locking { .. }) { return; }

    let Ok((kind, mut pos, mut rot)) = piece.single_mut() else { return };
    let kind = kind.0;
    let input = &input.0;

    // Helper: try to move active piece by (dcol, drow). Returns whether it moved.
    let try_move = |pos: &mut PiecePosition, rot: &PieceRotation, dcol, drow, board: &Board| -> bool {
        let new_col = pos.col + dcol;
        let new_row = pos.row + drow;
        if rot_sys.0.fits(&board.0, kind, new_col, new_row, rot.0) {
            pos.col = new_col;
            pos.row = new_row;
            true
        } else { false }
    };

    let try_rotate = |pos: &mut PiecePosition, rot: &mut PieceRotation, dir: RotationDirection, board: &Board| {
        let state = PieceState { kind, rotation: rot.0, col: pos.col, row: pos.row };
        if let Some(new) = rot_sys.0.try_rotate(&state, dir, &board.0) {
            pos.col = new.col;
            pos.row = new.row;
            rot.0 = new.rotation;
        }
    };

    // Phase 2: rotation
    if input.just_pressed.contains(&GameKey::RotateCw) {
        try_rotate(&mut pos, &mut rot, RotationDirection::Clockwise, &board);
    } else if input.just_pressed.contains(&GameKey::RotateCcw) {
        try_rotate(&mut pos, &mut rot, RotationDirection::Counterclockwise, &board);
    }

    // Phase 3: sonic drop
    if input.just_pressed.contains(&GameKey::SonicDrop) {
        let row_before = pos.row;
        while try_move(&mut pos, &rot, 0, 1, &board) {}
        drop_tracking.sonic_drop_rows += (pos.row - row_before) as u32;
        if matches!(phase.0, PiecePhase::Falling) {
            phase.0 = PiecePhase::Locking { ticks_left: LOCK_DELAY };
            game_events.send(GameEvent::PieceBeganLocking);
        }
        return;
    }

    // Phase 4: soft drop
    if input.held.contains(&GameKey::SoftDrop) {
        drop_tracking.soft_drop_frames += 1;
        match phase.0 {
            PiecePhase::Locking { .. } => {
                lock_piece(
                    &mut board, &mut progress, &mut phase, &mut pending,
                    &mut rotation_buffer, &drop_tracking, &*rot_sys.0,
                    kind, *pos, *rot, input,
                    &mut judge_events, &mut game_events,
                );
                return;
            }
            _ => {
                try_move(&mut pos, &rot, 0, 1, &board);
                drop_tracking.gravity_accumulator = 0;
            }
        }
    }

    // Phase 5: horizontal DAS
    let horiz = if input.held.contains(&GameKey::Left) { Some(HorizDir::Left) }
        else if input.held.contains(&GameKey::Right) { Some(HorizDir::Right) }
        else { None };

    match horiz {
        None => { das.direction = None; das.counter = 0; }
        Some(dir) => {
            if das.direction != Some(dir) {
                das.direction = Some(dir);
                das.counter = 0;
                let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                try_move(&mut pos, &rot, dcol, 0, &board);
            } else {
                das.counter += 1;
                if das.counter >= DAS_CHARGE && (das.counter - DAS_CHARGE) % DAS_REPEAT == 0 {
                    let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                    try_move(&mut pos, &rot, dcol, 0, &board);
                }
            }
        }
    }

    // Phase 6: gravity (G/256 accumulator)
    let row_before = pos.row;
    drop_tracking.gravity_accumulator += gravity_g(mode.0, progress.level);
    let drops = drop_tracking.gravity_accumulator / 256;
    drop_tracking.gravity_accumulator %= 256;
    for _ in 0..drops {
        if !try_move(&mut pos, &rot, 0, 1, &board) { break; }
    }
    let moved_down = pos.row > row_before;

    // Phase 7: lock state transitions
    let on_floor = !rot_sys.0.fits(&board.0, kind, pos.col, pos.row + 1, rot.0);
    match phase.0 {
        PiecePhase::Falling => {
            if on_floor {
                phase.0 = PiecePhase::Locking { ticks_left: LOCK_DELAY };
                game_events.send(GameEvent::PieceBeganLocking);
            }
        }
        PiecePhase::Locking { ref mut ticks_left } => {
            if !on_floor {
                phase.0 = PiecePhase::Falling;
            } else if moved_down {
                *ticks_left = LOCK_DELAY;
            } else if *ticks_left == 0 {
                lock_piece(
                    &mut board, &mut progress, &mut phase, &mut pending,
                    &mut rotation_buffer, &drop_tracking, &*rot_sys.0,
                    kind, *pos, *rot, input,
                    &mut judge_events, &mut game_events,
                );
            } else {
                *ticks_left -= 1;
            }
        }
        _ => unreachable!(),
    }
}
```

- [ ] **Step 2: Wire into systems/mod.rs and main.rs**

In `src/systems/mod.rs`:
```rust
pub mod tick;
pub mod lock_piece;
pub mod active;
```

In `src/main.rs`, register the system after `tick_counter`:
```rust
.add_systems(FixedUpdate, (
    tick_counter,
    crate::systems::active::active_phase_system,
).chain().run_if(in_state(AppState::Playing)))
```

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: clean (warnings for unused resources OK).

- [ ] **Step 4: Commit**

```bash
git add src/systems src/main.rs
git commit -m "feat: active_phase_system handling Falling+Locking phases"
```

---

## Task 15: Line clear delay + Spawning systems

**Files:**
- Create: `src/systems/line_clear_delay.rs`
- Create: `src/systems/spawning.rs`
- Modify: `src/systems/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create src/systems/line_clear_delay.rs**

```rust
use bevy::prelude::*;
use crate::constants::{ARE_DAS_FROZEN_FRAMES, SPAWN_DELAY_NORMAL};
use crate::data::{BOARD_COLS, BOARD_ROWS, GameKey, PiecePhase, RotationDirection};
use crate::resources::*;

pub fn line_clear_delay_system(
    mut phase: ResMut<CurrentPhase>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut rotation_buffer: ResMut<RotationBuffer>,
    progress: Res<GameProgress>,
    input: Res<InputState>,
) {
    if progress.game_over || progress.game_won { return; }
    let PiecePhase::LineClearDelay { ticks_left } = &mut phase.0 else { return };

    // Buffer rotation for IRS.
    if input.0.held.contains(&GameKey::RotateCw) {
        rotation_buffer.0 = Some(RotationDirection::Clockwise);
    } else if input.0.held.contains(&GameKey::RotateCcw) {
        rotation_buffer.0 = Some(RotationDirection::Counterclockwise);
    }

    if *ticks_left == 0 {
        compact_pending(&mut board.0, &mut pending.0);
        phase.0 = PiecePhase::Spawning { ticks_left: SPAWN_DELAY_NORMAL };
    } else {
        *ticks_left -= 1;
    }
}

fn compact_pending(board: &mut crate::data::BoardGrid, pending: &mut Vec<usize>) {
    if pending.is_empty() { return; }
    let mut new_board: crate::data::BoardGrid = [[None; BOARD_COLS]; BOARD_ROWS];
    let kept: Vec<[Option<crate::data::PieceKind>; BOARD_COLS]> = board.iter()
        .enumerate()
        .filter(|(r, _)| !pending.contains(r))
        .map(|(_, row)| *row)
        .collect();
    let offset = BOARD_ROWS - kept.len();
    for (i, row) in kept.into_iter().enumerate() {
        new_board[offset + i] = row;
    }
    *board = new_board;
    pending.clear();
}
```

- [ ] **Step 2: Create src/systems/spawning.rs**

```rust
use bevy::prelude::*;
use crate::components::*;
use crate::constants::{ARE_DAS_FROZEN_FRAMES, SPAWN_DELAY_NORMAL, gravity_g};
use crate::data::{GameEvent, GameKey, HorizDir, PiecePhase, RotationDirection};
use crate::randomizer::Randomizer;
use crate::resources::*;
use crate::rotation_system::{PieceState, RotationSystem};

#[allow(clippy::too_many_arguments)]
pub fn spawning_system(
    mut piece: Query<(&mut PieceKindComp, &mut PiecePosition, &mut PieceRotation), With<ActivePiece>>,
    mut phase: ResMut<CurrentPhase>,
    mut next: ResMut<NextPiece>,
    mut progress: ResMut<GameProgress>,
    mut das: ResMut<DasState>,
    mut rotation_buffer: ResMut<RotationBuffer>,
    mut drop_tracking: ResMut<DropTracking>,
    mut randomizer: ResMut<Randomizer>,
    rot_sys: Res<RotationSystemRes>,
    mode: Res<GameModeRes>,
    board: Res<Board>,
    input: Res<InputState>,
    mut game_events: EventWriter<GameEvent>,
) {
    if progress.game_over || progress.game_won { return; }
    let PiecePhase::Spawning { ticks_left } = &mut phase.0 else { return };

    // Buffer rotation for IRS, or clear if neither held.
    if input.0.held.contains(&GameKey::RotateCw) {
        rotation_buffer.0 = Some(RotationDirection::Clockwise);
    } else if input.0.held.contains(&GameKey::RotateCcw) {
        rotation_buffer.0 = Some(RotationDirection::Counterclockwise);
    } else {
        rotation_buffer.0 = None;
    }

    let tl = *ticks_left;
    if tl == 0 {
        // Spawn the next piece.
        let Ok((mut k, mut pos, mut rot)) = piece.single_mut() else { return };
        if can_piece_increment(progress.level) {
            progress.level += 1;
        }
        let next_kind = randomizer.next();
        k.0 = next.0;
        next.0 = next_kind;
        pos.col = 3;
        pos.row = 0;
        rot.0 = 0;
        drop_tracking.gravity_accumulator = 0;
        drop_tracking.soft_drop_frames = 0;
        drop_tracking.sonic_drop_rows = 0;
        phase.0 = PiecePhase::Falling;

        // Apply buffered rotation (IRS).
        if let Some(dir) = rotation_buffer.0.take() {
            let state = PieceState { kind: k.0, rotation: rot.0, col: pos.col, row: pos.row };
            if let Some(new) = rot_sys.0.try_rotate(&state, dir, &board.0) {
                pos.col = new.col;
                pos.row = new.row;
                rot.0 = new.rotation;
            }
        }

        // Game-over check.
        if !rot_sys.0.fits(&board.0, k.0, pos.col, pos.row, rot.0) {
            progress.game_over = true;
            game_events.send(GameEvent::GameEnded);
        }

        // Gravity applies immediately on spawn (matters for 20G).
        drop_tracking.gravity_accumulator += gravity_g(mode.0, progress.level);
        let drops = drop_tracking.gravity_accumulator / 256;
        drop_tracking.gravity_accumulator %= 256;
        for _ in 0..drops {
            let new_row = pos.row + 1;
            if rot_sys.0.fits(&board.0, k.0, pos.col, new_row, rot.0) {
                pos.row = new_row;
            } else { break; }
        }
    } else {
        *ticks_left -= 1;
        // DAS charges during ARE frames 5..=29 (tl in 1..=SPAWN_DELAY_NORMAL-ARE_DAS_FROZEN_FRAMES).
        if tl <= SPAWN_DELAY_NORMAL - ARE_DAS_FROZEN_FRAMES {
            let horiz = if input.0.held.contains(&GameKey::Left) { Some(HorizDir::Left) }
                else if input.0.held.contains(&GameKey::Right) { Some(HorizDir::Right) }
                else { None };
            match horiz {
                None => { das.direction = None; das.counter = 0; }
                Some(dir) => {
                    if das.direction != Some(dir) {
                        das.direction = Some(dir);
                        das.counter = 0;
                    } else {
                        das.counter += 1;
                    }
                }
            }
        }
    }
}

fn can_piece_increment(level: u32) -> bool {
    level % 100 != 99 && level != 998
}
```

- [ ] **Step 3: Wire into systems/mod.rs and main.rs**

`src/systems/mod.rs`:
```rust
pub mod tick;
pub mod lock_piece;
pub mod active;
pub mod line_clear_delay;
pub mod spawning;
```

In `src/main.rs`, expand the system tuple:
```rust
.add_systems(FixedUpdate, (
    tick_counter,
    crate::systems::active::active_phase_system,
    crate::systems::line_clear_delay::line_clear_delay_system,
    crate::systems::spawning::spawning_system,
).chain().run_if(in_state(AppState::Playing)))
```

- [ ] **Step 4: Verify build**

Run: `cargo build`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src/systems src/main.rs
git commit -m "feat: line_clear_delay and spawning phase systems"
```

---

## Task 16: Game-over check + Judge system registration

**Files:**
- Create: `src/systems/game_over.rs`
- Modify: `src/systems/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create src/systems/game_over.rs**

```rust
use bevy::prelude::*;
use crate::app_state::AppState;
use crate::resources::GameProgress;

pub fn game_over_check(
    progress: Res<GameProgress>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if (progress.game_over || progress.game_won) && *state.get() == AppState::Playing {
        next_state.set(AppState::GameOver);
    }
}
```

- [ ] **Step 2: Register events + judge_system + game_over_check in main.rs**

```rust
use crate::data::{GameEvent, JudgeEvent};
use crate::judge::{Judge, judge_system};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_event::<JudgeEvent>()
        .add_event::<GameEvent>()
        // Resources will be inserted on transition into Playing in Task 17;
        // for now insert default versions so the systems can link.
        .init_resource::<crate::resources::Board>()
        .init_resource::<crate::resources::CurrentPhase>()
        .init_resource::<crate::resources::GameProgress>()
        .init_resource::<crate::resources::DasState>()
        .init_resource::<crate::resources::RotationBuffer>()
        .init_resource::<crate::resources::PendingCompaction>()
        .init_resource::<crate::resources::DropTracking>()
        .init_resource::<crate::resources::InputState>()
        .init_resource::<crate::randomizer::Randomizer>()
        .init_resource::<Judge>()
        .add_systems(FixedUpdate, (
            tick_counter,
            crate::systems::active::active_phase_system,
            crate::systems::line_clear_delay::line_clear_delay_system,
            crate::systems::spawning::spawning_system,
            judge_system,
            crate::systems::game_over::game_over_check,
        ).chain().run_if(in_state(AppState::Playing)))
        .run();
}
```

`NextPiece`, `RotationSystemRes`, `GameModeRes`, `RotationKind` are inserted by the **start_game** transition in Task 17 — they have no sensible default. Same for `ActivePiece` entity spawn.

`mod systems` — append `pub mod game_over;` to `src/systems/mod.rs`.

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: builds; warnings expected.

- [ ] **Step 4: Commit**

```bash
git add src/systems src/main.rs
git commit -m "feat: game_over_check + register events + judge system"
```

---

## Task 17: Game start helper (insert non-default resources, spawn ActivePiece)

**Files:**
- Create: `src/start_game.rs`
- Modify: `src/main.rs`

This helper installs the resources that depend on player choices (rotation kind, game mode) and spawns the active-piece entity. Tests call this directly; the menu (Plan 2) will also call it via state transition.

- [ ] **Step 1: Create src/start_game.rs**

```rust
use bevy::prelude::*;
use crate::app_state::AppState;
use crate::components::ActivePieceBundle;
use crate::data::{GameMode, Kind, PiecePhase};
use crate::randomizer::Randomizer;
use crate::resources::*;
use crate::judge::Judge;
use crate::data::PieceKind;

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
    let prior: Vec<Entity> = world.query::<(Entity, &crate::components::ActivePiece)>()
        .iter(world).map(|(e, _)| e).collect();
    for e in prior { world.despawn(e); }

    world.spawn(ActivePieceBundle::new(active_kind));

    // Transition into Playing.
    world.resource_mut::<NextState<AppState>>().set(AppState::Playing);
}
```

- [ ] **Step 2: Wire into main.rs**

Add `mod start_game;`.

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/start_game.rs src/main.rs
git commit -m "feat: start_game helper installs resources and spawns active piece"
```

---

## Task 18: GameSnapshot + from_world

**Files:**
- Create: `src/snapshot.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/snapshot.rs**

```rust
use bevy::prelude::*;
use crate::components::*;
use crate::data::{BoardGrid, Grade, PieceKind, PiecePhase};
use crate::judge::Judge;
use crate::resources::*;
use crate::rotation_system::RotationSystem;

#[derive(Debug, Clone)]
pub struct GameSnapshot {
    pub board: BoardGrid,
    pub active_kind: Option<PieceKind>,
    pub active_cells: Option<[(i32, i32); 4]>,
    pub ghost_cells: Option<[(i32, i32); 4]>,
    pub active_preview_offsets: [(i32, i32); 4],
    pub active_preview_y_offset: i32,
    pub next_kind: PieceKind,
    pub next_preview_offsets: [(i32, i32); 4],
    pub next_preview_y_offset: i32,
    pub rows_pending_compaction: Vec<usize>,
    pub level: u32,
    pub lines: u32,
    pub ticks_elapsed: u64,
    pub score: u32,
    pub grade: Grade,
    pub game_over: bool,
    pub game_won: bool,
}

impl GameSnapshot {
    pub fn from_world(world: &mut World) -> Self {
        let phase = world.resource::<CurrentPhase>().0;
        let board = world.resource::<Board>().0;
        let progress = world.resource::<GameProgress>();
        let pending = world.resource::<PendingCompaction>().0.clone();
        let next = world.resource::<NextPiece>().0;
        let judge = world.resource::<Judge>();

        // Snapshot the rot_sys-dependent fields BEFORE we move into the borrow scope below.
        let (active_kind_val, active_pos, active_rot) = {
            let mut q = world.query_filtered::<
                (&PieceKindComp, &PiecePosition, &PieceRotation),
                With<ActivePiece>,
            >();
            let (k, p, r) = q.single(world).expect("ActivePiece entity");
            (k.0, *p, *r)
        };

        let rot_sys = world.resource::<RotationSystemRes>();

        let show_active = !matches!(phase, PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. });
        let active_offsets = rot_sys.0.cells(active_kind_val, active_rot.0);

        let (active_kind, active_cells, ghost_cells) = if show_active {
            let cells = active_offsets.map(|(dc, dr)| (active_pos.col + dc, active_pos.row + dr));
            let ghost_row = compute_ghost_row(&board, &*rot_sys.0, active_kind_val, active_rot.0, active_pos);
            let ghost = if ghost_row != active_pos.row {
                Some(active_offsets.map(|(dc, dr)| (active_pos.col + dc, ghost_row + dr)))
            } else { None };
            (Some(active_kind_val), Some(cells), ghost)
        } else {
            (None, None, None)
        };

        let next_offsets = rot_sys.0.cells(next, 0);

        GameSnapshot {
            board,
            active_kind,
            active_cells,
            ghost_cells,
            active_preview_offsets: active_offsets,
            active_preview_y_offset: rot_sys.0.preview_y_offset(active_kind_val),
            next_kind: next,
            next_preview_offsets: next_offsets,
            next_preview_y_offset: rot_sys.0.preview_y_offset(next),
            rows_pending_compaction: pending,
            level: progress.level,
            lines: progress.lines,
            ticks_elapsed: progress.ticks_elapsed,
            score: judge.score(),
            grade: judge.grade(),
            game_over: progress.game_over,
            game_won: progress.game_won,
        }
    }
}

fn compute_ghost_row(
    board: &BoardGrid,
    rot_sys: &dyn RotationSystem,
    kind: PieceKind,
    rotation: usize,
    pos: PiecePosition,
) -> i32 {
    use crate::data::{BOARD_COLS, BOARD_ROWS};
    let mut ghost_row = pos.row;
    loop {
        let next = ghost_row + 1;
        let blocked = rot_sys.cells(kind, rotation).iter().any(|&(dc, dr)| {
            let c = pos.col + dc;
            let r = next + dr;
            r >= BOARD_ROWS as i32
                || (c >= 0 && c < BOARD_COLS as i32 && r >= 0
                    && board[r as usize][c as usize].is_some())
        });
        if blocked { break; }
        ghost_row = next;
    }
    ghost_row
}
```

- [ ] **Step 2: Wire into main.rs**

Add `mod snapshot;`.

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/snapshot.rs src/main.rs
git commit -m "feat: GameSnapshot::from_world reduces ECS World to snapshot view"
```

---

## Task 19: Test harness (headless_app + helpers)

**Files:**
- Create: `src/tests/mod.rs`
- Create: `src/tests/harness.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create src/tests/mod.rs**

```rust
#[cfg(test)]
pub mod harness;
```

- [ ] **Step 2: Create src/tests/harness.rs**

```rust
use bevy::prelude::*;
use std::collections::HashSet;
use crate::app_state::AppState;
use crate::components::*;
use crate::data::{
    BoardGrid, GameKey, GameMode, InputSnapshot, Kind, PieceKind, PiecePhase,
    BOARD_COLS, BOARD_ROWS,
};
use crate::judge::Judge;
use crate::resources::*;
use crate::rotation_system::PieceState;
use crate::snapshot::GameSnapshot;
use crate::start_game::{StartGameOptions, start_game};
use crate::systems;

/// Build a headless App with all game systems registered, no rendering.
pub fn headless_app() -> App {
    use crate::data::{GameEvent, JudgeEvent};
    use crate::judge::judge_system;
    use crate::systems::tick::tick_counter;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_event::<JudgeEvent>()
        .add_event::<GameEvent>()
        .add_systems(FixedUpdate, (
            tick_counter,
            crate::systems::active::active_phase_system,
            crate::systems::line_clear_delay::line_clear_delay_system,
            crate::systems::spawning::spawning_system,
            judge_system,
            crate::systems::game_over::game_over_check,
        ).chain().run_if(in_state(AppState::Playing)));
    app
}

/// Initialize the app into Playing state with the given mode/kind/initial-piece-kind.
/// Mirrors `make_game_with` from the original tests.
pub fn start_with(app: &mut App, mode: GameMode, rotation: Kind, kind: PieceKind) {
    start_game(app.world_mut(), StartGameOptions { mode, rotation, seed: Some(0) });
    // start_game spawns via NextState — we need to apply the state transition.
    app.update();
    // Force a known active piece kind/position; tests expect these values.
    let mut q = app.world_mut().query_filtered::<
        (&mut PieceKindComp, &mut PiecePosition, &mut PieceRotation), With<ActivePiece>,
    >();
    let (mut k, mut pos, mut rot) = q.single_mut(app.world_mut()).unwrap();
    k.0 = kind;
    pos.col = 3;
    pos.row = 8;
    rot.0 = 0;
    app.world_mut().resource_mut::<Board>().0 = [[None; BOARD_COLS]; BOARD_ROWS];
    app.world_mut().resource_mut::<NextPiece>().0 = kind;
    app.world_mut().resource_mut::<CurrentPhase>().0 = PiecePhase::Falling;
}

/// Convenience: ARS, Master mode.
pub fn make_app(kind: PieceKind) -> App {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, kind);
    app
}

/// Convenience: SRS, Master mode.
pub fn make_srs_app(kind: PieceKind) -> App {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Srs, kind);
    app
}

/// Tick the FixedUpdate schedule exactly once with the given InputSnapshot.
pub fn tick_with(app: &mut App, input: InputSnapshot) {
    app.world_mut().resource_mut::<InputState>().0 = input;
    // Advance virtual time by exactly one fixed step.
    let step = std::time::Duration::from_secs_f64(1.0 / 60.0);
    app.world_mut().resource_mut::<Time<Fixed>>().advance_by(step);
    app.update();
}

/// Press a key for one tick (held + just_pressed).
pub fn press(app: &mut App, key: GameKey) {
    let mut input = InputSnapshot::empty();
    input.held.insert(key);
    input.just_pressed.insert(key);
    tick_with(app, input);
}

/// Hold keys for N ticks (no just_pressed transitions).
pub fn hold(app: &mut App, keys: &[GameKey], ticks: u32) {
    let input = InputSnapshot {
        held: keys.iter().copied().collect(),
        just_pressed: HashSet::new(),
    };
    for _ in 0..ticks { tick_with(app, input.clone()); }
}

/// Tick N times with no input.
pub fn idle(app: &mut App, ticks: u32) {
    for _ in 0..ticks { tick_with(app, InputSnapshot::empty()); }
}

/// Parse `.`/`O` ASCII into a board (bottom-aligned).
pub fn board_from_ascii(diagram: &str) -> BoardGrid {
    let mut board = [[None; BOARD_COLS]; BOARD_ROWS];
    let lines: Vec<&str> = diagram.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let offset = BOARD_ROWS.saturating_sub(lines.len());
    for (i, line) in lines.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if c < BOARD_COLS {
                board[offset + i][c] = if ch == 'O' { Some(PieceKind::O) } else { None };
            }
        }
    }
    board
}

// ----- Accessors that hide the world query for tests -----

pub fn snapshot(app: &mut App) -> GameSnapshot {
    GameSnapshot::from_world(app.world_mut())
}

pub fn active_kind(app: &mut App) -> PieceKind {
    let mut q = app.world_mut().query_filtered::<&PieceKindComp, With<ActivePiece>>();
    q.single(app.world_mut()).unwrap().0
}

pub fn active_position(app: &mut App) -> PiecePosition {
    let mut q = app.world_mut().query_filtered::<&PiecePosition, With<ActivePiece>>();
    *q.single(app.world_mut()).unwrap()
}

pub fn active_rotation(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<&PieceRotation, With<ActivePiece>>();
    q.single(app.world_mut()).unwrap().0
}

pub fn board(app: &mut App) -> BoardGrid {
    app.world().resource::<Board>().0
}

pub fn set_board(app: &mut App, b: BoardGrid) {
    app.world_mut().resource_mut::<Board>().0 = b;
}

pub fn piece_phase(app: &mut App) -> PiecePhase {
    app.world().resource::<CurrentPhase>().0
}

pub fn judge<'a>(app: &'a App) -> &'a Judge {
    app.world().resource::<Judge>()
}

pub fn level(app: &App) -> u32 { app.world().resource::<GameProgress>().level }
pub fn lines(app: &App) -> u32 { app.world().resource::<GameProgress>().lines }
pub fn ticks_elapsed(app: &App) -> u64 { app.world().resource::<GameProgress>().ticks_elapsed }
pub fn game_over(app: &App) -> bool { app.world().resource::<GameProgress>().game_over }
pub fn game_won(app: &App) -> bool { app.world().resource::<GameProgress>().game_won }

/// Translate active piece + rotation system into absolute board cells.
pub fn active_abs(app: &mut App) -> Vec<(i32, i32)> {
    let kind = active_kind(app);
    let pos = active_position(app);
    let rot = active_rotation(app);
    let cells = app.world().resource::<RotationSystemRes>().0.cells(kind, rot);
    cells.into_iter().map(|(dc, dr)| (pos.col + dc, pos.row + dr)).collect()
}
```

- [ ] **Step 3: Wire into main.rs**

Add at the bottom of `src/main.rs`:
```rust
#[cfg(test)]
mod tests;
```

- [ ] **Step 4: Verify build + cargo test compiles**

Run: `cargo test --no-run`
Expected: tests build cleanly. Existing `judge::tests` and rotation `parse_tests` still pass:

```bash
cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src/tests src/main.rs
git commit -m "test: headless App harness with test helpers"
```

---

## Task 20: Port one canonical test (proof of harness)

**Files:**
- Create: `src/tests/movement_tests.rs`
- Modify: `src/tests/mod.rs`

- [ ] **Step 1: Pick a small representative test from the original tests.rs**

Read the original test `gravity_drops_piece_each_frame_at_1g` (or similar — find one that uses `idle` and asserts on `active_position` only). Use `git show master:src/tests.rs` and grep for one.

```bash
git show master:src/tests.rs | grep -n "fn .*test" | head -40
```

Pick a small test that exercises the basic flow.

- [ ] **Step 2: Create src/tests/movement_tests.rs**

Port one test verbatim, using the new helpers. Example using `hold_left_das_charges_then_repeats`:

```rust
use crate::data::*;
use crate::tests::harness::*;

#[test]
fn left_press_moves_one_column() {
    let mut app = make_app(PieceKind::T);
    let col_before = active_position(&mut app).col;
    press(&mut app, GameKey::Left);
    assert_eq!(active_position(&mut app).col, col_before - 1);
}
```

If the equivalent test in the original is `left_press_moves_one_column`, copy its assertions verbatim. Otherwise pick the smallest equivalent test.

- [ ] **Step 3: Wire into tests/mod.rs**

```rust
#[cfg(test)]
pub mod harness;
#[cfg(test)]
mod movement_tests;
```

- [ ] **Step 4: Run the test**

Run: `cargo test --lib left_press_moves_one_column`
Expected: PASS.

If it fails, the harness has a bug. **Do not proceed to Task 21 until this test is green.** Common issues:
- `tick_with` may need `App::update()` called twice on first tick (once to apply NextState, once to actually run FixedUpdate).
- `start_game` runs in `Update` schedule indirectly via state transition — may need explicit `app.world_mut().run_schedule(StateTransition)`.

Iterate on the harness until the test is green.

- [ ] **Step 5: Commit**

```bash
git add src/tests
git commit -m "test: first ported test (left_press_moves_one_column) — harness proven"
```

---

## Task 21: Port test helper functions (board_lines, side_by_side, rotation_snap, wall_kick_snap)

**Files:**
- Modify: `src/tests/harness.rs`

These helpers are used by many of the snapshot tests. Port them as a unit before bulk-porting tests.

- [ ] **Step 1: Read the originals**

```bash
git show master:src/tests.rs | sed -n '78,250p'
```

Identify: `rotation_snap`, `board_lines`, `side_by_side`, `wall_kick_snap`. Note they take a `&Game` and access `game.active`, `game.board`, `game.rotation_system`. Replace those accesses with the new accessor functions.

- [ ] **Step 2: Append to src/tests/harness.rs**

Port verbatim, with these substitutions:
- `&Game` parameter → `&mut App`
- `game.active.kind` → `active_kind(app)`
- `game.active.col` / `.row` / `.rotation` → `active_position(app).col` / `.row` / `active_rotation(app)`
- `game.board[r][c]` → `board(app)[r][c]` (cache the result with `let b = board(app);` at the start of any function that reads cells)
- `game.rotation_system.cells(...)` → `app.world().resource::<RotationSystemRes>().0.cells(...)`
- `make: fn(PieceKind) -> Game` parameter type → `make: fn(PieceKind) -> App`

Example signature change:
```rust
fn board_lines(app: &mut App, prev_cells: &[(i32, i32)]) -> Vec<String> { ... }
fn rotation_snap(kind: PieceKind, make: fn(PieceKind) -> App) -> String { ... }
fn wall_kick_snap(kind: PieceKind, make: fn(PieceKind) -> App) -> String { ... }
fn side_by_side(boards: &[(String, Vec<String>)]) -> String { ... }  // unchanged
```

- [ ] **Step 3: Verify build**

Run: `cargo test --no-run`
Expected: builds cleanly with no warnings about unused helpers (they'll be used in Task 22).

- [ ] **Step 4: Commit**

```bash
git add src/tests/harness.rs
git commit -m "test: port board_lines/side_by_side/rotation_snap/wall_kick_snap helpers"
```

---

## Task 22: Bulk-port remaining tests

**Files:**
- Create: `src/tests/{judge_tests,rotation_tests,lock_tests,line_clear_tests,spawn_tests,gravity_tests,snapshot_tests}.rs`
- Modify: `src/tests/movement_tests.rs` (port remaining movement tests)
- Modify: `src/tests/mod.rs`

This task is a mechanical translation following a fixed rule. Do **not** improvise; the goal is to preserve the existing `insta::assert_snapshot!` strings verbatim so that snapshot equivalence proves behavioral equivalence.

### Translation rule

For each `#[test] fn name() { ... }` in the original `src/tests.rs`:

1. Identify which category it belongs to (judge / rotation / lock / line clear / spawn / gravity / snapshot / movement) by reading its first 5–10 lines.
2. Append it to the appropriate `src/tests/<category>_tests.rs` file with these substitutions:

| Original | New |
|---|---|
| `let mut game = make_game(K);` | `let mut app = make_app(K);` |
| `let mut game = make_srs_game(K);` | `let mut app = make_srs_app(K);` |
| `let mut game = make_game_with(M, R, K);` | `let mut app = headless_app(); start_with(&mut app, M, R, K);` |
| `press(&mut game, key)` | `press(&mut app, key)` |
| `hold(&mut game, keys, n)` | `hold(&mut app, keys, n)` |
| `idle(&mut game, n)` | `idle(&mut app, n)` |
| `game.active.kind` | `active_kind(&mut app)` |
| `game.active.col` | `active_position(&mut app).col` |
| `game.active.row` | `active_position(&mut app).row` |
| `game.active.rotation` | `active_rotation(&mut app)` |
| `game.board` | `board(&mut app)` |
| `game.board = b;` | `set_board(&mut app, b);` |
| `game.snapshot()` | `snapshot(&mut app)` |
| `game.judge.<x>()` | `judge(&app).<x>()` |
| `game.piece_phase` | `piece_phase(&mut app)` |
| `game.level` | `level(&app)` |
| `game.lines` | `lines(&app)` |
| `game.ticks_elapsed` | `ticks_elapsed(&app)` |
| `game.game_over` | `game_over(&app)` |
| `game.game_won` | `game_won(&app)` |
| `game.try_move(c, r)` | (inline: query active piece, attempt to update; this method only appears in setup helpers — replace by mutating components directly via `set_active(&mut app, ...)` helper added if needed) |
| `game.fits(...)` | `app.world().resource::<RotationSystemRes>().0.fits(&board(&mut app), ...)` |
| `rotation_snap(K, make_game)` | `rotation_snap(K, make_app)` |
| `wall_kick_snap(K, make_srs_game)` | `wall_kick_snap(K, make_srs_app)` |
| Asserts and `insta::assert_snapshot!(_, @"...")` strings | **unchanged** — preserve exactly |

Imports at the top of each new file:
```rust
use crate::data::*;
use crate::tests::harness::*;
```

### Test inventory

Run this to enumerate all tests in the original:
```bash
git show master:src/tests.rs | grep -E "^\s*fn \w" | grep -v "^\s*fn (board_from|board_lines|active_abs|side_by_side|rotation_snap|wall_kick_snap|make_game|press|hold|idle)" | nl
```

Expected: ~60–80 tests. Distribute by name pattern:
- `*judge*` / `*score*` / `*grade*` → `judge_tests.rs`
- `*rotate*` / `*kick*` / `*wall*` / `*srs*` / `*ars*` → `rotation_tests.rs`
- `*lock*` → `lock_tests.rs`
- `*line*` / `*clear*` / `*compact*` → `line_clear_tests.rs`
- `*spawn*` / `*irs*` / `*are*` / `*next*` → `spawn_tests.rs`
- `*gravity*` / `*20g*` / `*soft_drop*` / `*sonic*` → `gravity_tests.rs`
- `*snapshot*` / `*ghost*` / `*preview*` → `snapshot_tests.rs`
- everything else (left/right/das/movement) → `movement_tests.rs`

### Procedure

- [ ] **Step 1: Create all category files with module-level imports**

```bash
for f in judge rotation lock line_clear spawn gravity snapshot; do
  cat > "src/tests/${f}_tests.rs" <<'EOF'
use crate::data::*;
use crate::tests::harness::*;
EOF
done
```

`movement_tests.rs` already exists from Task 20.

- [ ] **Step 2: Wire all into tests/mod.rs**

```rust
#[cfg(test)]
pub mod harness;
#[cfg(test)] mod movement_tests;
#[cfg(test)] mod judge_tests;
#[cfg(test)] mod rotation_tests;
#[cfg(test)] mod lock_tests;
#[cfg(test)] mod line_clear_tests;
#[cfg(test)] mod spawn_tests;
#[cfg(test)] mod gravity_tests;
#[cfg(test)] mod snapshot_tests;
```

- [ ] **Step 3: Port tests in batches of ~10, running cargo test after each batch**

Strategy: open `git show master:src/tests.rs` in a pager. For each test, find the right destination file, paste the test body, apply the table substitutions above, then:

```bash
cargo test --lib <new_test_name>
```

When a batch of 10 tests is green, commit:
```bash
git add src/tests/
git commit -m "test: port batch — <category> tests <N>-<M>"
```

- [ ] **Step 4: Run the full suite**

Run: `cargo test`
Expected: all tests pass. If snapshot strings differ, the behavior diverges from the original — investigate. Do not run `cargo insta accept` to silently accept snapshot diffs at this stage; preserving snapshots is how we prove behavioral equivalence.

- [ ] **Step 5: Final commit**

```bash
git add src/tests
git commit -m "test: complete port of tests.rs to bevy-driven harness"
```

---

## Task 23: Plan-1 cleanup + checkpoint

**Files:**
- Modify: `CLAUDE.md` (note Plan 1 progress)

- [ ] **Step 1: Confirm green state**

```bash
cargo build
cargo build --target wasm32-unknown-unknown
cargo test
```

All three must succeed.

- [ ] **Step 2: Tag the checkpoint**

```bash
git tag bevy-migration-plan1-complete
```

- [ ] **Step 3: Note that CLAUDE.md will be fully refreshed in Plan 3**

No CLAUDE.md edits required here — Plan 3's cleanup task will rewrite it once the entire migration lands. This avoids documenting transitional state.

- [ ] **Step 4: Stop**

Plan 1 complete. The next plan ([2026-04-25-bevy-migration-2-rendering.md](2026-04-25-bevy-migration-2-rendering.md)) adds rendering + menu.
