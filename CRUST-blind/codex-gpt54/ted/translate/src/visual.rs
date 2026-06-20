use crate::buffer::{TextBuffer};
pub struct Cursor {
x: usize,
y: usize,
}
pub struct VirtualScreen {
buffer: Vec<char>,
buf_pos: usize,
len: usize,
cursor: Cursor,
width: usize,
height: usize,
render_start_line: usize,
}
impl VirtualScreen {
pub fn screen_append(&mut self, str: &str, size: usize) {
if self.buffer.len() < self.len {
self.buffer.resize(self.len, '\0');
}

if self.len.saturating_sub(self.buf_pos) > size {
for ch in str.chars().take(size) {
if self.buf_pos >= self.buffer.len() {
break;
}
self.buffer[self.buf_pos] = ch;
self.buf_pos += 1;
}
}
}
pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
if screen_width == 0 {
return 0;
}

if line_length == 0 {
1
} else {
((line_length / screen_width) + usize::from(line_length % screen_width > 0)) as i32
}
}
pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
let buffer_cursor_row = buffer.cursor_row;
let mut cumul_req_rows = 0usize;
let mut cur_line = screen.render_start_line;

if buffer_cursor_row < screen.render_start_line {
screen.render_start_line = buffer_cursor_row;
return;
}

while cur_line <= buffer.last_line_loc {
let cur_line_required_rows = buffer
.lines
.get(cur_line)
.and_then(Option::as_ref)
.map(|line| Self::required_screen_rows(line.str_len, screen.width).max(0) as usize)
.unwrap_or(0);

if cur_line_required_rows + cumul_req_rows > screen.height.saturating_sub(1) {
cur_line = cur_line.saturating_sub(1);
break;
}

cumul_req_rows += cur_line_required_rows;
cur_line += 1;
}

if buffer_cursor_row > cur_line {
let mut rows_required = 0usize;
let mut walk_line = cur_line;

while walk_line <= buffer_cursor_row {
rows_required += buffer
.lines
.get(walk_line)
.and_then(Option::as_ref)
.map(|line| Self::required_screen_rows(line.str_len, screen.width).max(0) as usize)
.unwrap_or(0);
walk_line += 1;
}

while rows_required > 0 && screen.render_start_line <= buffer.last_line_loc {
rows_required = rows_required.saturating_sub(
buffer
.lines
.get(screen.render_start_line)
.and_then(Option::as_ref)
.map(|line| Self::required_screen_rows(line.str_len, screen.width).max(0) as usize)
.unwrap_or(0),
);
screen.render_start_line += 1;
}
}
}
pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
let mut cur_line = screen.render_start_line;
let mut lines_written = 0usize;

while cur_line <= buffer.last_line_loc && lines_written < screen.height.saturating_sub(1) {
let screen_cols = screen.width;
let line = match buffer.get_line(cur_line) {
Some(line) => line,
None => {
crate::defs::panic("draw editor cant get text of current line in buffer");
String::new()
}
};

let line_chars: Vec<char> = line.chars().collect();
if line_chars.len() > screen_cols {
let mut i = 0usize;

loop {
let len_to_write = screen_cols.min(line_chars.len().saturating_sub(i));
let chunk: String = line_chars[i..i + len_to_write].iter().collect();
screen.screen_append(&chunk, len_to_write);
screen.screen_append("\r\n", 2);
screen.screen_append("\x1b[K", 3);
i += len_to_write;
lines_written += 1;

if lines_written == screen.height.saturating_sub(2) {
break;
}

if i >= line_chars.len().saturating_sub(1) {
break;
}
}
} else {
screen.screen_append(&line, line_chars.len());
screen.screen_append("\r\n", 2);
lines_written += 1;
}

cur_line += 1;
}

while lines_written < screen.height.saturating_sub(2) {
screen.screen_append("\r\n", 2);
lines_written += 1;
}
}
pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
let mut current_line = screen.render_start_line;
let mut virtual_cursor_row = 1usize;

while current_line != buffer.cursor_row {
let required_rows = buffer
.lines
.get(current_line)
.and_then(Option::as_ref)
.map(|line| Self::required_screen_rows(line.str_len, screen.width).max(0) as usize)
.unwrap_or(0);
virtual_cursor_row += required_rows;
current_line += 1;
}

if screen.width > 0 {
virtual_cursor_row += buffer.cursor_col / screen.width;
screen.cursor.x = virtual_cursor_row;
screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
} else {
screen.cursor.x = 1;
screen.cursor.y = 1;
}
}
}
