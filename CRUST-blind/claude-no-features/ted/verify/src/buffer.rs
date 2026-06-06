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

    let mut buf_lines: Vec<Option<GapBuffer>> = Vec::with_capacity(lines);
    // The first line is allocated, the rest are None.
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
    // Rust handles deallocation automatically.
    drop(self);
}
pub fn move_cursor(&mut self, row: usize, col: usize) {
    let mut row = row;
    if row > self.last_line_loc {
        row = self.last_line_loc;
    }

    self.cursor_row = row;

    let line_str_len = match &self.lines[row] {
        Some(b) => b.str_len,
        None => 0,
    };

    let mut col = col;
    if col > line_str_len {
        col = line_str_len;
    }

    if self.cursor_col != col {
        self.cursor_col_moved = true;
    }

    self.cursor_col = col;
}
pub fn insert(&mut self, ch: char) -> i32 {
    // If the cursor column changed, move the gap buffer before inserting
    if self.cursor_col_moved {
        let target_col = self.cursor_col;
        let line = match self.lines[self.cursor_row].as_mut() {
            Some(l) => l,
            None => return MEM_ERROR,
        };
        let err = line.move_gap(target_col);
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
        let target_col = self.cursor_col;
        let line = match self.lines[self.cursor_row].as_mut() {
            Some(l) => l,
            None => return MEM_ERROR,
        };
        let err = line.move_gap(target_col);
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
    // Move the gap if needed
    if self.cursor_col_moved {
        let target_col = self.cursor_col;
        let line = match self.lines[self.cursor_row].as_mut() {
            Some(l) => l,
            None => return MEM_ERROR,
        };
        let err = line.move_gap(target_col);
        if err != 0 {
            return err;
        }
        self.cursor_col_moved = false;
    }

    // Split the current GapBuffer
    let (newline, new_gap_loc) = {
        let line = match self.lines[self.cursor_row].as_mut() {
            Some(l) => l,
            None => return MEM_ERROR,
        };
        let split_buf = line.split();
        let gloc = split_buf.gap_loc;
        (split_buf, gloc)
    };

    // Ensure capacity for a new line
    if self.last_line_loc == self.lines_capacity - 1 {
        let new_cap = self.lines_capacity * 2;
        self.lines.resize(new_cap, None);
        self.lines_capacity = new_cap;
    }

    // Shift lines down by one starting from cursor_row+1
    // Move the slice [cursor_row+1 .. last_line_loc+1] to [cursor_row+2 .. last_line_loc+2]
    // We do this manually since we don't have memmove on Option<>.
    let mut i = self.last_line_loc;
    while i > self.cursor_row {
        self.lines[i + 1] = self.lines[i].take();
        if i == 0 {
            break;
        }
        i -= 1;
    }

    // Insert the new line at cursor_row+1
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
    match &self.lines[row] {
        Some(line) => Some(line.get_string()),
        None => None,
    }
}
pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
    let mut new_tbuffer = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

    // Read lines from the file
    // We treat the file the same as the C function: read each line, strip trailing newline,
    // create a gap buffer for it. The first line slot starts unused (overwritten).
    // First "delete" the existing first line - in Rust we'll just replace it.
    new_tbuffer.lines[0] = None;
    // Use isize-equivalent: track whether last_line_loc is "uninitialized" (-1 in C).
    // We use Option<usize> for clarity.
    let mut last_line_loc: Option<usize> = None;

    let reader = BufReader::new(fp);
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => return None,
        };

        // Strip trailing \r if present (BufRead::lines() already strips \n)
        let line = if line.ends_with('\r') {
            &line[..line.len() - 1]
        } else {
            line.as_str()
        };

        let next_idx = match last_line_loc {
            Some(idx) => idx + 1,
            None => 0,
        };

        // Reallocate if out of space
        if next_idx >= new_tbuffer.lines_capacity {
            let new_cap = new_tbuffer.lines_capacity * 2;
            new_tbuffer.lines.resize(new_cap, None);
            new_tbuffer.lines_capacity = new_cap;
        }

        let read = line.len();
        let line_gap_size = if read * 2 < DEFAULT_GAP_BUF_CAP {
            DEFAULT_GAP_BUF_CAP
        } else {
            read * 2
        };

        new_tbuffer.lines[next_idx] = Some(GapBuffer::create_from_string(line, line_gap_size));
        last_line_loc = Some(next_idx);
    }

    // If no lines were read, the buffer should still be valid: keep initial empty line.
    match last_line_loc {
        Some(idx) => {
            new_tbuffer.last_line_loc = idx;
        }
        None => {
            // No lines read. Restore an empty first line for safe operation.
            new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
            new_tbuffer.last_line_loc = 0;
        }
    }

    Some(new_tbuffer)
}
}
