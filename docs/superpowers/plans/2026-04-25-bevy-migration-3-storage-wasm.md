# Bevy Migration Plan 3: Storage, WASM, Deploy & Cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace stub storage with `bevy_pkv`, ship a working WASM build via `trunk`, deploy it to GitHub Pages, then delete dead files and update `CLAUDE.md`.

**Architecture:** `bevy_pkv` provides a typed key-value store backed by `sled` natively and `localStorage` on WASM. `Res<PkvStore>` replaces both the stub `HiScoresRes`/`GameConfigRes`/`MutedRes` resources and the bespoke `src/storage.rs`. The web shell becomes a `trunk` template. Final commit deletes `src/storage.rs`, `src/audio_player.rs`, `web/mq_js_bundle.js`, `web/fetris-storage.js`, the stub-storage module, and updates `CLAUDE.md`.

**Tech Stack:** `bevy_pkv`, `trunk`, `wasm-bindgen-cli`, `binaryen` (`wasm-opt`), GitHub Actions.

**Reads spec:** [docs/superpowers/specs/2026-04-25-bevy-migration-design.md](../specs/2026-04-25-bevy-migration-design.md). This plan covers spec phases 6–8 ("Storage", "WASM + deploy pipeline", "Cleanup").

**Builds on:** [Plan 1](2026-04-25-bevy-migration-1-logic.md) and [Plan 2](2026-04-25-bevy-migration-2-render-menu.md). The native game must be playable end-to-end before starting.

---

## Pre-flight

**Worktree / branch:** still `.worktrees/bevy-migration` from Plans 1–2.

**Verify Plan 2 status:**

```bash
cd .worktrees/bevy-migration
cargo test
cargo run --release
```

Walk the manual checklist from Plan 2 Task 15. If any item fails, fix it in Plan 2 before starting Plan 3.

**Out of scope:**
- Visual / gameplay tuning

**Tooling note:** trunk and wasm-bindgen-cli are CLI tools installed via `cargo install`. The first install is several minutes. Get this kicked off early.

---

## Task 1: Add bevy_pkv

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dep**

```toml
bevy_pkv = "0.13"
```

(Pin to whatever version is current at start of work.)

- [ ] **Step 2: Cargo build**

```bash
cargo build
```

Expected: succeeds. (`bevy_pkv` pulls in `sled` for native, `web-sys` shim for WASM.)

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat(storage): add bevy_pkv dependency"
```

---

## Task 2: Initialize PkvStore as a resource

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Insert PkvStore at startup**

```rust
use bevy_pkv::PkvStore;

// In main():
.insert_resource(PkvStore::new("fetris", "fetris"))
```

`PkvStore::new(organization, app_name)` creates a `sled` DB under the OS app-data dir natively, or uses `localStorage` on WASM. Pick names that match what we want as a long-term identity — "fetris"/"fetris" is fine.

- [ ] **Step 2: Build + run native**

```bash
cargo run
```

Expected: window opens, game still works, no panic. A `~/.local/share/fetris/fetris/` directory (or platform equivalent) gets created with sled DB files.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat(storage): insert PkvStore resource"
```

---

## Task 3: Hi-scores wrapper module backed by PkvStore

**Files:**
- Create: `src/hiscores.rs` (replaces the existing one — overwrite)
- Modify: `src/main.rs`

