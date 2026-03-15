mod game;
mod input;
mod piece;
mod renderer;
#[cfg(test)]
mod tests;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use std::io::stdout;

use game::Game;
use input::GameAction;

const TICK_RATE_MS: u64 = 500; // gravity tick interval

#[derive(Debug)]
enum AppEvent {
    Input(GameAction),
    Tick,
    Quit,
}

fn main() -> anyhow::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run(&mut terminal);

    // Always restore terminal, even on error
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<AppEvent>();

    // Timer thread: sends Tick at a fixed interval
    let tick_tx = tx.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(TICK_RATE_MS));
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Input thread: reads crossterm events and maps them to GameActions
    let input_tx = tx;
    thread::spawn(move || {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    let app_event = match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => AppEvent::Quit,
                        other => {
                            if let Some(action) = input::map_key(other) {
                                AppEvent::Input(action)
                            } else {
                                continue;
                            }
                        }
                    };
                    if input_tx.send(app_event).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut game = Game::new();

    loop {
        terminal.draw(|frame| renderer::render(frame, &game))?;

        match rx.recv()? {
            AppEvent::Quit => break,
            AppEvent::Tick => game.tick(),
            AppEvent::Input(action) => game.handle_action(action),
        }
    }

    Ok(())
}
