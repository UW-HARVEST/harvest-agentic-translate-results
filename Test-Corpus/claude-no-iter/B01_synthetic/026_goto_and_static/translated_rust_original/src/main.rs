use std::io::{self, Read, Write};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;
    let y = unsafe { Y };

    if x != 1 {
        println!("Error: x != 1");
        result = 1;
        println!("Operation failed");
        return result;
    }

    if y != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
        println!("Operation failed");
        return result;
    }

    if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
        println!("Operation failed");
        return result;
    }

    println!("Ok!");
    result
}

/// Reads all of stdin, then yields integer tokens parsed from it,
/// matching C's scanf("%d", ...) behavior of skipping whitespace
/// (including newlines) before each integer.
struct ScanfInts {
    buf: Vec<u8>,
    pos: usize,
}

impl ScanfInts {
    fn new() -> Self {
        let mut buf = Vec::new();
        let _ = io::stdin().read_to_end(&mut buf);
        ScanfInts { buf, pos: 0 }
    }

    fn next_int(&mut self) -> Option<i32> {
        // Skip whitespace
        while self.pos < self.buf.len()
            && (self.buf[self.pos] as char).is_ascii_whitespace()
        {
            self.pos += 1;
        }
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        // Optional sign
        if self.buf[self.pos] == b'+' || self.buf[self.pos] == b'-' {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while self.pos < self.buf.len()
            && (self.buf[self.pos] as char).is_ascii_digit()
        {
            self.pos += 1;
        }
        if self.pos == digits_start {
            // No digits found; restore position and fail
            self.pos = start;
            return None;
        }
        let s = std::str::from_utf8(&self.buf[start..self.pos]).ok()?;
        // Use wrapping parsing to mirror C int overflow loosely; for our
        // purposes, normal parse is fine.
        s.parse::<i32>().ok()
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut z: i32 = 0;

    let mut scanner = ScanfInts::new();
    if let Some(v) = scanner.next_int() {
        x = v;
        if let Some(v) = scanner.next_int() {
            unsafe { Y = v; }
            if let Some(v) = scanner.next_int() {
                z = v;
            }
        }
    }

    let result = multi_stage(x, z);
    println!("Result: {}", result);

    // Ensure stdout is flushed before exit.
    let _ = io::stdout().flush();
}
