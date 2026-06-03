use crate::buffer::{TextBuffer};
use crate::defs::panic as ted_panic;

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
    if (self.len - self.buf_pos) > size {
        for (i, ch) in str.chars().take(size).enumerate() {
            if self.buf_pos + i < self.buffer.len() {
                self.buffer[self.buf_pos + i] = ch;
            } else {
                self.buffer.push(ch);
            }
        }
        self.buf_pos += size;
    }
}
pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
    if line_length == 0 {
        1
    } else {
        let div = line_length / screen_width;
        let rem = if line_length % screen_width > 0 { 1 } else { 0 };
        (div + rem) as i32
    }
}
pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let buffer_cursor_x = buffer.cursor_row;

    // The render_start_line is guaranteed to render
    if buffer_cursor_x < screen.render_start_line {
        screen.render_start_line = buffer_cursor_x;
        return;
    }

    // Calculate the current range visible in the buffer
    let mut cumul_req_rows: i32 = 0;
    let mut cur_line = screen.render_start_line;
    let mut cur_line_required_rows: i32;

    while cur_line <= buffer.last_line_loc {
        let line_str_len = match &buffer.lines[cur_line] {
            Some(gb) => gb.str_len,
            None => 0,
        };

        cur_line_required_rows = VirtualScreen::required_screen_rows(line_str_len, screen.width);

        if (cur_line_required_rows + cumul_req_rows) > (screen.height as i32) - 1 {
            if cur_line > 0 {
                cur_line -= 1;
            }
            break;
        }

        cumul_req_rows += cur_line_required_rows;
        cur_line += 1;
    }

    // If the cursor is not in view, shift the text displayed until it is
    if buffer_cursor_x > cur_line {
        let mut rows_required: i32 = 0;
        let mut walker = cur_line;

        while walker <= buffer_cursor_x {
            let line_str_len = match &buffer.lines[walker] {
                Some(gb) => gb.str_len,
                None => 0,
            };
            rows_required += VirtualScreen::required_screen_rows(line_str_len, screen.width);
            walker += 1;
        }

        // Shift the render start line down until we've made enough room
        while rows_required > 0 {
            let line_str_len = match &buffer.lines[screen.render_start_line] {
                Some(gb) => gb.str_len,
                None => 0,
            };
            rows_required -= VirtualScreen::required_screen_rows(line_str_len, screen.width);
            screen.render_start_line += 1;
        }
    }
}
pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut cur_line = screen.render_start_line;
    let mut lines_written: usize = 0;

    while cur_line <= buffer.last_line_loc && lines_written < screen.height - 1 {
        let screen_cols = screen.width;

        // Draw cur_line, using as many screen rows as needed
        let line = match buffer.get_line(cur_line) {
            Some(l) => l,
            None => {
                ted_panic("draw editor cant get text of current line in buffer");
                return;
            }
        };

        if line.len() > screen_cols {
            // Need multiple screen rows to draw this line
            let mut i: usize = 0;
            let line_len = line.len();

            loop {
                let remaining = line_len - i;
                let len_to_write = if screen_cols < remaining { screen_cols } else { remaining };

                let slice: String = line.chars().skip(i).take(len_to_write).collect();
                screen.screen_append(&slice, len_to_write);
                screen.screen_append("\r\n", 2);
                screen.screen_append("\x1b[K", 3);
                i += len_to_write;
                lines_written += 1;

                if lines_written == screen.height - 2 {
                    break;
                }

                if i >= line_len.saturating_sub(1) {
                    break;
                }
            }
        } else {
            let line_len = line.len();
            screen.screen_append(&line, line_len);
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }

        cur_line += 1;
    }

    // Fill any remaining screen lines with blanks
    while lines_written < screen.height.saturating_sub(2) {
        screen.screen_append("\r\n", 2);
        lines_written += 1;
    }
}
pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut current_line = screen.render_start_line;
    let mut virtual_cursor_row: usize = 1;
    let mut required_rows: i32;

    while current_line != buffer.cursor_row {
        let line_str_len = match &buffer.lines[current_line] {
            Some(gb) => gb.str_len,
            None => 0,
        };

        required_rows = VirtualScreen::required_screen_rows(line_str_len, screen.width);

        virtual_cursor_row += required_rows as usize;
        current_line += 1;
    }

    // If the cursor line wraps, shift the cursor down by however many wraps
    virtual_cursor_row += buffer.cursor_col / screen.width;

    screen.cursor.x = virtual_cursor_row;
    screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
}
}
