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
pub fn create(num_lines: usize, line_size: usize) -> Option<Self> {
    if num_lines == 0 {
        return None;
    }
    let mut lines: Vec<Option<GapBuffer>> = Vec::with_capacity(num_lines);
    lines.push(Some(GapBuffer::create(line_size)));
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
    let line_str_len = self.lines[row].as_ref().map(|l| l.str_len).unwrap_or(0);
    let col = if col > line_str_len { line_str_len } else { col };
    if self.cursor_col != col {
        self.cursor_col_moved = true;
    }
    self.cursor_col = col;
}
pub fn insert(&mut self, ch: char) -> i32 {
    if self.cursor_col_moved {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let err = if let Some(line) = self.lines[row].as_mut() {
            line.move_gap(col)
        } else {
            return MEM_ERROR;
        };
        if err != 0 {
            return err;
        }
        self.cursor_col_moved = false;
    }
    let row = self.cursor_row;
    let err = if let Some(line) = self.lines[row].as_mut() {
        line.insert_char(ch)
    } else {
        return MEM_ERROR;
    };
    if err != 0 {
        return err;
    }
    self.cursor_col = self.lines[row].as_ref().map(|l| l.gap_loc).unwrap_or(0);
    0
}
pub fn backspace(&mut self) -> i32 {
    if self.cursor_col_moved {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let err = if let Some(line) = self.lines[row].as_mut() {
            line.move_gap(col)
        } else {
            return MEM_ERROR;
        };
        if err != 0 {
            return err;
        }
        self.cursor_col_moved = false;
    }
    let row = self.cursor_row;
    if let Some(line) = self.lines[row].as_mut() {
        line.backspace();
    } else {
        return MEM_ERROR;
    }
    self.cursor_col = self.lines[row].as_ref().map(|l| l.gap_loc).unwrap_or(0);
    0
}
pub fn new_line(&mut self) -> i32 {
    if self.cursor_col_moved {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let err = if let Some(line) = self.lines[row].as_mut() {
            line.move_gap(col)
        } else {
            return MEM_ERROR;
        };
        if err != 0 {
            return err;
        }
        self.cursor_col_moved = false;
    }
    // Split the current GapBuffer at the gap location
    let row = self.cursor_row;
    let newline = match self.lines[row].as_mut() {
        Some(line) => line.split(),
        None => return MEM_ERROR,
    };

    // ensure space
    if self.last_line_loc == self.lines_capacity - 1 {
        let new_cap = self.lines_capacity * 2;
        self.lines.resize_with(new_cap, || None);
        self.lines_capacity = new_cap;
    }

    // Shift lines down by one (from cursor_row+1 to last_line_loc, move to cursor_row+2)
    // count = last_line_loc - cursor_row
    let count = self.last_line_loc - self.cursor_row;
    if count > 0 {
        // move from cursor_row+1..cursor_row+1+count to cursor_row+2..cursor_row+2+count
        // i.e. take elements out and reinsert
        for i in (0..count).rev() {
            let src = self.cursor_row + 1 + i;
            let dst = self.cursor_row + 2 + i;
            self.lines[dst] = self.lines[src].take();
        }
    }

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
    self.lines[row].as_ref().map(|l| l.get_string())
}
pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
    let mut new_tbuffer = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;
    // Drop the first line (replace with None) and reset last_line_loc
    new_tbuffer.lines[0] = None;
    // Use sentinel: in C last_line_loc = -1 to indicate "no lines yet";
    // we'll track it via Option-style using a separate flag.
    // We'll use a temporary i32 to mirror C semantics.
    let mut last_line_loc: i32 = -1;

    let reader = BufReader::new(fp);
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => return None,
        };
        // BufReader::lines() already strips the trailing '\n'.
        let read = line.len() + 1; // approximate read length (with newline)

        // reallocate if out of space
        if last_line_loc == (new_tbuffer.lines_capacity as i32) - 1 {
            let new_cap = new_tbuffer.lines_capacity * 2;
            new_tbuffer.lines.resize_with(new_cap, || None);
            new_tbuffer.lines_capacity = new_cap;
        }

        let line_gap_size = if read * 2 < DEFAULT_GAP_BUF_CAP {
            DEFAULT_GAP_BUF_CAP
        } else {
            read * 2
        };

        let new_line_buf = GapBuffer::create_from_string(&line, line_gap_size);
        new_tbuffer.lines[(last_line_loc + 1) as usize] = Some(new_line_buf);
        last_line_loc += 1;
    }

    if last_line_loc < 0 {
        // No lines were read; reset to a fresh single empty line
        new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
        new_tbuffer.last_line_loc = 0;
    } else {
        new_tbuffer.last_line_loc = last_line_loc as usize;
    }
    Some(new_tbuffer)
}
}
