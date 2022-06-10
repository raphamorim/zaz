use std::{io};
use tui::{
    backend::CrosstermBackend,
    Terminal
};
use crossterm::{
    terminal::{enable_raw_mode},
};

use zaz::screen::CleanUp;
use zaz::editor::editor::{Editor};

fn main() -> crossterm::Result<()> {
    let _clean_up = CleanUp;
    enable_raw_mode()?;
    let mut editor = Editor::new();
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let _terminal = Terminal::new(backend)?;

    while editor.run()? {}
    Ok(())
}
