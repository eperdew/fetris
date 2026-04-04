# macroquad Frontend Design

## Context

The terminal frontend works well for development but has several limitations that prevent wider distribution:

- Players need a compatible terminal with keyboard enhancement support; rendering varies terminal-to-terminal
- No path to browser hosting — the primary distribution goal is a shareable link anyone can click and play
- Visual improvements like ghost pieces and per-cell background shading are awkward in a character grid
- Audio is possible in a terminal but would be non-standard

This design replaces the terminal frontend (`main.rs`, `renderer.rs`) with a macroquad-based frontend that compiles to both native and WebAssembly from a single codebase. All game logic is untouched.

## Chosen Approach

**macroquad** — a simple Rust 2D game framework (cross-platform, WASM-first, built-in audio path). Chosen over raw wasm-bindgen (too much boilerplate) and Bevy (ECS model would require restructuring game logic; overkill for this scope).

## Architecture

### What changes

| File | Change |
|---|---|
| `src/main.rs` | Replaced — macroquad async loop replaces threads + mpsc channel |
| `src/renderer.rs` | Replaced — ratatui widgets replaced with `draw_rectangle` / `draw_text` |

### What stays untouched

`game.rs`, `piece.rs`, `randomizer.rs`, `constants.rs`, `input.rs`, `tests.rs`

The `GameKey` / `InputState` abstraction in `input.rs` remains the bridge between frontend and game logic. Only the code that populates `InputState` changes.

### New dependency

Add `macroquad` to `Cargo.toml`. Remove `ratatui`, `crossterm`.

## Main Loop

macroquad uses a single-threaded async loop. The threading and channel complexity in the current `main.rs` goes away entirely:

```rust
#[macroquad::main("fetris")]
async fn main() {
    let mut game = Game::new();
    loop {
        let input = build_input_state();  // query macroquad key state
        game.tick(&input);
        render(&game);
        next_frame().await;
    }
}
```

The loop runs at the display refresh rate (typically 60Hz), matching the current 16ms tick target.

## Input

macroquad provides:
- `is_key_down(KeyCode)` — true while key is held → populates `InputState::held`
- `is_key_pressed(KeyCode)` — true only on the first frame → populates `InputState::just_pressed`

This maps directly onto the existing `InputState` struct with no changes. The crossterm keyboard enhancement protocol (needed for reliable key-release in the terminal) is not required — macroquad handles this correctly on all platforms including WASM.

Full key mapping is unchanged: arrow keys / hjl for movement, x/z for rotation, space for sonic drop, q/Esc to quit.

## Rendering

### Layout

Fixed pixel layout, window sized to fit:

- **Cell size:** 32×32px
- **Board:** 10 cols × 20 rows = 320×640px, with a 1px border
- **Sidebar:** 160px wide, right of the board — next piece preview, level, lines, elapsed time
- **Total window:** ~500×660px (with padding)

### Board

- Dark background rectangle behind the grid
- Each filled cell: `draw_rectangle` with a 2px inset gap on all sides so cells read as individual blocks
- Active piece overlaid on board each frame (same as current renderer logic)
- During `LineClearDelay` and `Spawning` phases: same visibility rules as today

### Ghost piece

Compute ghost position by stepping the active piece down until the next step would collide. Render as the same color as the active piece at ~25% alpha.

### Piece colors

Unchanged from the terminal renderer:

| Piece | Color |
|---|---|
| I | Red |
| O | Yellow |
| T | Cyan |
| S | Magenta |
| Z | Green |
| J | Blue |
| L | Light Red |

### Sidebar

Drawn with `draw_text` and small rectangles for the next piece preview. Contents: Next piece, Level, Lines, Time (MM:SS.sss format — move `format_time` from `renderer.rs` into the new renderer or a small shared helper).

### Overlays

- **Game over:** "GAME OVER" text centered over the board
- **Victory:** "LEVEL 999" + final time, same as today

## Web Deployment

### Build

```sh
cargo build --target wasm32-unknown-unknown --release
```

macroquad provides a standard `index.html` template that handles canvas setup, WASM loading, and input focus. The build output is `fetris.wasm` + `index.html`.

### Hosting

GitHub Pages serves the static artifacts. A GitHub Actions workflow builds and deploys on every push to `master`:

1. Install Rust + `wasm32-unknown-unknown` target
2. `cargo build --target wasm32-unknown-unknown --release`
3. Copy `fetris.wasm` + `index.html` to the `gh-pages` branch
4. GitHub Pages serves from `gh-pages`

Result: a stable URL at `https://eperdew.github.io/fetris` (or equivalent).

## Out of Scope

**Audio** is explicitly deferred. The architecture supports adding it later via `macroquad-audio` (same author, same crate family) by:
1. Loading `.ogg`/`.wav` assets at startup
2. Playing them on game events: piece lock, line clear, game over, win

No audio code is included in this implementation.

## Verification

1. `cargo run` launches a native window at ~60fps, game plays correctly
2. Ghost piece appears below the active piece and updates as the piece moves
3. All controls work: movement, rotation, soft drop, sonic drop, quit
4. DAS, ARE, line clear delay, section stops all behave identically to the terminal version
5. `cargo build --target wasm32-unknown-unknown --release` succeeds
6. Game runs in a browser from the generated `index.html` + `.wasm`
7. Existing snapshot tests (`cargo test`) still pass (game logic untouched)
