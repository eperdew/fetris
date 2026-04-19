# Hi Scores System Design

**Date:** 2026-04-14

## Overview

Persistent, per-combination hi score leaderboards stored in `localStorage` (via `quad-storage`) and displayed on the existing Hi Scores menu screen. Anonymous entries ranked by grade descending, then time ascending.

## Data Model

```rust
// src/hiscores.rs
#[derive(Serialize, Deserialize, Clone)]
pub struct HiScoreEntry {
    pub grade: Grade,   // best grade reached during the run
    pub ticks: u64,     // ticks_elapsed when that grade was first crossed
}
```

- `Grade` gains `#[derive(Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]`
- Ranking: higher `Grade` wins; ties broken by lower `ticks`
- Each leaderboard holds at most 5 entries, kept in sorted order (best first)
- 4 leaderboards, one per `(GameMode, rotation::Kind)` combination:
  - Master + ARS
  - Master + SRS
  - 20G + ARS
  - 20G + SRS

## Storage Module (`src/hiscores.rs`)

```rust
pub fn load(mode: GameMode, rotation: Kind) -> Vec<HiScoreEntry>
pub fn save(mode: GameMode, rotation: Kind, entries: Vec<HiScoreEntry>)
pub fn submit(mode: GameMode, rotation: Kind, entry: HiScoreEntry) -> Vec<HiScoreEntry>
```

- Storage key: `"hi_master_ars"`, `"hi_master_srs"`, `"hi_20g_ars"`, `"hi_20g_srs"`
- Serialization: `serde_json` — value is a JSON array of entry objects
- `submit` loads, inserts in sorted order, truncates to 5, saves, returns updated list
- Load failures (missing key, parse error) silently return empty vec

Dependencies to add to `Cargo.toml`:
- `quad-storage` (use latest available version — verify on crates.io at implementation time)
- `serde = { version = "1", features = ["derive"] }`
- `serde_json = "1"`

## Grade Tracking in `Judge`

Add to `Judge`:
```rust
best_grade: Grade,     // highest grade reached so far
grade_ticks: u64,      // ticks_elapsed when best_grade was first crossed
```

- `Judge::on_event` receives `ticks_elapsed: u64` alongside the existing event data (add to `JudgeEvent::ClearedLines`)
- After updating score, check if `Grade::of_score(self.score) > self.best_grade`; if so, update both fields
- `judge.grade_entry() -> HiScoreEntry` returns `HiScoreEntry { grade: self.best_grade, ticks: self.grade_ticks }`

## Submission Flow

In `main.rs`, in the `AppState::Playing` branch of the game loop:

1. Add `score_submitted: bool` to `Game` (initialized `false`)
2. Each frame, after `game.tick()`: if `(game.game_over || game.game_won) && !game.score_submitted`:
   - Call `hiscores::submit(game.game_mode, rotation_kind, game.judge.grade_entry())`
   - Set `game.score_submitted = true`

The rotation kind is stored on `Game` alongside the rotation system box (add a `rotation_kind: Kind` field).

## Hi Scores UI

### Menu changes (`src/menu.rs`)

- Add `hi_scores_tab: usize` field to `Menu` (0–3, cycles through the 4 combos in order: Master+ARS, Master+SRS, 20G+ARS, 20G+SRS)
- On entering `MenuScreen::HiScores`, load all 4 leaderboards and store them on `Menu`
- Left/right on the HiScores screen cycles `hi_scores_tab`

### Renderer changes (`src/renderer.rs`)

Replace the `render_subscreen("HI SCORES")` stub with `render_hi_scores(menu)`:

- Tab header: e.g. `"< MASTER / ARS >"` centered at top, with left/right arrows implying cyclability
- 5 rows below: `"1.  S3   08:14.233"` (rank, grade, formatted time)
- Empty slots: `"---"`
- Back reminder: `"BKSP to go back"` at bottom

Time formatting reuses the existing `format_time(ticks: u64) -> String` from `renderer.rs`.

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `quad-storage`, `serde`, `serde_json` |
| `src/judge.rs` | Add `best_grade`, `grade_ticks` fields; update `on_event`; add `grade_entry()` |
| `src/hiscores.rs` | New module: `HiScoreEntry`, `load`, `save`, `submit` |
| `src/game.rs` | Add `score_submitted`, `rotation_kind` fields |
| `src/menu.rs` | Add `hi_scores_tab`, leaderboard data; handle left/right on HiScores screen |
| `src/renderer.rs` | Replace HiScores stub with `render_hi_scores` |
| `src/main.rs` | Submit score on game end; pass `rotation_kind` to `Game::new` |
