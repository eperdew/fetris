use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::game::{BOARD_COLS, BOARD_ROWS, Game, HorizDir, PiecePhase};
use crate::input::GameKey;
use crate::piece::PieceKind;

// Board: 20 rows + 2 borders tall; (10 cols * 2 chars) + 2 borders = 22 wide
// Sidebar: 14 wide
const GAME_WIDTH: u16 = 36;
const GAME_HEIGHT: u16 = 22;

pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}

pub fn render(frame: &mut Frame, game: &Game, held: &HashSet<GameKey>) {
    let area = frame.area();

    if area.width < GAME_WIDTH || area.height < GAME_HEIGHT {
        let msg = Paragraph::new(format!(
            "Window too small ({}x{}). Please resize to at least {}x{}.",
            area.width, area.height, GAME_WIDTH, GAME_HEIGHT
        ))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(msg, area);
        return;
    }

    // Center the game vertically so the play area doesn't float at the top
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(GAME_HEIGHT), Constraint::Fill(1)])
        .split(area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Length(14)])
        .split(v_chunks[1]);

    render_board(frame, game, chunks[0]);
    render_sidebar(frame, game, held, chunks[1]);
}

fn render_board(frame: &mut Frame, game: &Game, area: ratatui::layout::Rect) {
    if game.game_won {
        let time_str = format_time(game.ticks_elapsed);
        let victory = Paragraph::new(vec![
            Line::from(""),
            Line::from("  LEVEL 999"),
            Line::from(""),
            Line::from("  Time:"),
            Line::from(format!("  {}", time_str)),
            Line::from(""),
        ])
        .block(Block::default().title("fetris").borders(Borders::ALL));
        frame.render_widget(victory, area);
        return;
    }

    // Build a display grid: start from locked board, then overlay active piece.
    // During spawn delay the old piece is already in the board; don't re-draw it.
    let mut display = game.board;
    if !matches!(game.piece_phase, PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. }) {
        for (dc, dr) in game.active.cells() {
            let c = (game.active.col + dc) as usize;
            let r = (game.active.row + dr) as usize;
            if r < BOARD_ROWS && c < BOARD_COLS {
                display[r][c] = Some(game.active.kind);
            }
        }
    }

    let rows: Vec<Line> = display
        .iter()
        .map(|row| {
            let spans: Vec<Span> = row
                .iter()
                .map(|cell| match cell {
                    None => Span::raw("  "),
                    Some(kind) => Span::styled("[]", Style::default().fg(piece_color(*kind))),
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let title = if game.game_over {
        "GAME OVER"
    } else {
        "fetris"
    };
    let board = Paragraph::new(rows).block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(board, area);
}

fn render_sidebar(frame: &mut Frame, game: &Game, held: &HashSet<GameKey>, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    // Next piece preview
    let mut preview = [[None; 4]; 4];
    for (dc, dr) in game.next.cells() {
        let c = dc as usize;
        let r = dr as usize;
        if r < 4 && c < 4 {
            preview[r][c] = Some(game.next.kind);
        }
    }
    let preview_rows: Vec<Line> = preview
        .iter()
        .map(|row| {
            let spans: Vec<Span> = row
                .iter()
                .map(|cell| match cell {
                    None => Span::raw("  "),
                    Some(kind) => Span::styled("[]", Style::default().fg(piece_color(*kind))),
                })
                .collect();
            Line::from(spans)
        })
        .collect();
    let next_widget =
        Paragraph::new(preview_rows).block(Block::default().title("Next").borders(Borders::ALL));
    frame.render_widget(next_widget, chunks[0]);

    // Input display
    let k = |key: GameKey, label: &'static str| if held.contains(&key) { label } else { "·" };
    let keys_line = format!(
        "{} {} {} {} {} {}",
        k(GameKey::Left, "←"),
        k(GameKey::Right, "→"),
        k(GameKey::SoftDrop, "↓"),
        k(GameKey::SonicDrop, "⎵"),
        k(GameKey::RotateCw, "x"),
        k(GameKey::RotateCcw, "z"),
    );
    let das_line = match game.das_direction {
        None => "DAS: -".to_string(),
        Some(HorizDir::Left)  => format!("DAS:← {}", game.das_counter),
        Some(HorizDir::Right) => format!("DAS:→ {}", game.das_counter),
    };

    // Stats
    let stats = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("Level: {}", game.level)),
        Line::from(format!("Lines: {}", game.lines)),
        Line::from(format_time(game.ticks_elapsed)),
        Line::from(""),
        Line::from("←→  move"),
        Line::from("x   rotate ↻"),
        Line::from("z   rotate ↺"),
        Line::from("↓   soft drop"),
        Line::from("SPC hard drop"),
        Line::from("q  quit"),
        Line::from(""),
        Line::from(keys_line),
        Line::from(das_line),
    ])
    .block(Block::default().title("Stats").borders(Borders::ALL));
    frame.render_widget(stats, chunks[1]);
}

fn piece_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::Red,
        PieceKind::O => Color::Yellow,
        PieceKind::T => Color::Cyan,
        PieceKind::S => Color::Magenta,
        PieceKind::Z => Color::Green,
        PieceKind::J => Color::Blue,
        PieceKind::L => Color::LightRed,
    }
}