The original [src/hiscores.rs:14-18](../../../src/hiscores.rs#L14-L18) has a `storage_key(mode, rotation)` function returning a `&'static str` per slot — preserve those exact keys so any user with existing localStorage data still sees their scores.

- [ ] **Step 1: Replace src/hiscores.rs**

```rust
use bevy::prelude::*;
use bevy_pkv::PkvStore;
use serde::{Deserialize, Serialize};
use crate::data::{GameMode, HiScoreEntry, Kind};

const MAX_ENTRIES: usize = 5;

fn storage_key(mode: GameMode, rotation: Kind) -> &'static str {
    match (mode, rotation) {
        (GameMode::Master, Kind::Ars) => "hi_master_ars",
        (GameMode::Master, Kind::Srs) => "hi_master_srs",
        (GameMode::TwentyG, Kind::Ars) => "hi_20g_ars",
        (GameMode::TwentyG, Kind::Srs) => "hi_20g_srs",
    }
}

pub fn load(pkv: &PkvStore, mode: GameMode, rotation: Kind) -> Vec<HiScoreEntry> {
    pkv.get::<Vec<HiScoreEntry>>(storage_key(mode, rotation)).unwrap_or_default()
}

pub fn save(pkv: &mut PkvStore, mode: GameMode, rotation: Kind, entries: &Vec<HiScoreEntry>) {
    let _ = pkv.set(storage_key(mode, rotation), entries);
}

pub fn submit(pkv: &mut PkvStore, mode: GameMode, rotation: Kind, entry: HiScoreEntry) {
    let mut entries = load(pkv, mode, rotation);
    insert_entry(&mut entries, entry, MAX_ENTRIES);
    save(pkv, mode, rotation, &entries);
}

pub fn insert_entry(entries: &mut Vec<HiScoreEntry>, entry: HiScoreEntry, max: usize) {
    entries.push(entry);
    entries.sort_by(|a, b| b.grade.cmp(&a.grade).then(a.ticks.cmp(&b.ticks)));
    entries.truncate(max);
}

#[cfg(test)]
mod tests {
    // PRESERVE the existing 4 tests from src/hiscores.rs:54-97 verbatim.
    // They test insert_entry directly and don't touch storage at all, so they
    // port unchanged. Copy the entire `mod tests { ... }` block from master.
}
```

`HiScoreEntry` already derives `Serialize`/`Deserialize` (verify with `grep "HiScoreEntry" src/types.rs`). If not, add `#[derive(Serialize, Deserialize)]`.

- [ ] **Step 2: Wire submit into game-over**

In [src/main.rs:166-174](../../../src/main.rs#L166-L174) (master), `hiscores::submit` is called once on game end. The bevy version: an `OnEnter(AppState::GameOver)` system. Add to `main.rs`:

```rust
fn submit_score_on_game_over(
    mut pkv: ResMut<PkvStore>,
    judge: Res<crate::judge::Judge>,
    progress: Res<crate::resources::GameProgress>,
    config: Res<crate::stub_storage::GameConfigRes>, // Plan 2's stub; Task 4 below moves this onto PkvStore
) {
    let entry = judge.grade_entry();
    crate::hiscores::submit(&mut pkv, config.game_mode, config.rotation, entry);
    let _ = progress; // unused once we use grade_entry; keep param if grade_entry doesn't carry ticks
}

// In main():
.add_systems(OnEnter(AppState::GameOver), submit_score_on_game_over)
```

Verify the exact "what gets submitted" semantics by reading [src/main.rs:166-174](../../../src/main.rs#L166-L174) and `Judge::grade_entry()` in master — adjust accordingly.

- [ ] **Step 3: Hi-scores screen reads from PkvStore directly**

Update `src/menu/hi_scores.rs` to read from `Res<PkvStore>` instead of `Res<HiScoresRes>`:

```rust
pub fn hi_scores_system(
    mut contexts: EguiContexts,
    mut menu: ResMut<MenuState>,
    keys: Res<ButtonInput<KeyCode>>,
    pkv: Res<PkvStore>,
) {
    // ...
    let entries = match menu.hi_scores_tab {
        0 => crate::hiscores::load(&pkv, GameMode::Master, Kind::Ars),
        1 => crate::hiscores::load(&pkv, GameMode::Master, Kind::Srs),
        2 => crate::hiscores::load(&pkv, GameMode::TwentyG, Kind::Ars),
        3 => crate::hiscores::load(&pkv, GameMode::TwentyG, Kind::Srs),
        _ => vec![],
    };
    // ... rest unchanged
}
```

(Loading on every frame is fine — sled is fast and the screen is gated by `MenuScreen::HiScores`.)

- [ ] **Step 4: Run + test**

```bash
cargo test
cargo run
```

Manual: play a game to game-over, return to menu, navigate to HI SCORES → verify your score appears in the right tab. Quit and re-launch → verify it persisted.

- [ ] **Step 5: Commit**

```bash
git add src/hiscores.rs src/menu/ src/main.rs
git commit -m "feat(storage): hi-scores backed by PkvStore"
```

---

## Task 4: GameConfig and Mute persistence

**Files:**
- Modify: `src/main.rs` (replace stub_storage usage)
- Modify: `src/menu/main_screen.rs`
- Modify: `src/systems/global_input.rs`

The original game persists `game_config` (the chosen mode + rotation, so the menu remembers your last selection) and `muted` (the audio mute flag). Move both to PkvStore.

- [ ] **Step 1: Verify serializable types exist**

Plan 1 already declares `GameConfig` in `src/data.rs` with `serde::Serialize` + `serde::Deserialize` derives, plus `GameMode` and `Kind` with the same derives. No type changes needed in this step. If `GameConfig` is missing `Default`, add a `#[derive(Default)]` and pick `GameMode::Master` / `Kind::Ars` as the `#[default]` variants.

- [ ] **Step 2: Initial menu state loads from PkvStore**

Change `MenuState::default` to `MenuState::new(pkv: &PkvStore)`:

```rust
impl MenuState {
    pub fn new(pkv: &PkvStore) -> Self {
        let config: GameConfig = pkv.get("game_config").unwrap_or_default();
        Self {
            screen: MenuScreen::Main,
            cursor: 0,
            game_mode: config.game_mode,
            rotation: config.rotation,
            hi_scores_tab: 0,
        }
    }
}
```

Replace `app.init_resource::<MenuState>()` (added in Plan 2) with a startup system:

```rust
fn init_menu_state(mut commands: Commands, pkv: Res<PkvStore>) {
    commands.insert_resource(MenuState::new(&pkv));
}

// In main():
.add_systems(Startup, init_menu_state)
```

(Order: this Startup system must run after the `PkvStore` is inserted. `insert_resource` from `App` is available at `Startup`, so it's fine.)

- [ ] **Step 3: Save GameConfig on START**

In `src/menu/main_screen.rs`, when `start_game = true`:

```rust
let _ = pkv.set("game_config", &crate::data::GameConfig {
    game_mode: menu.game_mode,
    rotation: menu.rotation,
});
next_state.set(crate::AppState::Ready);
```

Add `mut pkv: ResMut<PkvStore>` to the system signature. Remove the `mut config: ResMut<GameConfigRes>` parameter.

- [ ] **Step 4: Mute persists**

In `src/systems/global_input.rs`, replace `Res<MutedRes>` with `Res<PkvStore>`:

```rust
pub fn handle_global_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut pkv: ResMut<PkvStore>,
    /* other params */
) {
    if keys.just_pressed(KeyCode::KeyM) {
        let muted: bool = pkv.get("muted").unwrap_or(false);
        let _ = pkv.set("muted", &!muted);
    }
    // ... rest unchanged
}
```

Update the menu's mute display similarly — `pkv.get::<bool>("muted").unwrap_or(false)` in `main_menu_system`.

- [ ] **Step 5: Test persistence**

```bash
cargo run
```

Manual: change GAME MODE to 20G, ROTATION to SRS, press M to mute, START a game, quit. Re-launch: menu should remember 20G + SRS + muted.

- [ ] **Step 6: Commit**

```bash
git add src/types.rs src/menu/ src/systems/ src/main.rs
git commit -m "feat(storage): persist game config and mute via PkvStore"
```

---

## Task 5: Delete stub storage module

**Files:**
- Delete: `src/stub_storage.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Verify nothing still references it**

```bash
```
<!-- Use Grep tool: pattern "stub_storage" -->

Search for any remaining references to `stub_storage`, `HiScoresRes`, `GameConfigRes`, `MutedRes`, `slot_index`. There should be none after Task 4.

- [ ] **Step 2: Delete the file**

```bash
git rm src/stub_storage.rs
```

Remove `mod stub_storage;` and the three `init_resource` lines from `src/main.rs`.

- [ ] **Step 3: Build + test**

```bash
cargo build
cargo test
cargo run
```

All green; manual smoke-test the menu still works.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "refactor: remove stub storage module"
```

---

## Task 6: Delete legacy storage.rs

> **Note:** `src/storage.rs` was already deleted during Plans 1–2. Verify, then skip.

**Files:**
- Delete: `src/storage.rs` (likely already gone)

- [ ] **Step 1: Check if it still exists**

```bash
ls src/storage.rs 2>/dev/null && echo "EXISTS" || echo "ALREADY GONE"
```

If "ALREADY GONE": no work needed. Mark done and continue.

If it exists: search for remaining references (`crate::storage`, `storage::Storage`, `mod storage`), remove them, then `git rm src/storage.rs`, build, test, and commit as `"refactor: delete bespoke storage module (replaced by bevy_pkv)"`.

---

## Task 7: Implement audio via bevy_audio

**Files:**
- Modify: `Cargo.toml` (add `bevy_audio`, `vorbis`, `wav` features)
- Create: `src/audio.rs`
- Modify: `src/judge.rs` (emit `GradeAdvanced` from `judge_system`)
- Modify: `src/main.rs` (register module + systems)

The spec says: "bevy_audio (built-in) — replaces macroquad audio. The `AudioPlayer` trait is **deleted**; tests omit the audio plugin from their headless `App`." `audio_player.rs` was already deleted in the worktree. This task adds the bevy_audio-based replacement.

Sounds and their triggers:

| Sound file | Trigger |
|---|---|
| `piece_begin_locking.wav` | `GameEvent::PieceBeganLocking` |
| `single.ogg` | `GameEvent::LineClear { count: 1 }` |
| `double.ogg` | `GameEvent::LineClear { count: 2 }` |
| `triple.ogg` | `GameEvent::LineClear { count: 3 }` |
| `fetris.ogg` | `GameEvent::LineClear { count: 4+ }` |
| `grade_*.ogg` | `GameEvent::GradeAdvanced(grade)` |
| `game_over.ogg` | `GameEvent::GameEnded` |
| `ready.ogg` | `OnEnter(AppState::Ready)` |

Mute state: read from `PkvStore` key `"muted"` (Task 4 wires this up). All playback is skipped when muted.

- [ ] **Step 1: Add audio features to Cargo.toml**

Change:
```toml
bevy = { version = "0.18", default-features = false, features = ["2d", "bevy_state"] }
```
to:
```toml
bevy = { version = "0.18", default-features = false, features = ["2d", "bevy_state", "bevy_audio", "vorbis", "wav"] }
```

`bevy_audio` enables `AudioPlugin` in `DefaultPlugins`. `vorbis` decodes `.ogg` files. `wav` decodes `.wav` files.

- [ ] **Step 2: Verify build**

```bash
cargo build
```

Expected: succeeds. If cargo complains about an unknown feature name, verify exact feature names against:

```bash
cat ~/.cargo/registry/src/**/bevy-0.18.*/Cargo.toml | grep -A 200 "^\[features\]" | head -50
```

- [ ] **Step 3: Emit GradeAdvanced from judge_system**

`GameEvent::GradeAdvanced(Grade)` already exists in `src/data.rs` but `judge_system` never emits it. Fix that now.

In `src/judge.rs`, change the import and `judge_system` function:

```rust
// Change import line:
use crate::data::{GameEvent, Grade, HiScoreEntry, JudgeEvent};

// Change judge_system:
pub fn judge_system(
    mut judge: ResMut<Judge>,
    mut judge_events: MessageReader<JudgeEvent>,
    mut game_events: MessageWriter<GameEvent>,
) {
    for event in judge_events.read() {
        let before = judge.grade();
        judge.on_event(event);
        let after = judge.grade();
        if after > before {
            game_events.write(GameEvent::GradeAdvanced(after));
        }
    }
}
```

`judge.grade()` returns `Grade::of_score(self.score)` which only ever increases, so this correctly fires once per grade threshold crossed.

- [ ] **Step 4: Run tests**

```bash
cargo test
```

Expected: all green. The headless test harness registers `MinimalPlugins` and doesn't register `AudioPlugin`, so no audio-related failures.

- [ ] **Step 5: Create src/audio.rs**

```rust
use bevy::audio::{AudioPlayer, PlaybackSettings};
use bevy::prelude::*;
use bevy_pkv::PkvStore;
use crate::data::{GameEvent, Grade};

#[derive(Resource)]
pub struct AudioHandles {
    pub piece_begin_locking: Handle<AudioSource>,
    pub ready: Handle<AudioSource>,
    pub single: Handle<AudioSource>,
    pub double: Handle<AudioSource>,
    pub triple: Handle<AudioSource>,
    pub fetris: Handle<AudioSource>,
    pub game_over: Handle<AudioSource>,
    // Index 0 = Grade::Nine (worst), 17 = Grade::SNine (best) — matches original audio_player.rs ordering
    pub grades: Vec<Handle<AudioSource>>,
}

pub fn setup_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let grade_files = [
        "audio/grade_9.ogg", "audio/grade_8.ogg", "audio/grade_7.ogg",
        "audio/grade_6.ogg", "audio/grade_5.ogg", "audio/grade_4.ogg",
        "audio/grade_3.ogg", "audio/grade_2.ogg", "audio/grade_1.ogg",
        "audio/grade_s1.ogg", "audio/grade_s2.ogg", "audio/grade_s3.ogg",
        "audio/grade_s4.ogg", "audio/grade_s5.ogg", "audio/grade_s6.ogg",
        "audio/grade_s7.ogg", "audio/grade_s8.ogg", "audio/grade_s9.ogg",
    ];
    commands.insert_resource(AudioHandles {
        piece_begin_locking: asset_server.load("audio/piece_begin_locking.wav"),
        ready: asset_server.load("audio/ready.ogg"),
        single: asset_server.load("audio/single.ogg"),
        double: asset_server.load("audio/double.ogg"),
        triple: asset_server.load("audio/triple.ogg"),
        fetris: asset_server.load("audio/fetris.ogg"),
        game_over: asset_server.load("audio/game_over.ogg"),
        grades: grade_files.iter().map(|f| asset_server.load(*f)).collect(),
    });
}

fn grade_handle(handles: &AudioHandles, grade: Grade) -> Handle<AudioSource> {
    let idx = match grade {
        Grade::Nine => 0, Grade::Eight => 1, Grade::Seven => 2, Grade::Six => 3,
        Grade::Five => 4, Grade::Four => 5, Grade::Three => 6, Grade::Two => 7,
        Grade::One => 8, Grade::SOne => 9, Grade::STwo => 10, Grade::SThree => 11,
        Grade::SFour => 12, Grade::SFive => 13, Grade::SSix => 14, Grade::SSeven => 15,
        Grade::SEight => 16, Grade::SNine => 17,
    };
    handles.grades[idx].clone()
}

pub fn audio_event_system(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    handles: Res<AudioHandles>,
    pkv: Res<PkvStore>,
) {
    if pkv.get::<bool>("muted").unwrap_or(false) {
        return;
    }
    for event in events.read() {
        let handle: Handle<AudioSource> = match event {
            GameEvent::PieceBeganLocking => handles.piece_begin_locking.clone(),
            GameEvent::LineClear { count } => match count {
                1 => handles.single.clone(),
                2 => handles.double.clone(),
                3 => handles.triple.clone(),
                _ => handles.fetris.clone(),
            },
            GameEvent::GradeAdvanced(grade) => grade_handle(&handles, *grade),
            GameEvent::GameEnded => handles.game_over.clone(),
        };
        commands.spawn((AudioPlayer::new(handle), PlaybackSettings::DESPAWN));
    }
}

pub fn play_ready_sound(
    mut commands: Commands,
    handles: Res<AudioHandles>,
    pkv: Res<PkvStore>,
) {
    if !pkv.get::<bool>("muted").unwrap_or(false) {
        commands.spawn((AudioPlayer::new(handles.ready.clone()), PlaybackSettings::DESPAWN));
    }
}
```

`PlaybackSettings::DESPAWN` auto-despawns the audio entity when playback finishes, preventing entity accumulation.

If `bevy::audio::AudioPlayer` import doesn't compile, try `use bevy::prelude::AudioPlayer;` — bevy re-exports it in prelude.

- [ ] **Step 6: Wire up in main.rs**

Add `mod audio;` to the module declarations.

In `main()`, add:

```rust
.add_systems(Startup, (setup_camera, audio::setup_audio))
.add_systems(OnEnter(AppState::Ready), (start_game_on_ready, audio::play_ready_sound))
.add_systems(
    Update,
    audio::audio_event_system.run_if(in_state(AppState::Playing)),
)
```

Keep the existing `start_game_on_ready` registration; change it to a tuple to include `play_ready_sound`. The audio event system only needs to run while playing since `GameEvent`s are only emitted in `FixedUpdate` during that state.

- [ ] **Step 7: Build + run + verify audio**

```bash
cargo run --release
```

Manual checklist:
- [ ] Start a game — `ready.ogg` plays during the READY countdown
- [ ] Piece begins locking — tick sound plays
- [ ] Clear 1 line — `single.ogg` plays
- [ ] Clear 2 lines — `double.ogg` plays
- [ ] Clear 3 lines — `triple.ogg` plays
- [ ] Clear 4 lines — `fetris.ogg` plays
- [ ] Grade advance (score crosses a threshold) — corresponding `grade_N.ogg` plays
- [ ] Game over — `game_over.ogg` plays
- [ ] Press M to mute — audio stops; subsequent events are silent
- [ ] Press M again to unmute — sounds resume

If sounds don't play at all: confirm `bevy_audio` feature is in `Cargo.toml` and `DefaultPlugins` is used (not `MinimalPlugins`) in `main()`.

- [ ] **Step 8: Commit**

```bash
git add src/audio.rs src/judge.rs src/main.rs Cargo.toml Cargo.lock
git commit -m "feat(audio): wire up bevy_audio with event-driven sound system"
```

---

## Task 8: Add cargo wasm-release profile

**Files:**
- Modify: `Cargo.toml`

Per spec: `[profile.wasm-release]` with `opt-level = "z"`, `lto = true`, `codegen-units = 1`.

- [ ] **Step 1: Add the profile**

Append to `Cargo.toml`:

```toml
[profile.wasm-release]
inherits = "release"
opt-level = "z"
lto = true
codegen-units = 1
```

- [ ] **Step 2: Verify it parses**

```bash
cargo build --profile wasm-release
```

(Native build with this profile to confirm `Cargo.toml` parses. WASM target comes in Task 10.)

Expected: long compile, succeeds. Binary is in `target/wasm-release/`.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat(wasm): add wasm-release cargo profile"
```

---

## Task 9: Remove the stale .cargo/config.toml workaround

**Files:**
- Modify: `.cargo/config.toml`

Per spec: "the existing 'no getrandom / no wasm-bindgen' workaround is **removed** — bevy requires both, properly."

- [ ] **Step 1: Read current contents**

[.cargo/config.toml](../../../.cargo/config.toml) — confirm it only contains the comment from current master:

```
# No special WASM configuration needed — macroquad's built-in rand handles
# randomness on all targets without requiring getrandom or wasm-bindgen.
```

- [ ] **Step 2: Replace the comment with bevy-relevant config (or delete the file)**

If empty after removing the comment, delete it:

```bash
git rm .cargo/config.toml
```

If you discover bevy needs cargo-level config later (e.g., a target-specific rustflag), put that here. Otherwise leave the file deleted.

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: remove obsolete .cargo/config.toml comment"
```

---

## Task 10: Set up trunk + wasm toolchain locally

**Files:** none (toolchain installation)

- [ ] **Step 1: Add the wasm32 target if missing**

```bash
rustup target add wasm32-unknown-unknown
```

- [ ] **Step 2: Install trunk**

```bash
cargo install --locked trunk
```

(~2-5 minutes.) Verify: `trunk --version`.

- [ ] **Step 3: Install wasm-bindgen-cli matching the bevy crate version**

Check the version in `Cargo.lock`:

```bash
grep "name = \"wasm-bindgen\"" -A 1 Cargo.lock | head
```

Install the matching CLI:

```bash
cargo install --locked wasm-bindgen-cli --version <version-from-cargo-lock>
```

(Trunk picks this up automatically. Mismatched versions cause runtime errors.)

- [ ] **Step 4: Install binaryen for wasm-opt**

macOS:

```bash
brew install binaryen
```

Verify: `wasm-opt --version`.

- [ ] **Step 5: No commit — these are environment changes only.**

---

## Task 11: Trunk template (web/index.html)

**Files:**
- Modify: `web/index.html` (rewrite as trunk template)
- Delete: `web/mq_js_bundle.js`, `web/fetris-storage.js`

- [ ] **Step 1: Replace web/index.html**

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <title>fetris</title>
  <link data-trunk rel="rust" data-bin="fetris" data-cargo-features="" data-wasm-opt="z" />
  <link data-trunk rel="copy-dir" href="../assets" />
  <style>
    html, body { margin: 0; padding: 0; background: #0a0a12; height: 100%; overflow: hidden; }
    body { display: flex; align-items: center; justify-content: center; }
    canvas { display: block; }
  </style>
</head>
<body>
</body>
</html>
```

The `data-trunk rel="rust"` line tells trunk to compile the workspace's `fetris` binary to wasm. `data-wasm-opt="z"` runs `wasm-opt -Oz` post-build. The `copy-dir` line ensures the `assets/` directory is included in the dist output (bevy's AssetServer reads from there at runtime).

**Note about `data-cargo-profile`:** trunk reads the profile via `--release` flag plus `Trunk.toml`. The wasm-release profile from Task 8 needs explicit selection — see Task 12.

- [ ] **Step 2: Delete the macroquad shell scripts**

```bash
git rm web/mq_js_bundle.js web/fetris-storage.js
```

- [ ] **Step 3: Commit**

```bash
git add web/
git commit -m "feat(wasm): trunk index.html template; delete macroquad shell"
```

---

## Task 12: Trunk.toml configuration

**Files:**
- Create: `Trunk.toml`

- [ ] **Step 1: Create Trunk.toml at repo root**

```toml
[build]
target = "web/index.html"
dist = "dist"
release = true
cargo_profile = "wasm-release"

[serve]
address = "127.0.0.1"
port = 8080
```

`cargo_profile = "wasm-release"` instructs trunk to use the profile we added in Task 8 instead of the default `release`.

- [ ] **Step 2: Commit**

```bash
git add Trunk.toml
git commit -m "feat(wasm): Trunk.toml with wasm-release profile"
```

---

## Task 13: First WASM build via trunk

**Files:** none

- [ ] **Step 1: Run trunk build**

```bash
trunk build --release
```

Expected: long build (5–15 min on first run, much of it cargo). Output in `dist/`:
- `index.html` (with hashed asset references)
- `fetris-<hash>.js` (wasm-bindgen glue)
- `fetris-<hash>.wasm` (the compiled, opt-z, wasm-opt-passed binary)
- `assets/` (copied via the `copy-dir` directive)

```bash
ls -lh dist/
```

Verify the .wasm is somewhere in the 8–15 MB range as predicted by the spec. If it's significantly larger, double-check `cargo_profile = "wasm-release"` in Trunk.toml is being honored.

- [ ] **Step 2: Diagnose common failures**

| Error | Likely cause |
|---|---|
| `wasm-bindgen-cli version mismatch` | Re-install wasm-bindgen-cli at the version reported in the error |
| `assets/font/Oxanium-Regular.ttf: not found` | `copy-dir` not configured, or asset path wrong |
| `linker errors involving rand or getrandom` | `rand` may need `wasm-bindgen` feature; add `getrandom = { version = "0.2", features = ["js"] }` to Cargo.toml |
| `panicked at "no event loop available"` at runtime | bevy needs `winit` with the `web` feature — bevy enables this by default but verify `DefaultPlugins` is being used |

- [ ] **Step 3: Don't commit dist/ — add it to .gitignore**

```bash
echo "/dist" >> .gitignore
git add .gitignore
git commit -m "chore: gitignore dist/"
```

---

## Task 14: Local WASM smoke test via trunk serve

**Files:** none

- [ ] **Step 1: Serve the build**

```bash
trunk serve --release
```

This builds, then serves at `http://127.0.0.1:8080`. Open it.

- [ ] **Step 2: Manual checklist (in browser)**

- [ ] Page loads; canvas appears
- [ ] Main menu renders correctly
- [ ] Keyboard input works (arrow keys, hjkl, Space, Enter, Backspace, X, Z, M)
- [ ] Click into the canvas if needed for keyboard focus
- [ ] **Audio gate:** the spec notes "the existing menu's first input is the gate." After pressing any key to navigate the menu, start a game and verify sounds play. If bevy warns about autoplay before the first input, that's expected; audio should work after the first keypress.
- [ ] Start a game; play; clear lines; particles render; score updates; game over
- [ ] Hi-scores persist across page reloads (open devtools → Application → Local Storage → check the `fetris/fetris/*` keys exist)
- [ ] Mute (M) persists across reloads

- [ ] **Step 3: If anything broke**

The most likely issues:
- bevy_pkv on WASM uses `localStorage` but only for keys it knows about; if `pkv.get::<T>(key)` is failing silently due to JSON deserialization, log the error and inspect what's in localStorage.
- Canvas focus / keyboard input: bevy 0.18+ should handle this; if not, add a small JS bootstrap to focus the canvas.
- Frame rate may be lower than native — that's normal under wasm-opt-z.

Fix anything broken inline. Iterate `trunk serve` cycles until clean.

- [ ] **Step 4: No commit** (no source changes, just verification)

If you DID need source changes to make WASM work, commit those:

```bash
git add -A
git commit -m "fix(wasm): <what was broken>"
```

---

## Task 15: Update GitHub Actions workflow

**Files:**
- Modify: `.github/workflows/deploy.yml`

- [ ] **Step 1: Replace deploy.yml**

```yaml
name: Deploy to GitHub Pages

on:
  push:
    branches: [master]

jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            ~/.cargo/bin
            target
          key: ${{ runner.os }}-cargo-bevy-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-bevy-

      - name: Install trunk
        run: |
          if ! command -v trunk &> /dev/null; then
            cargo install --locked trunk
          fi

      - name: Install wasm-bindgen-cli
        run: |
          WBG_VERSION=$(grep -A 1 'name = "wasm-bindgen"' Cargo.lock | grep version | head -1 | cut -d'"' -f2)
          if ! wasm-bindgen --version 2>/dev/null | grep -q "$WBG_VERSION"; then
            cargo install --locked wasm-bindgen-cli --version "$WBG_VERSION"
          fi

      - name: Install binaryen
        run: |
          sudo apt-get update
          sudo apt-get install -y binaryen

      - name: Build WASM via trunk
        run: trunk build --release

      - name: Deploy to gh-pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./dist
```

Cache key changed from `cargo` to `cargo-bevy` to avoid colliding with cached macroquad builds. Also caches `~/.cargo/bin` so trunk + wasm-bindgen-cli persist across runs.

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/deploy.yml
git commit -m "ci: trunk-based WASM build and deploy"
```

- [ ] **Step 3: Don't push yet** — the merge to master is the final step (Task 18). The CI run on that merge is the production validation.

---

## Task 16: CLAUDE.md rewrite

**Files:**
- Modify: `CLAUDE.md`

Per spec section "CLAUDE.md Updates Required."

- [ ] **Step 1: Rewrite the source layout table**

Replace the existing table at [CLAUDE.md "Source layout"](../../../CLAUDE.md) with the new module structure:

| File | Purpose |
|---|---|
| `src/main.rs` | App setup, plugin registration, `States` declaration, top-level systems |
| `src/data.rs` | Pure data types: `PieceKind`, `BoardGrid`, `PiecePhase`, `GameKey`, `Kind`, `GameMode`, `Grade`, `GameEvent`, `JudgeEvent`, `HiScoreEntry`, `GameConfig`, `MenuScreen` |
| `src/components.rs` | ECS components on the active-piece entity: `ActivePiece`, `PieceKindComp`, `PiecePosition`, `PieceRotation` |
| `src/resources.rs` | Resources holding game state: `Board(BoardGrid)`, `CurrentPhase`, `NextPiece`, `GameProgress`, `DasState`, `RotationBuffer`, `PendingCompaction`, `DropTracking`, `InputState`, `RotationSystemRes`, `GameModeRes`, `RotationKind` |
| `src/constants.rs` | Tuning constants: gravity table, delays, particle/animation timings |
| `src/rotation_system.rs` | `RotationSystem` trait + `Ars` and `Srs` impls; stored as `RotationSystemRes(Box<dyn RotationSystem>)` |
| `src/randomizer.rs` | TGM history-based piece bag (Resource) |
| `src/judge.rs` | TGM scoring; `Judge` is a Resource; consumes `JudgeEvent`s |
| `src/hiscores.rs` | Per-(mode, rotation) hi-score persistence backed by `bevy_pkv` |
| `src/audio.rs` | `bevy_audio` event-driven sound system; `AudioHandles` resource; mute via `PkvStore` |
| `src/systems/` | Game-logic systems running in `FixedUpdate` at 60 Hz: `input`, `gravity`, `lock`, `line_clear`, `spawn`, `judge`, `game_over_check`, `global_input`, `post_game` |
| `src/render/` | Rendering systems running in `Update`: `board`, `piece`, `particles`, `overlays`, `hud`, `assets` |
| `src/menu/` | bevy_egui menu screens: `main_screen`, `hi_scores`, `controls`, `state` |
| `src/tests.rs` | Headless tests using `MinimalPlugins` + `GameSnapshot::from_world`; `insta` inline snapshots only |

- [ ] **Step 2: Rewrite the architecture section**

Replace with bevy-specific text:

```markdown
## Architecture

**Bevy `App`** with `DefaultPlugins`, `bevy_egui::EguiPlugin`, and game-logic plugins. Game state lives in resources (`Board`, `Judge`, `Randomizer`, `InputState`, `GameProgress`, `Box<dyn RotationSystem>`) plus an active-piece *entity* with `PieceKindComp` / `PiecePosition` / `PieceRotation` / `PiecePhase` components.

**`AppState` machine** uses bevy `States`: `Menu` → `Ready` → `Playing` → `GameOver` → `Menu`. Systems are gated with `run_if(in_state(...))`.

**Schedules:**
- `FixedUpdate` at 60 Hz — all game logic.
- `Update` — rendering, input sampling, particle motion, menu UI.

**Tick model:** `Time::<Fixed>::from_hz(60.0)` keeps game logic decoupled from frame rate; bevy runs `FixedUpdate` zero or more times per frame to catch up.

**Piece phases** (`PiecePhase` component): `Falling`, `Locking { ticks_left }`, `LineClearDelay { ticks_left }`, `Spawning { ticks_left }`. Phase transitions drive timing logic.

**IRS (Initial Rotation System)**: holding rotation keys during the previous piece's spawn delay (or pre-game Ready countdown) causes the next piece to spawn pre-rotated. Folded into the `spawn` system.

**Gravity**: fractional G/256 system — gravity accumulates per tick from `MASTER_GRAVITY_TABLE` in `constants.rs` (TGM1 values).

**Game / Renderer separation**: render systems read snapshot data from the `Board` resource and active-piece entity. They never write back. Particles are entities with `Particle` + `Sprite` + `Transform`; spawned by an `EventReader<GameEvent>` system handling `GameEvent::LineClear`, ticked in `FixedUpdate`.

**Rotation systems**: `RotationSystem` trait (`Send + Sync`) with `Ars` and `Srs` impls, stored as `Resource<Box<dyn RotationSystem>>`. Hi-scores tracked separately per rotation system.

**Scoring**: `Judge` resource consumes `JudgeEvent`s emitted by game-logic systems and tracks score, combo, and best `Grade` reached.

**Hi-scores & config**: stored via `bevy_pkv::PkvStore` (sled native, localStorage on WASM). Per-(GameMode, Kind) slot, top 5 by grade. Storage keys preserved from the macroquad version for backward compatibility with existing user data.

**Audio**: `bevy_audio` (built-in) plays sounds in response to `GameEvent`s (`PieceBeganLocking`, `LineClear`, `GradeAdvanced`, `GameEnded`) and on `OnEnter(AppState::Ready)`. Mute state persists via `PkvStore`. No `AudioPlayer` trait — direct bevy_audio calls in `src/audio.rs`.
```

- [ ] **Step 3: Rewrite the WASM section**

```markdown
## WASM target

Build via trunk:

```sh
trunk build --release
```

Output is in `dist/` — `index.html`, `fetris-<hash>.js`, `fetris-<hash>.wasm`, `assets/`. The `wasm-release` cargo profile (in `Cargo.toml`) compiles with `opt-level = "z"`, `lto = true`, `codegen-units = 1`. Trunk runs `wasm-opt -Oz` post-build via the `data-wasm-opt` attribute in `web/index.html`.

For local iteration: `trunk serve --release` builds and serves at `http://127.0.0.1:8080`.

`.github/workflows/deploy.yml` installs trunk + wasm-bindgen-cli + binaryen, runs `trunk build --release`, deploys `dist/` to GitHub Pages on every push to `master`.

**Gotchas**:
- `wasm-bindgen-cli` must match the `wasm-bindgen` crate version in `Cargo.lock` exactly. Mismatches cause runtime errors. The CI workflow auto-detects and installs the right version.
- `bevy_pkv` on WASM uses `localStorage`; data is per-origin and survives page reloads but not domain changes.
- Bevy + wasm-opt-z produces a ~10MB binary. Acceptable trade-off for small fetris.
```

- [ ] **Step 4: Update build & test commands**

```markdown
## Build & test

```sh
cargo build
cargo test
cargo run --release
trunk build --release       # WASM build
trunk serve --release       # WASM dev server at localhost:8080
```
```

- [ ] **Step 5: Update conventions if needed**

The "tests use `insta` for snapshot assertions — always inline" line stays. The pre-commit hook line stays. The worktree convention stays.

Add a note about the bevy version pin:

```markdown
- Bevy and bevy_egui versions are pinned in `Cargo.toml`. Don't bump them as part of unrelated changes — bevy ecosystem versioning churns and a casual upgrade can break a lot of code.
```

- [ ] **Step 6: Update the "Maintaining this file" section**

Add to the "Update when changing" list:
- Bevy plugin set, new resources/components added at the app-wide level
- Storage backend (e.g., switching away from bevy_pkv)
- The `AppState` variants

- [ ] **Step 7: Read it back, fix anything that no longer makes sense**

```bash
```
<!-- Use the Read tool on CLAUDE.md to read it after edits and make sure no line lies. -->

- [ ] **Step 8: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: rewrite CLAUDE.md for bevy migration"
```

---

## Task 17: Final acceptance — native + WASM end-to-end

**Files:** none (verification only)

- [ ] **Step 1: Native check**

```bash
cargo test
cargo run --release
```

Walk Plan 2 Task 15's checklist again. All items still pass. Plus: hi-scores persist across launches, and all audio cues play correctly (ready sound, lock tick, line clears, grade advances, game over, mute toggle).

- [ ] **Step 2: WASM check**

```bash
trunk serve --release
```

Walk the WASM checklist from Task 14 again. All items still pass.

- [ ] **Step 3: Inspect for dead code**

```bash
cargo build 2>&1 | grep "warning" | head -50
```

Address any `unused import`, `dead_code`, or `unused variable` warnings. Don't suppress with `#[allow]` — delete the dead code.

- [ ] **Step 4: cargo fmt**

```bash
cargo fmt
git diff
```

If anything changed, commit:

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Task 18: Merge to master

**Files:** none

- [ ] **Step 1: Switch to master and pull**

```bash
cd /Users/eperdew/Software/fetris  # primary worktree
git pull
```

- [ ] **Step 2: Merge the migration branch**

```bash
git merge --no-ff <bevy-migration-branch-name>
```

`--no-ff` keeps the branch structure visible in history.

- [ ] **Step 3: Push and watch CI**

```bash
git push origin master
```

Open the GitHub Actions tab. The deploy workflow should:
1. Install Rust + wasm32 target
2. Restore cache (will be cold the first time — long build)
3. Install trunk + wasm-bindgen-cli + binaryen
4. Run `trunk build --release` (long)
5. Deploy `dist/` to gh-pages

Total time: ~15–25 min on first run, ~5 min on cached subsequent runs.

- [ ] **Step 4: Verify deployment**

Open the GitHub Pages URL after the workflow completes. Walk the WASM checklist one more time, in production.

- [ ] **Step 5: Clean up worktree**

If everything works:

```bash
git worktree remove .worktrees/bevy-migration
git branch -d <bevy-migration-branch-name>
```

If something failed in production: leave the worktree, fix the issue, push a follow-up.

- [ ] **Step 6: Done.**

The macroquad → bevy migration is complete.
