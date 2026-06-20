use ted::buffer::{TextBuffer};
use ted::defs::{panic};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;

thread_local! {
static EDITOR_STATE: RefCell<Option<EditorState>> = const { RefCell::new(None) };
}

pub struct EditorState {
orig_termios: termios::Termios,
file_name: Option<String>,
file_path: Option<String>,
flushed: bool,
current_buffer: TextBuffer,
screen: VirtualScreen,
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
pub struct Cursor {
x: usize,
y: usize,
}

impl EditorState {
pub fn initialize(argc: i32, argv: Vec<String>) {
let file_path = if argc >= 2 {
argv.get(1).cloned().unwrap_or_else(|| "Empty Buffer".to_string())
} else {
"Empty Buffer".to_string()
};
let file_name = Path::new(&file_path)
.file_name()
.map(|name| name.to_string_lossy().into_owned())
.or_else(|| Some(file_path.clone()));
let current_buffer = File::open(&file_path)
.ok()
.and_then(|file| TextBuffer::create_from_file(&file))
.or_else(|| TextBuffer::create(100, 100))
.unwrap_or_else(|| {
panic("failed to initialize text buffer");
TextBuffer::create(1, 1).unwrap_or_else(|| unreachable!())
});
let orig_termios = termios::Termios::from_fd(std::io::stdin().as_raw_fd())
.or_else(|_| termios::Termios::from_fd(std::io::stdout().as_raw_fd()))
.or_else(|_| termios::Termios::from_fd(std::io::stderr().as_raw_fd()))
.unwrap_or_else(|_| {
panic("tcgetattr");
termios::Termios::from_fd(std::io::stdin().as_raw_fd()).ok().unwrap_or_else(|| unreachable!())
});
let mut screen = VirtualScreen {
buffer: Vec::new(),
buf_pos: 0,
len: 0,
cursor: Cursor { x: 1, y: 1 },
width: 80,
height: 24,
render_start_line: 0,
};
screen.len = screen.width * screen.height * 2;
screen.buffer = vec!['\0'; screen.len];

EDITOR_STATE.with(|state| {
*state.borrow_mut() = Some(EditorState {
orig_termios,
file_name,
file_path: Some(file_path),
flushed: true,
current_buffer,
screen,
});
});

Self::set_window_size();
}
pub fn cleanup() {
EDITOR_STATE.with(|state| {
*state.borrow_mut() = None;
});
}
pub fn set_window_size() {
let width = std::env::var("COLUMNS")
.ok()
.and_then(|value| value.parse::<usize>().ok())
.filter(|value| *value > 0)
.unwrap_or(80);
let height = std::env::var("LINES")
.ok()
.and_then(|value| value.parse::<usize>().ok())
.filter(|value| *value > 0)
.unwrap_or(24);

EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
editor.screen.width = width;
editor.screen.height = height;
editor.screen.len = width.saturating_mul(height).saturating_mul(2);
editor.screen.buffer.resize(editor.screen.len, '\0');
}
});
}
pub fn disable_raw_mode() {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow().as_ref() {
let _ = termios::tcsetattr(std::io::stdin().as_raw_fd(), termios::TCSAFLUSH, &editor.orig_termios);
}
});
}
pub fn enable_raw_mode() {
let fd = std::io::stdin().as_raw_fd();
let mut raw = match termios::Termios::from_fd(fd) {
Ok(raw) => raw,
Err(_) => return,
};

raw.c_lflag &= !(termios::ECHO | termios::ICANON | termios::ISIG | termios::IEXTEN);
raw.c_iflag &= !(termios::ICRNL | termios::IXON | termios::BRKINT | termios::INPCK);
raw.c_oflag &= !termios::OPOST;
raw.c_cflag |= termios::CS8;
raw.c_cc[termios::VMIN] = 0;
raw.c_cc[termios::VTIME] = 1;

let _ = termios::tcsetattr(fd, termios::TCSAFLUSH, &raw);
}
pub fn render_screen() {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow().as_ref() {
let output: String = editor.screen.buffer.iter().take(editor.screen.buf_pos).collect();
let _ = std::io::stdout().write_all(output.as_bytes());
let _ = std::io::stdout().flush();
}
});
}
pub fn draw_screen() {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
editor.screen.buf_pos = 0;
editor.screen.buffer.fill('\0');

for row in 0..=editor.current_buffer.last_line_loc {
if let Some(line) = editor.current_buffer.get_line(row) {
for ch in line.chars() {
if editor.screen.buf_pos >= editor.screen.buffer.len() {
editor.screen.buffer.push(ch);
editor.screen.len = editor.screen.buffer.len();
} else {
editor.screen.buffer[editor.screen.buf_pos] = ch;
}
editor.screen.buf_pos += 1;
}
if editor.screen.buf_pos + 2 <= editor.screen.buffer.len() {
editor.screen.buffer[editor.screen.buf_pos] = '\r';
editor.screen.buffer[editor.screen.buf_pos + 1] = '\n';
} else {
editor.screen.buffer.push('\r');
editor.screen.buffer.push('\n');
editor.screen.len = editor.screen.buffer.len();
}
editor.screen.buf_pos += 2;
}
}
}
});
}
pub fn draw_status_line(line_size: usize) {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let status = format!(
"{} - {} lines{}",
editor.file_name.clone().unwrap_or_else(|| "Empty Buffer".to_string()),
line_size,
if editor.flushed { "" } else { " (modified)" }
);
for ch in status.chars() {
if editor.screen.buf_pos >= editor.screen.buffer.len() {
editor.screen.buffer.push(ch);
editor.screen.len = editor.screen.buffer.len();
} else {
editor.screen.buffer[editor.screen.buf_pos] = ch;
}
editor.screen.buf_pos += 1;
}
}
});
}
pub fn up_arrow() {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let new_row = editor.current_buffer.cursor_row.saturating_sub(1);
let col = editor.current_buffer.cursor_col;
editor.current_buffer.move_cursor(new_row, col);
}
});
}
pub fn down_arrow() {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let new_row = editor.current_buffer.cursor_row.saturating_add(1);
let col = editor.current_buffer.cursor_col;
editor.current_buffer.move_cursor(new_row, col);
}
});
}
pub fn right_arrow() {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let row = editor.current_buffer.cursor_row;
let new_col = editor.current_buffer.cursor_col.saturating_add(1);
editor.current_buffer.move_cursor(row, new_col);
}
});
}
pub fn left_arrow() {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let row = editor.current_buffer.cursor_row;
let new_col = editor.current_buffer.cursor_col.saturating_sub(1);
editor.current_buffer.move_cursor(row, new_col);
}
});
}
pub fn read_char() -> i32 {
let mut buf = [0u8; 1];
match std::io::stdin().read(&mut buf) {
Ok(1) => i32::from(buf[0]),
_ => -1,
}
}
pub fn process_keypress() {
match Self::read_char() {
127 => {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let _ = editor.current_buffer.backspace();
editor.flushed = false;
}
});
}
10 | 13 => {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let _ = editor.current_buffer.new_line();
editor.flushed = false;
}
});
}
ch if (32..=126).contains(&ch) => {
if let Some(ch) = char::from_u32(ch as u32) {
EDITOR_STATE.with(|state| {
if let Some(editor) = state.borrow_mut().as_mut() {
let _ = editor.current_buffer.insert(ch);
editor.flushed = false;
}
});
}
}
_ => {}
}
}
pub fn flush_buffer_to_file() -> i32 {
EDITOR_STATE.with(|state| {
let mut state_ref = state.borrow_mut();
let editor = match state_ref.as_mut() {
Some(editor) => editor,
None => return -1,
};

let path = match editor.file_path.clone() {
Some(path) if path != "Empty Buffer" => path,
_ => return -1,
};

let mut file = match File::create(path) {
Ok(file) => file,
Err(_) => return -1,
};

for row in 0..=editor.current_buffer.last_line_loc {
let line = match editor.current_buffer.get_line(row) {
Some(line) => line,
None => continue,
};
if file.write_all(line.as_bytes()).is_err() {
return -1;
}
if row != editor.current_buffer.last_line_loc && file.write_all(b"\n").is_err() {
return -1;
}
}

editor.flushed = true;
0
})
}
pub fn load_file_and_initialize_buffer() -> i32 {
EDITOR_STATE.with(|state| {
let mut state_ref = state.borrow_mut();
let editor = match state_ref.as_mut() {
Some(editor) => editor,
None => return -1,
};

let file_path = match editor.file_path.clone() {
Some(path) if path != "Empty Buffer" => path,
_ => {
editor.current_buffer = TextBuffer::create(100, 100).unwrap_or_else(|| {
panic("failed to create empty buffer");
TextBuffer::create(1, 1).unwrap_or_else(|| unreachable!())
});
return -1;
}
};

match File::open(file_path) {
Ok(file) => match TextBuffer::create_from_file(&file) {
Some(buffer) => {
editor.current_buffer = buffer;
0
}
None => -1,
},
Err(_) => {
editor.current_buffer = TextBuffer::create(100, 100).unwrap_or_else(|| {
panic("failed to create empty buffer");
TextBuffer::create(1, 1).unwrap_or_else(|| unreachable!())
});
 -1
}
}
})
}
}
pub fn main(){}
