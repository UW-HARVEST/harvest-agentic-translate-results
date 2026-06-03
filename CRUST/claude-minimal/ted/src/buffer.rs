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

    // First line: a fresh gap buffer
    lines_vec.push(Some(GapBuffer::create(line_size)));

    // The rest of the lines are NULL (None)
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
    // Rust handles deallocation automatically when self is dropped.
    drop(self);
}
pub fn move_cursor(&mut self, row: usize, col: usize) {
    let mut row = row;
    if row > self.last_line_loc {
        row = self.last_line_loc;
    }

    self.cursor_row = row;

    let row_str_len = match &self.lines[row] {
        Some(gb) => gb.str_len,
        None => 0,
    };

    let mut col = col;
    if col > row_str_len {
        col = row_str_len;
    }

    // Indicate column moved so the gap can be moved before inserting
    if self.cursor_col != col {
        self.cursor_col_moved = true;
    }

    self.cursor_col = col;
}
pub fn insert(&mut self, ch: char) -> i32 {
    // If cursor column changed, move the gap buffer before inserting
    if self.cursor_col_moved {
        let row = self.cursor_row;
        let col = self.cursor_col;
        if let Some(line) = self.lines[row].as_mut() {
            let err = line.move_gap(col);
            if err != 0 {
                return err;
            }
        }
        self.cursor_col_moved = false;
    }

    let row = self.cursor_row;
    if let Some(line) = self.lines[row].as_mut() {
        let err = line.insert_char(ch);
        if err != 0 {
            return err;
        }
        self.cursor_col = line.gap_loc;
    }

    0
}
pub fn backspace(&mut self) -> i32 {
    if self.cursor_col_moved {
        let row = self.cursor_row;
        let col = self.cursor_col;
        if let Some(line) = self.lines[row].as_mut() {
            let err = line.move_gap(col);
            if err != 0 {
                return err;
            }
        }
        self.cursor_col_moved = false;
    }

    let row = self.cursor_row;
    if let Some(line) = self.lines[row].as_mut() {
        line.backspace();
        self.cursor_col = line.gap_loc;
    }

    0
}
pub fn new_line(&mut self) -> i32 {
    // First ensure the gap location reflects the cursor position
    if self.cursor_col_moved {
        let row = self.cursor_row;
        let col = self.cursor_col;
        if let Some(line) = self.lines[row].as_mut() {
            let err = line.move_gap(col);
            if err != 0 {
                return err;
            }
        }
        self.cursor_col_moved = false;
    }

    // Split the current GapBuffer at the gap location
    let row = self.cursor_row;
    let newline = match self.lines[row].as_mut() {
        Some(line) => line.split(),
        None => return MEM_ERROR,
    };

    // Check if there's space to add a new line. If not, expand the lines vector
    if self.last_line_loc == self.lines_capacity - 1 {
        let new_capacity = self.lines_capacity * 2;
        self.lines.resize_with(new_capacity, || None);
        self.lines_capacity = new_capacity;
    }

    // Shift lines down by one starting from cursor_row + 1
    // memmove(lines + row + 2, lines + row + 1, sizeof * (last_line_loc - row))
    // Move elements from positions [row+1 .. last_line_loc] to [row+2 .. last_line_loc+1]
    let count = self.last_line_loc - self.cursor_row;
    if count > 0 {
        // Shift right starting from the end to avoid overwriting
        for i in (0..count).rev() {
            self.lines.swap(self.cursor_row + 1 + i, self.cursor_row + 2 + i);
        }
    }

    // Set the new line to the next slot
    let new_gap_loc = newline.gap_loc;
    self.lines[self.cursor_row + 1] = Some(newline);

    // Update the last line location
    self.last_line_loc += 1;

    // Update the cursor position
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
    let mut new_tbuffer = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

    // Reset state: clear the first line and reset last_line_loc
    new_tbuffer.lines[0] = None;
    // C uses last_line_loc = -1; we'll use a flag-style approach with i32 or work via index offset.
    // Since last_line_loc is usize, we'll track via insertion index instead.
    let mut next_index: usize = 0;
    let mut have_lines = false;

    let reader = BufReader::new(fp);
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => return None,
        };

        // Reallocate if out of space
        if next_index == new_tbuffer.lines_capacity {
            let new_capacity = new_tbuffer.lines_capacity * 2;
            new_tbuffer.lines.resize_with(new_capacity, || None);
            new_tbuffer.lines_capacity = new_capacity;
        }

        // gap size = max(DEFAULT_GAP_BUF_CAP, line.len() * 2)
        let line_gap_size = if line.len() * 2 < DEFAULT_GAP_BUF_CAP {
            DEFAULT_GAP_BUF_CAP
        } else {
            line.len() * 2
        };

        new_tbuffer.lines[next_index] = Some(GapBuffer::create_from_string(&line, line_gap_size));
        next_index += 1;
        have_lines = true;
    }

    if have_lines {
        new_tbuffer.last_line_loc = next_index - 1;
    } else {
        // No lines were read; restore an empty initial line
        new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
        new_tbuffer.last_line_loc = 0;
    }

    Some(new_tbuffer)
}
}
