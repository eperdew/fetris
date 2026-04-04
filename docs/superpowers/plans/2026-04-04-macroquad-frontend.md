# macroquad Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the ratatui/crossterm terminal frontend with a macroquad-based frontend that runs natively and compiles to WASM for GitHub Pages hosting, with a ghost piece added to the renderer.

**Architecture:** `main.rs` and `renderer.rs` are replaced wholesale; all game logic (`game.rs`, `piece.rs`, `randomizer.rs`, `constants.rs`, `input.rs`) is untouched. macroquad's async loop calls `game.tick()` once per frame at ~60fps, building `InputState` from macroquad key queries each frame.

**Tech Stack:** Rust, macroquad 0.4, wasm32-unknown-unknown target, GitHub Actions, GitHub Pages

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | Modify | Remove ratatui/crossterm; add macroquad |
| `src/main.rs` | Replace | Window config, macroquad async loop, input→InputState, quit handling |
| `src/renderer.rs` | Replace | Draw board, pieces, ghost piece, sidebar, overlays using macroquad |
| `web/index.html` | Create | Static HTML page that loads the WASM |
| `web/mq_js_bundle.js` | Create | macroquad's JS glue bundle (downloaded once, committed) |
| `.github/workflows/deploy.yml` | Create | Build WASM + deploy `web/` + `.wasm` to GitHub Pages |

---

## Layout Constants (reference for Tasks 2–3)

```
CELL  = 32px
PAD   = 20px

Board origin:  (PAD, PAD) = (20, 20)
Board size:    320 × 640  (10 cols × 20 rows × 32px)
Sidebar origin: (20 + 320 + 10, 20) = (350, 20)
Sidebar width: 160px

Window: 530 × 680
```

---

## Task 1: Swap deps, replace main.rs, stub renderer.rs

**Files:**
- Modify: `Cargo.toml`
- Replace: `src/main.rs`
- Replace: `src/renderer.rs` (stub only — full rendering in Task 2)

- [ ] **Step 1: Update Cargo.toml**

Replace the `[dependencies]` block with:

```toml
[dependencies]
macroquad = "0.4"
rand = "0.9"
```

(`ratatui`, `crossterm`, and `anyhow` are removed — `anyhow` was only used in the old `main.rs` return type.)

- [ ] **Step 2: Write stub renderer.rs**

`src/renderer.rs`:

```rust
use macroquad::prelude::*;
use crate::game::Game;

pub fn render(_game: &Game) {
    clear_background(BLACK);
}

pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}
```

- [ ] **Step 3: Write new main.rs**

`src/main.rs`:

```rust
mod constants;
mod game;
mod input;
mod piece;
mod randomizer;
mod renderer;
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use macroquad::prelude::*;
use game::Game;
use input::{GameKey, InputState};

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("fetris"),
        window_width: 530,
        window_height: 680,
        window_resizable: false,
        ..Default::default()
    }
}

fn build_input_state() -> InputState {
    let mappings: &[(KeyCode, GameKey)] = &[
        (KeyCode::Left,  GameKey::Left),
        (KeyCode::H,     GameKey::Left),
        (KeyCode::Right, GameKey::Right),
        (KeyCode::L,     GameKey::Right),
        (KeyCode::Down,  GameKey::SoftDrop),
        (KeyCode::J,     GameKey::SoftDrop),
        (KeyCode::Space, GameKey::SonicDrop),
        (KeyCode::X,     GameKey::RotateCw),
        (KeyCode::Z,     GameKey::RotateCcw),
    ];
    let mut held = HashSet::new();
    let mut just_pressed = HashSet::new();
    for &(kc, gk) in mappings {
        if is_key_down(kc)     { held.insert(gk); }
        if is_key_pressed(kc)  { just_pressed.insert(gk); }
    }
    InputState { held, just_pressed }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();
    loop {
        if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            break;
        }
        let input = build_input_state();
        game.tick(&input);
        renderer::render(&game);
        next_frame().await;
    }
}
```

- [ ] **Step 4: Run existing tests**

```bash
cargo test
```

Expected: all tests pass (game logic is untouched; ratatui is gone but tests never imported it).

- [ ] **Step 5: Verify it compiles and opens a window**

```bash
cargo run
```

Expected: a 530×680 black window opens, game ticks silently, Q/Esc closes it.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/renderer.rs
git commit -m "chore: swap ratatui/crossterm for macroquad, stub renderer"
```

---

## Task 2: Full renderer — board, pieces, sidebar, overlays

**Files:**
- Replace: `src/renderer.rs`

- [ ] **Step 1: Write the full renderer**

`src/renderer.rs`:

```rust
use macroquad::prelude::*;
use crate::game::{BOARD_COLS, BOARD_ROWS, Game, PiecePhase};
use crate::piece::PieceKind;

