use std::io::Write;
use std::io::{stdout, Result};
use std::{cmp};
use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;
use crossterm::terminal::ClearType;
use crossterm::event::KeyCode;
use crossterm::{terminal, execute, cursor, style, queue};

use crate::screen::Screen;
use crate::syntax::highlight::{Highlight, HighlightType, RustHighlight};
use crate::editor::content::Content;
use crate::editor::cursor::Cursor;
use crate::editor::rows::Rows;
use crate::prompt;

pub struct Output {
    pub win_size: (usize, usize),
    pub editor_contents: Content,
    pub cursor: Cursor,
    pub rows: Rows,
    pub status_message: StatusMessage,
    pub dirty: u64,
    search_index: SearchIndex,
    pub syntax_highlight: Option<Box<dyn Highlight>>,
}

enum SearchDirection {
    Forward,
    Backward,
}

const VERSION: &str = "0.1.0";

impl Output {
    pub fn select_syntax(extension: &str) -> Option<Box<dyn Highlight>> {
        let list: Vec<Box<dyn Highlight>> = vec![Box::new(RustHighlight::new())];
        list.into_iter()
            .find(|it| it.extensions().contains(&extension))
    }

    pub fn new() -> Self {
        let win_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize - 2))
            .unwrap();
        let mut syntax_highlight = None; // modify
        Self {
            win_size,
            editor_contents: Content::new(),
            cursor: Cursor::new(win_size),
            rows: Rows::new(&mut syntax_highlight), //modify
            status_message: StatusMessage::new(
                "HELP: Ctrl-S = Save | Ctrl-Q = Quit | Ctrl-F = Find".into(),
            ),
            dirty: 0,
            search_index: SearchIndex::new(),
            syntax_highlight,
        }
    }

    pub fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    fn find_callback(output: &mut Output, keyword: &str, key_code: KeyCode) {
        if let Some((index, highlight)) = output.search_index.previous_highlight.take() {
            output.rows.get_editor_row_mut(index).highlight = highlight;
        }
        match key_code {
            KeyCode::Esc | KeyCode::Enter => {
                output.search_index.reset();
            }
            _ => {
                output.search_index.y_direction = None;
                output.search_index.x_direction = None;
                match key_code {
                    KeyCode::Down => {
                        output.search_index.y_direction = SearchDirection::Forward.into()
                    }
                    KeyCode::Up => {
                        output.search_index.y_direction = SearchDirection::Backward.into()
                    }
                    KeyCode::Left => {
                        output.search_index.x_direction = SearchDirection::Backward.into()
                    }
                    KeyCode::Right => {
                        output.search_index.x_direction = SearchDirection::Forward.into()
                    }
                    _ => {}
                }
                for i in 0..output.rows.number_of_rows() {
                    let row_index = match output.search_index.y_direction.as_ref() {
                        None => {
                            if output.search_index.x_direction.is_none() {
                                output.search_index.y_index = i;
                            }
                            output.search_index.y_index
                        }
                        Some(dir) => {
                            if matches!(dir, SearchDirection::Forward) {
                                output.search_index.y_index + i + 1
                            } else {
                                let res = output.search_index.y_index.saturating_sub(i);
                                if res == 0 {
                                    break;
                                }
                                res - 1
                            }
                        }
                    };
                    if row_index > output.rows.number_of_rows() - 1 {
                        break;
                    }
                    let row = output.rows.get_editor_row_mut(row_index);
                    let index = match output.search_index.x_direction.as_ref() {
                        None => row.render.find(&keyword),
                        Some(dir) => {
                            let index = if matches!(dir, SearchDirection::Forward) {
                                let start =
                                    cmp::min(row.render.len(), output.search_index.x_index + 1);
                                row.render[start..]
                                    .find(&keyword)
                                    .map(|index| index + start)
                            } else {
                                row.render[..output.search_index.x_index].rfind(&keyword)
                            };
                            if index.is_none() {
                                break;
                            }
                            index
                        }
                    };
                    if let Some(index) = index {
                        output.search_index.previous_highlight =
                            Some((row_index, row.highlight.clone()));
                        (index..index + keyword.len())
                            .for_each(|index| row.highlight[index] = HighlightType::SearchMatch);
                        output.cursor.cursor_y = row_index;
                        output.search_index.y_index = row_index;
                        output.search_index.x_index = index;
                        output.cursor.cursor_x = row.get_row_content_x(index);
                        output.cursor.row_offset = output.rows.number_of_rows();
                        break;
                    }
                }
            }
        }
    }

    pub fn find(&mut self) -> Result<()> {
        let cursor = self.cursor;
        if prompt!(
            self,
            "Search: {} (Use ESC / Arrows / Enter)",
            callback = Output::find_callback
        )
        .is_none()
        {
            self.cursor = cursor
        }
        Ok(())
    }

    fn draw_message_bar(&mut self) {
        queue!(
            self.editor_contents,
            terminal::Clear(ClearType::UntilNewLine)
        )
        .unwrap();
        if let Some(msg) = self.status_message.message() {
            self.editor_contents
                .push_str(&msg[..cmp::min(self.win_size.0, msg.len())]);
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor.cursor_y == self.rows.number_of_rows() {
            return;
        }
        if self.cursor.cursor_y == 0 && self.cursor.cursor_x == 0 {
            return;
        }
        if self.cursor.cursor_x > 0 {
            self.rows
                .get_editor_row_mut(self.cursor.cursor_y)
                .delete_char(self.cursor.cursor_x - 1);
            self.cursor.cursor_x -= 1;
        } else {
            let previous_row_content = self
                .rows
                .get_row(self.cursor.cursor_y - 1);
            self.cursor.cursor_x = previous_row_content.len();
            self.rows
                .join_adjacent_rows(self.cursor.cursor_y);
            self.cursor.cursor_y -= 1;
        }
        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor.cursor_y,
                &mut self.rows.row_contents,
            );
        }
        self.dirty += 1;
    }

    pub fn insert_newline(&mut self) {
        if self.cursor.cursor_x == 0 {
            self.rows
                .insert_row(self.cursor.cursor_y, String::new())
        } else {
            let current_row = self
                .rows
                .get_editor_row_mut(self.cursor.cursor_y);
            let new_row_content = current_row.row_content[self.cursor.cursor_x..].into();
            current_row
                .row_content
                .truncate(self.cursor.cursor_x);
            Rows::render_row(current_row);
            self.rows
                .insert_row(self.cursor.cursor_y + 1, new_row_content);
            if let Some(it) = self.syntax_highlight.as_ref() {
                it.update_syntax(
                    self.cursor.cursor_y,
                    &mut self.rows.row_contents,
                );
                it.update_syntax(
                    self.cursor.cursor_y + 1,
                    &mut self.rows.row_contents,
                )
            }
        }
        self.cursor.cursor_x = 0;
        self.cursor.cursor_y += 1;
        self.dirty += 1;
    }

    pub fn insert_char(&mut self, ch: char) {
        if self.cursor.cursor_y == self.rows.number_of_rows() {
            self.rows
                .insert_row(self.rows.number_of_rows(), String::new());
            self.dirty += 1;
        }
        self.rows
            .get_editor_row_mut(self.cursor.cursor_y)
            .insert_char(self.cursor.cursor_x, ch);
        if let Some(it) = self.syntax_highlight.as_ref() {
            it.update_syntax(
                self.cursor.cursor_y,
                &mut self.rows.row_contents,
            )
        }
        self.cursor.cursor_x += 1;
        self.dirty += 1;
    }

    fn draw_status_bar(&mut self) {
        self.editor_contents
            .push_str(&style::Attribute::Reverse.to_string());
        let info = format!(
            "{} {} -- {} lines",
            self.rows
                .filename
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("[No Name]"),
            if self.dirty > 0 { "(modified)" } else { "" },
            self.rows.number_of_rows()
        );
        let info_len = cmp::min(info.len(), self.win_size.0);
        /* modify the following */
        let line_info = format!(
            "{} | {}/{}",
            self.syntax_highlight
                .as_ref()
                .map(|highlight| highlight.file_type())
                .unwrap_or("no ft"),
            self.cursor.cursor_y + 1,
            self.rows.number_of_rows()
        );
        self.editor_contents.push_str(&info[..info_len]);
        for i in info_len..self.win_size.0 {
            if self.win_size.0 - i == line_info.len() {
                self.editor_contents.push_str(&line_info);
                break;
            } else {
                self.editor_contents.push(' ')
            }
        }
        self.editor_contents
            .push_str(&style::Attribute::Reset.to_string());
        self.editor_contents.push_str("\r\n");
    }

    pub fn draw_rows(&mut self) {
        let screen_rows = self.win_size.1;
        let screen_columns = self.win_size.0;
        for i in 0..screen_rows {
            let file_row = i + self.cursor.row_offset;
            if file_row >= self.rows.number_of_rows() {
                if self.rows.number_of_rows() == 0 && i == screen_rows / 3 {
                    let mut welcome = format!("ZAZ - {}", VERSION);
                    if welcome.len() > screen_columns {
                        welcome.truncate(screen_columns)
                    }
                    let mut padding = (screen_columns - welcome.len()) / 2;
                    if padding != 0 {
                        self.editor_contents.push('~');
                        padding -= 1
                    }
                    (0..padding).for_each(|_| self.editor_contents.push(' '));
                    self.editor_contents.push_str(&welcome);
                } else {
                    self.editor_contents.push('~');
                }
            } else {
                let row = self.rows.get_editor_row(file_row);
                let render = &row.render;
                let column_offset = self.cursor.column_offset;
                let len = cmp::min(render.len().saturating_sub(column_offset), screen_columns);
                let start = if len == 0 { 0 } else { column_offset };
                let render = render.chars().skip(start).take(len).collect::<String>();
                self.syntax_highlight
                    .as_ref()
                    .map(|syntax_highlight| {
                        syntax_highlight.color_row(
                            &render,
                            &row.highlight[start..start + len],
                            &mut self.editor_contents,
                        )
                    })
                    .unwrap_or_else(|| self.editor_contents.push_str(&render));
            }
            queue!(
                self.editor_contents,
                terminal::Clear(ClearType::UntilNewLine)
            )
            .unwrap();
            self.editor_contents.push_str("\r\n");
        }
    }

    pub fn move_cursor(&mut self, direction: KeyCode) {
        self.cursor
            .move_cursor(direction, &self.rows);
    }

    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        self.cursor.scroll(&self.rows);
        queue!(self.editor_contents, cursor::Hide, cursor::MoveTo(0, 0))?;
        self.draw_rows();
        self.draw_status_bar();
        self.draw_message_bar();
        let cursor_x = self.cursor.render_x - self.cursor.column_offset;
        let cursor_y = self.cursor.cursor_y - self.cursor.row_offset;
        queue!(
            self.editor_contents,
            cursor::MoveTo(cursor_x as u16, cursor_y as u16),
            cursor::Show
        )?;
        self.editor_contents.flush()
    }
}

pub struct StatusMessage {
    message: Option<String>,
    set_time: Option<Instant>,
}

impl StatusMessage {
    pub fn new(initial_message: String) -> Self {
        Self {
            message: Some(initial_message),
            set_time: Some(Instant::now()),
        }
    }

    pub fn set_message(&mut self, message: String) {
        self.message = Some(message);
        self.set_time = Some(Instant::now())
    }

    pub fn message(&mut self) -> Option<&String> {
        self.set_time.and_then(|time| {
            if time.elapsed() > Duration::from_secs(5) {
                self.message = None;
                self.set_time = None;
                None
            } else {
                Some(self.message.as_ref().unwrap())
            }
        })
    }
}

struct SearchIndex {
    pub x_index: usize,
    pub y_index: usize,
    pub x_direction: Option<SearchDirection>,
    pub y_direction: Option<SearchDirection>,
    pub previous_highlight: Option<(usize, Vec<HighlightType>)>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            x_index: 0,
            y_index: 0,
            x_direction: None,
            y_direction: None,
            previous_highlight: None,
        }
    }

    pub fn reset(&mut self) {
        self.y_index = 0;
        self.x_index = 0;
        self.y_direction = None;
        self.x_direction = None;
        self.previous_highlight = None
    }
}