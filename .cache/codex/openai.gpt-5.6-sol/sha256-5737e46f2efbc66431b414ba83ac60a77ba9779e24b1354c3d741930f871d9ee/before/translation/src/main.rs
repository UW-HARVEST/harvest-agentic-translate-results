use std::io::{self, BufRead, Write};

struct Scanner<R> {
    reader: R,
}

impl<R: BufRead> Scanner<R> {
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn peek(&mut self) -> Option<u8> {
        self.reader.fill_buf().ok()?.first().copied()
    }

    fn consume(&mut self) {
        self.reader.consume(1);
    }

    fn scan_int(&mut self) -> Option<i32> {
        while matches!(
            self.peek(),
            Some(b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
        ) {
            self.consume();
        }

        let negative = match self.peek() {
            Some(b'-') => {
                self.consume();
                true
            }
            Some(b'+') => {
                self.consume();
                false
            }
            _ => false,
        };

        let limit = if negative {
            (i64::MAX as u64) + 1
        } else {
            i64::MAX as u64
        };
        let mut has_digit = false;
        let mut value = 0u64;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            has_digit = true;
            value = value
                .saturating_mul(10)
                .saturating_add(u64::from(byte - b'0'))
                .min(limit);
            self.consume();
        }

        if !has_digit {
            return None;
        }

        let value = if negative {
            if value == (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                -(value as i64)
            }
        } else {
            value as i64
        };
        Some(value as i32)
    }
}

fn foo<W: Write>(mut x: i32, mut y: i32, output: &mut W) {
    'outer: while x > 0 || y > 0 {
        let _ = output.write_all(b"loop\n");

        if x != 1 || y != 4 {
            if x > 0 {
                let _ = output.write_all(b"x\n");
                x = x.wrapping_sub(1);
            }
        }

        loop {
            if y == 0 {
                continue 'outer;
            }
            let _ = output.write_all(b"y\n");
            y = y.wrapping_sub(1);
            if x >= 3 {
                break;
            }
            if x > 0 {
                let _ = output.write_all(b"x\n");
                x = x.wrapping_sub(1);
            }
        }
    }
}

fn main() {
    let mut x = 0;
    let mut y = 0;
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());
    if let Some(value) = scanner.scan_int() {
        x = value;
        if let Some(value) = scanner.scan_int() {
            y = value;
        }
    }

    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    foo(x, y, &mut output);
}
