use crate::gap::{GapBuffer};
use crate::gap::MEM_ERROR;
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

let mut text_buffer = Self {
lines: vec![None; lines],
lines_capacity: lines,
cursor_row: 0,
cursor_col: 0,
cursor_col_moved: false,
last_line_loc: 0,
};
text_buffer.lines[0] = Some(GapBuffer::create(line_size));
Some(text_buffer)
}
pub fn destroy(self) {
drop(self);
}
pub fn move_cursor(&mut self, row: usize, col: usize) {
let clamped_row = row.min(self.last_line_loc);
self.cursor_row = clamped_row;

let max_col = self
.lines
.get(clamped_row)
.and_then(Option::as_ref)
.map(|line| line.str_len)
.unwrap_or(0);
let clamped_col = col.min(max_col);

if self.cursor_col != clamped_col {
self.cursor_col_moved = true;
}

self.cursor_col = clamped_col;
}
pub fn insert(&mut self, ch: char) -> i32 {
if self.cursor_col_moved {
let cursor_col = self.cursor_col;
let line = match self.current_line_mut() {
Some(line) => line,
None => return MEM_ERROR,
};
let err = line.move_gap(cursor_col);
if err != 0 {
return err;
}
self.cursor_col_moved = false;
}

let line = match self.current_line_mut() {
Some(line) => line,
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
let cursor_col = self.cursor_col;
let line = match self.current_line_mut() {
Some(line) => line,
None => return MEM_ERROR,
};
let err = line.move_gap(cursor_col);
if err != 0 {
return err;
}
self.cursor_col_moved = false;
}

let line = match self.current_line_mut() {
Some(line) => line,
None => return MEM_ERROR,
};
line.backspace();
self.cursor_col = line.gap_loc;
0
}
pub fn new_line(&mut self) -> i32 {
if self.cursor_col_moved {
let cursor_col = self.cursor_col;
let line = match self.current_line_mut() {
Some(line) => line,
None => return MEM_ERROR,
};
let err = line.move_gap(cursor_col);
if err != 0 {
return err;
}
self.cursor_col_moved = false;
}

let newline = match self.lines.get(self.cursor_row).and_then(Option::as_ref) {
Some(line) => line.split(),
None => return MEM_ERROR,
};

if self.last_line_loc == self.lines_capacity.saturating_sub(1) {
let new_capacity = self.lines_capacity.saturating_mul(2).max(1);
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
let mut new_tbuffer = Self::create(DEFAULT_CAPACITY, DEFAULT_GAP_BUF_CAP)?;
let mut reader = BufReader::new(fp);
let mut line = String::new();
let mut next_line = 0usize;
let mut read_any = false;

loop {
line.clear();
let read = reader.read_line(&mut line).ok()?;
if read == 0 {
break;
}

read_any = true;

if line.ends_with('\n') {
line.pop();
}
if line.ends_with('\r') {
line.pop();
}

if next_line == new_tbuffer.lines_capacity {
let new_capacity = new_tbuffer.lines_capacity.saturating_mul(2).max(1);
new_tbuffer.lines.resize_with(new_capacity, || None);
new_tbuffer.lines_capacity = new_capacity;
}

let line_gap_size = (read * 2).max(DEFAULT_GAP_BUF_CAP);
new_tbuffer.lines[next_line] = Some(GapBuffer::create_from_string(&line, line_gap_size));
next_line += 1;
}

if read_any {
new_tbuffer.last_line_loc = next_line.saturating_sub(1);
} else {
new_tbuffer.lines[0] = Some(GapBuffer::create(DEFAULT_GAP_BUF_CAP));
new_tbuffer.last_line_loc = 0;
}

Some(new_tbuffer)
}

fn current_line_mut(&mut self) -> Option<&mut GapBuffer> {
self.lines.get_mut(self.cursor_row).and_then(Option::as_mut)
}
}
