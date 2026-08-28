use bilibili_tui::app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    // --open <spec>: deep-link into a page for development screenshots/tests
    let open_spec = std::env::args()
        .nth(1)
        .filter(|a| a == "--open")
        .and_then(|_| std::env::args().nth(2));

    // Initialize terminal
    let mut terminal = ratatui::init();
    terminal.clear()?;

    // Enable mouse capture
    execute!(std::io::stdout(), EnableMouseCapture)?;

    // Run the application
    let app = App::new_with_open(open_spec.as_deref());
    let result = app.run(&mut terminal).await;

    // Disable mouse capture before restoring
    let _ = execute!(std::io::stdout(), DisableMouseCapture);

    // Restore terminal
    ratatui::restore();

    result
}
