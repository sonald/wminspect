/// Simple terminal-based GUI using ratatui
#[cfg(feature = "gui")]
#[allow(dead_code)]
pub fn gui_functionality() {
    use crossterm::{ExecutableCommand, event, terminal};
    use ratatui::widgets::{Block, Borders};
    use ratatui::{Terminal, backend::CrosstermBackend};
    use std::io::{Write, stdout};
    use std::time::Duration;

    if terminal::enable_raw_mode().is_err() {
        eprintln!("Failed to enable raw mode for GUI");
        return;
    }

    let mut stdout_handle = stdout();
    stdout_handle.execute(terminal::EnterAlternateScreen).ok();

    let backend = CrosstermBackend::new(stdout_handle);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    let mut should_quit = false;
    while !should_quit {
        terminal
            .draw(|f| {
                let size = f.size();
                let block = Block::default().title("wminspect").borders(Borders::ALL);
                f.render_widget(block, size);
            })
            .expect("draw failed");

        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            if let event::Event::Key(k) = event::read().unwrap() {
                if let event::KeyCode::Char('q') = k.code {
                    should_quit = true;
                }
            }
        } else {
            // In automated environments with no input, exit immediately
            should_quit = true;
        }
    }

    terminal::disable_raw_mode().ok();
    drop(terminal);
    let mut stdout_handle = stdout();
    stdout_handle.execute(terminal::LeaveAlternateScreen).ok();
    stdout_handle.flush().ok();
}

#[cfg(not(feature = "gui"))]
#[allow(dead_code)]
pub fn gui_functionality() {}
