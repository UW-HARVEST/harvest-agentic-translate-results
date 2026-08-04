use std::io::{self, Read};

struct ScanfReader<R> {
    reader: R,
    pushed: Option<u8>,
}

impl<R: Read> ScanfReader<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            pushed: None,
        }
    }

    fn read_byte(&mut self) -> Option<u8> {
        if self.pushed.is_some() {
            return self.pushed.take();
        }

        let mut byte = [0_u8; 1];
        match self.reader.read(&mut byte) {
            Ok(1) => Some(byte[0]),
            _ => None,
        }
    }

    fn unread_byte(&mut self, byte: u8) {
        self.pushed = Some(byte);
    }

    fn skip_scanf_whitespace(&mut self) {
        while let Some(byte) = self.read_byte() {
            if matches!(byte, b' ' | b'\n' | b'\r' | b'\t' | 0x0b | 0x0c) {
                continue;
            }

            self.unread_byte(byte);
            break;
        }
    }

    fn scan_decimal_int(&mut self) -> Option<i32> {
        self.skip_scanf_whitespace();

        let mut sign = 1_i64;
        match self.read_byte() {
            Some(b'-') => sign = -1,
            Some(b'+') => {}
            Some(byte) => self.unread_byte(byte),
            None => return None,
        }

        let mut saw_digit = false;
        let mut value = 0_i64;
        while let Some(byte) = self.read_byte() {
            if byte.is_ascii_digit() {
                saw_digit = true;
                value = value.wrapping_mul(10).wrapping_add((byte - b'0') as i64);
            } else {
                self.unread_byte(byte);
                break;
            }
        }

        if saw_digit {
            Some((value.wrapping_mul(sign)) as i32)
        } else {
            None
        }
    }
}

fn scanf_three_ints<R: Read>(scanner: &mut ScanfReader<R>, x: &mut i32, y: &mut i32, z: &mut i32) {
    if let Some(value) = scanner.scan_decimal_int() {
        *x = value;
    } else {
        return;
    }

    if let Some(value) = scanner.scan_decimal_int() {
        *y = value;
    } else {
        return;
    }

    if let Some(value) = scanner.scan_decimal_int() {
        *z = value;
    }
}

fn multi_stage(x: i32, y: i32, z: i32) -> i32 {
    let result;

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
    0
}

fn main() {
    let mut x = 0;
    let mut y = 123;
    let mut z = 0;

    let stdin = io::stdin();
    let mut scanner = ScanfReader::new(stdin.lock());
    scanf_three_ints(&mut scanner, &mut x, &mut y, &mut z);

    let result = multi_stage(x, y, z);
    println!("Result: {}", result);
}
