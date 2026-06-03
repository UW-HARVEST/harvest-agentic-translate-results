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
        // Take up to `size` chars from the input string.
        let chars: Vec<char> = str.chars().take(size).collect();
        let to_write = chars.len();

        // Only write if there's room (matches C semantics: len - buf_pos > size).
        if self.len > self.buf_pos && (self.len - self.buf_pos) > to_write {
            for (i, c) in chars.into_iter().enumerate() {
                if self.buf_pos + i < self.buffer.len() {
                    self.buffer[self.buf_pos + i] = c;
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

        let rows = (line_length / screen_width) as i32;
        let remainder = if line_length % screen_width > 0 { 1 } else { 0 };
        rows + remainder
    }

    pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let buffer_cursor_x = buffer.cursor_row;

        if buffer_cursor_x < screen.render_start_line {
            screen.render_start_line = buffer_cursor_x;
        } else {
            let mut cumul_req_rows: i32 = 0;
            let mut cur_line = screen.render_start_line;

            while cur_line <= buffer.last_line_loc {
                let line_str_len = buffer
                    .lines
                    .get(cur_line)
                    .and_then(|x| x.as_ref())
                    .map(|g| g.str_len)
                    .unwrap_or(0);

                let cur_line_required_rows =
                    Self::required_screen_rows(line_str_len, screen.width);

                if cur_line_required_rows + cumul_req_rows > (screen.height as i32) - 1 {
                    if cur_line > 0 {
                        cur_line -= 1;
                    }
                    break;
                }

                cumul_req_rows += cur_line_required_rows;
                cur_line += 1;
            }

            // If the cursor row is below the visible range, shift the displayed area.
            if buffer_cursor_x > cur_line {
                let mut rows_required: i32 = 0;
                let mut cl = cur_line;
                while cl <= buffer_cursor_x {
                    let line_str_len = buffer
                        .lines
                        .get(cl)
                        .and_then(|x| x.as_ref())
                        .map(|g| g.str_len)
                        .unwrap_or(0);
                    rows_required += Self::required_screen_rows(line_str_len, screen.width);
                    cl += 1;
                }

                while rows_required > 0 {
                    let line_str_len = buffer
                        .lines
                        .get(screen.render_start_line)
                        .and_then(|x| x.as_ref())
                        .map(|g| g.str_len)
                        .unwrap_or(0);
                    rows_required -=
                        Self::required_screen_rows(line_str_len, screen.width);
                    screen.render_start_line += 1;
                }
            }
        }
    }

    pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut cur_line = screen.render_start_line;
        let mut lines_written: usize = 0;
        let screen_height = screen.height;
        let screen_cols = screen.width;

        while cur_line <= buffer.last_line_loc
            && (screen_height >= 1 && lines_written < screen_height - 1)
        {
            let line = match buffer.get_line(cur_line) {
                Some(l) => l,
                None => {
                    crate::defs::panic("draw editor cant get text of current line in buffer");
                    return;
                }
            };

            if line.len() > screen_cols {
                let chars: Vec<char> = line.chars().collect();
                let mut i: usize = 0;
                let total_len = chars.len();

                loop {
                    let remaining = total_len - i;
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

                    if screen_height >= 2 && lines_written == screen_height - 2 {
                        break;
                    }

                    if i >= total_len.saturating_sub(1) {
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

        // Pad remaining space with newlines
        if screen_height >= 2 {
            while lines_written < screen_height - 2 {
                screen.screen_append("\r\n", 2);
                lines_written += 1;
            }
        }
    }

    pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut current_line = screen.render_start_line;
        let mut virtual_cursor_row: usize = 1;

        while current_line != buffer.cursor_row {
            let line_str_len = buffer
                .lines
                .get(current_line)
                .and_then(|x| x.as_ref())
                .map(|g| g.str_len)
                .unwrap_or(0);

            let required_rows = Self::required_screen_rows(line_str_len, screen.width);
            virtual_cursor_row = virtual_cursor_row.saturating_add(required_rows as usize);
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
