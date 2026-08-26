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

    while index < bytes.len()
        && matches!(bytes[index], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
    {
        index += 1;
    }

    let negative = if index < bytes.len() && (bytes[index] == b'+' || bytes[index] == b'-')
    {
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

fn parse_bool(value: u8) -> bool {
    value == b'y' || value == b'Y'
}

fn apply_permissions(read: bool, write: bool, execute: bool) -> i32 {
    let mut permission_value = 0;

    if read {
        permission_value += 4;
    }
    if write {
        permission_value += 2;
    }
    if execute {
        permission_value += 1;
    }

    if read && write && execute {
        100 + permission_value
    } else if read && write {
        if permission_value == 6 {
            return 50 + permission_value;
        }
        permission_value
    } else if read && execute {
        30 + permission_value
    } else if write && execute {
        20 + permission_value
    } else if read {
        10 + permission_value
    } else if write {
        -10
    } else if execute {
        -20
    } else {
        0
    }
}

fn evaluate_conditions(cond1: bool, cond2: bool, cond3: bool, logic_op: i32) -> i32 {
    match logic_op {
        0 => {
            let result = cond1 && cond2 && cond3;

            if result {
                100
            } else {
                if cond1 && cond2 {
                    return 50;
                }
                if cond1 && cond3 {
                    return 51;
                }
                if cond2 && cond3 {
                    return 52;
                }
                if cond1 {
                    return 10;
                }
                if cond2 {
                    return 11;
                }
                if cond3 {
                    return 12;
                }
                0
            }
        }
        1 => {
            let result = cond1 || cond2 || cond3;

            if result {
                let mut count = 0;
                if cond1 {
                    count += 1;
                }
                if cond2 {
                    count += 1;
                }
                if cond3 {
                    count += 1;
                }
                return 100 + count;
            }
            0
        }
        2 => {
            let result = cond1 ^ cond2 ^ cond3;

            if result {
                if cond1 && !cond2 && !cond3 {
                    return 1;
                }
                if !cond1 && cond2 && !cond3 {
                    return 2;
                }
                if !cond1 && !cond2 && cond3 {
                    return 3;
                }
                if cond1 && cond2 && cond3 {
                    return 7;
                }
                return 90;
            }
            0
        }
        3 => {
            let result = !(cond1 && cond2 && cond3);

            if result {
                if !cond1 && !cond2 && !cond3 {
                    return 200;
                }
                if !cond1 {
                    return 150;
                }
                if !cond2 {
                    return 151;
                }
                if !cond3 {
                    return 152;
                }
                return 100;
            }
            0
        }
        _ => -1,
    }
}

fn configure_flags(decisions: &[bool]) -> i32 {
    let count = decisions.len();
    let mut _flags = 0_u32;
    let mut special_count = 0_i32;

    for (index, &decision) in decisions.iter().take(32).enumerate() {
        if decision {
            _flags |= 1_u32 << index;
            special_count += 1;
        }
    }

    if special_count == 0 {
        return 0;
    } else if special_count as usize == count {
        return 1000 + count as i32;
    } else if special_count == 1 {
        for (index, &decision) in decisions.iter().enumerate() {
            if decision {
                return 100 + index as i32;
            }
        }
    } else if special_count as usize == count - 1 {
        for (index, &decision) in decisions.iter().enumerate() {
            if !decision {
                return 200 + index as i32;
            }
        }
    }

    let mut alternating = true;
    for index in 1..count {
        if decisions[index] == decisions[index - 1] {
            alternating = false;
            break;
        }
    }

    if alternating {
        return 500 + special_count;
    }

    let mut max_consecutive = 0;
    let mut current_consecutive = 0;
    for &decision in decisions {
        if decision {
            current_consecutive += 1;
            if current_consecutive > max_consecutive {
                max_consecutive = current_consecutive;
            }
        } else {
            current_consecutive = 0;
        }
    }

    if max_consecutive >= 3 {
        return 300 + max_consecutive;
    }

    special_count
}

fn validate_sequence(sequence: &mut [u8]) -> i32 {
    let len = sequence.len();
    if len == 0 {
        return 0;
    }

    for value in sequence.iter_mut() {
        *value = u8::from(parse_bool(*value));
    }

    if sequence[0] == 0 {
        return -10;
    }

    if len > 1 && sequence[len - 1] != 0 {
        return -11;
    }

    let mut consecutive = 1;
    for index in 1..len {
        if sequence[index] == sequence[index - 1] {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    let mut transitions = 0_usize;
    for index in 1..len {
        if sequence[index] != sequence[index - 1] {
            transitions += 1;
        }
    }

    if len <= 3 {
        if transitions == 0 {
            return 1;
        }
        if transitions == len - 1 {
            return 2;
        }
        10 + transitions as i32
    } else if len <= 10 {
        if transitions < len / 3 {
            return 20;
        }
        if transitions > len / 2 {
            return 30;
        }
        25
    } else {
        if transitions < 3 {
            return 40;
        }
        if transitions > len - 3 {
            return 50;
        }
        45
    }
}

fn process_decisions(
    decision_string: &mut [u8],
    operation: i32,
    param: i32,
) -> i32 {
    let length = decision_string.len();
    if length == 0 {
        return -1;
    }

    match operation {
        0 => {
            if length < 3 {
                return -2;
            }

            let read = parse_bool(decision_string[0]);
            let write = parse_bool(decision_string[1]);
            let execute = parse_bool(decision_string[2]);
            apply_permissions(read, write, execute)
        }
        1 => {
            if length < 3 {
                return -2;
            }

            let cond1 = parse_bool(decision_string[0]);
            let cond2 = parse_bool(decision_string[1]);
            let cond3 = parse_bool(decision_string[2]);
            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            let count = length.min(32);
            let mut decisions = [false; 32];

            for index in 0..count {
                decisions[index] = parse_bool(decision_string[index]);
            }

            configure_flags(&decisions[..count])
        }
        3 => validate_sequence(decision_string),
        _ => -3,
    }
}

fn read_required_line<R: Read>(
    reader: &mut R,
    buffer: &mut Vec<u8>,
    error_message: &str,
) -> bool {
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

    let result = process_decisions(&mut input_buffer, operation, param);
    let _ = writeln!(io::stdout().lock(), "{result}");
}
