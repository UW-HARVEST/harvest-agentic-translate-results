use std::io::{self, Read, Write};

const MAX_INPUT_SIZE: usize = 1024;

fn c_fgets<R: Read>(reader: &mut R, max_size: usize) -> io::Result<Option<Vec<u8>>> {
    if max_size == 0 {
        return Ok(Some(Vec::new()));
    }

    let mut buffer = Vec::new();
    let mut byte = [0_u8; 1];

    while buffer.len() < max_size - 1 {
        match reader.read(&mut byte) {
            Ok(0) => {
                if buffer.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(buffer));
            }
            Ok(_) => {
                buffer.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(Some(buffer));
                }
            }
            Err(err) => return Err(err),
        }
    }

    Ok(Some(buffer))
}

fn c_atoi(bytes: &[u8]) -> i32 {
    let mut c_string = Vec::with_capacity(bytes.len() + 1);
    c_string.extend_from_slice(bytes);
    c_string.push(0);

    unsafe { libc::atoi(c_string.as_ptr().cast()) }
}

fn c_strlen(bytes: &[u8]) -> usize {
    let mut c_string = Vec::with_capacity(bytes.len() + 1);
    c_string.extend_from_slice(bytes);
    c_string.push(0);

    unsafe { libc::strlen(c_string.as_ptr().cast()) }
}

fn parse_bool(c: u8) -> bool {
    if c == b'y' || c == b'Y' {
        true
    } else if c == b'n' || c == b'N' {
        false
    } else {
        false
    }
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
        return 100 + permission_value;
    } else if read && write {
        if permission_value == 6 {
            return 50 + permission_value;
        }
    } else if read && execute {
        return 30 + permission_value;
    } else if write && execute {
        return 20 + permission_value;
    } else if read {
        return 10 + permission_value;
    } else if write {
        return -10;
    } else if execute {
        return -20;
    }

    0
}

fn evaluate_conditions(cond1: bool, cond2: bool, cond3: bool, logic_op: i32) -> i32 {
    let result;

    match logic_op {
        0 => {
            result = cond1 && cond2 && cond3;

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
            result = cond1 || cond2 || cond3;

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
                100 + count
            } else {
                0
            }
        }
        2 => {
            result = cond1 ^ cond2 ^ cond3;

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
                90
            } else {
                0
            }
        }
        3 => {
            result = !(cond1 && cond2 && cond3);

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
                100
            } else {
                0
            }
        }
        _ => -1,
    }
}

fn configure_flags(decisions: &[bool], count: usize) -> i32 {
    let mut _flags: u32 = 0;
    let mut special_count: i32 = 0;

    for i in 0..count.min(32) {
        if decisions[i] {
            _flags |= 1_u32 << i;
            special_count += 1;
        }
    }

    if special_count == 0 {
        return 0;
    } else if special_count == count as i32 {
        return 1000 + count as i32;
    } else if special_count == 1 {
        for (i, decision) in decisions.iter().take(count).enumerate() {
            if *decision {
                return 100 + i as i32;
            }
        }
    } else if special_count == count.wrapping_sub(1) as i32 {
        for (i, decision) in decisions.iter().take(count).enumerate() {
            if !*decision {
                return 200 + i as i32;
            }
        }
    }

    let mut alternating = true;
    for i in 1..count {
        if decisions[i] == decisions[i - 1] {
            alternating = false;
            break;
        }
    }

    if alternating {
        return 500 + special_count;
    }

    let mut max_consecutive: i32 = 0;
    let mut current_consecutive: i32 = 0;
    for decision in decisions.iter().take(count) {
        if *decision {
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

fn validate_sequence(sequence: &mut [u8], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    let mut bools = Vec::with_capacity(len);
    for byte in sequence.iter_mut().take(len) {
        let val = parse_bool(*byte);
        *byte = if val { 1 } else { 0 };
        bools.push(val);
    }

    if !bools[0] {
        return -10;
    }

    if len > 1 && bools[len - 1] {
        return -11;
    }

    let mut consecutive = 1;
    for i in 1..len {
        if bools[i] == bools[i - 1] {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    let mut transitions = 0;
    for i in 1..len {
        if bools[i] != bools[i - 1] {
            transitions += 1;
        }
    }

    if len <= 3 {
        if transitions == 0 {
            return 1;
        }
        if transitions == (len - 1) as i32 {
            return 2;
        }
        10 + transitions
    } else if len <= 10 {
        if transitions < (len / 3) as i32 {
            return 20;
        }
        if transitions > (len / 2) as i32 {
            return 30;
        }
        25
    } else {
        if transitions < 3 {
            return 40;
        }
        if transitions > (len as i32) - 3 {
            return 50;
        }
        45
    }
}

fn process_decisions(
    decision_string: Option<&mut [u8]>,
    length: usize,
    operation: i32,
    param: i32,
) -> i32 {
    if decision_string.is_none() || length == 0 {
        return -1;
    }

    let decision_string = decision_string.unwrap();

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
            let mut decisions = [false; 32];
            let count = length.min(32);

            for i in 0..count {
                decisions[i] = parse_bool(decision_string[i]);
            }

            configure_flags(&decisions, count)
        }
        3 => validate_sequence(decision_string, length),
        _ => -3,
    }
}

fn read_required_line<R: Read>(
    reader: &mut R,
    error_message: &[u8],
) -> Result<Vec<u8>, i32> {
    match c_fgets(reader, MAX_INPUT_SIZE) {
        Ok(Some(line)) => Ok(line),
        Ok(None) | Err(_) => {
            let _ = io::stderr().write_all(error_message);
            Err(1)
        }
    }
}

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    let operation_line = match read_required_line(&mut reader, b"Error reading operation\n") {
        Ok(line) => line,
        Err(code) => std::process::exit(code),
    };
    let operation = c_atoi(&operation_line);

    let param_line = match read_required_line(&mut reader, b"Error reading parameter\n") {
        Ok(line) => line,
        Err(code) => std::process::exit(code),
    };
    let param = c_atoi(&param_line);

    let mut input_buffer =
        match read_required_line(&mut reader, b"Error reading decision string\n") {
            Ok(line) => line,
            Err(code) => std::process::exit(code),
        };

    let mut len = c_strlen(&input_buffer);
    if len > 0 && input_buffer[len - 1] == b'\n' {
        input_buffer[len - 1] = 0;
        len -= 1;
    }

    let result = process_decisions(Some(&mut input_buffer), len, operation, param);
    println!("{result}");
}
