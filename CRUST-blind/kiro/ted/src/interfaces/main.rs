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

use std::sync::Mutex;
use std::io::{Read, Write};

static EDITOR_STATE: Mutex<Option<EditorState>> = Mutex::new(None);

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

const MEM_ERROR: i32 = 128;

fn ctrl_key(k: u8) -> i32 {
    (k & 0x1f) as i32
}

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut EditorState) -> R,
{
    let mut guard = EDITOR_STATE.lock().unwrap();
    f(guard.as_mut().expect("EditorState not initialized"))
}

fn screen_append(es: &mut EditorState, s: &str, size: usize) {
    if es.screen.len - es.screen.buf_pos > size {
        let bytes: Vec<u8> = s.bytes().take(size).collect();
        for (i, &b) in bytes.iter().enumerate() {
            es.screen.buffer[es.screen.buf_pos + i] = b as char;
        }
        es.screen.buf_pos += size;
    }
}

impl EditorState {
    pub fn initialize(argc: i32, argv: Vec<String>) {
        let file_path = if argc >= 2 {
            argv[1].clone()
        } else {
            "Empty Buffer".to_string()
        };

        let file_name = std::path::Path::new(&file_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone());

        // Get original termios
        let orig_termios = termios::Termios::from_fd(0).unwrap_or_else(|_| {
            panic("tcgetattr");
            unreachable!()
        });

        let mut es = EditorState {
            orig_termios,
            file_name: Some(file_name),
            file_path: Some(file_path),
            flushed: true,
            current_buffer: TextBuffer::create(1, 1).unwrap(), // placeholder
            screen: VirtualScreen {
                buffer: Vec::new(),
                buf_pos: 0,
                len: 0,
                cursor: Cursor { x: 0, y: 0 },
                width: 0,
                height: 0,
                render_start_line: 0,
            },
        };

        // Load file
        load_file_impl(&mut es);

        // Enable raw mode
        enable_raw_mode_impl(&mut es);

        // Set window size
        set_window_size_impl(&mut es);

        // Initialize screen buffer
        es.screen.len = es.screen.height * es.screen.width * 2;
        es.screen.buffer = vec!['\0'; es.screen.len];
        es.screen.buf_pos = 0;
        es.screen.render_start_line = 0;

        *EDITOR_STATE.lock().unwrap() = Some(es);
    }
    pub fn cleanup() {
        with_state(|es| {
            let _ = std::io::stdout().write_all(b"\x1b[2J");
            let _ = std::io::stdout().write_all(b"\x1b[H");
            // screen buffer and text buffer freed on drop
        });
    }
    pub fn set_window_size() {
        with_state(|es| set_window_size_impl(es));
    }
    pub fn disable_raw_mode() {
        with_state(|es| {
            let _ = termios::tcsetattr(0, termios::TCSAFLUSH, &es.orig_termios);
        });
    }
    pub fn enable_raw_mode() {
        with_state(|es| enable_raw_mode_impl(es));
    }
    pub fn render_screen() {
        with_state(|es| {
            let s: String = es.screen.buffer[..es.screen.buf_pos].iter().collect();
            let _ = std::io::stdout().write_all(s.as_bytes());
            let _ = std::io::stdout().flush();
        });
    }
    pub fn draw_screen() {
        with_state(|es| {
            es.screen.buf_pos = 0;

            screen_append(es, "\x1b[?25l", 6);
            screen_append(es, "\x1b[2J", 4);
            screen_append(es, "\x1b[H", 3);

            move_cursor_in_view_impl(es);
            draw_editor_window_impl(es);
            draw_status_line_impl(es, es.screen.width);

            set_virtual_cursor_position_impl(es);

            let row = es.screen.cursor.x;
            let col = es.screen.cursor.y;
            let buf = format!("\x1b[{};{}H", row, col);
            let buf_len = buf.len();
            screen_append(es, &buf, buf_len);

            screen_append(es, "\x1b[?25h", 6);
            screen_append(es, "\0", 1);
        });
    }
    pub fn draw_status_line(line_size: usize) {
        with_state(|es| draw_status_line_impl(es, line_size));
    }
    pub fn up_arrow() {
        with_state(|es| {
            let col = es.current_buffer.cursor_col;
            let row = es.current_buffer.cursor_row;
            if row > 0 {
                es.current_buffer.move_cursor(row - 1, col);
            }
        });
    }
    pub fn down_arrow() {
        with_state(|es| {
            let col = es.current_buffer.cursor_col;
            let row = es.current_buffer.cursor_row;
            if row < es.current_buffer.last_line_loc {
                es.current_buffer.move_cursor(row + 1, col);
            }
        });
    }
    pub fn right_arrow() {
        with_state(|es| {
            let row = es.current_buffer.cursor_row;
            let col = es.current_buffer.cursor_col + 1;
            es.current_buffer.move_cursor(row, col);
        });
    }
    pub fn left_arrow() {
        with_state(|es| {
            let row = es.current_buffer.cursor_row;
            let col = es.current_buffer.cursor_col.saturating_sub(1);
            es.current_buffer.move_cursor(row, col);
        });
    }
    pub fn read_char() -> i32 {
        let mut c = [0u8; 1];
        loop {
            match std::io::stdin().read(&mut c) {
                Ok(1) => break,
                Ok(_) => continue,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    panic("read_char: read() returned EAGAIN");
                }
                Err(_) => continue,
            }
        }

