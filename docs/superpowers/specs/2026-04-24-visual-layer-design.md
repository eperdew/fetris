# Visual Layer Design

**Date:** 2026-04-24
**Status:** Approved

## Goal

Decouple the renderer from game internals, make the renderer stateful so it can own persistent visual effects, and introduce two new effect systems: an improved particle burst on line clear and a scanline-shader text overlay for multi-line clear celebrations.

---

## 1. Event System

`Game` accumulates events internally during each `tick` into a `Vec<GameEvent>`. The caller drains them after each tick via:

```rust
pub fn drain_events(&mut self) -> Vec<GameEvent>
```

`GameEvent` starts with one variant:

```rust
pub enum GameEvent {
    LineClear { count: u32 }, // 1–4
}
```

Designed for extension — `Bravo`, `GradeUp`, etc. can be added later without breaking callers.

Single-line clears emit the event (for particles) but do not trigger a text overlay. The overlay only activates for count ≥ 2.

---

## 2. GameSnapshot

`Game` exposes a method:

```rust
pub fn snapshot(&self) -> GameSnapshot
```

`GameSnapshot` is a plain data struct the renderer consumes instead of reaching into `Game` fields directly:

```rust
pub struct GameSnapshot {
    pub board: Board,
    pub active: Option<Piece>,          // None during Spawning / LineClearDelay
    pub ghost_row: Option<i32>,         // None when active is None
    pub next: Piece,
    pub piece_phase: PiecePhase,
    pub rows_pending_compaction: Vec<usize>,
    pub level: u32,
    pub lines: u32,
    pub ticks_elapsed: u64,
    pub score: u32,
    pub grade: Grade,
    pub game_over: bool,
    pub game_won: bool,
}
```

`ghost_row` is computed inside `snapshot()` — the logic currently in `renderer.rs::compute_ghost_row` moves to `Game` as a private helper.

`Game`'s internal fields (`gravity_accumulator`, `das_counter`, `randomizer`, `rotation_buffer`, `soft_drop_frames`, `sonic_drop_rows`, `score_submitted`) become private. The renderer never reads them.

---

## 3. Stateful Renderer

`Renderer` grows owned effect state:

```rust
pub(crate) struct Renderer {
    cell_texture: Texture2D,
    font: Font,
    particles: Vec<Particle>,
    overlay: Option<LineClearOverlay>,
    overlay_target: RenderTarget,
    overlay_material: Material,
}
```

`overlay_target` and `overlay_material` are allocated once in `Renderer::new()` and reused every frame.

`render` takes `&GameSnapshot` and `&[GameEvent]`. It processes events first (spawning particles, setting overlay), then draws. This keeps `Game` untouched during rendering and the call site in `main.rs` simple.

---

## 4. Particle System

### Data

```rust
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: u32,
    lifetime: u32,
    color: Color,
}
```

`Renderer` holds `particles: Vec<Particle>`. Each call to `render` advances all particles (apply velocity, apply gravity, increment age), removes expired ones, then draws survivors.

### Spawning

Particles are spawned when a `LineClear` event arrives, using `rows_pending_compaction` from the snapshot to know which rows to burst.

Per cell in a cleared row:
- Base direction: outward from the horizontal center of the board, with a slight upward bias
- Random spread: ±30° from base direction
- Random speed: sampled from a range scaled by `count` (more lines = faster burst)
- Upward vy kick so particles arc before falling
- Lifetime: base + small random jitter so the burst doesn't all die at once

For count=4 (Fetris), spawn ~3× as many particles per cell and at higher speed than count=1.

### Rendering

Particles are drawn with `draw_cell_at` (the existing helper). Opacity fades linearly from 1.0 to 0.0 over the particle's lifetime.

---

## 5. Text Overlay

### Data

```rust
enum OverlayKind {
    Double,
    Triple,
    Fetris,
}

struct LineClearOverlay {
    kind: OverlayKind,
    frames_remaining: u32,
}
```

Total overlay lifetime: 90 frames (~1.5 seconds).

Labels:
- `Double` → "DOUBLE"
- `Triple` → "TRIPLE"
- `Fetris` → "FETRIS"

### Scanline Shader

A custom GLSL fragment shader is stored as a `const &str` in `renderer.rs`. It receives:
- `sampler2D` texture (the RenderTarget the text was drawn to)
- `uniform float frame_parity` (0.0 or 1.0, flips each frame)
- `uniform float hue_shift` (0.0–1.0, used for Fetris rainbow; 0.0 = no shift for Double/Triple)
- `uniform float opacity` (0.0–1.0, drives fade-out)

The shader:
1. Discards the fragment if `mod(floor(gl_FragCoord.y), 2.0) != frame_parity` — this produces the alternating scanline flicker
2. Applies hue rotation to the sampled color (for Fetris)
3. Multiplies alpha by `opacity`

### Rendering Flow

Each frame with an active overlay:
1. Set render target to `overlay_target`, clear to transparent
2. Draw the label text (white, large font) centered over the playfield
3. Restore default render target
4. Activate `overlay_material`, set uniforms (`frame_parity`, `hue_shift`, `opacity`)
5. Draw `overlay_target` as a texture over the playfield area
6. Deactivate material

`hue_shift` advances by a fixed amount each frame for `Fetris` (e.g. `frames_elapsed * 0.03 % 1.0`). `opacity` = `frames_remaining / 90.0`. `frame_parity` = `frames_remaining % 2`.

---

## 6. Data Flow Summary

```
Game::tick(input)
  → accumulates GameEvent into internal Vec
  → state changes

main.rs per tick:
  snapshot = game.snapshot()
  events = game.drain_events()

renderer.render(&snapshot, &events)
  → process events → spawn particles, set overlay
  → advance particle physics, decrement overlay
  → draw board from snapshot (no Game fields touched)
  → draw particles
  → draw overlay via RenderTarget + shader
```

---

## 7. What Does Not Change

- `Renderer::render_menu`, `render_hi_scores`, `render_controls` — these don't touch `Game` so they need no changes
- `Renderer::render_ready` — currently takes `&Game`; it reads `game.active`, `game.score()`, and `game.grade()`. It will be updated to take `&GameSnapshot` for consistency
- `Judge`, `hiscores`, `storage`, `menu`, `audio_player` — untouched
- `rotation_system` — untouched
- The `Game::tick` signature — still takes `&InputState`, return type stays `()`

---

## 8. Files Changed

| File | Change |
|---|---|
| `src/types.rs` | Add `GameSnapshot`, `GameEvent` |
| `src/game.rs` | Add `snapshot()`, `drain_events()`, move ghost calc, make internals private |
| `src/renderer.rs` | Add particle/overlay state, `RenderTarget`, `Material`, shader const; `render` and `render_ready` take snapshot + events |
| `src/main.rs` | Call `snapshot()` and `drain_events()` each tick, pass to `render` |
