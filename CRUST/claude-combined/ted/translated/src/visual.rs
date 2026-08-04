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
    /// Append the first `size` characters of `s` into the screen buffer.
    pub fn screen_append(&mut self, s: &str, size: usize) {
        // Only append if there's room in the buffer for `size` chars.
        if self.len.saturating_sub(self.buf_pos) > size {
            for (i, ch) in s.chars().take(size).enumerate() {
                if self.buf_pos + i < self.buffer.len() {
                    self.buffer[self.buf_pos + i] = ch;
                }
            }
            self.buf_pos += size;
        }
    }

    /// Returns number of screen rows required to print a line of length `line_length`.
    pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
        if line_length == 0 {
            return 1;
        }
        if screen_width == 0 {
            return 0;
        }
        let full = (line_length / screen_width) as i32;
        let extra = if line_length % screen_width > 0 { 1 } else { 0 };
        full + extra
    }

    /// If the buffer cursor is offscreen, shift `render_start_line` so it becomes visible.
    pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let cursor_row = buffer.cursor_row;

        if cursor_row < screen.render_start_line {
            screen.render_start_line = cursor_row;
            return;
        }

        // Calculate the visible line range starting at render_start_line.
        let mut cumul_req_rows: i32 = 0;
        let mut cur_line = screen.render_start_line;
        let height_minus_one = if screen.height == 0 { 0 } else { screen.height - 1 };

        while cur_line <= buffer.last_line_loc {
            let line_len = buffer.lines[cur_line]
                .as_ref()
                .map(|l| l.str_len)
                .unwrap_or(0);
            let cur_required = Self::required_screen_rows(line_len, screen.width);

            if (cur_required + cumul_req_rows) > height_minus_one as i32 {
                if cur_line > 0 {
                    cur_line -= 1;
                }
                break;
            }

            cumul_req_rows += cur_required;
            cur_line += 1;
        }

        // If cursor is below the last visible line, shift down
        if cursor_row > cur_line {
            let mut rows_required: i32 = 0;
            let mut walker = cur_line;
            while walker <= cursor_row {
                let line_len = buffer.lines[walker]
                    .as_ref()
                    .map(|l| l.str_len)
                    .unwrap_or(0);
                rows_required += Self::required_screen_rows(line_len, screen.width);
                walker += 1;
            }

            while rows_required > 0 {
                let line_len = buffer.lines[screen.render_start_line]
                    .as_ref()
                    .map(|l| l.str_len)
                    .unwrap_or(0);
                rows_required -= Self::required_screen_rows(line_len, screen.width);
                screen.render_start_line += 1;
            }
        }
    }

    /// Draw the editor window. Mirrors the C draw_editor_window logic.
    pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut cur_line = screen.render_start_line;
        let mut lines_written: usize = 0;
        let height_minus_one = if screen.height == 0 { 0 } else { screen.height - 1 };
        let height_minus_two = if screen.height >= 2 { screen.height - 2 } else { 0 };

        while cur_line <= buffer.last_line_loc && lines_written < height_minus_one {
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
                // Multi-row line
                let chars: Vec<char> = line.chars().collect();
                let mut i: usize = 0;

                loop {
                    let remaining = line_len - i;
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

                    if lines_written == height_minus_two {
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
        while lines_written < height_minus_two {
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }
    }

    /// Computes the screen cursor position from the text buffer cursor.
    pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut current_line = screen.render_start_line;
        let mut virtual_cursor_row: usize = 1;

        while current_line != buffer.cursor_row {
            let line_len = buffer.lines[current_line]
                .as_ref()
                .map(|l| l.str_len)
                .unwrap_or(0);
            let required_rows = Self::required_screen_rows(line_len, screen.width) as usize;
            virtual_cursor_row += required_rows;
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
