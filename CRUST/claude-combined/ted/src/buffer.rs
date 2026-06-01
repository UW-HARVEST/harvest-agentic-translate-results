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
    /// Create a new text buffer with the given number of lines and per-line size.
    pub fn create(num_lines: usize, line_size: usize) -> Option<Self> {
        if num_lines == 0 {
            return None;
        }

        let mut lines: Vec<Option<GapBuffer>> = Vec::with_capacity(num_lines);
        // First line is initialized with a gap buffer
        lines.push(Some(GapBuffer::create(line_size)));
        // Rest are None
        for _ in 1..num_lines {
            lines.push(None);
        }

        Some(TextBuffer {
            lines,
            lines_capacity: num_lines,
            cursor_row: 0,
            cursor_col: 0,
            cursor_col_moved: false,
            last_line_loc: 0,
        })
    }

    /// Cleanup. In Rust we just drop.
    pub fn destroy(self) {
        drop(self);
    }

    /// Move the cursor to (row, col), clamping to valid positions.
    pub fn move_cursor(&mut self, row: usize, col: usize) {
        let row = if row > self.last_line_loc {
            self.last_line_loc
        } else {
            row
        };
        self.cursor_row = row;

        let line_str_len = self.lines[row]
            .as_ref()
            .map(|gb| gb.str_len)
            .unwrap_or(0);
        let col = if col > line_str_len {
            line_str_len
        } else {
            col
        };

        if self.cursor_col != col {
            self.cursor_col_moved = true;
        }

        self.cursor_col = col;
    }

    /// Insert a character at the cursor.
    pub fn insert(&mut self, ch: char) -> i32 {
        if self.cursor_col_moved {
            let cur_col = self.cursor_col;
            let row = self.cursor_row;
            if let Some(gb) = self.lines[row].as_mut() {
                let err = gb.move_gap(cur_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        let row = self.cursor_row;
        let err = if let Some(gb) = self.lines[row].as_mut() {
            gb.insert_char(ch)
        } else {
            return MEM_ERROR;
        };

        if err != 0 {
            return err;
        }

        self.cursor_col = self.lines[row].as_ref().unwrap().gap_loc;
        0
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let cur_col = self.cursor_col;
            let row = self.cursor_row;
            if let Some(gb) = self.lines[row].as_mut() {
                let err = gb.move_gap(cur_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        let row = self.cursor_row;
        if let Some(gb) = self.lines[row].as_mut() {
            gb.backspace();
            self.cursor_col = gb.gap_loc;
        } else {
            return MEM_ERROR;
        }
        0
    }

    /// Insert a new line at the cursor.
    pub fn new_line(&mut self) -> i32 {
        if self.cursor_col_moved {
            let cur_col = self.cursor_col;
            let row = self.cursor_row;
            if let Some(gb) = self.lines[row].as_mut() {
                let err = gb.move_gap(cur_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        let row = self.cursor_row;
        let new_line_buf = if let Some(gb) = self.lines[row].as_mut() {
            gb.split()
        } else {
            return MEM_ERROR;
        };

        // Reallocate if necessary
        if self.last_line_loc == self.lines_capacity - 1 {
            let new_cap = self.lines_capacity * 2;
            self.lines.resize_with(new_cap, || None);
            self.lines_capacity = new_cap;
        }

        // Shift lines down: from cursor_row+1..=last_line_loc shift to cursor_row+2..=last_line_loc+1
        // i.e., insert at index (cursor_row + 1)
        // Move from (last_line_loc..=cursor_row+1).rev() to one position later
        let mut i = self.last_line_loc;
        while i > self.cursor_row {
            self.lines[i + 1] = self.lines[i].take();
            if i == 0 {
                break;
            }
            i -= 1;
        }

        // Set the new line
        self.lines[self.cursor_row + 1] = Some(new_line_buf);

        self.last_line_loc += 1;
        self.cursor_row += 1;
        self.cursor_col = self.lines[self.cursor_row].as_ref().unwrap().gap_loc;

        0
    }

    /// Returns the contents of the line at index `row`, if valid.
    pub fn get_line(&self, row: usize) -> Option<String> {
        if row > self.last_line_loc {
            return None;
        }
        self.lines.get(row).and_then(|opt| opt.as_ref()).map(|gb| gb.get_string())
    }

    /// Create a TextBuffer from a file.
    pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
        let mut new_tbuf = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

        // Reset: drop the first line and treat last_line_loc as "no lines yet"
        new_tbuf.lines[0] = None;
        // Use isize-like sentinel with a flag instead, since we use usize.
        // We'll track number of lines read via a local counter.
        let mut next_idx: usize = 0;
        let mut any_read = false;

        let reader = BufReader::new(fp.try_clone().ok()?);
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => return None,
            };

            // Reallocate if out of space
            if next_idx == new_tbuf.lines_capacity {
                let new_cap = new_tbuf.lines_capacity * 2;
                new_tbuf.lines.resize_with(new_cap, || None);
                new_tbuf.lines_capacity = new_cap;
            }

            // Compute gap size
            let read_len = line.chars().count();
            let line_gap_size = if read_len * 2 < DEFAULT_GAP_BUF_CAP {
                DEFAULT_GAP_BUF_CAP
            } else {
                read_len * 2
            };

            new_tbuf.lines[next_idx] = Some(GapBuffer::create_from_string(&line, line_gap_size));
            next_idx += 1;
            any_read = true;
        }

        if any_read {
            new_tbuf.last_line_loc = next_idx - 1;
        } else {
            // No lines read: re-initialize one empty line so the buffer is valid
            new_tbuf.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
            new_tbuf.last_line_loc = 0;
        }

        Some(new_tbuf)
    }
}
