use ted::buffer::TextBuffer;
use ted::defs;
use std::io::{Read, Write};

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

fn ctrl_key(k: u8) -> i32 { (k & 0x1f) as i32 }

static mut EDITOR_STATE: Option<EditorState> = None;
fn state() -> &'static mut EditorState { unsafe { EDITOR_STATE.as_mut().unwrap() } }

fn screen_append(screen: &mut VirtualScreen, s: &str, size: usize) {
    if screen.len.saturating_sub(screen.buf_pos) > size {
        for (i, ch) in s.chars().take(size).enumerate() {
            if screen.buf_pos + i < screen.buffer.len() {
                screen.buffer[screen.buf_pos + i] = ch;
            }
        }
        screen.buf_pos += size;
    }
}

fn required_screen_rows(line_length: usize, screen_width: usize) -> usize {
    if line_length == 0 { 1 }
    else { line_length / screen_width + if line_length % screen_width > 0 { 1 } else { 0 } }
}

fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    if buffer.cursor_row < screen.render_start_line {
        screen.render_start_line = buffer.cursor_row;
        return;
    }
    let mut cumul = 0usize;
    let mut cur_line = screen.render_start_line;
    while cur_line <= buffer.last_line_loc {
        let r = required_screen_rows(buffer.lines[cur_line].as_ref().unwrap().str_len, screen.width);
        if r + cumul > screen.height - 1 {
            if cur_line > 0 { cur_line -= 1; }
            break;
        }
        cumul += r;
        cur_line += 1;
    }
    if buffer.cursor_row > cur_line {
        let mut rows_req = 0usize;
        let mut cl = cur_line;
        while cl <= buffer.cursor_row {
            rows_req += required_screen_rows(buffer.lines[cl].as_ref().unwrap().str_len, screen.width);
            cl += 1;
        }
        while rows_req > 0 {
            let r = required_screen_rows(buffer.lines[screen.render_start_line].as_ref().unwrap().str_len, screen.width);
            if r > rows_req { rows_req = 0; } else { rows_req -= r; }
            screen.render_start_line += 1;
        }
    }
}

fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut cur_line = screen.render_start_line;
    let mut lines_written = 0usize;
    while cur_line <= buffer.last_line_loc && lines_written < screen.height - 1 {
        let screen_cols = screen.width;
        let line = buffer.lines[cur_line].as_ref().unwrap().get_string();
        if line.len() > screen_cols {
            let mut i = 0;
            loop {
                let remaining = line.len() - i;
                let len_to_write = if screen_cols < remaining { screen_cols } else { remaining };
                screen_append(screen, &line[i..i + len_to_write], len_to_write);
                screen_append(screen, "\r\n", 2);
                screen_append(screen, "\x1b[K", 3);
                i += len_to_write;
                lines_written += 1;
                if lines_written == screen.height - 2 { break; }
                if i >= line.len() - 1 { break; }
            }
        } else {
            screen_append(screen, &line, line.len());
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
    let mut vcr: usize = 1;
    while current_line != buffer.cursor_row {
        vcr += required_screen_rows(buffer.lines[current_line].as_ref().unwrap().str_len, screen.width);
        current_line += 1;
    }
    vcr += buffer.cursor_col / screen.width;
    screen.cursor.x = vcr;
    screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
}

#[repr(C)]
struct LibcWinsize { ws_row: u16, ws_col: u16, ws_xpixel: u16, ws_ypixel: u16 }
extern "C" { fn ioctl(fd: i32, request: u64, ...) -> i32; }
#[cfg(target_os = "linux")]
const TIOCGWINSZ: u64 = 0x5413;
#[cfg(target_os = "macos")]
const TIOCGWINSZ: u64 = 0x40087468;

impl EditorState {
    pub fn initialize(argc: i32, argv: Vec<String>) {
        let file_path = if argc >= 2 { Some(argv[1].clone()) } else { Some("Empty Buffer".to_string()) };
        let file_name = file_path.as_ref().map(|p| {
            std::path::Path::new(p).file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.clone())
        });
        let buffer = {
            let fp = file_path.as_deref().and_then(|p| std::fs::File::open(p).ok());
            match fp {
                Some(f) => TextBuffer::create_from_file(&f).unwrap_or_else(|| TextBuffer::create(100, 100).unwrap()),
                None => TextBuffer::create(100, 100).unwrap(),
            }
        };
        let orig = termios::Termios::from_fd(0).unwrap_or_else(|_| { defs::panic("tcgetattr"); unreachable!() });
        unsafe {
            EDITOR_STATE = Some(EditorState {
                orig_termios: orig, file_name, file_path, flushed: true, current_buffer: buffer,
                screen: VirtualScreen { buffer: Vec::new(), buf_pos: 0, len: 0, cursor: Cursor { x: 0, y: 0 }, width: 0, height: 0, render_start_line: 0 },
            });
        }
        Self::enable_raw_mode();
        Self::set_window_size();
        let s = state();
        s.screen.len = s.screen.height * s.screen.width * 2;
        s.screen.buffer = vec!['\0'; s.screen.len];
        s.screen.buf_pos = 0;
        s.screen.render_start_line = 0;
    }
    pub fn cleanup() {
        let _ = std::io::stdout().write_all(b"\x1b[2J");
        let _ = std::io::stdout().write_all(b"\x1b[H");
        let _ = std::io::stdout().flush();
    }
    pub fn set_window_size() {
        let s = state();
        unsafe {
            let mut ws: LibcWinsize = std::mem::zeroed();
            if ioctl(1, TIOCGWINSZ, &mut ws as *mut LibcWinsize) == -1 || ws.ws_col == 0 {
                s.screen.height = 24;
                s.screen.width = 80;
            } else {
                s.screen.height = ws.ws_row as usize;
                s.screen.width = ws.ws_col as usize;
            }
        }
    }
    pub fn disable_raw_mode() {
        let s = state();
        let _ = termios::tcsetattr(0, termios::TCSAFLUSH, &s.orig_termios);
    }
    pub fn enable_raw_mode() {
        let s = state();
        s.orig_termios = termios::Termios::from_fd(0).unwrap_or_else(|_| { defs::panic("tcgetattr"); unreachable!() });
        let mut raw = s.orig_termios;
        use termios::*;
        raw.c_lflag &= !(ECHO | ICANON | ISIG | IEXTEN);
        raw.c_iflag &= !(ICRNL | IXON | BRKINT | INPCK);
        raw.c_oflag &= !OPOST;
        raw.c_cflag |= CS8;
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 1;
        let _ = termios::tcsetattr(0, termios::TCSAFLUSH, &raw);
    }
    pub fn render_screen() {
        let s = state();
        let out_str: String = s.screen.buffer[..s.screen.buf_pos].iter().collect();
        let _ = std::io::stdout().write_all(out_str.as_bytes());
        let _ = std::io::stdout().flush();
    }
    pub fn draw_screen() {
        let s = state();
        s.screen.buf_pos = 0;
        screen_append(&mut s.screen, "\x1b[?25l", 6);
        screen_append(&mut s.screen, "\x1b[2J", 4);
        screen_append(&mut s.screen, "\x1b[H", 3);
        move_cursor_in_view(&s.current_buffer, &mut s.screen);
        draw_editor_window(&s.current_buffer, &mut s.screen);
        Self::draw_status_line(s.screen.width);
        let s = state();
        set_virtual_cursor_position(&s.current_buffer, &mut s.screen);
        let buf = format!("\x1b[{};{}H", s.screen.cursor.x, s.screen.cursor.y);
        screen_append(&mut s.screen, &buf, buf.len());
        screen_append(&mut s.screen, "\x1b[?25h", 6);
        screen_append(&mut s.screen, "\0", 1);
    }
    pub fn draw_status_line(line_size: usize) {
        let s = state();
        let commands = "Ctrl+Q-quit Ctrl+S-Save";
        let modified = "changed";
        let file_cursor_space = line_size.saturating_sub(commands.len() + modified.len());
        let cur_col_digits = format!("{}", s.current_buffer.cursor_col).len();
        let cur_row_digits = format!("{}", s.current_buffer.cursor_row).len();
        let f_name_space = file_cursor_space.saturating_sub(cur_col_digits + cur_row_digits + 5);
        let file_name = s.file_name.as_deref().unwrap_or("");
        screen_append(&mut s.screen, defs::INVERT_COLOUR, defs::INVERT_COLOUR_SIZE);
        if file_name.len() > f_name_space {
            let trunc = &file_name[..f_name_space.saturating_sub(4)];
            screen_append(&mut s.screen, trunc, trunc.len());
            screen_append(&mut s.screen, "... ", 4);
        } else {
            screen_append(&mut s.screen, file_name, file_name.len());
        }
        let cursor_info = format!(" | {},{} ", s.current_buffer.cursor_row, s.current_buffer.cursor_col);
        screen_append(&mut s.screen, &cursor_info, cursor_info.len());
        if !s.flushed {
            screen_append(&mut s.screen, modified, modified.len());
        } else {
            let sp: String = " ".repeat(modified.len());
            screen_append(&mut s.screen, &sp, modified.len());
        }
        let fill = f_name_space.saturating_sub(file_name.len());
        let sp: String = " ".repeat(fill);
        screen_append(&mut s.screen, &sp, fill);
        screen_append(&mut s.screen, commands, commands.len());
        screen_append(&mut s.screen, defs::RESET_STYLE_COLOUR, defs::INVERT_COLOUR_SIZE);
    }
    pub fn up_arrow() {
        let s = state();
        let (col, row) = (s.current_buffer.cursor_col, s.current_buffer.cursor_row);
        if row > 0 { s.current_buffer.move_cursor(row - 1, col); }
    }
    pub fn down_arrow() {
        let s = state();
        let (col, row) = (s.current_buffer.cursor_col, s.current_buffer.cursor_row);
        if row < s.current_buffer.last_line_loc { s.current_buffer.move_cursor(row + 1, col); }
    }
    pub fn right_arrow() {
        let s = state();
        let (row, col) = (s.current_buffer.cursor_row, s.current_buffer.cursor_col + 1);
        s.current_buffer.move_cursor(row, col);
    }
    pub fn left_arrow() {
        let s = state();
        let (row, col) = (s.current_buffer.cursor_row, s.current_buffer.cursor_col.saturating_sub(1));
        s.current_buffer.move_cursor(row, col);
    }
    pub fn read_char() -> i32 {
        let mut buf = [0u8; 1];
        loop {
            match std::io::stdin().lock().read(&mut buf) {
                Ok(1) => break,
                _ => continue,
            }
        }
        let c = buf[0];
        if c == defs::ESC as u8 {
            let mut seq = [0u8; 3];
            let mut stdin = std::io::stdin().lock();
            if stdin.read(&mut seq[0..1]).unwrap_or(0) != 1 { return defs::ESC as i32; }
            if stdin.read(&mut seq[1..2]).unwrap_or(0) != 1 { return defs::ESC as i32; }
            if seq[0] == b'[' {
                if seq[1] >= b'0' && seq[1] <= b'9' {
                    if stdin.read(&mut seq[2..3]).unwrap_or(0) != 1 { return defs::ESC as i32; }
                    if seq[2] == b'~' {
                        return match seq[1] {
                            b'1' | b'7' => HOME_KEY, b'3' => DEL_KEY, b'4' | b'8' => END_KEY,
                            b'5' => PAGE_UP, b'6' => PAGE_DOWN, _ => defs::ESC as i32,
                        };
                    }
                } else {
                    return match seq[1] {
                        b'A' => ARROW_UP, b'B' => ARROW_DOWN, b'C' => ARROW_RIGHT, b'D' => ARROW_LEFT,
                        b'H' => HOME_KEY, b'F' => END_KEY, _ => defs::ESC as i32,
                    };
                }
            } else if seq[0] == b'O' {
                return match seq[1] { b'H' => HOME_KEY, b'F' => END_KEY, _ => defs::ESC as i32 };
            }
            defs::ESC as i32
        } else { c as i32 }
    }
    pub fn process_keypress() {
        let c = Self::read_char();
        match c {
            c if c == b'\r' as i32 => { state().current_buffer.new_line(); }
            c if c == ARROW_UP => Self::up_arrow(),
            c if c == ARROW_DOWN => Self::down_arrow(),
            c if c == ARROW_LEFT => Self::left_arrow(),
            c if c == ARROW_RIGHT => Self::right_arrow(),
            c if c == PAGE_UP || c == PAGE_DOWN || c == HOME_KEY || c == END_KEY || c == DEL_KEY => {}
            c if c == ctrl_key(b'l') || c == defs::ESC as i32 => {}
            c if c == ctrl_key(b's') => { let _ = Self::flush_buffer_to_file(); state().flushed = true; }
            c if c == ctrl_key(b'q') => { Self::cleanup(); Self::disable_raw_mode(); std::process::exit(0); }
            c if c == BACKSPACE || c == ctrl_key(b'h') => { state().current_buffer.backspace(); state().flushed = false; }
            _ => { state().current_buffer.insert(c as u8 as char); state().flushed = false; }
        }
    }
    pub fn flush_buffer_to_file() -> i32 {
        let s = state();
        let path = match &s.file_path { Some(p) => p.clone(), None => return -1 };
        let mut fp = match std::fs::File::create(&path) { Ok(f) => f, Err(_) => return -1 };
        for i in 0..=s.current_buffer.last_line_loc {
            match s.current_buffer.get_line(i) {
                Some(l) => { if fp.write_all(l.as_bytes()).is_err() { return -2; } }
                None => return -2,
            }
        }
        0
    }
    pub fn load_file_and_initialize_buffer() -> i32 {
        let s = state();
        let path = s.file_path.clone();
        match path.as_deref().and_then(|p| std::fs::File::open(p).ok()) {
            Some(f) => match TextBuffer::create_from_file(&f) { Some(tb) => { s.current_buffer = tb; 0 } None => 128 },
            None => match TextBuffer::create(100, 100) { Some(tb) => { s.current_buffer = tb; -1 } None => 128 },
        }
    }
}

pub fn main() {}
