use ted::buffer::{TextBuffer};
use ted::defs::{panic};
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
let _ = (argc, argv);
}
pub fn cleanup() {
}
pub fn set_window_size() {
}
pub fn disable_raw_mode() {
}
pub fn enable_raw_mode() {
}
pub fn render_screen() {
}
pub fn draw_screen() {
}
pub fn draw_status_line(line_size: usize) {
let _ = line_size;
}
pub fn up_arrow() {
}
pub fn down_arrow() {
}
pub fn right_arrow() {
}
pub fn left_arrow() {
}
pub fn read_char() -> i32 {
-1
}
pub fn process_keypress() {
}
pub fn flush_buffer_to_file() -> i32 {
-1
}
pub fn load_file_and_initialize_buffer() -> i32 {
-1
}
}
pub fn main(){}
