use crate::buffer::TextBuffer;
use crate::defs::panic;

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
    if !self.buffer.is_empty() && self.len.saturating_sub(self.buf_pos) > size {
        let chars: Vec<char> = str.chars().collect();
        let to_write = size.min(chars.len());
        for i in 0..to_write {
            if self.buf_pos + i < self.buffer.len() {
                self.buffer[self.buf_pos + i] = chars[i];
            }
        }
        self.buf_pos += to_write;
    }
}
pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
    if line_length == 0 {
        return 1;
    }
    if screen_width == 0 {
        return 1;
    }
    let rows = (line_length / screen_width) + if (line_length % screen_width) > 0 { 1 } else { 0 };
    rows as i32
}
pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let buffer_cursor_x = buffer.cursor_row;

    let mut cumul_req_rows: i32 = 0;
    let mut cur_line: usize = screen.render_start_line;

    if buffer_cursor_x < screen.render_start_line {
        screen.render_start_line = buffer_cursor_x;
        return;
    }

    // calculate the current range in the buffer that is visible
    let mut last_visible_line: i64 = cur_line as i64;
    while cur_line <= buffer.last_line_loc {
        let str_len = match &buffer.lines[cur_line] {
            Some(b) => b.str_len,
            None => 0,
        };
        let cur_line_required_rows = VirtualScreen::required_screen_rows(str_len, screen.width);

        if (cur_line_required_rows + cumul_req_rows) > (screen.height as i32 - 1) {
            last_visible_line = cur_line as i64 - 1;
            break;
        }

        cumul_req_rows += cur_line_required_rows;
        last_visible_line = cur_line as i64;
        cur_line += 1;
    }

    // If the cursor is not in view, shift the text displayed until it is
    if (buffer_cursor_x as i64) > last_visible_line {
        let mut rows_required: i32 = 0;
        let mut walk: usize = if last_visible_line < 0 { 0 } else { last_visible_line as usize };
        // Proceed from walk inclusive (matches C: cur_line is last_visible_line after break)
        while walk <= buffer_cursor_x {
            let str_len = match &buffer.lines[walk] {
                Some(b) => b.str_len,
                None => 0,
            };
            rows_required += VirtualScreen::required_screen_rows(str_len, screen.width);
            walk += 1;
        }

        // shift the render start line down until we've made enough room
        while rows_required > 0 && screen.render_start_line < buffer.last_line_loc {
            let str_len = match &buffer.lines[screen.render_start_line] {
                Some(b) => b.str_len,
                None => 0,
            };
            rows_required -= VirtualScreen::required_screen_rows(str_len, screen.width);
            screen.render_start_line += 1;
        }
    }
}
pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut cur_line = screen.render_start_line;
    let mut lines_written: usize = 0;
    let screen_cols = screen.width;

    while cur_line <= buffer.last_line_loc && lines_written < screen.height.saturating_sub(1) {
        let line = match buffer.get_line(cur_line) {
            Some(l) => l,
            None => {
                panic("draw editor cant get text of current line in buffer");
                return;
            }
        };

        let line_chars: Vec<char> = line.chars().collect();
        let line_len = line_chars.len();

        if line_len > screen_cols {
            // Multiple rows needed
            let mut i = 0usize;
            loop {
                let remaining = line_len - i;
                let len_to_write = if screen_cols < remaining { screen_cols } else { remaining };

                let chunk: String = line_chars[i..i + len_to_write].iter().collect();
                screen.screen_append(&chunk, len_to_write);
                screen.screen_append("\r\n", 2);
                screen.screen_append("\x1b[K", 3);
                i += len_to_write;
                lines_written += 1;

                if lines_written == screen.height.saturating_sub(2) {
                    break;
                }

                if i >= line_len.saturating_sub(1) {
                    break;
                }
            }
        } else {
            screen.screen_append(&line, line_len);
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }

        cur_line += 1;
    }

    // Fill remaining space
    while lines_written < screen.height.saturating_sub(2) {
        screen.screen_append("\r\n", 2);
        lines_written += 1;
    }
}
pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut current_line = screen.render_start_line;
    let mut virtual_cursor_row: usize = 1;

    while current_line != buffer.cursor_row {
        let str_len = match &buffer.lines[current_line] {
            Some(b) => b.str_len,
            None => 0,
        };
        let required_rows = VirtualScreen::required_screen_rows(str_len, screen.width);
        virtual_cursor_row += required_rows as usize;
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
