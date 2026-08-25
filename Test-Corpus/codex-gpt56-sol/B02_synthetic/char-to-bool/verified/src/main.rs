use std::io::{self, Read, Write};

const MAX_INPUT_SIZE: usize = 1024;

fn fgets<R: Read>(reader: &mut R, buffer: &mut Vec<u8>) -> io::Result<bool> {
    buffer.clear();

    while buffer.len() < MAX_INPUT_SIZE - 1 {
        let mut byte = [0_u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buffer.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }

    Ok(!buffer.is_empty())
}

fn c_string_prefix(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&byte| byte == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}

fn c_atoi(bytes: &[u8]) -> i32 {
    let bytes = c_string_prefix(bytes);
    let mut index = 0;

    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = if index < bytes.len() && (bytes[index] == b'+' || bytes[index] == b'-') {
        let negative = bytes[index] == b'-';
        index += 1;
        negative
    } else {
        false
    };

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value = 0_u64;
    let mut overflowed = false;

    while index < bytes.len() && bytes[index].is_ascii_digit() {
        let digit = u64::from(bytes[index] - b'0');
        if value > (limit - digit) / 10 {
            overflowed = true;
        } else if !overflowed {
            value = value * 10 + digit;
        }
        index += 1;
    }

    let signed = if overflowed {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        if value == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(value as i64)
        }
    } else {
        value as i64
    };

    signed as i32
}

fn read_required_line<R: Read>(reader: &mut R, buffer: &mut Vec<u8>, error_message: &str) -> bool {
    match fgets(reader, buffer) {
        Ok(true) => true,
        Ok(false) | Err(_) => {
            let _ = writeln!(io::stderr().lock(), "{error_message}");
            false
        }
    }
}

fn main() {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut input_buffer = Vec::with_capacity(MAX_INPUT_SIZE - 1);

    if !read_required_line(&mut stdin, &mut input_buffer, "Error reading operation") {
        std::process::exit(1);
    }
    let operation = c_atoi(&input_buffer);

    if !read_required_line(&mut stdin, &mut input_buffer, "Error reading parameter") {
        std::process::exit(1);
    }
    let param = c_atoi(&input_buffer);

    if !read_required_line(
        &mut stdin,
        &mut input_buffer,
        "Error reading decision string",
    ) {
        std::process::exit(1);
    }

    let mut len = c_string_prefix(&input_buffer).len();
    if len > 0 && input_buffer[len - 1] == b'\n' {
        len -= 1;
    }
    input_buffer.truncate(len);

    let result =
        unsafe { driver::process_decisions(input_buffer.as_mut_ptr(), len, operation, param) };
    let _ = writeln!(io::stdout().lock(), "{result}");
}
