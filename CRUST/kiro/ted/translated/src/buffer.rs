use crate::gap::GapBuffer;
use std::io::BufRead;

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
        let mut v: Vec<Option<GapBuffer>> = Vec::with_capacity(lines);
        v.push(Some(GapBuffer::create(line_size)));
        for _ in 1..lines {
            v.push(None);
        }
        Some(TextBuffer {
            lines: v,
            lines_capacity: lines,
            cursor_row: 0,
            cursor_col: 0,
            cursor_col_moved: false,
            last_line_loc: 0,
        })
    }
    pub fn destroy(self) {
        // drop
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
            let line = self.lines[self.cursor_row].as_mut().unwrap();
            let err = line.move_gap(self.cursor_col);
            if err != 0 { return err; }
            self.cursor_col_moved = false;
        }
        let line = self.lines[self.cursor_row].as_mut().unwrap();
        let err = line.insert_char(ch);
        if err != 0 { return err; }
        self.cursor_col = self.lines[self.cursor_row].as_ref().unwrap().gap_loc;
        0
    }
    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let line = self.lines[self.cursor_row].as_mut().unwrap();
            let err = line.move_gap(self.cursor_col);
            if err != 0 { return err; }
            self.cursor_col_moved = false;
        }
        let line = self.lines[self.cursor_row].as_mut().unwrap();
        line.backspace();
        self.cursor_col = self.lines[self.cursor_row].as_ref().unwrap().gap_loc;
        0
    }
    pub fn new_line(&mut self) -> i32 {
        if self.cursor_col_moved {
            let line = self.lines[self.cursor_row].as_mut().unwrap();
            let err = line.move_gap(self.cursor_col);
            if err != 0 { return err; }
            self.cursor_col_moved = false;
        }
        let newline = self.lines[self.cursor_row].as_ref().unwrap().split();

        // Grow if needed
        if self.last_line_loc == self.lines_capacity - 1 {
            let new_cap = self.lines_capacity * 2;
            self.lines.resize_with(new_cap, || None);
            self.lines_capacity = new_cap;
        }

        // Shift lines down
        let shift_count = self.last_line_loc - self.cursor_row;
        for i in (0..shift_count).rev() {
            let src = self.cursor_row + 1 + i;
            let dst = self.cursor_row + 2 + i;
            self.lines[dst] = self.lines[src].take();
        }

        let gap_loc = newline.gap_loc;
        self.lines[self.cursor_row + 1] = Some(newline);
        self.last_line_loc += 1;
        self.cursor_row += 1;
        self.cursor_col = gap_loc;
        0
    }
    pub fn get_line(&self, row: usize) -> Option<String> {
        if row > self.last_line_loc {
            return None;
        }
        self.lines[row].as_ref().map(|gb| gb.get_string())
    }
    pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
        let mut tb = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;
        // Destroy the first line, reset
        tb.lines[0] = None;
        // Use a signed counter to track; start at -1 like C code
        let mut line_count: i64 = -1;

        let reader = std::io::BufReader::new(fp);
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => break,
            };

            // Reallocate if out of space
            if line_count as usize == tb.lines_capacity.wrapping_sub(1) && line_count >= 0 {
                let new_cap = tb.lines_capacity * 2;
                tb.lines.resize_with(new_cap, || None);
                tb.lines_capacity = new_cap;
            }

            let read_len = line.len();
            let gap_size = if read_len * 2 < DEFAULT_GAP_BUF_CAP { DEFAULT_GAP_BUF_CAP } else { read_len * 2 };
            line_count += 1;
            tb.lines[line_count as usize] = Some(GapBuffer::create_from_string(&line, gap_size));
        }

        if line_count >= 0 {
            tb.last_line_loc = line_count as usize;
        } else {
            // No lines read - create empty buffer like C does
            tb.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
            tb.last_line_loc = 0;
        }

        Some(tb)
    }
}
