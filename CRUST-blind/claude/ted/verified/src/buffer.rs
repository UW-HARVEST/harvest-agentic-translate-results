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
        // First line is allocated; rest are None.
        lines_vec.push(Some(GapBuffer::create(line_size)));
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
        // Drop drops everything.
        drop(self);
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        // Clamp row to [0, last_line_loc]
        let row = if row > self.last_line_loc {
            self.last_line_loc
        } else {
            row
        };

        self.cursor_row = row;

        let line_str_len = match &self.lines[row] {
            Some(gb) => gb.str_len,
            None => 0,
        };

        let col = if col > line_str_len { line_str_len } else { col };

        if self.cursor_col != col {
            self.cursor_col_moved = true;
        }

        self.cursor_col = col;
    }

    pub fn insert(&mut self, ch: char) -> i32 {
        // If the cursor column changed, we need to move the gap before inserting
        if self.cursor_col_moved {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let err = match self.lines[row].as_mut() {
                Some(gb) => gb.move_gap(col),
                None => return MEM_ERROR,
            };
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        let row = self.cursor_row;
        let err = match self.lines[row].as_mut() {
            Some(gb) => gb.insert_char(ch),
            None => return MEM_ERROR,
        };
        if err != 0 {
            return err;
        }

        self.cursor_col = match &self.lines[row] {
            Some(gb) => gb.gap_loc,
            None => 0,
        };
        0
    }

    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let err = match self.lines[row].as_mut() {
                Some(gb) => gb.move_gap(col),
                None => return MEM_ERROR,
            };
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        let row = self.cursor_row;
        if let Some(gb) = self.lines[row].as_mut() {
            gb.backspace();
            self.cursor_col = gb.gap_loc;
        }
        0
    }

    pub fn new_line(&mut self) -> i32 {
        // Ensure the gap location reflects the cursor position
        if self.cursor_col_moved {
            let row = self.cursor_row;
            let col = self.cursor_col;
            let err = match self.lines[row].as_mut() {
                Some(gb) => gb.move_gap(col),
                None => return MEM_ERROR,
            };
            if err != 0 {
                return err;
            }
            self.cursor_col_moved = false;
        }

        // Split the current GapBuffer at the gap location.
        // We need to mutate the original to truncate it to the prefix, and create a new buffer
        // for the suffix.
        let row = self.cursor_row;
        let new_line = match self.lines[row].as_mut() {
            Some(gb) => {
                let capacity = gb.gap_len + gb.str_len;
                let second_half_len = gb.str_len - gb.gap_loc;

                // Build the new gap buffer for the second half
                let mut new_buffer: Vec<char> = vec!['\0'; capacity];
                for i in 0..second_half_len {
                    new_buffer[(capacity - second_half_len) + i] =
                        gb.buffer[gb.gap_loc + gb.gap_len + i];
                }
                let new_gb = GapBuffer {
                    buffer: new_buffer,
                    str_len: second_half_len,
                    gap_loc: 0,
                    gap_len: capacity - second_half_len,
                };

                // Now mutate the original to keep only the first half
                gb.str_len = gb.gap_loc;
                gb.gap_len = capacity - gb.str_len;

                new_gb
            }
            None => return MEM_ERROR,
        };

        // Ensure capacity for one more line.
        if self.last_line_loc == self.lines_capacity - 1 {
            let new_cap = self.lines_capacity * 2;
            self.lines.resize_with(new_cap, || None);
            self.lines_capacity = new_cap;
        }

        // Shift lines[cursor_row+1 .. last_line_loc+1] to one slot down.
        // Insert `new_line` at cursor_row + 1.
        // Equivalent to: memmove(lines + r+2, lines + r+1, sizeof(*) * (last_line_loc - r))
        // i.e. positions [r+1 .. last_line_loc] get moved to [r+2 .. last_line_loc+1]
        // We'll use Vec::insert to insert the new line at index r+1.
        // But we must keep lines_capacity unchanged (Vec::insert grows the Vec). To keep
        // the structure consistent with C, pop the last (None) element after insert if needed.
        let insert_idx = self.cursor_row + 1;
        self.lines.insert(insert_idx, Some(new_line));
        // After insert, the Vec grew by one; remove from the end to preserve capacity invariant.
        if self.lines.len() > self.lines_capacity {
            self.lines.pop();
        }

        self.last_line_loc += 1;
        self.cursor_row += 1;
        // The new line's gap_loc is 0
        self.cursor_col = 0;

        0
    }

    pub fn get_line(&self, row: usize) -> Option<String> {
        if row > self.last_line_loc {
            return None;
        }
        match &self.lines[row] {
            Some(gb) => Some(gb.get_string()),
            None => None,
        }
    }

    pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
        let mut new_tbuffer = TextBuffer::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;

        // Reset the first line
        new_tbuffer.lines[0] = None;
        // Use isize-like sentinel: in C this was -1.
        // We model "no lines yet" via a separate flag. Since `last_line_loc: usize`, use a
        // local counter that starts at 0 with empty flag.
        let mut have_any_line = false;
        let mut next_index: usize = 0;

        let reader = BufReader::new(fp);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => return None,
            };

            // Reallocate if out of space
            if next_index == new_tbuffer.lines_capacity {
                let new_cap = new_tbuffer.lines_capacity * 2;
                new_tbuffer.lines.resize_with(new_cap, || None);
                new_tbuffer.lines_capacity = new_cap;
            }

            // gap size = max(DEFAULT_GAP_BUF_CAP, line.len() * 2)
            let read_len = line.chars().count();
            let line_gap_size = if read_len * 2 < DEFAULT_GAP_BUF_CAP {
                DEFAULT_GAP_BUF_CAP
            } else {
                read_len * 2
            };

            new_tbuffer.lines[next_index] =
                Some(GapBuffer::create_from_string(&line, line_gap_size));

            next_index += 1;
            have_any_line = true;
        }

        if have_any_line {
            new_tbuffer.last_line_loc = next_index - 1;
        } else {
            // If no lines were read, we need at least one empty line.
            new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
            new_tbuffer.last_line_loc = 0;
        }

        Some(new_tbuffer)
    }
}
