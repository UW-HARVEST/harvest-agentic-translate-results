use crate::buffer::TextBuffer;

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
    pub fn screen_append(&mut self, s: &str, size: usize) {
        if self.len - self.buf_pos > size {
            for (i, ch) in s.chars().take(size).enumerate() {
                if self.buf_pos + i < self.buffer.len() {
                    self.buffer[self.buf_pos + i] = ch;
                }
            }
            self.buf_pos += size;
        }
    }
    pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
        if line_length == 0 {
            1
        } else {
            (line_length / screen_width) as i32 + if line_length % screen_width > 0 { 1 } else { 0 }
        }
    }
    pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let buffer_cursor_x = buffer.cursor_row;

        if buffer_cursor_x < screen.render_start_line {
            screen.render_start_line = buffer_cursor_x;
            return;
        }

        let mut cumul_req_rows = 0i32;
        let mut cur_line = screen.render_start_line;

        while cur_line <= buffer.last_line_loc {
            let line_len = buffer.lines[cur_line].as_ref().unwrap().str_len;
            let cur_line_required_rows = Self::required_screen_rows(line_len, screen.width);
            if cur_line_required_rows + cumul_req_rows > screen.height as i32 - 1 {
                if cur_line > 0 { cur_line -= 1; }
                break;
            }
            cumul_req_rows += cur_line_required_rows;
            cur_line += 1;
        }

        if buffer_cursor_x > cur_line {
            let mut rows_required = 0i32;
            let mut cl = cur_line;
            while cl <= buffer_cursor_x {
                let line_len = buffer.lines[cl].as_ref().unwrap().str_len;
                rows_required += Self::required_screen_rows(line_len, screen.width);
                cl += 1;
            }
            while rows_required > 0 {
                let line_len = buffer.lines[screen.render_start_line].as_ref().unwrap().str_len;
                rows_required -= Self::required_screen_rows(line_len, screen.width);
                screen.render_start_line += 1;
            }
        }
    }
    pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
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
                    screen.screen_append(&line[i..i + len_to_write], len_to_write);
                    screen.screen_append("\r\n", 2);
                    screen.screen_append("\x1b[K", 3);
                    i += len_to_write;
                    lines_written += 1;
                    if lines_written == screen.height - 2 {
                        break;
                    }
                    if i >= line.len() - 1 {
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

        while lines_written < screen.height - 2 {
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }
    }
    pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut current_line = screen.render_start_line;
        let mut virtual_cursor_row: usize = 1;

        while current_line != buffer.cursor_row {
            let line_len = buffer.lines[current_line].as_ref().unwrap().str_len;
            let required_rows = Self::required_screen_rows(line_len, screen.width);
            virtual_cursor_row += required_rows as usize;
            current_line += 1;
        }

        virtual_cursor_row += buffer.cursor_col / screen.width;
        screen.cursor.x = virtual_cursor_row;
        screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
    }
}
