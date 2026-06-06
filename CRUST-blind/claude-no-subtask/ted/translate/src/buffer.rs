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
        // First line is allocated.
        buf_lines.push(Some(GapBuffer::create(line_size)));

        // Remaining lines are None
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
        // In Rust, the buffer is dropped automatically when self is dropped.
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        // row is usize, so it can't be < 0. Clamp to last_line_loc.
        let row = if row > self.last_line_loc {
            self.last_line_loc
        } else {
            row
        };

        self.cursor_row = row;

        let line_str_len = match &self.lines[row] {
            Some(line) => line.str_len,
            None => 0,
        };

        let col = if col > line_str_len { line_str_len } else { col };

        if self.cursor_col != col {
            self.cursor_col_moved = true;
        }

        self.cursor_col = col;
    }

    pub fn insert(&mut self, ch: char) -> i32 {
        if self.cursor_col_moved {
            let target_col = self.cursor_col;
            if let Some(line) = self.lines[self.cursor_row].as_mut() {
                let err = line.move_gap(target_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        let new_gap_loc;
        if let Some(line) = self.lines[self.cursor_row].as_mut() {
            let err = line.insert_char(ch);
            if err != 0 {
                return err;
            }
            new_gap_loc = line.gap_loc;
        } else {
            return MEM_ERROR;
        }

        self.cursor_col = new_gap_loc;
        0
    }

    pub fn backspace(&mut self) -> i32 {
        if self.cursor_col_moved {
            let target_col = self.cursor_col;
            if let Some(line) = self.lines[self.cursor_row].as_mut() {
                let err = line.move_gap(target_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        let new_gap_loc;
        if let Some(line) = self.lines[self.cursor_row].as_mut() {
            line.backspace();
            new_gap_loc = line.gap_loc;
        } else {
            return MEM_ERROR;
        }

        self.cursor_col = new_gap_loc;
        0
    }

    pub fn new_line(&mut self) -> i32 {
        // Ensure gap is at the cursor location
        if self.cursor_col_moved {
            let target_col = self.cursor_col;
            if let Some(line) = self.lines[self.cursor_row].as_mut() {
                let err = line.move_gap(target_col);
                if err != 0 {
                    return err;
                }
            } else {
                return MEM_ERROR;
            }
            self.cursor_col_moved = false;
        }

        // Split the current GapBuffer at the gap location.
        // We need to mutate the current line as well: in C, GapBufferSplit modifies the original.
        let (new_line, _) = {
            let line_ref = match self.lines[self.cursor_row].as_mut() {
                Some(l) => l,
                None => return MEM_ERROR,
            };

            // Compute split parameters
            let capacity = line_ref.gap_len + line_ref.str_len;
            let second_half_of_str_len = line_ref.str_len - line_ref.gap_loc;

            // Build new buffer for the new line.
            let mut new_buf: Vec<char> = vec!['\0'; capacity];
            let dst_start = capacity - second_half_of_str_len;
            let src_start = line_ref.gap_loc + line_ref.gap_len;
            for i in 0..second_half_of_str_len {
                new_buf[dst_start + i] = line_ref.buffer[src_start + i];
            }

            let new_line = GapBuffer {
                buffer: new_buf,
                str_len: second_half_of_str_len,
                gap_loc: 0,
                gap_len: capacity - second_half_of_str_len,
            };

            // Update the original line
            line_ref.str_len = line_ref.gap_loc;
            line_ref.gap_len = capacity - line_ref.str_len;

            (new_line, ())
        };

        // Check if we need to grow the lines array
        if self.last_line_loc == self.lines_capacity - 1 {
            let new_cap = self.lines_capacity * 2;
            // Extend with None
            self.lines.resize(new_cap, None);
            self.lines_capacity = new_cap;
        }

        // Shift the lines down by one (from cursor_row+1 onwards through last_line_loc)
        // Move element at index `cursor_row+1+i` to `cursor_row+2+i` for i in 0..(last_line_loc-cursor_row)
        let shift_count = self.last_line_loc - self.cursor_row;
        if shift_count > 0 {
            for i in (0..shift_count).rev() {
                let src = self.cursor_row + 1 + i;
                let dst = self.cursor_row + 2 + i;
                self.lines[dst] = self.lines[src].take();
            }
        }

        // Place the new line at cursor_row + 1
        let new_gap_loc = new_line.gap_loc;
        self.lines[self.cursor_row + 1] = Some(new_line);

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

        // Reset: drop the first line and indicate no lines yet (use a sentinel mechanism).
        // C uses last_line_loc = -1 (signed). We'll use an Option-style approach: set
        // last_line_loc to 0 but mark the first slot as None, and after the first line is
        // added, increment from -1 -> 0 (in C terms). To replicate: we'll track a separate
        // counter.
        new_tbuffer.lines[0] = None;
        // Use isize-style counter: we'll start from -1
        let mut last_line: isize = -1;

        let reader = BufReader::new(fp);
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => return None,
            };

            // Reallocate if out of space
            if (last_line + 1) as usize == new_tbuffer.lines_capacity - 1 {
                let new_cap = new_tbuffer.lines_capacity * 2;
                new_tbuffer.lines.resize(new_cap, None);
                new_tbuffer.lines_capacity = new_cap;
            }

            // Determine gap size: max(DEFAULT_GAP_BUF_CAP, line.len() * 2)
            let line_gap_size = if line.len() * 2 < DEFAULT_GAP_BUF_CAP {
                DEFAULT_GAP_BUF_CAP
            } else {
                line.len() * 2
            };

            let gb = GapBuffer::create_from_string(&line, line_gap_size);

            let new_index = (last_line + 1) as usize;
            new_tbuffer.lines[new_index] = Some(gb);
            last_line += 1;
        }

        if last_line < 0 {
            // No lines were read; the C version would leave last_line_loc at -1, but our
            // type uses usize. Reset to a single empty line state.
            new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
            new_tbuffer.last_line_loc = 0;
        } else {
            new_tbuffer.last_line_loc = last_line as usize;
        }

        Some(new_tbuffer)
    }
}
