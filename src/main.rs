mod constants;
mod game;
mod input;
mod piece;
mod randomizer;
mod renderer;
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use std::io::stdout;

use game::Game;
use input::{GameKey, InputState};

const TICK_RATE_MS: u64 = 16; // ~60Hz

/// Maps a KeyCode to a GameKey. Returns None for unrecognised keys.
fn map_game_key(code: KeyCode) -> Option<GameKey> {
    match code {
        KeyCode::Left | KeyCode::Char('h')  => Some(GameKey::Left),
        KeyCode::Right | KeyCode::Char('l') => Some(GameKey::Right),
        KeyCode::Down | KeyCode::Char('j')  => Some(GameKey::SoftDrop),
        KeyCode::Char(' ')                  => Some(GameKey::SonicDrop),
        KeyCode::Char('x')                  => Some(GameKey::RotateCw),
        KeyCode::Char('z')                  => Some(GameKey::RotateCcw),
        _ => None,
    }
}

#[derive(Debug)]
enum AppEvent {
    KeyDown(GameKey),
    KeyUp(GameKey),
    Tick,
    Quit,
}

fn main() -> anyhow::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    stdout().execute(PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
    ))?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run(&mut terminal);

    // Always restore terminal, even on error
    let _ = stdout().execute(PopKeyboardEnhancementFlags);
    let _ = disable_raw_mode();
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<AppEvent>();

    // Timer thread: sends Tick at a fixed interval (~60Hz)
    let tick_tx = tx.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(TICK_RATE_MS));
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    // Input thread: reads crossterm events and sends KeyDown/KeyUp
    let input_tx = tx;
    thread::spawn(move || {
        loop {
            if event::poll(Duration::from_millis(5)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    let app_event = match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if key.kind != KeyEventKind::Press {
                                continue;
                            }
                            AppEvent::Quit
                        }
                        other => match map_game_key(other) {
                            Some(game_key) => match key.kind {
                                KeyEventKind::Press | KeyEventKind::Repeat => AppEvent::KeyDown(game_key),
                                KeyEventKind::Release => AppEvent::KeyUp(game_key),
                            },
                            None => continue,
                        },
                    };
                    if input_tx.send(app_event).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut game = Game::new();

    let mut held: HashSet<GameKey> = HashSet::new();
    let mut just_pressed: HashSet<GameKey> = HashSet::new();
    let mut quit = false;

    loop {
        match rx.recv()? {
            AppEvent::Quit => break,
            AppEvent::KeyDown(key) => {
                held.insert(key);
                just_pressed.insert(key);
            }
            AppEvent::KeyUp(key) => {
                held.remove(&key);
            }
            AppEvent::Tick => {
                // Drain remaining queued events (including stacked Ticks).
                while let Ok(ev) = rx.try_recv() {
                    match ev {
                        AppEvent::KeyDown(key) => {
                            held.insert(key);
                            just_pressed.insert(key);
                        }
                        AppEvent::KeyUp(key) => {
                            held.remove(&key);
                        }
                        AppEvent::Tick => {
                            let input = InputState {
                                held: held.clone(),
                                just_pressed: just_pressed.clone(),
                            };
                            game.tick(&input);
                            just_pressed.clear();
                        }
                        AppEvent::Quit => {
                            quit = true;
                        }
                    }
                }
                if quit { break; }
                let input = InputState { held: held.clone(), just_pressed };
                game.tick(&input);
                just_pressed = HashSet::new();
                terminal.draw(|frame| renderer::render(frame, &game))?;
            }
        }
    }

    Ok(())
}