        let ch = c[0];
        if ch == 0x1b {
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
            0x1b as i32
        } else {
            ch as i32
        }
    }
    pub fn process_keypress() {
        let c = Self::read_char();

        match c {
            c if c == '\r' as i32 => {
                with_state(|es| { es.current_buffer.new_line(); });
            }
            c if c == ARROW_UP => Self::up_arrow(),
            c if c == ARROW_DOWN => Self::down_arrow(),
            c if c == ARROW_LEFT => Self::left_arrow(),
            c if c == ARROW_RIGHT => Self::right_arrow(),
            c if c == PAGE_UP || c == PAGE_DOWN || c == HOME_KEY || c == END_KEY || c == DEL_KEY => {}
            c if c == ctrl_key(b'l') || c == 0x1b => {}
            c if c == ctrl_key(b's') => {
                with_state(|es| {
                    flush_buffer_impl(es);
                    es.flushed = true;
                });
            }
            c if c == ctrl_key(b'q') => {
                Self::cleanup();
                std::process::exit(0);
            }
            c if c == BACKSPACE || c == ctrl_key(b'h') => {
                with_state(|es| {
                    es.current_buffer.backspace();
                    es.flushed = false;
                });
            }
            c => {
                with_state(|es| {
                    es.current_buffer.insert(c as u8 as char);
                    es.flushed = false;
                });
            }
        }
    }
    pub fn flush_buffer_to_file() -> i32 {
        with_state(|es| flush_buffer_impl(es))
    }
    pub fn load_file_and_initialize_buffer() -> i32 {
        with_state(|es| load_file_impl(es))
    }
}

fn load_file_impl(es: &mut EditorState) -> i32 {
    let path = es.file_path.as_ref().unwrap().clone();
    match std::fs::File::open(&path) {
        Ok(fp) => {
            match TextBuffer::create_from_file(&fp) {
                Some(tb) => { es.current_buffer = tb; 0 }
                None => MEM_ERROR,
            }
        }
        Err(_) => {
            es.current_buffer = TextBuffer::create(
                ted::buffer::DEFAULT_CAPACITY,
                ted::buffer::DEFAULT_GAP_BUF_CAP,
            ).unwrap();
            -1
        }
    }
}

fn enable_raw_mode_impl(es: &mut EditorState) {
    use termios::*;
    let mut raw = es.orig_termios;
    raw.c_lflag &= !(ECHO | ICANON | ISIG | IEXTEN);
    raw.c_iflag &= !(ICRNL | IXON | BRKINT | INPCK);
    raw.c_oflag &= !(OPOST);
    raw.c_cflag |= CS8;
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 1;
    if tcsetattr(0, TCSAFLUSH, &raw).is_err() {
        panic("tcsetattr");
    }
}

fn set_window_size_impl(es: &mut EditorState) {
    let mut ws: libc_winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe { ioctl_tiocgwinsz(1, &mut ws) };
    if ret == -1 || ws.ws_col == 0 {
        let _ = std::io::stdout().write_all(b"\x1b[999C\x1b[999B");
        let _ = std::io::stdout().flush();

        let mut buf = [0u8; 32];
        let mut i = 0;
        loop {
            if i >= buf.len() { break; }
            if std::io::stdin().read(&mut buf[i..i+1]).unwrap_or(0) != 1 {
                panic("Failed to get window size");
            }
            if buf[i] == b'R' { break; }
            i += 1;
        }
        let s = std::str::from_utf8(&buf[2..i]).unwrap_or("");
        let parts: Vec<&str> = s.split(';').collect();
        if parts.len() == 2 {
            es.screen.height = parts[0].parse().unwrap_or(24);
            es.screen.width = parts[1].parse().unwrap_or(80);
        } else {
            panic("Failed to get window size");
        }
    } else {
        es.screen.height = ws.ws_row as usize;
        es.screen.width = ws.ws_col as usize;
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

fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
    if line_length == 0 {
        1
    } else {
        (line_length / screen_width) as i32 + if line_length % screen_width > 0 { 1 } else { 0 }
    }
}

fn move_cursor_in_view_impl(es: &mut EditorState) {
    let buffer_cursor_x = es.current_buffer.cursor_row;

    if buffer_cursor_x < es.screen.render_start_line {
        es.screen.render_start_line = buffer_cursor_x;
        return;
    }

    let mut cumul_req_rows: i32 = 0;
    let mut cur_line = es.screen.render_start_line;

    while cur_line <= es.current_buffer.last_line_loc {
        let cur_line_required_rows = required_screen_rows(
            es.current_buffer.lines[cur_line].as_ref().unwrap().str_len,
            es.screen.width,
        );
        if cur_line_required_rows + cumul_req_rows > es.screen.height as i32 - 1 {
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
                es.current_buffer.lines[cl].as_ref().unwrap().str_len,
                es.screen.width,
            );
            cl += 1;
        }
        while rows_required > 0 {
            rows_required -= required_screen_rows(
                es.current_buffer.lines[es.screen.render_start_line].as_ref().unwrap().str_len,
                es.screen.width,
            );
            es.screen.render_start_line += 1;
        }
    }
}

