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
let chars: Vec<char> = str.chars().take(size).collect();
let required_len = self.buf_pos + chars.len();
if required_len > self.buffer.len() {
self.buffer.resize(required_len, '\0');
}
for ch in chars {
self.buffer[self.buf_pos] = ch;
self.buf_pos += 1;
}
if self.buf_pos > self.len {
self.len = self.buf_pos;
}
}
pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
if line_length == 0 {
1
} else if screen_width == 0 {
0
} else {
((line_length / screen_width) + usize::from(line_length % screen_width > 0)) as i32
}
}
pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
let buffer_cursor_row = buffer.cursor_row;
let mut cumul_req_rows = 0usize;
let mut cur_line = screen.render_start_line;
let height_limit = screen.height.saturating_sub(1);

if buffer_cursor_row < screen.render_start_line {
screen.render_start_line = buffer_cursor_row;
return;
}

while cur_line <= buffer.last_line_loc {
let cur_line_required_rows = buffer
.lines
.get(cur_line)
.and_then(Option::as_ref)
.map(|line| Self::required_screen_rows(line.str_len, screen.width) as usize)
.unwrap_or(0);

if cur_line_required_rows + cumul_req_rows > height_limit {
cur_line = cur_line.saturating_sub(1);
break;
}

cumul_req_rows += cur_line_required_rows;
cur_line += 1;
}

if buffer_cursor_row > cur_line {
let mut rows_required = 0usize;
let mut probe_line = cur_line;

while probe_line <= buffer_cursor_row {
rows_required += buffer
.lines
.get(probe_line)
.and_then(Option::as_ref)
.map(|line| Self::required_screen_rows(line.str_len, screen.width) as usize)
.unwrap_or(0);
probe_line += 1;
}

while rows_required > 0 {
rows_required = rows_required.saturating_sub(
buffer
.lines
.get(screen.render_start_line)
.and_then(Option::as_ref)
.map(|line| Self::required_screen_rows(line.str_len, screen.width) as usize)
.unwrap_or(0),
);
screen.render_start_line += 1;
}
}
}
pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
let mut cur_line = screen.render_start_line;
let mut lines_written = 0usize;
let max_rows = screen.height.saturating_sub(1);
let fill_rows = screen.height.saturating_sub(2);

while cur_line <= buffer.last_line_loc && lines_written < max_rows {
let screen_cols = screen.width;
let line = if let Some(line) = buffer.get_line(cur_line) {
line
} else {
crate::defs::panic("draw editor cant get text of current line in buffer");
String::new()
};

if line.len() > screen_cols {
let chars: Vec<char> = line.chars().collect();
let mut i = 0usize;

loop {
let len_to_write = screen_cols.min(chars.len().saturating_sub(i));
let chunk: String = chars[i..i + len_to_write].iter().collect();
screen.screen_append(&chunk, len_to_write);
screen.screen_append("\r\n", 2);
screen.screen_append("\x1b[K", 3);
i += len_to_write;
lines_written += 1;

if lines_written == fill_rows {
break;
}

if i >= chars.len().saturating_sub(1) {
break;
}
}
} else {
screen.screen_append(&line, line.len());
screen.screen_append("\r\n", 2);
lines_written += 1;
}

cur_line += 1;
}

for _ in lines_written..fill_rows {
screen.screen_append("\r\n", 2);
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
.map(|line| Self::required_screen_rows(line.str_len, screen.width) as usize)
.unwrap_or(0);
virtual_cursor_row += required_rows;
current_line += 1;
}

if screen.width > 0 {
virtual_cursor_row += buffer.cursor_col / screen.width;
screen.cursor.x = virtual_cursor_row;
screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
} else {
screen.cursor.x = virtual_cursor_row;
screen.cursor.y = 1;
}
}
}
