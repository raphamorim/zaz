use std::time::{Duration};
use crossterm::event::*;
use crossterm::{event, terminal};

use crate::editor::output::Output;

pub const TAB_STOP: usize = 8;

pub struct Screen;

impl Screen {
    pub fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if event::poll(Duration::from_millis(500))? {
                if let Event::Key(event) = event::read()? {
                    return Ok(event);
                }
            }
        }
    }
}

pub struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Unable to disable raw mode");
        Output::clear_screen().expect("error");
    }
}