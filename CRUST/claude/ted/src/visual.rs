use crate::buffer::{TextBuffer};
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
    if (self.len - self.buf_pos) > size {
        for (i, ch) in str.chars().take(size).enumerate() {
            if self.buf_pos + i < self.buffer.len() {
                self.buffer[self.buf_pos + i] = ch;
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
    let extra = if line_length % screen_width > 0 { 1 } else { 0 };
    ((line_length / screen_width) as i32) + extra
}
pub fn move_cursor_in_view(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let buffer_cursor_x = buffer.cursor_row;
    let mut cumul_req_rows: i32 = 0;
    let mut cur_line_required_rows: i32;
    let mut cur_line: usize = screen.render_start_line;

    if buffer_cursor_x < screen.render_start_line {
        screen.render_start_line = buffer_cursor_x;
    } else {
        while cur_line <= buffer.last_line_loc {
            let line_str_len = buffer
                .lines
                .get(cur_line)
                .and_then(|l| l.as_ref())
                .map(|l| l.str_len)
                .unwrap_or(0);
            cur_line_required_rows =
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

        if buffer_cursor_x > cur_line {
            let mut rows_required: i32 = 0;
            let mut cl = cur_line;
            while cl <= buffer_cursor_x {
                let line_str_len = buffer
                    .lines
                    .get(cl)
                    .and_then(|l| l.as_ref())
                    .map(|l| l.str_len)
                    .unwrap_or(0);
                rows_required += Self::required_screen_rows(line_str_len, screen.width);
                cl += 1;
            }
            while rows_required > 0 {
                let line_str_len = buffer
                    .lines
                    .get(screen.render_start_line)
                    .and_then(|l| l.as_ref())
                    .map(|l| l.str_len)
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
    let mut lines_written: i32 = 0;
    let height = screen.height as i32;
    while cur_line <= buffer.last_line_loc && lines_written < height - 1 {
        let screen_cols = screen.width;
        let line = match buffer.get_line(cur_line) {
            Some(s) => s,
            None => {
                crate::defs::panic("draw editor cant get text of current line in buffer");
                return;
            }
        };
        if line.len() > screen_cols {
            let mut i: usize = 0;
            loop {
                let remaining = line.len() - i;
                let len_to_write = if screen_cols < remaining {
                    screen_cols
                } else {
                    remaining
                };
                let slice: String = line.chars().skip(i).take(len_to_write).collect();
                screen.screen_append(&slice, len_to_write);
                screen.screen_append("\r\n", 2);
                screen.screen_append("\x1b[K", 3);
                i += len_to_write;
                lines_written += 1;
                if lines_written == height - 2 {
                    break;
                }
                if i >= line.len() {
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
    while lines_written < height - 2 {
        screen.screen_append("\r\n", 2);
        lines_written += 1;
    }
}
pub fn set_virtual_cursor_position(buffer: &TextBuffer, screen: &mut VirtualScreen) {
    let mut current_line = screen.render_start_line;
    let mut virtual_cursor_row: i32 = 1;
    while current_line != buffer.cursor_row {
        let line_str_len = buffer
            .lines
            .get(current_line)
            .and_then(|l| l.as_ref())
            .map(|l| l.str_len)
            .unwrap_or(0);
        let required = Self::required_screen_rows(line_str_len, screen.width);
        virtual_cursor_row += required;
        current_line += 1;
    }
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

#[allow(dead_code)]
impl VirtualScreen {
    fn _suppress_unused(&self) {
        let _ = self.buffer.len();
        let _ = self.buf_pos;
        let _ = self.len;
        let _ = (self.cursor.x, self.cursor.y);
        let _ = self.width;
        let _ = self.height;
        let _ = self.render_start_line;
    }
}