const CELL: f32 = 32.0;
const PAD: f32 = 20.0;
const BOARD_X: f32 = PAD;
const BOARD_Y: f32 = PAD;
const SIDEBAR_X: f32 = BOARD_X + BOARD_COLS as f32 * CELL + 10.0;
const BOARD_BG: Color = Color::new(0.06, 0.06, 0.10, 1.0);

/// Draw a single CELL×CELL block at grid position (col, row) relative to (origin_x, origin_y).
fn draw_cell(origin_x: f32, origin_y: f32, col: usize, row: usize, color: Color) {
    const INSET: f32 = 2.0;
    draw_rectangle(
        origin_x + col as f32 * CELL + INSET,
        origin_y + row as f32 * CELL + INSET,
        CELL - INSET * 2.0,
        CELL - INSET * 2.0,
        color,
    );
}

fn piece_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::from_rgba(200, 50,  50,  255),
        PieceKind::O => Color::from_rgba(220, 200, 0,   255),
        PieceKind::T => Color::from_rgba(0,   200, 200, 255),
        PieceKind::S => Color::from_rgba(200, 0,   200, 255),
        PieceKind::Z => Color::from_rgba(0,   160, 0,   255),
        PieceKind::J => Color::from_rgba(50,  100, 220, 255),
        PieceKind::L => Color::from_rgba(255, 150, 100, 255),
    }
}

pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}

fn render_board(game: &Game) {
    // Background
    draw_rectangle(
        BOARD_X, BOARD_Y,
        BOARD_COLS as f32 * CELL,
        BOARD_ROWS as f32 * CELL,
        BOARD_BG,
    );

    // Locked cells
    for (r, row) in game.board.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if let Some(kind) = cell {
                draw_cell(BOARD_X, BOARD_Y, c, r, piece_color(*kind));
            }
        }
    }

    // Active piece (hidden during spawn delay and line clear)
    if !matches!(
        game.piece_phase,
        PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. }
    ) {
        for (dc, dr) in game.active.cells() {
            let c = (game.active.col + dc) as usize;
            let r = (game.active.row + dr) as usize;
            if r < BOARD_ROWS && c < BOARD_COLS {
                draw_cell(BOARD_X, BOARD_Y, c, r, piece_color(game.active.kind));
            }
        }
    }
}

fn render_sidebar(game: &Game) {
    let x = SIDEBAR_X;
    let mut y = BOARD_Y + 16.0;

    draw_text("NEXT", x, y, 18.0, LIGHTGRAY);
    y += 8.0;

    for (dc, dr) in game.next.cells() {
        let c = dc as usize;
        let r = dr as usize;
        draw_cell(x, y, c, r, piece_color(game.next.kind));
    }
    y += 4.0 * CELL + 16.0;

    draw_text(&format!("LV  {}", game.level), x, y, 18.0, WHITE);
    y += 26.0;
    draw_text(&format!("LN  {}", game.lines), x, y, 18.0, WHITE);
    y += 26.0;
    draw_text(&format_time(game.ticks_elapsed), x, y, 18.0, WHITE);
}

fn render_overlay(game: &Game) {
    let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
    let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
    if game.game_won {
        draw_text("LEVEL 999", cx - 60.0, cy - 16.0, 28.0, WHITE);
        draw_text(&format_time(game.ticks_elapsed), cx - 50.0, cy + 20.0, 22.0, LIGHTGRAY);
    } else if game.game_over {
        draw_text("GAME OVER", cx - 62.0, cy, 28.0, WHITE);
    }
}

