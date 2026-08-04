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
if lines == 0 {
return None;
}

let mut buffer_lines = vec![None; lines];
buffer_lines[0] = Some(GapBuffer::create(line_size));

Some(Self {
lines: buffer_lines,
lines_capacity: lines,
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
let row = row.min(self.last_line_loc);
self.cursor_row = row;

let line_len = self
.lines
.get(row)
.and_then(Option::as_ref)
.map(|line| line.str_len)
.unwrap_or(0);
let col = col.min(line_len);

if self.cursor_col != col {
self.cursor_col_moved = true;
}

self.cursor_col = col;
}
pub fn insert(&mut self, ch: char) -> i32 {
if self.cursor_col_moved {
let cursor_col = self.cursor_col;
let err = match self.current_line_mut() {
Some(line) => line.move_gap(cursor_col),
None => return crate::gap::MEM_ERROR,
};
if err != 0 {
return err;
}
self.cursor_col_moved = false;
}

let err = match self.current_line_mut() {
Some(line) => line.insert_char(ch),
None => return crate::gap::MEM_ERROR,
};
if err != 0 {
return err;
}

if let Some(line) = self.current_line_ref() {
self.cursor_col = line.gap_loc;
}
0
}
pub fn backspace(&mut self) -> i32 {
if self.cursor_col_moved {
let cursor_col = self.cursor_col;
let err = match self.current_line_mut() {
Some(line) => line.move_gap(cursor_col),
None => return crate::gap::MEM_ERROR,
};
if err != 0 {
return err;
}
self.cursor_col_moved = false;
}

if let Some(line) = self.current_line_mut() {
line.backspace();
self.cursor_col = line.gap_loc;
return 0;
}

crate::gap::MEM_ERROR
}
pub fn new_line(&mut self) -> i32 {
if self.cursor_col_moved {
let cursor_col = self.cursor_col;
let err = match self.current_line_mut() {
Some(line) => line.move_gap(cursor_col),
None => return crate::gap::MEM_ERROR,
};
if err != 0 {
return err;
}
self.cursor_col_moved = false;
}

let newline = {
let current = match self.current_line_mut() {
Some(line) => line,
None => return crate::gap::MEM_ERROR,
};

let capacity = current.str_len + current.gap_len;
let second_half_len = current.str_len.saturating_sub(current.gap_loc);
let mut newline = GapBuffer::create(capacity);
let dst_start = capacity - second_half_len;
let src_start = current.gap_loc + current.gap_len;
newline.buffer[dst_start..dst_start + second_half_len]
.copy_from_slice(&current.buffer[src_start..src_start + second_half_len]);
newline.str_len = second_half_len;
newline.gap_loc = 0;
newline.gap_len = capacity - second_half_len;

current.str_len = current.gap_loc;
current.gap_len = capacity - current.str_len;
newline
};

if self.last_line_loc == self.lines_capacity.saturating_sub(1) {
let new_capacity = self.lines_capacity.saturating_mul(2).max(self.lines_capacity + 1);
self.lines.resize_with(new_capacity, || None);
self.lines_capacity = new_capacity;
}

if self.last_line_loc > self.cursor_row {
for idx in (self.cursor_row + 1..=self.last_line_loc).rev() {
self.lines[idx + 1] = self.lines[idx].take();
}
}

self.lines[self.cursor_row + 1] = Some(newline);
self.last_line_loc += 1;
self.cursor_row += 1;
self.cursor_col = self
.lines
.get(self.cursor_row)
.and_then(Option::as_ref)
.map(|line| line.gap_loc)
.unwrap_or(0);
0
}
pub fn get_line(&self, row: usize) -> Option<String> {
if row > self.last_line_loc {
return None;
}

self.lines.get(row).and_then(Option::as_ref).map(GapBuffer::get_string)
}
pub fn create_from_file(fp: &std::fs::File) -> Option<Self> {
use std::io::{BufRead, BufReader};

let mut new_tbuffer = Self::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;
new_tbuffer.lines[0] = None;
new_tbuffer.cursor_row = 0;
new_tbuffer.cursor_col = 0;
new_tbuffer.cursor_col_moved = false;
new_tbuffer.last_line_loc = 0;

let mut inserted_any = false;
let reader = BufReader::new(fp);

for line in reader.lines() {
let line = line.ok()?;

if inserted_any && new_tbuffer.last_line_loc == new_tbuffer.lines_capacity - 1 {
let new_capacity = new_tbuffer.lines_capacity.saturating_mul(2).max(new_tbuffer.lines_capacity + 1);
new_tbuffer.lines.resize_with(new_capacity, || None);
new_tbuffer.lines_capacity = new_capacity;
}

let gap_size = (line.len() * 2).max(DEFAULT_GAP_BUF_CAP);
let next_index = if inserted_any {
new_tbuffer.last_line_loc + 1
} else {
0
};
new_tbuffer.lines[next_index] = Some(GapBuffer::create_from_string(&line, gap_size));
new_tbuffer.last_line_loc = next_index;
inserted_any = true;
}

if !inserted_any {
new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
new_tbuffer.last_line_loc = 0;
}

Some(new_tbuffer)
}
}

impl TextBuffer {
fn current_line_mut(&mut self) -> Option<&mut GapBuffer> {
self.lines.get_mut(self.cursor_row).and_then(Option::as_mut)
}

fn current_line_ref(&self) -> Option<&GapBuffer> {
self.lines.get(self.cursor_row).and_then(Option::as_ref)
}
}
