use crate::gap::{GapBuffer};
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
    let mut vec_lines: Vec<Option<GapBuffer>> = Vec::with_capacity(lines);
    vec_lines.push(Some(GapBuffer::create(line_size)));
    for _ in 1..lines {
        vec_lines.push(None);
    }
    Some(TextBuffer {
        lines: vec_lines,
        lines_capacity: lines,
        cursor_row: 0,
        cursor_col: 0,
        cursor_col_moved: false,
        last_line_loc: 0,
    })
}
pub fn destroy(self) {
    // resources released automatically
}
pub fn move_cursor(&mut self, row: usize, col: usize) {
    let mut row = row;
    if row > self.last_line_loc {
        row = self.last_line_loc;
    }

    self.cursor_row = row;

    let line_str_len = self.lines[row]
        .as_ref()
        .map(|l| l.str_len)
        .unwrap_or(0);

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
    if self.cursor_col_moved {
        let err = if let Some(line) = self.lines[self.cursor_row].as_mut() {
            line.move_gap(self.cursor_col)
        } else {
            return crate::gap::MEM_ERROR;
        };

        if err != 0 {
            return err;
        }

        self.cursor_col_moved = false;
    }

    let (err, new_gap_loc) = if let Some(line) = self.lines[self.cursor_row].as_mut() {
        let e = line.insert_char(ch);
        (e, line.gap_loc)
    } else {
        return crate::gap::MEM_ERROR;
    };

    if err != 0 {
        return err;
    }

    self.cursor_col = new_gap_loc;
    0
}
pub fn backspace(&mut self) -> i32 {
    if self.cursor_col_moved {
        let err = if let Some(line) = self.lines[self.cursor_row].as_mut() {
            line.move_gap(self.cursor_col)
        } else {
            return crate::gap::MEM_ERROR;
        };

        if err != 0 {
            return err;
        }

        self.cursor_col_moved = false;
    }

    let new_gap_loc = if let Some(line) = self.lines[self.cursor_row].as_mut() {
        line.backspace();
        line.gap_loc
    } else {
        return crate::gap::MEM_ERROR;
    };

    self.cursor_col = new_gap_loc;
    0
}
pub fn new_line(&mut self) -> i32 {
    if self.cursor_col_moved {
        let err = if let Some(line) = self.lines[self.cursor_row].as_mut() {
            line.move_gap(self.cursor_col)
        } else {
            return crate::gap::MEM_ERROR;
        };

        if err != 0 {
            return err;
        }

        self.cursor_col_moved = false;
    }

    // Split the current GapBuffer at the gap location
    let newline = if let Some(line) = self.lines[self.cursor_row].as_mut() {
        line.split()
    } else {
        return crate::gap::MEM_ERROR;
    };

    let newline_gap_loc = newline.gap_loc;

    // Check if there's space to add a new line. If not, expand `lines`.
    if self.last_line_loc == self.lines_capacity - 1 {
        let new_capacity = self.lines_capacity * 2;
        self.lines.resize_with(new_capacity, || None);
        self.lines_capacity = new_capacity;
    }

    // Shift the lines array down one place from cursor_row+1 to last_line_loc.
    let shift_count = self.last_line_loc - self.cursor_row;
    if shift_count > 0 {
        // Move lines[cursor_row+1..=last_line_loc] to lines[cursor_row+2..=last_line_loc+1]
        for i in (0..shift_count).rev() {
            let src = self.cursor_row + 1 + i;
            let dst = self.cursor_row + 2 + i;
            self.lines[dst] = self.lines[src].take();
        }
    }

    // Set the new line to the next available slot
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

    self.lines[row].as_ref().map(|l| l.get_string())
}
pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
    let mut new_tbuffer = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

    // Reset the first line; we will append lines from file.
    new_tbuffer.lines[0] = None;
    // Use a sentinel: we set last_line_loc to usize::MAX initially to act as -1.
    // Since the field is usize, we model "-1" by tracking with an Option-style flag.
    // We'll instead start at 0 then handle the first line specially.
    let mut have_any_lines = false;

    let reader = BufReader::new(fp);
    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => return None,
        };

        let read = line.chars().count();
        let line_gap_size = if read * 2 < DEFAULT_GAP_BUF_CAP {
            DEFAULT_GAP_BUF_CAP
        } else {
            read * 2
        };

        let next_index = if !have_any_lines {
            0
        } else {
            new_tbuffer.last_line_loc + 1
        };

        // Reallocate if out of space
        if next_index >= new_tbuffer.lines_capacity {
            let new_capacity = new_tbuffer.lines_capacity * 2;
            new_tbuffer.lines.resize_with(new_capacity, || None);
            new_tbuffer.lines_capacity = new_capacity;
        }

        new_tbuffer.lines[next_index] =
            Some(GapBuffer::create_from_string(&line, line_gap_size));

        if !have_any_lines {
            have_any_lines = true;
            new_tbuffer.last_line_loc = 0;
        } else {
            new_tbuffer.last_line_loc += 1;
        }
    }

    if !have_any_lines {
        // File is empty; restore an empty first line.
        new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
        new_tbuffer.last_line_loc = 0;
    }

    Some(new_tbuffer)
}
}
