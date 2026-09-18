use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use sonora_common::{init_logging, LogConfig, Result, SonoraError};
use sonora_core::{SonoraApp, SonoraConfig};
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // TUI logging disabled to terminal to avoid screen disruption
    init_logging(&LogConfig {
        enable_ansi: false,
        ..Default::default()
    });

    let config = SonoraConfig::default();
    let _app = SonoraApp::in_memory(config)?;

    // Terminal setup
    enable_raw_mode().map_err(|e| SonoraError::Io(e))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| SonoraError::Io(e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| SonoraError::Io(e))?;

    let res = run_tui(&mut terminal).await;

    // Terminal teardown
    disable_raw_mode().map_err(|e| SonoraError::Io(e))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| SonoraError::Io(e))?;
    terminal.show_cursor().map_err(|e| SonoraError::Io(e))?;

    res
}

async fn run_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    loop {
        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(5),
                        Constraint::Length(3),
                    ])
                    .split(f.area());

                let header = Paragraph::new("Sonora Terminal Audio Player (v0.1.0)")
                    .style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .block(Block::default().borders(Borders::ALL).title("Sonora TUI"));
                f.render_widget(header, chunks[0]);

                let body = Paragraph::new(
                    "Welcome to Sonora.\n\n\
                    Bootstrap foundation active:\n\
                    - Real-time audio core linked\n\
                    - SQLite + FTS5 indexing engine initialized\n\
                    - Asynchronous runtime ready\n\n\
                    Press 'q' or Esc to exit.",
                )
                .block(Block::default().borders(Borders::ALL).title("Status"));
                f.render_widget(body, chunks[1]);

                let footer = Paragraph::new("Controls: [q/Esc] Quit | [/] Search | [Space] Play/Pause")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(footer, chunks[2]);
            })
            .map_err(|e| SonoraError::Io(e))?;

        if event::poll(Duration::from_millis(100)).map_err(|e| SonoraError::Io(e))? {
            if let Event::Key(key) = event::read().map_err(|e| SonoraError::Io(e))? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}
