use std::io::{stdout, ErrorKind, Result, Write};

pub struct Content {
    content: String,
}

impl Content {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    pub fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    pub fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}

impl Write for Content {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(s.len())
            }
            Err(_) => Err(ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> Result<()> {
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        out
    }
}
