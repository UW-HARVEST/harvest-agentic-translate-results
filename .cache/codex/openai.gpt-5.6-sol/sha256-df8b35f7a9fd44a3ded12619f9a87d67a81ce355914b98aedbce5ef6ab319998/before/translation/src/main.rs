use std::io::{self, Read};
use std::os::raw::c_char;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> i32;
    fn strncpy(dest: *mut c_char, src: *const c_char, count: usize) -> *mut c_char;
}

fn print_line(line: *const c_char) {
    const FORMAT: &[u8] = b"%s\n\0";

    if !line.is_null() {
        unsafe {
            printf(FORMAT.as_ptr().cast(), line);
        }
    }
}

fn fgets_14() -> io::Result<Option<Vec<u8>>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut buffer = Vec::with_capacity(13);
    let mut byte = [0_u8; 1];

    while buffer.len() < 13 {
        match input.read(&mut byte)? {
            0 if buffer.is_empty() => return Ok(None),
            0 => break,
            _ => {
                buffer.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    Ok(Some(buffer))
}

fn atoi(input: &[u8]) -> i32 {
    let mut index = 0;
    while index < input.len() && matches!(input[index], b' ' | b'\t'..=b'\r') {
        index += 1;
    }

    let negative = match input.get(index) {
        Some(b'-') => {
            index += 1;
            true
        }
        Some(b'+') => {
            index += 1;
            false
        }
        _ => false,
    };

    let mut value = 0_i64;
    while let Some(&digit @ b'0'..=b'9') = input.get(index) {
        value = value * 10 + i64::from(digit - b'0');
        index += 1;
    }

    if negative {
        (-value) as i32
    } else {
        value as i32
    }
}

fn main() {
    let mut data = -1;

    match fgets_14() {
        Ok(Some(input)) => data = atoi(&input),
        Ok(None) | Err(_) => {
            const ERROR: &[u8] = b"fgets() failed.\0";
            print_line(ERROR.as_ptr().cast());
        }
    }

    let mut source = [b'A'; 100];
    source[99] = 0;
    let mut dest = [0_u8; 100];

    if data < 100 {
        // Preserve the C program's signed-to-size_t conversion and unchecked index.
        unsafe {
            strncpy(
                dest.as_mut_ptr().cast(),
                source.as_ptr().cast(),
                data as usize,
            );
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    print_line(dest.as_ptr().cast());
}
