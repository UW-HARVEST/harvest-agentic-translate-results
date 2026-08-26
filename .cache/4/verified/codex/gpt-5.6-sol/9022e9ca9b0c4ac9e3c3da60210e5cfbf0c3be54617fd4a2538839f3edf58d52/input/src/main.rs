use std::io::{self, Read, Write};

struct Scanner<R> {
    reader: R,
    lookahead: Option<u8>,
}

impl<R: Read> Scanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            lookahead: None,
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if let Some(byte) = self.lookahead.take() {
            return Some(byte);
        }

        let mut byte = [0];
        loop {
            match self.reader.read(&mut byte) {
                Ok(0) => return None,
                Ok(_) => return Some(byte[0]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => return None,
            }
        }
    }

    fn scan_decimal_i32(&mut self) -> Option<i32> {
        let mut byte = loop {
            let byte = self.read_byte()?;
            if !matches!(byte, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
                break byte;
            }
        };

        let negative = match byte {
            b'+' => {
                byte = self.read_byte()?;
                false
            }
            b'-' => {
                byte = self.read_byte()?;
                true
            }
            _ => false,
        };

        if !byte.is_ascii_digit() {
            return None;
        }

        let limit = if negative {
            (i64::MAX as u64) + 1
        } else {
            i64::MAX as u64
        };
        let mut magnitude = 0_u64;
        let mut overflowed = false;

        loop {
            let digit = u64::from(byte - b'0');
            if !overflowed {
                if magnitude > (limit - digit) / 10 {
                    magnitude = limit;
                    overflowed = true;
                } else {
                    magnitude = magnitude * 10 + digit;
                }
            }

            match self.read_byte() {
                Some(next) if next.is_ascii_digit() => byte = next,
                Some(next) => {
                    self.lookahead = Some(next);
                    break;
                }
                None => break,
            }
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

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    let mut out = vec![0; len];
    let ones = vec![1; len];
    let zeros = vec![0; len];

    out[0] = 0;
    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

fn main() {
    let stdin = io::stdin();
    let mut scanner = Scanner::new(stdin.lock());
    let mut data = [0_i32; 100];
    let mut len = 0;

    while len < data.len() {
        let Some(value) = scanner.scan_decimal_i32() else {
            break;
        };
        data[len] = value;
        len += 1;
    }

    let result = call_fma(&data, len);
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let _ = writeln!(stdout, "{result}");
}