fn draw_editor_window_impl(es: &mut EditorState) {
    let mut cur_line = es.screen.render_start_line;
    let mut lines_written: usize = 0;

    while cur_line <= es.current_buffer.last_line_loc && lines_written < es.screen.height - 1 {
        let screen_cols = es.screen.width;
        let line = es.current_buffer.get_line(cur_line)
            .unwrap_or_else(|| { panic("draw editor cant get text of current line in buffer"); unreachable!() });

        let line_len = line.len();

        if line_len > screen_cols {
            let mut i = 0;
            loop {
                let remaining = line_len - i;
                let len_to_write = if screen_cols < remaining { screen_cols } else { remaining };
                screen_append(es, &line[i..i + len_to_write], len_to_write);
                screen_append(es, "\r\n", 2);
                screen_append(es, "\x1b[K", 3);
                i += len_to_write;
                lines_written += 1;
                if lines_written == es.screen.height - 2 { break; }
                if i >= line_len - 1 { break; }
            }
        } else {
            screen_append(es, &line, line_len);
            screen_append(es, "\r\n", 2);
            lines_written += 1;
        }
        cur_line += 1;
    }

    while lines_written < es.screen.height - 2 {
        screen_append(es, "\r\n", 2);
        lines_written += 1;
    }
}

fn draw_status_line_impl(es: &mut EditorState, line_size: usize) {
    let commands = "Ctrl+Q-quit Ctrl+S-Save";
    let commands_len = commands.len();
    let modified = "changed";
    let modified_len = modified.len();

    let file_cursor_space = line_size.saturating_sub(commands_len + modified_len);
    let cur_col_str = format!("{}", es.current_buffer.cursor_col);
    let cur_row_str = format!("{}", es.current_buffer.cursor_row);
    let f_name_space = file_cursor_space.saturating_sub(cur_col_str.len() + cur_row_str.len() + 5);

    let file_name = es.file_name.as_deref().unwrap_or("Empty Buffer");
    let file_name_size = file_name.len();

    // Invert colours
    screen_append(es, "\x1b[7m", 4);

    // Write file name
    if file_name_size > f_name_space {
        let trunc = f_name_space.saturating_sub(4);
        screen_append(es, &file_name[..trunc], trunc);
        screen_append(es, "... ", 4);
    } else {
        screen_append(es, file_name, file_name_size);
    }

    // Write cursor info
    let cursor_info = format!(" | {},{} ", es.current_buffer.cursor_row, es.current_buffer.cursor_col);
    let ci_len = cursor_info.len();
    screen_append(es, &cursor_info, ci_len);

    // Modified indicator
    if !es.flushed {
        screen_append(es, modified, modified_len);
    } else {
        let spaces = " ".repeat(modified_len);
        screen_append(es, &spaces, modified_len);
    }

    // Fill with whitespace
    let fill = f_name_space.saturating_sub(file_name_size);
    let spaces = " ".repeat(fill);
    screen_append(es, &spaces, fill);

    // Print help
    screen_append(es, commands, commands_len);

    // Reset colour
    screen_append(es, "\x1b[0m", 4);
}

fn set_virtual_cursor_position_impl(es: &mut EditorState) {
    let mut current_line = es.screen.render_start_line;
    let mut virtual_cursor_row: usize = 1;

    while current_line != es.current_buffer.cursor_row {
        let rr = required_screen_rows(
            es.current_buffer.lines[current_line].as_ref().unwrap().str_len,
            es.screen.width,
        );
        virtual_cursor_row += rr as usize;
        current_line += 1;
    }

    virtual_cursor_row += es.current_buffer.cursor_col / es.screen.width;

    es.screen.cursor.x = virtual_cursor_row;
    es.screen.cursor.y = (es.current_buffer.cursor_col % es.screen.width) + 1;
}

fn flush_buffer_impl(es: &mut EditorState) -> i32 {
    let path = es.file_path.as_ref().unwrap().clone();
    let fp = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    let mut writer = std::io::BufWriter::new(fp);

    for i in 0..=es.current_buffer.last_line_loc {
        let line = match es.current_buffer.get_line(i) {
            Some(l) => l,
            None => return -2,
        };
        if writer.write_all(line.as_bytes()).is_err() {
            return -2;
        }
        if i < es.current_buffer.last_line_loc {
            if writer.write_all(b"\n").is_err() {
                return -2;
            }
        }
    }
    0
}

pub fn main() {}
