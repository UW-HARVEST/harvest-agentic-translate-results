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
    /// Append a string into the screen's internal buffer.
    /// Mirrors the C `screen_append`: it writes only if there is enough
    /// remaining space (len - buf_pos) > size.
    pub fn screen_append(&mut self, s: &str, size: usize) {
        if !self.buffer.is_empty() && (self.len - self.buf_pos) > size {
            let chars: Vec<char> = s.chars().collect();
            let to_copy = std::cmp::min(size, chars.len());
            for i in 0..to_copy {
                if self.buf_pos + i < self.buffer.len() {
                    self.buffer[self.buf_pos + i] = chars[i];
                }
            }
            self.buf_pos += size;
        }
    }

    /// Returns the number of screen rows required to print a line of the
    /// given length. Behavior is undefined for line_length < 0.
    pub fn required_screen_rows(line_length: usize, screen_width: usize) -> i32 {
        if line_length == 0 {
            1
        } else {
            let mut rows = (line_length / screen_width) as i32;
            if line_length % screen_width > 0 {
                rows += 1;
            }
            rows
        }
    }

    /// Determines whether the cursor is off the screen, and if so, shifts the
    /// `render_start_line` until the cursor is back in view.
    pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let cursor_x = buffer.cursor_row;

        // The render_start_line is fully rendered. If cursor is above it,
        // simply shift up.
        if cursor_x < screen.render_start_line {
            screen.render_start_line = cursor_x;
            return;
        }

        // Otherwise, calculate the visible range.
        let mut cumul_req_rows: i32 = 0;
        let mut cur_line = screen.render_start_line;

        while cur_line <= buffer.last_line_loc {
            let line = match buffer.lines.get(cur_line) {
                Some(Some(l)) => l,
                _ => break,
            };
            let cur_line_required_rows =
                VirtualScreen::required_screen_rows(line.str_len, screen.width);

            if (cur_line_required_rows + cumul_req_rows) > (screen.height as i32 - 1) {
                // back up one line
                if cur_line > 0 {
                    cur_line -= 1;
                }
                break;
            }

            cumul_req_rows += cur_line_required_rows;
            cur_line += 1;
        }

        // If the cursor is below cur_line, we need to scroll down
        if cursor_x > cur_line {
            let mut rows_required: i32 = 0;
            let mut k = cur_line;
            while k <= cursor_x {
                let line = match buffer.lines.get(k) {
                    Some(Some(l)) => l,
                    _ => break,
                };
                rows_required +=
                    VirtualScreen::required_screen_rows(line.str_len, screen.width);
                k += 1;
            }

            while rows_required > 0 {
                let line = match buffer.lines.get(screen.render_start_line) {
                    Some(Some(l)) => l,
                    _ => break,
                };
                rows_required -=
                    VirtualScreen::required_screen_rows(line.str_len, screen.width);
                screen.render_start_line += 1;
            }
        }
    }

    /// Render the visible portion of the buffer into the screen's append
    /// buffer.
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

            let line_chars: Vec<char> = line.chars().collect();

            if line_chars.len() > screen_cols {
                let mut i: usize = 0;
                loop {
                    let remaining = line_chars.len() - i;
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
                    if i >= line_chars.len().saturating_sub(1) {
                        break;
                    }
                }
            } else {
                screen.screen_append(&line, line_chars.len());
                screen.screen_append("\r\n", 2);
                lines_written += 1;
            }

            cur_line += 1;
        }

        // Fill remaining space with blanks
        while lines_written < screen.height.saturating_sub(2) {
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }
    }

    /// Compute the virtual cursor position based on the buffer's cursor and
    /// the current `render_start_line`.
    pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut current_line = screen.render_start_line;
        let mut virtual_cursor_row: i32 = 1;

        while current_line != buffer.cursor_row {
            let line = match buffer.lines.get(current_line) {
                Some(Some(l)) => l,
                _ => break,
            };
            let required_rows =
                VirtualScreen::required_screen_rows(line.str_len, screen.width);
            virtual_cursor_row += required_rows;
            current_line += 1;
        }

        // wrap-aware cursor offset
        if screen.width > 0 {
            virtual_cursor_row += (buffer.cursor_col / screen.width) as i32;
        }

        screen.cursor.x = virtual_cursor_row as usize;
        let col = if screen.width > 0 {
            (buffer.cursor_col % screen.width) + 1
        } else {
            1
        };
        screen.cursor.y = col;
    }
}
