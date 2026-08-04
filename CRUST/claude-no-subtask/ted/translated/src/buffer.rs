use crate::gap::{GapBuffer, MEM_ERROR};
use std::io::{BufRead, BufReader};

pub const DEFAULT_GAP_BUF_CAP: usize = 100;
pub const DEFAULT_CAPACITY: usize = 100;

pub struct TextBuffer {
    pub lines: Vec<Option<GapBuffer>>,
    pub lines_capacity: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_col_moved: bool,
    pub last_line_loc: usize,
}

impl TextBuffer {
    pub fn create(lines: usize, line_size: usize) -> Option<Self> {
        if lines == 0 {
            return None;
        }

        let mut lines_vec: Vec<Option<GapBuffer>> = Vec::with_capacity(lines);
        // Initialize the first line
        lines_vec.push(Some(GapBuffer::create(line_size)));
        // Fill the rest with None
        for _ in 1..lines {
            lines_vec.push(None);
        }

        Some(TextBuffer {
            lines: lines_vec,
            lines_capacity: lines,
            cursor_row: 0,
            cursor_col: 0,
            cursor_col_moved: false,
            last_line_loc: 0,
        })
    }

    pub fn destroy(self) {
        drop(self);
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        let row = if row > self.last_line_loc {
            self.last_line_loc
        } else {
            row
        };

        self.cursor_row = row;

        let str_len = self.lines[row].as_ref().map(|gb| gb.str_len).unwrap_or(0);
        let col = if col > str_len { str_len } else { col };

        if self.cursor_col != col {
            self.cursor_col_moved = true;
        }

        self.cursor_col = col;
    }

    pub fn insert(&mut self, ch: char) -> i32 {
        if self.cursor_col_moved {
            let cursor_col = self.cursor_col;
            if let Some(gb) = self.lines[self.cursor_row].as_mut() {
                let err = gb.move_gap(cursor_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        if let Some(gb) = self.lines[self.cursor_row].as_mut() {
            let err = gb.insert_char(ch);
            if err != 0 {
                return err;
            }
            self.cursor_col = gb.gap_loc;
        } else {
            return MEM_ERROR;
        }

        0
    }

    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let cursor_col = self.cursor_col;
            if let Some(gb) = self.lines[self.cursor_row].as_mut() {
                let err = gb.move_gap(cursor_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        if let Some(gb) = self.lines[self.cursor_row].as_mut() {
            gb.backspace();
            self.cursor_col = gb.gap_loc;
        } else {
            return MEM_ERROR;
        }

        0
    }

    pub fn new_line(&mut self) -> i32 {
        if self.cursor_col_moved {
            let cursor_col = self.cursor_col;
            if let Some(gb) = self.lines[self.cursor_row].as_mut() {
                let err = gb.move_gap(cursor_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        // Split the current GapBuffer at the gap location
        let newline = if let Some(gb) = self.lines[self.cursor_row].as_mut() {
            gb.split()
        } else {
            return MEM_ERROR;
        };

        // Reallocate if no space
        if self.last_line_loc == self.lines_capacity - 1 {
            let new_cap = self.lines_capacity * 2;
            self.lines.resize(new_cap, None);
            self.lines_capacity = new_cap;
        }

        // Shift the lines down by one starting from cursor_row + 1
        // memmove(lines + cursor_row + 2, lines + cursor_row + 1, sizeof * (last_line_loc - cursor_row))
        // i.e. shift entries [cursor_row + 1 .. last_line_loc + 1) down by one to [cursor_row + 2 .. last_line_loc + 2)
        let count = self.last_line_loc - self.cursor_row;
        if count > 0 {
            for i in (0..count).rev() {
                self.lines[self.cursor_row + 2 + i] = self.lines[self.cursor_row + 1 + i].take();
            }
        }

        let newline_gap_loc = newline.gap_loc;
        self.lines[self.cursor_row + 1] = Some(newline);

        self.last_line_loc += 1;
        self.cursor_row += 1;
        self.cursor_col = newline_gap_loc;

        0
    }

    pub fn get_line(&self, row: usize) -> Option<String> {
        if row > self.last_line_loc {
            return None;
        }

        self.lines[row].as_ref().map(|gb| gb.get_string())
    }

    pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
        let mut new_tbuffer = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

        // Drop the first line, set last_line_loc to a sentinel (-1 in C, here we use a special handling)
        new_tbuffer.lines[0] = None;
        // Use isize-like behavior: last_line_loc starts at -1, but we use usize.
        // Track via a separate variable.
        let mut last_line_loc: isize = -1;

        let reader = BufReader::new(fp);

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => return None,
            };

            // Reallocate if out of space
            if last_line_loc == new_tbuffer.lines_capacity as isize - 1 {
                let new_cap = new_tbuffer.lines_capacity * 2;
                new_tbuffer.lines.resize(new_cap, None);
                new_tbuffer.lines_capacity = new_cap;
            }

            let read = line.chars().count();
            let line_gap_size = if read * 2 < DEFAULT_GAP_BUF_CAP {
                DEFAULT_GAP_BUF_CAP
            } else {
                read * 2
            };

            let new_gap = GapBuffer::create_from_string(&line, line_gap_size);
            let new_idx = (last_line_loc + 1) as usize;
            new_tbuffer.lines[new_idx] = Some(new_gap);
            last_line_loc += 1;
        }

        if last_line_loc < 0 {
            new_tbuffer.last_line_loc = 0;
        } else {
            new_tbuffer.last_line_loc = last_line_loc as usize;
        }

        Some(new_tbuffer)
    }
}
