use std::io::{self, Read, Write};

const MAX_INPUT_SIZE: usize = 1024;

fn read_fgets<R: Read>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut input = Vec::with_capacity(MAX_INPUT_SIZE - 1);
    let mut byte = [0_u8; 1];

    while input.len() < MAX_INPUT_SIZE - 1 {
        match reader.read(&mut byte)? {
            0 => break,
            _ => {
                input.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
        }
    }

    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

fn c_atoi(input: &[u8]) -> i32 {
    let mut index = 0;
    while index < input.len() && matches!(input[index], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        index += 1;
    }

    let negative = if input.get(index) == Some(&b'-') {
        index += 1;
        true
    } else {
        if input.get(index) == Some(&b'+') {
            index += 1;
        }
        false
    };

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value = 0_u64;

    while let Some(&byte) = input.get(index) {
        if !byte.is_ascii_digit() {
            break;
        }

        value = value
            .checked_mul(10)
            .and_then(|current| current.checked_add((byte - b'0') as u64))
            .unwrap_or(limit)
            .min(limit);
        index += 1;
    }

    let signed = if negative {
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
    let mut flags = 0_u32;
    let mut special_count = 0_i32;

    for (index, &decision) in decisions.iter().enumerate().take(32) {
        if decision {
            flags |= 1_u32 << index;
            special_count += 1;
        }
    }
    let _ = flags;

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

fn validate_sequence(sequence: &[u8]) -> i32 {
    let len = sequence.len();
    if len == 0 {
        return 0;
    }

    let bools: Vec<bool> = sequence.iter().copied().map(parse_bool).collect();

    if !bools[0] {
        return -10;
    }

    if len > 1 && bools[len - 1] {
        return -11;
    }

    let mut consecutive = 1;
    for index in 1..len {
        if bools[index] == bools[index - 1] {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    let mut transitions = 0;
    for index in 1..len {
        if bools[index] != bools[index - 1] {
            transitions += 1;
        }
    }

    if len <= 3 {
        if transitions == 0 {
            return 1;
        }
        if transitions as usize == len - 1 {
            return 2;
        }
        10 + transitions
    } else if len <= 10 {
        if (transitions as usize) < len / 3 {
            return 20;
        }
        if (transitions as usize) > len / 2 {
            return 30;
        }
        25
    } else {
        if transitions < 3 {
            return 40;
        }
        if (transitions as usize) > len - 3 {
            return 50;
        }
        45
    }
}

fn process_decisions(decision_string: &[u8], operation: i32, param: i32) -> i32 {
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
            let decisions: Vec<bool> = decision_string[..count]
                .iter()
                .copied()
                .map(parse_bool)
                .collect();
            configure_flags(&decisions)
        }
        3 => validate_sequence(decision_string),
        _ => -3,
    }
}

fn write_error(message: &[u8]) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(message);
}

fn main() {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let operation_input = match read_fgets(&mut stdin) {
        Ok(Some(input)) => input,
        _ => {
            write_error(b"Error reading operation\n");
            std::process::exit(1);
        }
    };
    let operation_end = operation_input
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(operation_input.len());
    let operation = c_atoi(&operation_input[..operation_end]);

    let param_input = match read_fgets(&mut stdin) {
        Ok(Some(input)) => input,
        _ => {
            write_error(b"Error reading parameter\n");
            std::process::exit(1);
        }
    };
    let param_end = param_input
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(param_input.len());
    let param = c_atoi(&param_input[..param_end]);

    let decision_input = match read_fgets(&mut stdin) {
        Ok(Some(input)) => input,
        _ => {
            write_error(b"Error reading decision string\n");
            std::process::exit(1);
        }
    };
    let mut decision_end = decision_input
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(decision_input.len());
    if decision_end > 0 && decision_input[decision_end - 1] == b'\n' {
        decision_end -= 1;
    }

    let result = process_decisions(&decision_input[..decision_end], operation, param);
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{result}");
}
