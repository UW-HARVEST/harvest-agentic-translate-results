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

const BACKSPACE: i32 = 127;
const ARROW_UP: i32 = 1000;
const ARROW_DOWN: i32 = 1001;
const ARROW_LEFT: i32 = 1002;
const ARROW_RIGHT: i32 = 1003;
const PAGE_UP: i32 = 1004;
const PAGE_DOWN: i32 = 1005;
const HOME_KEY: i32 = 1006;
const END_KEY: i32 = 1007;
const DEL_KEY: i32 = 1008;

fn ctrl_key(k: u8) -> i32 {
    (k & 0x1f) as i32
}

use std::sync::Mutex;
static EDITOR_STATE: Mutex<Option<EditorState>> = Mutex::new(None);

fn screen_append(screen: &mut VirtualScreen, s: &str, size: usize) {
    if screen.len - screen.buf_pos > size {
        for (i, c) in s.chars().take(size).enumerate() {
            if screen.buf_pos < screen.buffer.len() {
                screen.buffer[screen.buf_pos] = c;
                screen.buf_pos += 1;
            }
        }
    }
}

fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
    if line_length == 0 {
        1
    } else {
        (line_length / screen_width) as i32 + if (line_length % screen_width) > 0 { 1 } else { 0 }
    }
}

fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let buffer_cursor_x = buffer.cursor_row;

    if buffer_cursor_x < screen.render_start_line {
        screen.render_start_line = buffer_cursor_x;
        return;
    }

    let mut cumul_req_rows: i32 = 0;
    let mut cur_line = screen.render_start_line;

    while cur_line <= buffer.last_line_loc {
        let cur_line_required_rows = required_screen_rows(
            buffer.lines[cur_line].as_ref().unwrap().str_len,
            screen.width,
        );
        if (cur_line_required_rows + cumul_req_rows) > (screen.height as i32 - 1) {
            if cur_line > 0 { cur_line -= 1; }
            break;
        }
        cumul_req_rows += cur_line_required_rows;
        cur_line += 1;
    }

    if buffer_cursor_x > cur_line {
        let mut rows_required: i32 = 0;
        let mut cl = cur_line;
        while cl <= buffer_cursor_x {
            rows_required += required_screen_rows(
                buffer.lines[cl].as_ref().unwrap().str_len,
                screen.width,
            );
            cl += 1;
        }
        while rows_required > 0 {
            rows_required -= required_screen_rows(
                buffer.lines[screen.render_start_line].as_ref().unwrap().str_len,
                screen.width,
            );
            screen.render_start_line += 1;
        }
    }
}

fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut cur_line = screen.render_start_line;
    let mut lines_written: usize = 0;

    while cur_line <= buffer.last_line_loc && lines_written < screen.height - 1 {
        let screen_cols = screen.width;
        let line = buffer.get_line(cur_line).unwrap_or_else(|| {
            panic("draw editor cant get text of current line in buffer");
            String::new()
        });

        let line_chars: Vec<char> = line.chars().collect();
        let line_len = line_chars.len();

        if line_len > screen_cols {
            let mut i = 0;
            loop {
                let remaining = line_len - i;
                let len_to_write = if screen_cols < remaining { screen_cols } else { remaining };
                let chunk: String = line_chars[i..i + len_to_write].iter().collect();
                screen_append(screen, &chunk, len_to_write);
                screen_append(screen, "\r\n", 2);
                screen_append(screen, "\x1b[K", 3);
                i += len_to_write;
                lines_written += 1;
                if lines_written == screen.height - 2 { break; }
                if i >= line_len - 1 { break; }
            }
        } else {
            screen_append(screen, &line, line_len);
            screen_append(screen, "\r\n", 2);
            lines_written += 1;
        }
        cur_line += 1;
    }

    while lines_written < screen.height - 2 {
        screen_append(screen, "\r\n", 2);
        lines_written += 1;
    }
}

fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut current_line = screen.render_start_line;
    let mut virtual_cursor_row: usize = 1;

    while current_line != buffer.cursor_row {
        let rr = required_screen_rows(
            buffer.lines[current_line].as_ref().unwrap().str_len,
            screen.width,
        );
        virtual_cursor_row += rr as usize;
        current_line += 1;
    }

    virtual_cursor_row += buffer.cursor_col / screen.width;

    screen.cursor.x = virtual_cursor_row;
    screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
}

impl EditorState {
pub fn initialize(argc: i32, argv: Vec<String>) {
    use std::fs::File;

    let file_path = if argc >= 2 {
        Some(argv[1].clone())
    } else {
        None
    };

    let file_name = file_path.as_ref().map(|p| {
        std::path::Path::new(p)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.clone())
    });

