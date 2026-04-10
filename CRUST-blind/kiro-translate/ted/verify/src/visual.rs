use crate::buffer::{TextBuffer};
pub struct Cursor {
pub x: usize,
pub y: usize,
}
pub struct VirtualScreen {
pub buffer: Vec<char>,
pub buf_pos: usize,
pub len: usize,
pub cursor: Cursor,
pub width: usize,
pub height: usize,
pub render_start_line: usize,
}
impl VirtualScreen {
pub fn screen_append(&mut self, str: &str, size: usize) {
    if self.len - self.buf_pos > size {
        let chars: Vec<char> = str.chars().take(size).collect();
        for &c in &chars {
            if self.buf_pos < self.buffer.len() {
                self.buffer[self.buf_pos] = c;
                self.buf_pos += 1;
            }
        }
    }
}
pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
    if line_length == 0 {
        1
    } else {
        (line_length / screen_width) as i32 + if (line_length % screen_width) > 0 { 1 } else { 0 }
    }
}
pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let buffer_cursor_x = buffer.cursor_row;

    if buffer_cursor_x < screen.render_start_line {
        screen.render_start_line = buffer_cursor_x;
        return;
    }

    let mut cumul_req_rows: i32 = 0;
    let mut cur_line = screen.render_start_line;

    while cur_line <= buffer.last_line_loc {
        let cur_line_required_rows = Self::required_screen_rows(
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
            rows_required += Self::required_screen_rows(
                buffer.lines[cl].as_ref().unwrap().str_len,
                screen.width,
            );
            cl += 1;
        }
        while rows_required > 0 {
            rows_required -= Self::required_screen_rows(
                buffer.lines[screen.render_start_line].as_ref().unwrap().str_len,
                screen.width,
            );
            screen.render_start_line += 1;
        }
    }
}
pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut cur_line = screen.render_start_line;
    let mut lines_written: usize = 0;

    while cur_line <= buffer.last_line_loc && lines_written < screen.height - 1 {
        let screen_cols = screen.width;
        let line = buffer.get_line(cur_line).unwrap_or_else(|| {
            crate::defs::panic("draw editor cant get text of current line in buffer");
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
                screen.screen_append(&chunk, len_to_write);
                screen.screen_append("\r\n", 2);
                screen.screen_append("\x1b[K", 3);
                i += len_to_write;
                lines_written += 1;
                if lines_written == screen.height - 2 { break; }
                if i >= line_len - 1 { break; }
            }
        } else {
            screen.screen_append(&line, line_len);
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }
        cur_line += 1;
    }

    while lines_written < screen.height - 2 {
        screen.screen_append("\r\n", 2);
        lines_written += 1;
    }
}
pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut current_line = screen.render_start_line;
    let mut virtual_cursor_row: usize = 1;

    while current_line != buffer.cursor_row {
        let required_rows = Self::required_screen_rows(
            buffer.lines[current_line].as_ref().unwrap().str_len,
            screen.width,
        );
        virtual_cursor_row += required_rows as usize;
        current_line += 1;
    }

    virtual_cursor_row += buffer.cursor_col / screen.width;

    screen.cursor.x = virtual_cursor_row;
    screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
}
}
