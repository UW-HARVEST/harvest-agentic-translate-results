use crate::gap::GapBuffer;
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
        let mut buf_lines: Vec<Option<GapBuffer>> = Vec::with_capacity(lines);
        buf_lines.push(Some(GapBuffer::create(line_size)));
        for _ in 1..lines {
            buf_lines.push(None);
        }

        Some(TextBuffer {
            lines: buf_lines,
            lines_capacity: lines,
            cursor_row: 0,
            cursor_col: 0,
            cursor_col_moved: false,
            last_line_loc: 0,
        })
    }

    pub fn destroy(self) {
        // Rust drops everything for us when self is consumed.
        drop(self);
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        // row is usize so >= 0 always; clamp to last_line_loc
        let mut row = row;
        if row > self.last_line_loc {
            row = self.last_line_loc;
        }

        self.cursor_row = row;

        let mut col = col;
        let line = self.lines[row].as_ref().expect("line must exist");
        if col > line.str_len {
            col = line.str_len;
        }

        if self.cursor_col != col {
            self.cursor_col_moved = true;
        }

        self.cursor_col = col;
    }

    pub fn insert(&mut self, ch: char) -> i32 {
        if self.cursor_col_moved {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let line = self.lines[row].as_mut().expect("line must exist");
            let err = line.move_gap(col);
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        let row = self.cursor_row;
        let line = self.lines[row].as_mut().expect("line must exist");
        let err = line.insert_char(ch);
        if err != 0 {
            return err;
        }

        self.cursor_col = line.gap_loc;
        0
    }

    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let line = self.lines[row].as_mut().expect("line must exist");
            let err = line.move_gap(col);
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        let row = self.cursor_row;
        let line = self.lines[row].as_mut().expect("line must exist");
        line.backspace();
        self.cursor_col = line.gap_loc;

        0
    }

    pub fn new_line(&mut self) -> i32 {
        if self.cursor_col_moved {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let line = self.lines[row].as_mut().expect("line must exist");
            let err = line.move_gap(col);
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        // Split the current GapBuffer at the gap location.
        // The C code mutates the original. Replicate by computing the new line
        // and then mutating the original to drop its second half.
        let row = self.cursor_row;
        let newline = {
            let line = self.lines[row].as_ref().expect("line must exist");
            line.split()
        };

        // mutate original: str_len = gap_loc, gap_len = capacity - str_len
        {
            let line = self.lines[row].as_mut().expect("line must exist");
            let capacity = line.gap_len + line.str_len;
            line.str_len = line.gap_loc;
            line.gap_len = capacity - line.str_len;
        }

        // Check capacity
        if self.last_line_loc == self.lines_capacity - 1 {
            let new_cap = self.lines_capacity * 2;
            self.lines.resize(new_cap, None);
            self.lines_capacity = new_cap;
        }

        // Shift lines down: lines[row+2..last_line_loc+2] = lines[row+1..last_line_loc+1]
        // Equivalent to inserting at position row+1.
        // Move elements one slot right between row+1 and last_line_loc inclusive.
        let shift_count = self.last_line_loc - self.cursor_row;
        // We do an explicit shift; since we have Option<GapBuffer>, we can swap.
        for i in (0..shift_count).rev() {
            // index moving from (row+1+i) to (row+2+i)
            let src = self.cursor_row + 1 + i;
            let dst = self.cursor_row + 2 + i;
            self.lines[dst] = self.lines[src].take();
        }

        // Place new line and update cursor
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
        match self.lines.get(row) {
            Some(Some(line)) => Some(line.get_string()),
            _ => None,
        }
    }

    pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
        let mut new_tbuffer = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

        // Read all lines from file
        let reader = BufReader::new(fp);

        let mut first = true;
        // last_line_loc starts at 0 (with one empty line). For the first line read,
        // we replace line 0 directly. For subsequent lines we append.
        let mut current_lines = 0usize;
        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => return None,
            };

            // grow capacity if needed
            // The "out of space" check kicks in when current_lines == capacity-1
            // (we are about to put data into slot current_lines, but treating it
            // like the C version which checks last_line_loc).
            if current_lines == new_tbuffer.lines_capacity - 1 {
                let new_cap = new_tbuffer.lines_capacity * 2;
                new_tbuffer.lines.resize(new_cap, None);
                new_tbuffer.lines_capacity = new_cap;
            }

            // gap size = max(DEFAULT_GAP_BUF_CAP, line.len() * 2)
            let read_len = line.len();
            let gap_size = if read_len * 2 < DEFAULT_GAP_BUF_CAP {
                DEFAULT_GAP_BUF_CAP
            } else {
                read_len * 2
            };

            let gb = GapBuffer::create_from_string(&line, gap_size);

            if first {
                // Replace existing line 0 (which is empty) with the new line
                new_tbuffer.lines[0] = Some(gb);
                new_tbuffer.last_line_loc = 0;
                first = false;
            } else {
                let idx = current_lines;
                new_tbuffer.lines[idx] = Some(gb);
                new_tbuffer.last_line_loc = idx;
            }
            current_lines += 1;
        }

        Some(new_tbuffer)
    }
}
