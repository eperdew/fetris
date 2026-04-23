# Grade Progress Bar — Design Spec

## Overview

Add a vertical grade progress bar to the right of the play field, plus a "next grade score" display in the sidebar. The bar shows how close the player is to the next grade; the sidebar shows the exact score threshold (or `??????` at max grade).

## Layout Changes

**New constants in `renderer.rs`:**
- `BAR_WIDTH: f32 = 24.0`
- `BAR_GAP: f32 = 6.0`
- `BAR_X: f32 = BOARD_X + BOARD_COLS as f32 * CELL + BAR_GAP`

**`SIDEBAR_X` updated to:**
```
BAR_X + BAR_WIDTH + BAR_GAP
```
This shifts the existing sidebar text ~34px further right to make room for the bar.

## Grade Bar

**Position:** `BAR_X`, from `BOARD_Y` to `BOARD_Y + BOARD_ROWS as f32 * CELL` (full play field height, 640px at CELL=32).

**Background:** Dark rectangle the full bar dimensions.

**Fill:** A rectangle drawn from the bottom up.
- Height = `board_height * progress`
- `progress = (score - prev_threshold) as f32 / (next_threshold - prev_threshold) as f32`
- At max grade (`SNine`): progress = 1.0, bar stays full.

**Color:** Cycles through ROYGBIV by `grade_index % 7`:

| Index mod 7 | Color      | RGBA approx         |
|-------------|------------|---------------------|
| 0           | Red        | (220, 50, 50, 255)  |
| 1           | Orange     | (230, 130, 0, 255)  |
| 2           | Yellow     | (220, 210, 0, 255)  |
| 3           | Green      | (50, 180, 50, 255)  |
| 4           | Blue       | (50, 100, 220, 255) |
| 5           | Indigo     | (80, 0, 200, 255)   |
| 6           | Violet     | (150, 0, 220, 255)  |

Grade index = position in `Grade::SCORE_TABLE` (Nine=0, Eight=1, ..., SNine=17).

## Data Helper

Add to `Grade` in `types.rs`:

```rust
pub fn grade_progress(score: u32) -> (u32, Option<u32>) {
    // Returns (prev_threshold, Some(next_threshold)) or (prev_threshold, None) at SNine.
}
```

Uses `SCORE_TABLE`. Finds the highest entry where `score >= threshold` (same logic as `of_score`), then returns the next entry's threshold if it exists.

## Sidebar Update

In `render_sidebar`, after the existing `GRADE` row, add:
- Label: `"NEXT"` (same dimmed style as other labels)
- Value: next threshold as a number, or `"??????"` if `grade_progress` returns `None`

## Files Changed

- `src/types.rs` — add `Grade::grade_progress`
- `src/renderer.rs` — add bar constants, update `SIDEBAR_X`, add `render_grade_bar`, update `render_sidebar`
