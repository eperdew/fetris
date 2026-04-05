# fetris

Reimplementation of [TGM1](https://tetris.wiki/Tetris_The_Grand_Master) in Rust. **[Play in browser](https://eperdew.github.io/fetris/)**

## Development

Install the pre-commit hook to enforce formatting:

```sh
cp hooks/pre-commit .git/hooks/pre-commit
```

## Build & Run

Build and run with cargo.

```sh
cargo run --release
```

## Controls

| Key | Action |
|---|---|
| ←/→ or h/l | Move |
| ↓ or j | Soft drop |
| Space | Sonic drop |
| X | Rotate CW |
| Z | Rotate CCW |
| Esc | Quit |