    // Load file and initialize buffer
    let current_buffer = if let Some(ref path) = file_path {
        if let Ok(fp) = File::open(path) {
            TextBuffer::create_from_file(&fp).unwrap_or_else(|| {
                TextBuffer::create(ted::buffer::DEFAULT_CAPACITY, ted::buffer::DEFAULT_GAP_BUF_CAP).unwrap()
            })
        } else {
            TextBuffer::create(ted::buffer::DEFAULT_CAPACITY, ted::buffer::DEFAULT_GAP_BUF_CAP).unwrap()
        }
    } else {
        TextBuffer::create(ted::buffer::DEFAULT_CAPACITY, ted::buffer::DEFAULT_GAP_BUF_CAP).unwrap()
    };

    // Enable raw mode
    let orig_termios = termios::Termios::from_fd(0).unwrap_or_else(|_| {
        panic("tcgetattr");
        std::process::exit(1);
    });

    let mut raw = orig_termios;
    use termios::*;
    raw.c_lflag &= !(ECHO | ICANON | ISIG | IEXTEN);
    raw.c_iflag &= !(ICRNL | IXON | BRKINT | INPCK);
    raw.c_oflag &= !OPOST;
    raw.c_cflag |= CS8;
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 1;
    tcsetattr(0, TCSAFLUSH, &raw).unwrap_or_else(|_| {
        panic("tcsetattr");
    });

    // Get window size
    let (width, height) = get_window_size();

    let screen_len = height * width * 2;
    let screen = VirtualScreen {
        buffer: vec!['\0'; screen_len],
        buf_pos: 0,
        len: screen_len,
        cursor: Cursor { x: 0, y: 0 },
        width,
        height,
        render_start_line: 0,
    };

    let state = EditorState {
        orig_termios,
        file_name,
        file_path: file_path.or(Some("Empty Buffer".to_string())),
        flushed: true,
        current_buffer,
        screen,
    };

