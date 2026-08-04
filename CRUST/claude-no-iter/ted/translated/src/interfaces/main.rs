use ted::buffer::TextBuffer;
use ted::defs::panic;
use ted::gap::MEM_ERROR;

use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::sync::Mutex;
use std::sync::OnceLock;

const ESC_CHAR: char = '\x1b';
const INVERT_COLOUR: &str = "\x1b[7m";
const INVERT_COLOUR_SIZE: usize = 4;
const RESET_STYLE_COLOUR: &str = "\x1b[0m";

// Special key codes
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

fn ctrl_key(k: char) -> i32 {
    (k as i32) & 0x1f
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

// Global editor state — matches the C `editor_state` global.
fn editor_state() -> &'static Mutex<Option<EditorState>> {
    static STATE: OnceLock<Mutex<Option<EditorState>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

fn screen_append(state: &mut EditorState, s: &str, size: usize) {
    let chars: Vec<char> = s.chars().take(size).collect();
    let to_write = chars.len();
    if state.screen.len > state.screen.buf_pos
        && (state.screen.len - state.screen.buf_pos) > to_write
    {
        for (i, c) in chars.into_iter().enumerate() {
            if state.screen.buf_pos + i < state.screen.buffer.len() {
                state.screen.buffer[state.screen.buf_pos + i] = c;
            }
        }
        state.screen.buf_pos += to_write;
    }
}

fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
    if line_length == 0 {
        return 1;
    }
    if screen_width == 0 {
        return 1;
    }
    let rows = (line_length / screen_width) as i32;
    let rem = if line_length % screen_width > 0 { 1 } else { 0 };
    rows + rem
}

impl EditorState {
    pub fn initialize(argc: i32, argv: Vec<String>) {
        let file_path = if argc >= 2 {
            argv[1].clone()
        } else {
            String::from("Empty Buffer")
        };

        let file_name = std::path::Path::new(&file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| file_path.clone());

        // Initial empty buffer; will be replaced by load_file_and_initialize_buffer.
        let initial_buffer = match TextBuffer::create(
            ted::buffer::DEFAULT_CAPACITY,
            ted::buffer::DEFAULT_GAP_BUF_CAP,
        ) {
            Some(b) => b,
            None => {
                panic("Failed to allocate initial buffer");
                return;
            }
        };

        // We need a default termios — read it now.
        let stdin_fd = std::io::stdin().as_raw_fd();
        let orig_termios = match termios::Termios::from_fd(stdin_fd) {
            Ok(t) => t,
            Err(_) => {
                panic("tcgetattr");
                return;
            }
        };

        let state = EditorState {
            orig_termios,
            file_name: Some(file_name),
            file_path: Some(file_path),
            flushed: true,
            current_buffer: initial_buffer,
            screen: VirtualScreen {
                buffer: Vec::new(),
                buf_pos: 0,
                len: 0,
                cursor: Cursor { x: 1, y: 1 },
                width: 80,
                height: 24,
                render_start_line: 0,
            },
        };

        {
            let mut guard = editor_state().lock().unwrap();
            *guard = Some(state);
        }

        // Load the file and initialize the buffer.
        let _ = Self::load_file_and_initialize_buffer();

        // Initialize raw mode and screen.
        Self::enable_raw_mode();
        Self::set_window_size();

        // Allocate the screen buffer.
        {
            let mut guard = editor_state().lock().unwrap();
            if let Some(s) = guard.as_mut() {
                s.screen.len = s.screen.height * s.screen.width * 2;
                s.screen.buffer = vec!['\0'; s.screen.len];
                s.screen.buf_pos = 0;
                s.screen.render_start_line = 0;
                s.flushed = true;
            }
        }
    }

    pub fn cleanup() {
        // Clear the screen and move cursor to home.
        let _ = std::io::stdout().write_all(b"\x1b[2J");
        let _ = std::io::stdout().write_all(b"\x1b[H");
        let _ = std::io::stdout().flush();

        // Drop the state to release resources.
        let mut guard = editor_state().lock().unwrap();
        *guard = None;
    }

