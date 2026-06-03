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
        // First line is initialized
        lines_vec.push(Some(GapBuffer::create(line_size)));
        // The rest are None
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
        // Rust handles deallocation automatically when self is dropped
        drop(self);
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        let row = if row > self.last_line_loc {
            self.last_line_loc
        } else {
            row
        };

        self.cursor_row = row;

        let line_str_len = self.lines[row]
            .as_ref()
            .map(|g| g.str_len)
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

    pub fn insert(&mut self, ch: char) -> i32 {
        // If the cursor column changed, we need to move the gap buffer before inserting.
        if self.cursor_col_moved {
            let line = match self.lines[self.cursor_row].as_mut() {
                Some(l) => l,
                None => return MEM_ERROR,
            };
            let err = line.move_gap(self.cursor_col);
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        let line = match self.lines[self.cursor_row].as_mut() {
            Some(l) => l,
            None => return MEM_ERROR,
        };
        let err = line.insert_char(ch);
        if err != 0 {
            return err;
        }

        self.cursor_col = line.gap_loc;
        0
    }

    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let line = match self.lines[self.cursor_row].as_mut() {
                Some(l) => l,
                None => return MEM_ERROR,
            };
            let err = line.move_gap(self.cursor_col);
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        let line = match self.lines[self.cursor_row].as_mut() {
            Some(l) => l,
            None => return MEM_ERROR,
        };
        line.backspace();
        self.cursor_col = line.gap_loc;

        0
    }

    pub fn new_line(&mut self) -> i32 {
        // Move the gap to current cursor location if necessary.
        if self.cursor_col_moved {
            let line = match self.lines[self.cursor_row].as_mut() {
                Some(l) => l,
                None => return MEM_ERROR,
            };
            let err = line.move_gap(self.cursor_col);
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        // Split the current GapBuffer at the gap.
        let newline = {
            let line = match self.lines[self.cursor_row].as_ref() {
                Some(l) => l,
                None => return MEM_ERROR,
            };
            line.split()
        };

        // Ensure we have capacity to add a new line.
        if self.last_line_loc == self.lines_capacity - 1 {
            let new_cap = self.lines_capacity * 2;
            self.lines.resize_with(new_cap, || None);
            self.lines_capacity = new_cap;
        }

        // Shift the lines after cursor_row down by one position.
        // Move from positions [cursor_row+1 .. last_line_loc+1] to [cursor_row+2 .. last_line_loc+2].
        let mut i = self.last_line_loc + 1;
        while i > self.cursor_row + 1 {
            self.lines[i + 1] = self.lines[i].take();
            i -= 1;
        }
        // Now i == cursor_row + 1; place the new line there.
        let new_gap_loc = newline.gap_loc;
        self.lines[self.cursor_row + 1] = Some(newline);

        self.last_line_loc += 1;
        self.cursor_row += 1;
        self.cursor_col = new_gap_loc;

        0
    }

    pub fn get_line(&self, row: usize) -> Option<String> {
        if row > self.last_line_loc {
            return None;
        }

        match self.lines.get(row).and_then(|x| x.as_ref()) {
            Some(line) => Some(line.get_string()),
            None => None,
        }
    }

    pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
        let mut tbuffer = Self::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

        // Read all lines from the file
        let reader = BufReader::new(fp);
        let mut first_line = true;

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => return None,
            };

            // Compute line gap size: max(DEFAULT_GAP_BUF_CAP, line_len * 2 + 2)
            // C uses `read` which includes the newline; we strip it but compute based
            // on the original size + 1 newline byte.
            let read = line.len() + 1; // +1 for the newline char
            let line_gap_size = if read * 2 < DEFAULT_GAP_BUF_CAP {
                DEFAULT_GAP_BUF_CAP
            } else {
                read * 2
            };

            let new_buf = GapBuffer::create_from_string(&line, line_gap_size);

            if first_line {
                // Replace the initial blank line with the first read line.
                tbuffer.lines[0] = Some(new_buf);
                first_line = false;
                // last_line_loc remains 0 (one line)
            } else {
                // Ensure capacity
                if tbuffer.last_line_loc == tbuffer.lines_capacity - 1 {
                    let new_cap = tbuffer.lines_capacity * 2;
                    tbuffer.lines.resize_with(new_cap, || None);
                    tbuffer.lines_capacity = new_cap;
                }
                tbuffer.lines[tbuffer.last_line_loc + 1] = Some(new_buf);
                tbuffer.last_line_loc += 1;
            }
        }

        Some(tbuffer)
    }
}