    *EDITOR_STATE.lock().unwrap() = Some(state);
}
pub fn cleanup() {
    use std::io::Write;
    let _ = std::io::stdout().write_all(b"\x1b[2J");
    let _ = std::io::stdout().write_all(b"\x1b[H");
    let _ = std::io::stdout().flush();

    // Restore terminal
    let guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let _ = termios::tcsetattr(0, termios::TCSAFLUSH, &state.orig_termios);
    }
}
pub fn set_window_size() {
    let (w, h) = get_window_size();
    let mut guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.screen.width = w;
        state.screen.height = h;
    }
}
pub fn disable_raw_mode() {
    let guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let _ = termios::tcsetattr(0, termios::TCSAFLUSH, &state.orig_termios);
    }
}
pub fn enable_raw_mode() {
    let guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let mut raw = state.orig_termios;
        use termios::*;
        raw.c_lflag &= !(ECHO | ICANON | ISIG | IEXTEN);
        raw.c_iflag &= !(ICRNL | IXON | BRKINT | INPCK);
        raw.c_oflag &= !OPOST;
        raw.c_cflag |= CS8;
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 1;
        tcsetattr(0, TCSAFLUSH, &raw).unwrap_or_else(|_| {
            panic("tcsetattr");
        });
    }
}
pub fn render_screen() {
    use std::io::Write;
    let guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let s: String = state.screen.buffer[..state.screen.buf_pos].iter().collect();
        let _ = std::io::stdout().write_all(s.as_bytes());
        let _ = std::io::stdout().flush();
    }
}
pub fn draw_screen() {
    let mut guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.screen.buf_pos = 0;

        // Disable cursor
        screen_append(&mut state.screen, "\x1b[?25l", 6);
        // Clear screen
        screen_append(&mut state.screen, "\x1b[2J", 4);
        // Move cursor to top
        screen_append(&mut state.screen, "\x1b[H", 3);

        move_cursor_in_view(&state.current_buffer, &mut state.screen);
        draw_editor_window(&state.current_buffer, &mut state.screen);

        // Draw status line
        let width = state.screen.width;
        draw_status_line_impl(state, width);

        set_virtual_cursor_position(&state.current_buffer, &mut state.screen);

        let row = state.screen.cursor.x;
        let col = state.screen.cursor.y;
        let buf = format!("\x1b[{};{}H", row, col);
        screen_append(&mut state.screen, &buf, buf.len());

        // Enable cursor
        screen_append(&mut state.screen, "\x1b[?25h", 6);
        // Null terminate equivalent
        screen_append(&mut state.screen, "\0", 1);
    }
}
pub fn draw_status_line(line_size: usize) {
    let mut guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        draw_status_line_impl(state, line_size);
    }
}
pub fn up_arrow() {
    let mut guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let col = state.current_buffer.cursor_col;
        let row = state.current_buffer.cursor_row;
        if row > 0 {
            state.current_buffer.move_cursor(row - 1, col);
        }
    }
}
pub fn down_arrow() {
    let mut guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let col = state.current_buffer.cursor_col;
        let row = state.current_buffer.cursor_row;
        if row < state.current_buffer.last_line_loc {
            state.current_buffer.move_cursor(row + 1, col);
        }
    }
}
pub fn right_arrow() {
    let mut guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let row = state.current_buffer.cursor_row;
        let col = state.current_buffer.cursor_col + 1;
        state.current_buffer.move_cursor(row, col);
    }
}
pub fn left_arrow() {
    let mut guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let row = state.current_buffer.cursor_row;
        let col = state.current_buffer.cursor_col;
        if col > 0 {
            state.current_buffer.move_cursor(row, col - 1);
        }
    }
}
pub fn read_char() -> i32 {
    use std::io::Read;
    let mut buf = [0u8; 1];
    loop {
        match std::io::stdin().read(&mut buf) {
            Ok(1) => break,
            Ok(_) => continue,
            Err(ref e) if e.raw_os_error() == Some(11) => continue, // EAGAIN
            Err(_) => continue,
        }
    }
    let c = buf[0];

    if c == 0x1b {
        let mut seq = [0u8; 3];
        if std::io::stdin().read(&mut seq[0..1]).unwrap_or(0) != 1 { return 0x1b as i32; }
        if std::io::stdin().read(&mut seq[1..2]).unwrap_or(0) != 1 { return 0x1b as i32; }

        if seq[0] == b'[' {
            if seq[1] >= b'0' && seq[1] <= b'9' {
                if std::io::stdin().read(&mut seq[2..3]).unwrap_or(0) != 1 { return 0x1b as i32; }
                if seq[2] == b'~' {
                    return match seq[1] {
                        b'1' => HOME_KEY,
                        b'3' => DEL_KEY,
                        b'4' => END_KEY,
                        b'5' => PAGE_UP,
                        b'6' => PAGE_DOWN,
                        b'7' => HOME_KEY,
                        b'8' => END_KEY,
                        _ => 0x1b as i32,
                    };
                }
            } else {
                return match seq[1] {
                    b'A' => ARROW_UP,
                    b'B' => ARROW_DOWN,
                    b'C' => ARROW_RIGHT,
                    b'D' => ARROW_LEFT,
                    b'H' => HOME_KEY,
                    b'F' => END_KEY,
                    _ => 0x1b as i32,
                };
            }
        } else if seq[0] == b'O' {
            return match seq[1] {
                b'H' => HOME_KEY,
                b'F' => END_KEY,
                _ => 0x1b as i32,
            };
        }
        return 0x1b as i32;
    }
    c as i32
}
pub fn process_keypress() {
    let c = Self::read_char();

    match c {
        c if c == b'\r' as i32 => {
            let mut guard = EDITOR_STATE.lock().unwrap();
            if let Some(ref mut state) = *guard {
                let _ = state.current_buffer.new_line();
            }
        }
        c if c == ARROW_UP => Self::up_arrow(),
        c if c == ARROW_DOWN => Self::down_arrow(),
        c if c == ARROW_LEFT => Self::left_arrow(),
        c if c == ARROW_RIGHT => Self::right_arrow(),
        c if c == PAGE_UP || c == PAGE_DOWN || c == HOME_KEY || c == END_KEY || c == DEL_KEY => {}
        c if c == ctrl_key(b'l') || c == 0x1b => {}
        c if c == ctrl_key(b's') => {
            Self::flush_buffer_to_file();
            let mut guard = EDITOR_STATE.lock().unwrap();
            if let Some(ref mut state) = *guard {
                state.flushed = true;
            }
        }
        c if c == ctrl_key(b'q') => {
            Self::cleanup();
            std::process::exit(0);
        }
        c if c == BACKSPACE || c == ctrl_key(b'h') => {
            let mut guard = EDITOR_STATE.lock().unwrap();
            if let Some(ref mut state) = *guard {
                let _ = state.current_buffer.backspace();
                state.flushed = false;
            }
        }
        _ => {
            let mut guard = EDITOR_STATE.lock().unwrap();
            if let Some(ref mut state) = *guard {
                let _ = state.current_buffer.insert(c as u8 as char);
                state.flushed = false;
            }
        }
    }
}
pub fn flush_buffer_to_file() -> i32 {
    use std::io::Write;
    let guard = EDITOR_STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        let path = match &state.file_path {
            Some(p) => p.clone(),
            None => return -1,
        };
        let mut fp = match std::fs::File::create(&path) {
            Ok(f) => f,
            Err(_) => return -1,
        };
        for i in 0..=state.current_buffer.last_line_loc {
            let line = match state.current_buffer.get_line(i) {
                Some(l) => l,
                None => return -2,
            };
            if fp.write_all(line.as_bytes()).is_err() {
                return -2;
            }
            // Write newline between lines (matching C fputs behavior with newlines)
            if i < state.current_buffer.last_line_loc {
                if fp.write_all(b"\n").is_err() {
                    return -2;
                }
            }
        }
        0
    } else {
        -1
    }
}
pub fn load_file_and_initialize_buffer() -> i32 {
    // This is handled in initialize()
    0
}
}

