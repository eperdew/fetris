use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::game::{BOARD_COLS, BOARD_ROWS, Game};
use crate::piece::PieceKind;

pub fn render(frame: &mut Frame, game: &Game) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Length(14)])
        .split(frame.area());

    render_board(frame, game, chunks[0]);
    render_sidebar(frame, game, chunks[1]);
}

fn render_board(frame: &mut Frame, game: &Game, area: ratatui::layout::Rect) {
    // Build a display grid: start from locked board, then overlay active piece
    let mut display = game.board;
    for (dc, dr) in game.active.cells() {
        let c = (game.active.col + dc) as usize;
        let r = (game.active.row + dr) as usize;
        if r < BOARD_ROWS && c < BOARD_COLS {
            display[r][c] = Some(game.active.kind);
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

fn render_sidebar(frame: &mut Frame, game: &Game, area: ratatui::layout::Rect) {
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

    // Stats
    let stats = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!("Level: {}", game.level)),
        Line::from(format!("Lines: {}", game.lines)),
        Line::from(""),
        Line::from("←→  move"),
        Line::from("x   rotate ↻"),
        Line::from("z   rotate ↺"),
        Line::from("↓   soft drop"),
        Line::from("SPC hard drop"),
        Line::from("q  quit"),
    ])
    .block(Block::default().title("Stats").borders(Borders::ALL));
    frame.render_widget(stats, chunks[1]);
}

fn piece_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::Cyan,
        PieceKind::O => Color::Yellow,
        PieceKind::T => Color::Magenta,
        PieceKind::S => Color::Green,
        PieceKind::Z => Color::Red,
        PieceKind::J => Color::Blue,
        PieceKind::L => Color::LightRed,
    }
}