pub fn render(game: &Game) {
    clear_background(Color::from_rgba(10, 10, 18, 255));
    render_board(game);
    render_sidebar(game);
    render_overlay(game);
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test
```

Expected: all pass.

- [ ] **Step 3: Play-test visually**

```bash
cargo run
```

Verify:
- Board background is visible, pieces fall and lock with correct colors
- Sidebar shows next piece preview, level, lines, time updating
- Game over text appears on top out
- Victory text + final time appears at level 999
- All controls work: ←→hjl move, xz rotate, space sonic drop, ↓j soft drop, Q/Esc quit

- [ ] **Step 4: Commit**

```bash
git add src/renderer.rs
git commit -m "feat: macroquad renderer with board, pieces, sidebar, and overlays"
```

---

## Task 3: Ghost piece

**Files:**
- Modify: `src/renderer.rs`

- [ ] **Step 1: Add ghost piece computation and rendering to render_board**

Add the `compute_ghost_row` function and ghost rendering inside `render_board`, just before the locked cells loop. The final `render_board` function:

```rust
fn compute_ghost_row(game: &Game) -> i32 {
    let mut ghost_row = game.active.row;
    loop {
        let next = ghost_row + 1;
        let blocked = game.active.cells().iter().any(|&(dc, dr)| {
            let c = (game.active.col + dc) as usize;
            let r = next + dr;
            r >= BOARD_ROWS as i32 || (r >= 0 && game.board[r as usize][c].is_some())
        });
        if blocked { break; }
        ghost_row = next;
    }
    ghost_row
}

fn render_board(game: &Game) {
    // Background
    draw_rectangle(
        BOARD_X, BOARD_Y,
        BOARD_COLS as f32 * CELL,
        BOARD_ROWS as f32 * CELL,
        BOARD_BG,
    );

    let show_active = !matches!(
        game.piece_phase,
        PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. }
    );

    // Ghost piece
    if show_active {
        let ghost_row = compute_ghost_row(game);
        if ghost_row != game.active.row {
            let base = piece_color(game.active.kind);
            let ghost_color = Color { a: 0.25, ..base };
            for (dc, dr) in game.active.cells() {
                let c = (game.active.col + dc) as usize;
                let r = (ghost_row + dr) as usize;
                if r < BOARD_ROWS && c < BOARD_COLS {
                    draw_cell(BOARD_X, BOARD_Y, c, r, ghost_color);
                }
            }
        }
    }

    // Locked cells
    for (r, row) in game.board.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if let Some(kind) = cell {
                draw_cell(BOARD_X, BOARD_Y, c, r, piece_color(*kind));
            }
        }
    }

    // Active piece
    if show_active {
        for (dc, dr) in game.active.cells() {
            let c = (game.active.col + dc) as usize;
            let r = (game.active.row + dr) as usize;
            if r < BOARD_ROWS && c < BOARD_COLS {
                draw_cell(BOARD_X, BOARD_Y, c, r, piece_color(game.active.kind));
            }
        }
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test
```

Expected: all pass.

- [ ] **Step 3: Play-test ghost piece**

```bash
cargo run
```

Verify:
- Ghost piece appears as a translucent (25% alpha) outline below the active piece
- Ghost disappears during spawn delay and line clear phases
- Ghost is absent when the piece is already on the floor (ghost_row == active.row)
- Moving/rotating the piece updates the ghost immediately

- [ ] **Step 4: Commit**

```bash
git add src/renderer.rs
git commit -m "feat: add ghost piece to macroquad renderer"
```

---

## Task 4: Web assets

**Files:**
- Create: `web/index.html`
- Create: `web/mq_js_bundle.js` (downloaded once, committed)

- [ ] **Step 1: Download mq_js_bundle.js**

```bash
mkdir -p web
curl -L "https://raw.githubusercontent.com/not-fl3/macroquad/master/js/mq_js_bundle.js" \
  -o web/mq_js_bundle.js
```

- [ ] **Step 2: Create web/index.html**

`web/index.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>fetris</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { background: #000; display: flex; justify-content: center; align-items: center; min-height: 100vh; }
    canvas { display: block; }
  </style>
</head>
<body>
  <canvas id="glcanvas" tabindex="1"></canvas>
  <script src="mq_js_bundle.js"></script>
  <script>load("fetris.wasm");</script>
</body>
</html>
```

- [ ] **Step 3: Install wasm32 target and build**

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

Expected: build succeeds, `target/wasm32-unknown-unknown/release/fetris.wasm` exists.

- [ ] **Step 4: Test in browser**

```bash
cp target/wasm32-unknown-unknown/release/fetris.wasm web/
cd web && python3 -m http.server 8080
```

Open `http://localhost:8080` in a browser. Click the canvas to focus it, verify the game runs and controls work.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat: add web assets for WASM deployment"
```

---

## Task 5: GitHub Actions deploy workflow

**Files:**
- Create: `.github/workflows/deploy.yml`

- [ ] **Step 1: Enable GitHub Pages**

In the GitHub repo settings: **Pages → Source → Deploy from a branch → gh-pages → / (root)**. (If `gh-pages` branch doesn't exist yet, the first workflow run will create it.)

- [ ] **Step 2: Create the workflow file**

`.github/workflows/deploy.yml`:

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
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build WASM
        run: cargo build --target wasm32-unknown-unknown --release

      - name: Assemble dist
        run: |
          mkdir dist
          cp web/index.html web/mq_js_bundle.js dist/
          cp target/wasm32-unknown-unknown/release/fetris.wasm dist/

      - name: Deploy to gh-pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./dist
```

- [ ] **Step 3: Commit and push**

```bash
git add .github/workflows/deploy.yml
git commit -m "ci: add GitHub Actions workflow to build WASM and deploy to GitHub Pages"
git push
```

- [ ] **Step 4: Verify the workflow**

Go to the repo's **Actions** tab. The `Deploy to GitHub Pages` workflow should appear and run. Wait for it to succeed (typically 2–4 minutes).

Expected:
- Workflow completes with green checkmark
- `gh-pages` branch is created/updated with `index.html`, `mq_js_bundle.js`, `fetris.wasm`
- Game is playable at `https://<your-username>.github.io/fetris`
