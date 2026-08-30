use std::io::{self, ErrorKind, Read};

struct Scanner<R> {
    reader: R,
    lookahead: Option<u8>,
    reached_eof: bool,
}

impl<R: Read> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            lookahead: None,
            reached_eof: false,
        }
    }

    fn peek_byte(&mut self) -> Option<u8> {
        while self.lookahead.is_none() && !self.reached_eof {
            let mut byte = [0_u8; 1];
            match self.reader.read(&mut byte) {
                Ok(0) => self.reached_eof = true,
                Ok(_) => self.lookahead = Some(byte[0]),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(_) => self.reached_eof = true,
            }
        }
        self.lookahead
    }

    fn consume_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte();
        self.lookahead = None;
        byte
    }

    fn scan_decimal_int(&mut self) -> Option<i32> {
        while matches!(
            self.peek_byte(),
            Some(b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
        ) {
            self.consume_byte();
        }

        let negative = match self.peek_byte() {
            Some(b'-') => {
                self.consume_byte();
                true
            }
            Some(b'+') => {
                self.consume_byte();
                false
            }
            _ => false,
        };

        let limit = if negative {
            (i64::MAX as u64) + 1
        } else {
            i64::MAX as u64
        };
        let mut magnitude = 0_u64;
        let mut matched_digit = false;

        while let Some(byte @ b'0'..=b'9') = self.peek_byte() {
            self.consume_byte();
            matched_digit = true;
            let digit = u64::from(byte - b'0');
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|value| value.checked_add(digit))
                .unwrap_or(limit)
                .min(limit);
        }

        if !matched_digit {
            return None;
        }

        let value = if negative {
            if magnitude == (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }
}

fn multi_stage(x: i32, y: i32, z: i32) -> i32 {
    if x != 1 {
        println!("Error: x != 1");
        println!("Operation failed");
        return 1;
    }

    if y != 2 {
        println!("Error: x == 1 but y != 2");
        println!("Operation failed");
        return 2;
    }

    if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        println!("Operation failed");
        return 3;
    }

    println!("Ok!");
    0
}

fn main() {
    let mut x = 0;
    let mut y = 123;
    let mut z = 0;
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());

    if let Some(value) = scanner.scan_decimal_int() {
        x = value;
        if let Some(value) = scanner.scan_decimal_int() {
            y = value;
            if let Some(value) = scanner.scan_decimal_int() {
                z = value;
            }
        }
    }

    let result = multi_stage(x, y, z);
    println!("Result: {result}");
}
