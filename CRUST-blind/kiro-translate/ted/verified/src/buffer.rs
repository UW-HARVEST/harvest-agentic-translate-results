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
    // Rust drops automatically
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

    // Split the current gap buffer
    let current = self.lines[self.cursor_row].as_ref().unwrap();
    let new_gb = current.split();

    // Update the original buffer's str_len and gap_len (since split() is &self in Rust)
    let orig = self.lines[self.cursor_row].as_mut().unwrap();
    let capacity = orig.str_len + orig.gap_len;
    orig.str_len = orig.gap_loc;
    orig.gap_len = capacity - orig.str_len;

    // Ensure capacity
    if self.last_line_loc == self.lines_capacity - 1 {
        self.lines.resize_with(self.lines_capacity * 2, || None);
        self.lines_capacity *= 2;
    }

    // Shift lines down
    let new_gap_loc = new_gb.gap_loc;
    // Insert at cursor_row + 1, shifting everything after down
    self.lines.insert(self.cursor_row + 1, Some(new_gb));
    // Remove the extra None at the end to maintain capacity (insert grows the vec)
    // Actually we need to keep lines_capacity consistent. The insert added one element.
    // We should pop the last element if it was None to keep the vec at lines_capacity size.
    // But actually, let's just truncate to lines_capacity if needed.
    if self.lines.len() > self.lines_capacity {
        self.lines.truncate(self.lines_capacity);
    }

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
    // Use wrapping sub to simulate -1; we'll use a signed tracker
    let mut line_count: i64 = -1;

    let reader = BufReader::new(fp);
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };

        // Reallocate if out of space
        if line_count == (tb.lines_capacity as i64 - 1) {
            let new_cap = tb.lines_capacity * 2;
            tb.lines.resize_with(new_cap, || None);
            tb.lines_capacity = new_cap;
        }

        let read_len = line.len();
        let line_gap_size = if read_len * 2 < DEFAULT_GAP_BUF_CAP { DEFAULT_GAP_BUF_CAP } else { read_len * 2 };

        line_count += 1;
        tb.lines[line_count as usize] = Some(GapBuffer::create_from_string(&line, line_gap_size));
    }

    if line_count >= 0 {
        tb.last_line_loc = line_count as usize;
    } else {
        // Empty file - create a blank buffer like the C code would with no lines read
        // Actually in C, if no lines are read, last_line_loc stays at -1 which is problematic.
        // Let's match C behavior: if file was empty, we still have last_line_loc = -1 equivalent.
        // But since we use usize, let's just create a fresh buffer.
        return TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP);
    }

    Some(tb)
}
}
