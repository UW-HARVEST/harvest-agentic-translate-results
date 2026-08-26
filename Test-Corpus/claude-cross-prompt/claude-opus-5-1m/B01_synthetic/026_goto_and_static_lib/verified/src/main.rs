// Translated from c_src/src/driver.c, preserving exact behavior.

use std::io::{self, Read, Write};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;

    // Use a labeled block to emulate the C `goto fail;` pattern.
    'fail: {
        if x != 1 {
            println!("Error: x != 1");
            result = 1;
            break 'fail;
        }

        if unsafe { Y } != 2 {
            println!("Error: x == 1 but y != 2");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            println!("Error: x == 1 and y == 2, but z != 3");
            result = 3;
            break 'fail;
        }

        println!("Ok!");
        return result;
    }

    println!("Operation failed");
    result
}

fn driver(x: i32, local_y: i32, z: i32) {
    unsafe {
        Y = local_y;
    }
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}

/// Parse integers from stdin in the same way C's `scanf("%d", ...)` does:
/// skip leading whitespace (including newlines), then read an optional sign
/// followed by digits.
struct ScanfReader {
    buf: Vec<u8>,
    pos: usize,
}

impl ScanfReader {
    fn from_stdin() -> io::Result<Self> {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(Self { buf, pos: 0 })
    }

    fn read_int(&mut self) -> Option<i32> {
        // skip whitespace
        while self.pos < self.buf.len() && (self.buf[self.pos] as char).is_whitespace() {
            self.pos += 1;
        }
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        if self.buf[self.pos] == b'-' || self.buf[self.pos] == b'+' {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while self.pos < self.buf.len() && (self.buf[self.pos] as char).is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == digits_start {
            return None;
        }
        let s = std::str::from_utf8(&self.buf[start..self.pos]).ok()?;
        s.parse::<i32>().ok()
    }
}

fn main() {
    let mut reader = match ScanfReader::from_stdin() {
        Ok(r) => r,
        Err(_) => return,
    };
    let x = reader.read_int().unwrap_or(0);
    let y = reader.read_int().unwrap_or(0);
    let z = reader.read_int().unwrap_or(0);

    driver(x, y, z);

    // Make sure stdout is flushed
    let _ = io::stdout().flush();
}