fn get_window_size() -> (usize, usize) {
    use std::mem::zeroed;
    let mut ws: libc_winsize = unsafe { zeroed() };
    let ret = unsafe { ioctl_tiocgwinsz(1, &mut ws) };
    if ret == -1 || ws.ws_col == 0 {
        // Fallback: move cursor to bottom-right and query position
        use std::io::{Read, Write};
        let _ = std::io::stdout().write_all(b"\x1b[999C\x1b[999B");
        let _ = std::io::stdout().write_all(b"\x1b[6n");
        let _ = std::io::stdout().flush();

        let mut buf = [0u8; 32];
        let mut i = 0;
        loop {
            if std::io::stdin().read(&mut buf[i..i+1]).unwrap_or(0) != 1 { break; }
            if buf[i] == b'R' { break; }
            i += 1;
            if i >= 31 { break; }
        }
        let s = std::str::from_utf8(&buf[2..i]).unwrap_or("24;80");
        let parts: Vec<&str> = s.split(';').collect();
        if parts.len() == 2 {
            let h = parts[0].parse().unwrap_or(24);
            let w = parts[1].parse().unwrap_or(80);
            (w, h)
        } else {
            (80, 24)
        }
    } else {
        (ws.ws_col as usize, ws.ws_row as usize)
    }
}

#[repr(C)]
struct libc_winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg(target_os = "linux")]
const TIOCGWINSZ: u64 = 0x5413;
#[cfg(target_os = "macos")]
const TIOCGWINSZ: u64 = 0x40087468;

unsafe fn ioctl_tiocgwinsz(fd: i32, ws: &mut libc_winsize) -> i32 {
    extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    ioctl(fd, TIOCGWINSZ, ws as *mut libc_winsize)
}

fn draw_status_line_impl(state: &mut EditorState, line_size: usize) {
    let commands = "Ctrl+Q-quit Ctrl+S-Save";
    let commands_len = commands.len();
    let modified = "changed";
    let modified_len = modified.len();

    let file_cursor_space = line_size.saturating_sub(commands_len + modified_len);
    let cur_col_digits = format!("{}", state.current_buffer.cursor_col).len();
    let cur_row_digits = format!("{}", state.current_buffer.cursor_row).len();
    let f_name_space = file_cursor_space.saturating_sub(cur_col_digits + cur_row_digits + 5);

    let file_name = state.file_name.as_deref().unwrap_or("Empty Buffer");
    let file_name_size = file_name.len();

    // Invert colours
    screen_append(&mut state.screen, ted::defs::INVERT_COLOUR, ted::defs::INVERT_COLOUR_SIZE);

    // Write file name
    if file_name_size > f_name_space {
        let truncated = &file_name[..f_name_space.saturating_sub(4)];
        screen_append(&mut state.screen, truncated, truncated.len());
        screen_append(&mut state.screen, "... ", 4);
    } else {
        screen_append(&mut state.screen, file_name, file_name_size);
    }

    // Write cursor info
    let cursor_info = format!(" | {},{} ", state.current_buffer.cursor_row, state.current_buffer.cursor_col);
    screen_append(&mut state.screen, &cursor_info, cursor_info.len());

    // Modified indicator
    if !state.flushed {
        screen_append(&mut state.screen, modified, modified_len);
    } else {
        let spaces = " ".repeat(modified_len);
        screen_append(&mut state.screen, &spaces, modified_len);
    }

    // Fill with whitespace
    let fill = f_name_space.saturating_sub(file_name_size);
    if fill > 0 {
        let spaces = " ".repeat(fill);
        screen_append(&mut state.screen, &spaces, fill);
    }

    // Print help
    screen_append(&mut state.screen, commands, commands_len);

    // Reset colour
    screen_append(&mut state.screen, ted::defs::RESET_STYLE_COLOUR, ted::defs::INVERT_COLOUR_SIZE);
}

pub fn main(){
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len() as i32;
    EditorState::initialize(argc, args);

    loop {
        EditorState::draw_screen();
        EditorState::render_screen();
        EditorState::process_keypress();
    }
}
