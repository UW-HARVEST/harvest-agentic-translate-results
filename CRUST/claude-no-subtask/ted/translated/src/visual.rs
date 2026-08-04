use crate::buffer::TextBuffer;

pub struct Cursor {
    x: usize,
    y: usize,
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

impl VirtualScreen {
    pub fn screen_append(&mut self, str: &str, size: usize) {
        if !self.buffer.is_empty() && (self.len.saturating_sub(self.buf_pos)) > size {
            let mut count = 0;
            for ch in str.chars() {
                if count >= size {
                    break;
                }
                if self.buf_pos >= self.buffer.len() {
                    break;
                }
                self.buffer[self.buf_pos] = ch;
                self.buf_pos += 1;
                count += 1;
            }
        }
    }

    pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
        if line_length == 0 {
            return 1;
        }
        if screen_width == 0 {
            return 1;
        }
        let extra = if line_length % screen_width > 0 { 1 } else { 0 };
        (line_length / screen_width) as i32 + extra
    }

    pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let buffer_cursor_x = buffer.cursor_row;

        if buffer_cursor_x < screen.render_start_line {
            screen.render_start_line = buffer_cursor_x;
        } else {
            let mut cumul_req_rows: i32 = 0;
            let mut cur_line = screen.render_start_line;

            while cur_line <= buffer.last_line_loc {
                let str_len = buffer.lines[cur_line]
                    .as_ref()
                    .map(|gb| gb.str_len)
                    .unwrap_or(0);
                let cur_line_required_rows =
                    Self::required_screen_rows(str_len, screen.width);

                if (cur_line_required_rows + cumul_req_rows) > (screen.height as i32 - 1) {
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
                let mut probe = cur_line;

                while probe <= buffer_cursor_x && probe <= buffer.last_line_loc {
                    let str_len = buffer.lines[probe]
                        .as_ref()
                        .map(|gb| gb.str_len)
                        .unwrap_or(0);
                    rows_required += Self::required_screen_rows(str_len, screen.width);
                    probe += 1;
                }

                while rows_required > 0 && screen.render_start_line <= buffer.last_line_loc {
                    let str_len = buffer.lines[screen.render_start_line]
                        .as_ref()
                        .map(|gb| gb.str_len)
                        .unwrap_or(0);
                    rows_required -= Self::required_screen_rows(str_len, screen.width);
                    screen.render_start_line += 1;
                }
            }
        }
    }

    pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut cur_line = screen.render_start_line;
        let mut lines_written: usize = 0;

        while cur_line <= buffer.last_line_loc && lines_written < screen.height.saturating_sub(1) {
            let screen_cols = screen.width;
            let line = match buffer.get_line(cur_line) {
                Some(l) => l,
                None => {
                    crate::defs::panic("draw editor cant get text of current line in buffer");
                    return;
                }
            };

            let line_len = line.chars().count();

            if line_len > screen_cols {
                let chars: Vec<char> = line.chars().collect();
                let mut i: usize = 0;
                let total = chars.len();

                loop {
                    let remaining = total - i;
                    let len_to_write = if screen_cols < remaining {
                        screen_cols
                    } else {
                        remaining
                    };

                    let segment: String = chars[i..i + len_to_write].iter().collect();
                    screen.screen_append(&segment, len_to_write);
                    screen.screen_append("\r\n", 2);
                    screen.screen_append("\x1b[K", 3);
                    i += len_to_write;
                    lines_written += 1;

                    if lines_written == screen.height.saturating_sub(2) {
                        break;
                    }

                    if i >= total.saturating_sub(1) {
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

        // Fill remaining space with blank lines
        while lines_written < screen.height.saturating_sub(2) {
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }
    }

    pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut current_line = screen.render_start_line;
        let mut virtual_cursor_row: usize = 1;

        while current_line != buffer.cursor_row {
            let str_len = buffer.lines[current_line]
                .as_ref()
                .map(|gb| gb.str_len)
                .unwrap_or(0);
            let required_rows = Self::required_screen_rows(str_len, screen.width);
            virtual_cursor_row += required_rows as usize;
            current_line += 1;
        }

        if screen.width > 0 {
            virtual_cursor_row += buffer.cursor_col / screen.width;
            screen.cursor.x = virtual_cursor_row;
            screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
        } else {
            screen.cursor.x = virtual_cursor_row;
            screen.cursor.y = buffer.cursor_col + 1;
        }
    }
}
