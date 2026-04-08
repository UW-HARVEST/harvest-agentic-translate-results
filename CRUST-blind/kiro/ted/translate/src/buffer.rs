use crate::gap::{GapBuffer};
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
        let mut line_vec: Vec<Option<GapBuffer>> = Vec::with_capacity(lines);
        line_vec.push(Some(GapBuffer::create(line_size)));
        for _ in 1..lines {
            line_vec.push(None);
        }
        Some(TextBuffer {
            lines: line_vec,
            lines_capacity: lines,
            cursor_row: 0,
            cursor_col: 0,
            cursor_col_moved: false,
            last_line_loc: 0,
        })
    }
    pub fn destroy(self) {
        // Drop handles deallocation
    }
    pub fn move_cursor(&mut self, row: usize, col: usize) {
        let row = if row > self.last_line_loc { self.last_line_loc } else { row };
        self.cursor_row = row;

        let line = self.lines[row].as_ref().unwrap();
        let col = if col > line.str_len { line.str_len } else { col };

        if self.cursor_col != col {
            self.cursor_col_moved = true;
        }
        self.cursor_col = col;
    }
    pub fn insert(&mut self, ch: char) -> i32 {
        if self.cursor_col_moved {
            let err = self.lines[self.cursor_row].as_mut().unwrap().move_gap(self.cursor_col);
            if err != 0 { return err; }
            self.cursor_col_moved = false;
        }
        let err = self.lines[self.cursor_row].as_mut().unwrap().insert_char(ch);
        if err != 0 { return err; }
        self.cursor_col = self.lines[self.cursor_row].as_ref().unwrap().gap_loc;
        0
    }
    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let err = self.lines[self.cursor_row].as_mut().unwrap().move_gap(self.cursor_col);
            if err != 0 { return err; }
            self.cursor_col_moved = false;
        }
        self.lines[self.cursor_row].as_mut().unwrap().backspace();
        self.cursor_col = self.lines[self.cursor_row].as_ref().unwrap().gap_loc;
        0
    }
    pub fn new_line(&mut self) -> i32 {
        if self.cursor_col_moved {
            let err = self.lines[self.cursor_row].as_mut().unwrap().move_gap(self.cursor_col);
            if err != 0 { return err; }
            self.cursor_col_moved = false;
        }

        // split() takes &self, so we get the new line from it
        let current = self.lines[self.cursor_row].as_ref().unwrap();
        let new_gap = current.split();

        // Now update the original line: set str_len to gap_loc, gap_len to capacity - str_len
        let orig = self.lines[self.cursor_row].as_mut().unwrap();
        let capacity = orig.gap_len + orig.str_len;
        orig.str_len = orig.gap_loc;
        orig.gap_len = capacity - orig.str_len;

        // Ensure capacity for new line
        if self.last_line_loc == self.lines_capacity - 1 {
            self.lines.resize_with(self.lines_capacity * 2, || None);
            self.lines_capacity *= 2;
        }

        // Shift lines down
        let shift_count = self.last_line_loc - self.cursor_row;
        for i in (0..shift_count).rev() {
            let src = self.cursor_row + 1 + i;
            let dst = self.cursor_row + 2 + i;
            self.lines[dst] = self.lines[src].take();
        }

        let new_gap_loc = new_gap.gap_loc;
        self.lines[self.cursor_row + 1] = Some(new_gap);
        self.last_line_loc += 1;
        self.cursor_row += 1;
        self.cursor_col = new_gap_loc;
        0
    }
    pub fn get_line(&self, row: usize) -> Option<String> {
        if row > self.last_line_loc {
            return None;
        }
        self.lines[row].as_ref().map(|gb| gb.get_string())
    }
    pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
        use std::io::{BufRead, BufReader};

        let mut tb = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

        // Destroy the first line, reset last_line_loc
        tb.lines[0] = None;
        // Use wrapping sub to simulate -1; we'll use a flag instead
        let mut has_lines = false;
        let mut line_index: usize = 0;

        let reader = BufReader::new(fp);
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => return None,
            };

            // Reallocate if out of space
            if has_lines && line_index == tb.lines_capacity - 1 {
                tb.lines.resize_with(tb.lines_capacity * 2, || None);
                tb.lines_capacity *= 2;
            }

            let read_len = line.len();
            let line_gap_size = if read_len * 2 < DEFAULT_GAP_BUF_CAP { DEFAULT_GAP_BUF_CAP } else { read_len * 2 };

            if !has_lines {
                has_lines = true;
                line_index = 0;
            } else {
                line_index += 1;
            }

            tb.lines[line_index] = Some(GapBuffer::create_from_string(&line, line_gap_size));
        }

        if has_lines {
            tb.last_line_loc = line_index;
        } else {
            // Empty file - create a default empty buffer
            tb.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
            tb.last_line_loc = 0;
        }

        Some(tb)
    }
}
