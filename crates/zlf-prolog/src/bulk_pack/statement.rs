use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub(crate) struct StatementReader {
    reader: BufReader<File>,
    buffer: String,
    line: String,
    quote: Option<char>,
    escaped: bool,
    comment: bool,
    depth: usize,
}

impl StatementReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            reader: BufReader::new(File::open(path)?),
            buffer: String::new(),
            line: String::new(),
            quote: None,
            escaped: false,
            comment: false,
            depth: 0,
        })
    }

    pub fn next_statement(&mut self) -> io::Result<Option<String>> {
        loop {
            if self.line.is_empty() && self.reader.read_line(&mut self.line)? == 0 {
                return self.end_of_input();
            }
            if let Some(statement) = self.consume_line() {
                return Ok(Some(statement));
            }
            self.line.clear();
            self.comment = false;
        }
    }

    fn end_of_input(&self) -> io::Result<Option<String>> {
        if self.buffer.trim().is_empty() {
            Ok(None)
        } else {
            Err(io::Error::other("unterminated Prolog fact"))
        }
    }

    fn consume_line(&mut self) -> Option<String> {
        let characters = self.line.char_indices().collect::<Vec<_>>();
        for (offset, character) in characters {
            let consumed = offset + character.len_utf8();
            if self.consume_character(character) {
                self.buffer.push(character);
                let statement = std::mem::take(&mut self.buffer);
                self.line.drain(..consumed);
                return Some(statement);
            }
        }
        self.line.clear();
        None
    }

    fn consume_character(&mut self, character: char) -> bool {
        if self.comment {
            return false;
        }
        if self.consume_quoted(character) {
            return false;
        }
        match character {
            '\'' | '"' => {
                self.quote = Some(character);
                self.buffer.push(character);
            }
            '%' => self.comment = true,
            '(' | '[' | '{' => {
                self.depth += 1;
                self.buffer.push(character);
            }
            ')' | ']' | '}' => {
                self.depth = self.depth.saturating_sub(1);
                self.buffer.push(character);
            }
            '.' if self.depth == 0 => return true,
            _ => self.buffer.push(character),
        }
        false
    }

    fn consume_quoted(&mut self, character: char) -> bool {
        if self.escaped {
            self.buffer.push(character);
            self.escaped = false;
            return true;
        }
        if self.quote.is_some() && character == '\\' {
            self.buffer.push(character);
            self.escaped = true;
            return true;
        }
        if let Some(quote) = self.quote {
            self.buffer.push(character);
            if character == quote {
                self.quote = None;
            }
            return true;
        }
        false
    }
}
