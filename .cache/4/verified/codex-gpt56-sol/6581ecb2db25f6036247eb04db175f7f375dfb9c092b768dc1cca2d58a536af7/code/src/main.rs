use std::fmt::Write as _;
use std::io::{self, Read};
use std::process::ExitCode;

const INPUT_CAPACITY: usize = 256;
const BUFFER_CAPACITY: usize = INPUT_CAPACITY * 2;

struct Scanner {
    input: Vec<u8>,
    position: usize,
}

impl Scanner {
    fn from_stdin() -> Self {
        let mut input = Vec::new();
        let _ = io::stdin().read_to_end(&mut input);
        Self { input, position: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len()
            && matches!(
                self.input[self.position],
                b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'
            )
        {
            self.position += 1;
        }
    }

    fn sign_and_digits(&mut self) -> Option<(bool, u64, bool)> {
        self.skip_whitespace();

        let negative = match self.input.get(self.position) {
            Some(b'+') => {
                self.position += 1;
                false
            }
            Some(b'-') => {
                self.position += 1;
                true
            }
            _ => false,
        };

        let start = self.position;
        let mut value = 0_u64;
        let mut overflowed = false;
        while let Some(&byte) = self.input.get(self.position) {
            if !byte.is_ascii_digit() {
                break;
            }
            let digit = u64::from(byte - b'0');
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(next) if !overflowed => value = next,
                _ => {
                    value = u64::MAX;
                    overflowed = true;
                }
            }
            self.position += 1;
        }

        (self.position != start).then_some((negative, value, overflowed))
    }

    fn scan_unsigned(&mut self) -> Option<u64> {
        let (negative, magnitude, overflowed) = self.sign_and_digits()?;
        if overflowed {
            Some(u64::MAX)
        } else if negative {
            Some(0_u64.wrapping_sub(magnitude))
        } else {
            Some(magnitude)
        }
    }

    fn scan_u32(&mut self) -> Option<u32> {
        self.scan_unsigned().map(|value| value as u32)
    }

    fn scan_usize(&mut self) -> Option<usize> {
        self.scan_unsigned().map(|value| value as usize)
    }

    fn scan_i32(&mut self) -> Option<i32> {
        let (negative, magnitude, _) = self.sign_and_digits()?;
        let value = if negative {
            if magnitude >= (1_u64 << 63) {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }
}

fn run() -> Result<(), String> {
    let mut scanner = Scanner::from_stdin();

    let flags = scanner
        .scan_u32()
        .ok_or_else(|| "Error reading flags".to_owned())?;
    let param1 = scanner
        .scan_i32()
        .ok_or_else(|| "Error reading param1".to_owned())?;
    let param2 = scanner
        .scan_i32()
        .ok_or_else(|| "Error reading param2".to_owned())?;
    let length = scanner
        .scan_usize()
        .ok_or_else(|| "Error reading length".to_owned())?;

    if length > INPUT_CAPACITY {
        return Err(format!(
            "Error: length {length} exceeds maximum {INPUT_CAPACITY}"
        ));
    }

    let mut buffer = [0_u8; BUFFER_CAPACITY];
    for (i, byte) in buffer.iter_mut().take(length).enumerate() {
        let value = scanner
            .scan_u32()
            .ok_or_else(|| format!("Error reading byte {i}"))?;
        *byte = value as u8;
    }

    let new_length = driver::process_buffer_slice(&mut buffer, length, flags, param1, param2);
    let mut output = new_length.to_string();
    for byte in &buffer[..new_length] {
        write!(output, " {byte}").unwrap();
    }
    output.push('\n');
    print!("{output}");

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