    pub fn set_window_size() {
        // Try to read the window size via ioctl using libc-like Rust APIs.
        // Since we can't use libc/ffi, fall back to reading the COLUMNS/LINES env vars
        // or to the terminal escape code method.
        let cols: usize = std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(80);
        let rows: usize = std::env::var("LINES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);

        let mut guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_mut() {
            s.screen.width = cols;
            s.screen.height = rows;
        }
    }

    pub fn disable_raw_mode() {
        let guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_ref() {
            let stdin_fd = std::io::stdin().as_raw_fd();
            let _ = termios::tcsetattr(stdin_fd, termios::TCSAFLUSH, &s.orig_termios);
        }
    }

    pub fn enable_raw_mode() {
        let stdin_fd = std::io::stdin().as_raw_fd();
        let mut term = match termios::Termios::from_fd(stdin_fd) {
            Ok(t) => t,
            Err(_) => {
                panic("tcgetattr");
                return;
            }
        };

        // Save original termios.
        {
            let mut guard = editor_state().lock().unwrap();
            if let Some(s) = guard.as_mut() {
                s.orig_termios = term;
            }
        }

        // Disable various flags to put the terminal in raw mode.
        term.c_lflag &= !(termios::ECHO | termios::ICANON | termios::ISIG | termios::IEXTEN);
        term.c_iflag &=
            !(termios::ICRNL | termios::IXON | termios::BRKINT | termios::INPCK);
        term.c_oflag &= !termios::OPOST;
        term.c_cflag |= termios::CS8;
        term.c_cc[termios::VMIN] = 0;
        term.c_cc[termios::VTIME] = 1;

        if termios::tcsetattr(stdin_fd, termios::TCSAFLUSH, &term).is_err() {
            panic("tcsetattr");
        }
    }

    pub fn render_screen() {
        let guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_ref() {
            // Convert screen buffer (chars) up to first NUL into bytes for output.
            let mut out = Vec::with_capacity(s.screen.buf_pos);
            for &c in s.screen.buffer.iter() {
                if c == '\0' {
                    break;
                }
                let mut tmp = [0u8; 4];
                let s_str = c.encode_utf8(&mut tmp);
                out.extend_from_slice(s_str.as_bytes());
            }
            let _ = std::io::stdout().write_all(&out);
            let _ = std::io::stdout().flush();
        }
    }

    pub fn draw_screen() {
        let mut guard = editor_state().lock().unwrap();
        let s = match guard.as_mut() {
            Some(s) => s,
            None => return,
        };

        s.screen.buf_pos = 0;

        // Disable cursor
        screen_append(s, "\x1b[?25l", 6);
        // Clear screen
        screen_append(s, "\x1b[2J", 4);
        // Move cursor to top
        screen_append(s, "\x1b[H", 3);

        // move_cursor_in_view
        Self::do_move_cursor_in_view(s);
        // draw_editor_window
        Self::do_draw_editor_window(s);

        // draw status line
        let line_size = s.screen.width;
        Self::do_draw_status_line(s, line_size);

        // set_virtual_cursor_position
        Self::do_set_virtual_cursor_position(s);

        let row = s.screen.cursor.x;
        let col = s.screen.cursor.y;
        let cursor_pos = format!("\x1b[{};{}H", row, col);
        let cp_len = cursor_pos.len();
        screen_append(s, &cursor_pos, cp_len);

        // Enable cursor
        screen_append(s, "\x1b[?25h", 6);
        // Null terminator
        screen_append(s, "\0", 1);
    }

    fn do_move_cursor_in_view(s: &mut EditorState) {
        let buffer_cursor_x = s.current_buffer.cursor_row;

        if buffer_cursor_x < s.screen.render_start_line {
            s.screen.render_start_line = buffer_cursor_x;
        } else {
            let mut cumul_req_rows: i32 = 0;
            let mut cur_line = s.screen.render_start_line;

            while cur_line <= s.current_buffer.last_line_loc {
                let line_str_len = s
                    .current_buffer
                    .lines
                    .get(cur_line)
                    .and_then(|x| x.as_ref())
                    .map(|g| g.str_len)
                    .unwrap_or(0);

                let req = required_screen_rows(line_str_len, s.screen.width);

                if req + cumul_req_rows > (s.screen.height as i32) - 1 {
                    if cur_line > 0 {
                        cur_line -= 1;
                    }
                    break;
                }

                cumul_req_rows += req;
                cur_line += 1;
            }

            if buffer_cursor_x > cur_line {
                let mut rows_required: i32 = 0;
                let mut cl = cur_line;
                while cl <= buffer_cursor_x {
                    let line_str_len = s
                        .current_buffer
                        .lines
                        .get(cl)
                        .and_then(|x| x.as_ref())
                        .map(|g| g.str_len)
                        .unwrap_or(0);
                    rows_required += required_screen_rows(line_str_len, s.screen.width);
                    cl += 1;
                }

                while rows_required > 0 {
                    let line_str_len = s
                        .current_buffer
                        .lines
                        .get(s.screen.render_start_line)
                        .and_then(|x| x.as_ref())
                        .map(|g| g.str_len)
                        .unwrap_or(0);
                    rows_required -= required_screen_rows(line_str_len, s.screen.width);
                    s.screen.render_start_line += 1;
                }
            }
        }
    }

    fn do_draw_editor_window(s: &mut EditorState) {
        let mut cur_line = s.screen.render_start_line;
        let mut lines_written: usize = 0;
        let screen_height = s.screen.height;
        let screen_cols = s.screen.width;

        while cur_line <= s.current_buffer.last_line_loc
            && (screen_height >= 1 && lines_written < screen_height - 1)
        {
            let line = match s.current_buffer.get_line(cur_line) {
                Some(l) => l,
                None => {
                    panic("draw editor cant get text of current line in buffer");
                    return;
                }
            };

            if line.len() > screen_cols {
                let chars: Vec<char> = line.chars().collect();
                let total_len = chars.len();
                let mut i: usize = 0;

                loop {
                    let remaining = total_len - i;
                    let len_to_write = if screen_cols < remaining {
                        screen_cols
                    } else {
                        remaining
                    };

                    let segment: String = chars[i..i + len_to_write].iter().collect();
                    screen_append(s, &segment, len_to_write);
                    screen_append(s, "\r\n", 2);
                    screen_append(s, "\x1b[K", 3);
                    i += len_to_write;
                    lines_written += 1;

                    if screen_height >= 2 && lines_written == screen_height - 2 {
                        break;
                    }
                    if i >= total_len.saturating_sub(1) {
                        break;
                    }
                }
            } else {
                let line_len = line.len();
                screen_append(s, &line, line_len);
                screen_append(s, "\r\n", 2);
                lines_written += 1;
            }

            cur_line += 1;
        }

        if screen_height >= 2 {
            while lines_written < screen_height - 2 {
                screen_append(s, "\r\n", 2);
                lines_written += 1;
            }
        }
    }

    fn do_draw_status_line(s: &mut EditorState, line_size: usize) {
        let commands = "Ctrl+Q-quit Ctrl+S-Save";
        let commands_len = commands.len();

        let modified = "changed";
        let modified_len = modified.len();

        let cur_col = s.current_buffer.cursor_col;
        let cur_row = s.current_buffer.cursor_row;
        let cur_col_digits = cur_col.to_string().len();
        let cur_row_digits = cur_row.to_string().len();

        let file_cursor_space = line_size.saturating_sub(commands_len + modified_len);
        let f_name_space = file_cursor_space.saturating_sub(cur_col_digits + cur_row_digits + 5);
        let file_name = s.file_name.clone().unwrap_or_default();
        let file_name_size = file_name.len();

        // Invert colour
        screen_append(s, INVERT_COLOUR, INVERT_COLOUR_SIZE);

        if file_name_size > f_name_space {
            let cut = f_name_space.saturating_sub(4);
            screen_append(s, &file_name, cut);
            screen_append(s, "... ", 4);
        } else {
            screen_append(s, &file_name, file_name_size);
        }

        let cursor_info = format!(" | {},{} ", cur_row, cur_col);
        let ci_len = cursor_info.len();
        screen_append(s, &cursor_info, ci_len);

        if !s.flushed {
            screen_append(s, modified, modified_len);
        } else {
            // pad with spaces
            let spaces: String = " ".repeat(modified_len);
            screen_append(s, &spaces, modified_len);
        }

        // Pad with whitespace until status fills the bar.
        let pad = f_name_space.saturating_sub(file_name_size);
        let pad_str: String = " ".repeat(pad);
        screen_append(s, &pad_str, pad);

        // Append commands
        screen_append(s, commands, commands_len);

        // Reset colour
        screen_append(s, RESET_STYLE_COLOUR, INVERT_COLOUR_SIZE);
    }

    fn do_set_virtual_cursor_position(s: &mut EditorState) {
        let mut current_line = s.screen.render_start_line;
        let mut virtual_cursor_row: usize = 1;

        while current_line != s.current_buffer.cursor_row {
            let line_str_len = s
                .current_buffer
                .lines
                .get(current_line)
                .and_then(|x| x.as_ref())
                .map(|g| g.str_len)
                .unwrap_or(0);

            let req = required_screen_rows(line_str_len, s.screen.width);
            virtual_cursor_row = virtual_cursor_row.saturating_add(req as usize);
            current_line += 1;
        }

        if s.screen.width > 0 {
            virtual_cursor_row += s.current_buffer.cursor_col / s.screen.width;
            s.screen.cursor.x = virtual_cursor_row;
            s.screen.cursor.y = (s.current_buffer.cursor_col % s.screen.width) + 1;
        } else {
            s.screen.cursor.x = virtual_cursor_row;
            s.screen.cursor.y = 1;
        }
    }

    pub fn draw_status_line(line_size: usize) {
        let mut guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_mut() {
            Self::do_draw_status_line(s, line_size);
        }
    }

    pub fn up_arrow() {
        let mut guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_mut() {
            let row = s.current_buffer.cursor_row;
            let col = s.current_buffer.cursor_col;
            if row > 0 {
                s.current_buffer.move_cursor(row - 1, col);
            }
        }
    }

    pub fn down_arrow() {
        let mut guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_mut() {
            let row = s.current_buffer.cursor_row;
            let col = s.current_buffer.cursor_col;
            if row < s.current_buffer.last_line_loc {
                s.current_buffer.move_cursor(row + 1, col);
            }
        }
    }

    pub fn right_arrow() {
        let mut guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_mut() {
            let row = s.current_buffer.cursor_row;
            let col = s.current_buffer.cursor_col + 1;
            s.current_buffer.move_cursor(row, col);
        }
    }

    pub fn left_arrow() {
        let mut guard = editor_state().lock().unwrap();
        if let Some(s) = guard.as_mut() {
            let row = s.current_buffer.cursor_row;
            let col = s.current_buffer.cursor_col;
            if col > 0 {
                s.current_buffer.move_cursor(row, col - 1);
            } else {
                s.current_buffer.move_cursor(row, 0);
            }
        }
    }

    pub fn read_char() -> i32 {
        let mut buf = [0u8; 1];
        loop {
            match std::io::stdin().read(&mut buf) {
                Ok(1) => break,
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
        let c = buf[0] as i32;

        if c == ESC_CHAR as i32 {
            // Try to read 2 more bytes
            let mut seq = [0u8; 3];
            if std::io::stdin().read(&mut seq[0..1]).unwrap_or(0) != 1 {
                return ESC_CHAR as i32;
            }
            if std::io::stdin().read(&mut seq[1..2]).unwrap_or(0) != 1 {
                return ESC_CHAR as i32;
            }

            if seq[0] == b'[' {
                if seq[1] >= b'0' && seq[1] <= b'9' {
                    if std::io::stdin().read(&mut seq[2..3]).unwrap_or(0) != 1 {
                        return ESC_CHAR as i32;
                    }
                    if seq[2] == b'~' {
                        match seq[1] {
                            b'1' => return HOME_KEY,
                            b'3' => return DEL_KEY,
                            b'4' => return END_KEY,
                            b'5' => return PAGE_UP,
                            b'6' => return PAGE_DOWN,
                            b'7' => return HOME_KEY,
                            b'8' => return END_KEY,
                            _ => {}
                        }
                    }
                } else {
                    match seq[1] {
                        b'A' => return ARROW_UP,
                        b'B' => return ARROW_DOWN,
                        b'C' => return ARROW_RIGHT,
                        b'D' => return ARROW_LEFT,
                        b'H' => return HOME_KEY,
                        b'F' => return END_KEY,
                        _ => {}
                    }
                }
            } else if seq[0] == b'O' {
                match seq[1] {
                    b'H' => return HOME_KEY,
                    b'F' => return END_KEY,
                    _ => {}
                }
            }

            return ESC_CHAR as i32;
        }

        c
    }

    pub fn process_keypress() {
        let c = Self::read_char();

        match c {
            13 => {
                // '\r'
                let mut guard = editor_state().lock().unwrap();
                if let Some(s) = guard.as_mut() {
                    s.current_buffer.new_line();
                }
            }
            ARROW_UP => Self::up_arrow(),
            ARROW_DOWN => Self::down_arrow(),
            ARROW_LEFT => Self::left_arrow(),
            ARROW_RIGHT => Self::right_arrow(),
            PAGE_UP | PAGE_DOWN | HOME_KEY | END_KEY | DEL_KEY => {}
            _ if c == ctrl_key('l') || c == ESC_CHAR as i32 => {}
            _ if c == ctrl_key('s') => {
                let _ = Self::flush_buffer_to_file();
                let mut guard = editor_state().lock().unwrap();
                if let Some(s) = guard.as_mut() {
                    s.flushed = true;
                }
            }
            _ if c == ctrl_key('q') => {
                Self::cleanup();
                std::process::exit(0);
            }
            BACKSPACE => {
                let mut guard = editor_state().lock().unwrap();
                if let Some(s) = guard.as_mut() {
                    s.current_buffer.backspace();
                    s.flushed = false;
                }
            }
            _ if c == ctrl_key('h') => {
                let mut guard = editor_state().lock().unwrap();
                if let Some(s) = guard.as_mut() {
                    s.current_buffer.backspace();
                    s.flushed = false;
                }
            }
            _ => {
                if let Some(ch) = char::from_u32(c as u32) {
                    let mut guard = editor_state().lock().unwrap();
                    if let Some(s) = guard.as_mut() {
                        s.current_buffer.insert(ch);
                        s.flushed = false;
                    }
                }
            }
        }
    }

    pub fn flush_buffer_to_file() -> i32 {
        let mut guard = editor_state().lock().unwrap();
        let s = match guard.as_mut() {
            Some(s) => s,
            None => return -1,
        };

        let file_path = match s.file_path.as_ref() {
            Some(p) => p.clone(),
            None => return -1,
        };

        let mut file = match std::fs::File::create(&file_path) {
            Ok(f) => f,
            Err(_) => return -1,
        };

        for i in 0..=s.current_buffer.last_line_loc {
            let line = match s.current_buffer.get_line(i) {
                Some(l) => l,
                None => return -2,
            };
            if file.write_all(line.as_bytes()).is_err() {
                return -2;
            }
        }

        0
    }

    pub fn load_file_and_initialize_buffer() -> i32 {
        let mut guard = editor_state().lock().unwrap();
        let s = match guard.as_mut() {
            Some(s) => s,
            None => return MEM_ERROR,
        };

        let file_path = match s.file_path.as_ref() {
            Some(p) => p.clone(),
            None => return -1,
        };

        match std::fs::File::open(&file_path) {
            Ok(file) => match TextBuffer::create_from_file(&file) {
                Some(buf) => {
                    s.current_buffer = buf;
                    0
                }
                None => MEM_ERROR,
            },
            Err(_) => {
                // No file — keep blank buffer.
                -1
            }
        }
    }
}

pub fn main() {}
