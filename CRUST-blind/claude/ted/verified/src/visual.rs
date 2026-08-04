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
        if !self.buffer.is_empty() && (self.len.saturating_sub(self.buf_pos)) > size {
            // Take exactly `size` chars from `str` (matching C's memcpy with size bytes).
            // We treat the screen buffer as a Vec<char>, so push char-by-char.
            let mut count = 0usize;
            for ch in str.chars() {
                if count >= size {
                    break;
                }
                if self.buf_pos < self.buffer.len() {
                    self.buffer[self.buf_pos] = ch;
                } else {
                    self.buffer.push(ch);
                }
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
            return 0;
        }
        let q = (line_length / screen_width) as i32;
        let r = if line_length % screen_width > 0 { 1 } else { 0 };
        q + r
    }

    pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let buffer_cursor_x = buffer.cursor_row;

        let mut cumul_req_rows: i32 = 0;
        let mut cur_line_required_rows: i32;
        let mut cur_line: usize = screen.render_start_line;

        if buffer_cursor_x < screen.render_start_line {
            screen.render_start_line = buffer_cursor_x;
            return;
        }

        // Otherwise calculate visible range
        // The C code uses `int cur_line` and `cur_line--` semantics; we mirror with isize.
        let mut last_visible: isize = cur_line as isize;
        while cur_line <= buffer.last_line_loc {
            let str_len = match &buffer.lines[cur_line] {
                Some(gb) => gb.str_len,
                None => 0,
            };
            cur_line_required_rows =
                VirtualScreen::required_screen_rows(str_len, screen.width);

            if cur_line_required_rows + cumul_req_rows > (screen.height as i32) - 1 {
                last_visible = cur_line as isize - 1;
                break;
            }

            cumul_req_rows += cur_line_required_rows;
            last_visible = cur_line as isize;
            cur_line += 1;
        }

        // After the loop, last_visible holds the index of the last line that fits.
        // If buffer_cursor_x > last_visible, we need to shift down.
        // Match C semantics: the inclusive walk begins at last_visible (matches C's cur_line
        // after the `cur_line--; break;` step).
        if (buffer_cursor_x as isize) > last_visible {
            let mut rows_required: i32 = 0;
            let mut walk = if last_visible < 0 { 0 } else { last_visible as usize };
            while walk <= buffer_cursor_x {
                let str_len = match &buffer.lines[walk] {
                    Some(gb) => gb.str_len,
                    None => 0,
                };
                rows_required += VirtualScreen::required_screen_rows(str_len, screen.width);
                walk += 1;
            }

            while rows_required > 0 {
                let str_len = match &buffer.lines[screen.render_start_line] {
                    Some(gb) => gb.str_len,
                    None => 0,
                };
                rows_required -=
                    VirtualScreen::required_screen_rows(str_len, screen.width);
                screen.render_start_line += 1;
            }
        }
    }

    pub fn draw_editor_window(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut cur_line = screen.render_start_line;
        let mut lines_written: usize = 0;
        let screen_cols = screen.width;
        let screen_height = screen.height;

        while cur_line <= buffer.last_line_loc && lines_written + 1 < screen_height {
            let line_opt = buffer.get_line(cur_line);
            let line = match line_opt {
                Some(l) => l,
                None => {
                    panic("draw editor cant get text of current line in buffer");
                    return;
                }
            };

            let line_len = line.chars().count();

            if line_len > screen_cols {
                let chars: Vec<char> = line.chars().collect();
                let mut i = 0usize;

                loop {
                    let remaining = chars.len() - i;
                    let len_to_write = if screen_cols < remaining { screen_cols } else { remaining };

                    let segment: String = chars[i..i + len_to_write].iter().collect();
                    screen.screen_append(&segment, len_to_write);
                    screen.screen_append("\r\n", 2);
                    screen.screen_append("\x1b[K", 3);
                    i += len_to_write;
                    lines_written += 1;

                    // Screen full
                    if lines_written + 2 == screen_height {
                        break;
                    }

                    if i + 1 >= chars.len() {
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

        // Fill remaining with blank lines
        while lines_written + 2 < screen_height {
            screen.screen_append("\r\n", 2);
            lines_written += 1;
        }
    }

    pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
        let mut current_line = screen.render_start_line;
        let mut virtual_cursor_row: i32 = 1;
        let mut required_rows: i32;

        while current_line != buffer.cursor_row {
            let str_len = match &buffer.lines[current_line] {
                Some(gb) => gb.str_len,
                None => 0,
            };
            required_rows =
                VirtualScreen::required_screen_rows(str_len, screen.width);
            virtual_cursor_row += required_rows;
            current_line += 1;
        }

        // Cursor wrap
        if screen.width > 0 {
            virtual_cursor_row += (buffer.cursor_col / screen.width) as i32;
            screen.cursor.x = virtual_cursor_row as usize;
            screen.cursor.y = (buffer.cursor_col % screen.width) + 1;
        } else {
            screen.cursor.x = virtual_cursor_row as usize;
            screen.cursor.y = 1;
        }
    }
}
