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
        // Append `size` chars from `str` to self.buffer, starting at buf_pos, only
        // if there is enough remaining capacity.
        if self.len.saturating_sub(self.buf_pos) > size {
            let chars: Vec<char> = str.chars().take(size).collect();
            for (i, ch) in chars.iter().enumerate() {
                if self.buf_pos + i < self.buffer.len() {
                    self.buffer[self.buf_pos + i] = *ch;
                } else {
                    self.buffer.push(*ch);
                }
            }
            self.buf_pos += size;
        }
    }

    pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
        if line_length == 0 {
            return 1;
        }
        if screen_width == 0 {
            return 1;
        }

        let base = (line_length / screen_width) as i32;
        let extra = if line_length % screen_width > 0 { 1 } else { 0 };
        base + extra
    }

    pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let buffer_cursor_x = buffer.cursor_row;

        // The render start line is guaranteed to render
        if buffer_cursor_x < screen.render_start_line {
            screen.render_start_line = buffer_cursor_x;
            return;
        }

        let mut cumul_req_rows: i32 = 0;
        let mut cur_line = screen.render_start_line;

        while cur_line <= buffer.last_line_loc {
            let str_len = match &buffer.lines[cur_line] {
                Some(l) => l.str_len,
                None => 0,
            };
            let cur_line_required_rows = Self::required_screen_rows(str_len, screen.width);

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
            let mut iter_line = cur_line;
            while iter_line <= buffer_cursor_x {
                let str_len = match &buffer.lines[iter_line] {
                    Some(l) => l.str_len,
                    None => 0,
                };
                rows_required += Self::required_screen_rows(str_len, screen.width);
                iter_line += 1;
            }

            // shift the render start line down until enough room is made
            while rows_required > 0 && screen.render_start_line < buffer.lines.len() {
                let str_len = match &buffer.lines[screen.render_start_line] {
                    Some(l) => l.str_len,
                    None => 0,
                };
                rows_required -= Self::required_screen_rows(str_len, screen.width);
                screen.render_start_line += 1;
            }
        }
    }

    pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut cur_line = screen.render_start_line;
        let mut lines_written: usize = 0;

        while cur_line <= buffer.last_line_loc && lines_written < screen.height.saturating_sub(1) {
            let screen_cols = screen.width;

            // Get current line's text
            let line = match buffer.get_line(cur_line) {
                Some(l) => l,
                None => {
                    crate::defs::panic("draw editor cant get text of current line in buffer");
                    return;
                }
            };

            if line.len() > screen_cols {
                let bytes = line.as_bytes();
                let mut i: usize = 0;
                let total_len = bytes.len();

                loop {
                    let remaining = total_len - i;
                    let len_to_write = if screen_cols < remaining {
                        screen_cols
                    } else {
                        remaining
                    };

                    // safe slice as long as input is ASCII; convert to str carefully
                    let chunk = match std::str::from_utf8(&bytes[i..i + len_to_write]) {
                        Ok(s) => s.to_string(),
                        Err(_) => {
                            // fallback: take chars from the front
                            line.chars().skip(i).take(len_to_write).collect::<String>()
                        }
                    };
                    screen.screen_append(&chunk, len_to_write);
                    screen.screen_append("\r\n", 2);
                    screen.screen_append("\x1b[K", 3);
                    i += len_to_write;
                    lines_written += 1;

                    if lines_written == screen.height.saturating_sub(2) {
                        break;
                    }

                    if i >= total_len.saturating_sub(1) {
                        break;
                    }
                }
            } else {
                let n = line.len();
                screen.screen_append(&line, n);
                screen.screen_append("\r\n", 2);
                lines_written += 1;
            }

            cur_line += 1;
        }

        // If there's remaining space, fill with blanks
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
                Some(l) => l.str_len,
                None => 0,
            };
            let required_rows = Self::required_screen_rows(str_len, screen.width) as usize;
            virtual_cursor_row += required_rows;
            current_line += 1;
        }

        // If the cursor line wraps, shift the cursor down by the wrap count
        if screen.width > 0 {
            virtual_cursor_row += buffer.cursor_col / screen.width;
        }

        screen.cursor.x = virtual_cursor_row;
        screen.cursor.y = if screen.width > 0 {
            (buffer.cursor_col % screen.width) + 1
        } else {
            1
        };
    }
}
