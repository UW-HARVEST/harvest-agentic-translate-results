use std::io::{self, Read, Write};

const MAX_INPUT_SIZE: usize = 1024;

fn main() {
    let mut stdin = Vec::new();
    io::stdin().read_to_end(&mut stdin).unwrap();
    let mut cursor = 0usize;

    let Some(mut input_buffer) = fgets_like(&stdin, &mut cursor) else {
        let _ = io::stderr().write_all(b"Error reading operation\n");
        std::process::exit(1);
    };
    let operation = atoi_like(&input_buffer);

    let Some(buffer) = fgets_like(&stdin, &mut cursor) else {
        let _ = io::stderr().write_all(b"Error reading parameter\n");
        std::process::exit(1);
    };
    input_buffer = buffer;
    let param = atoi_like(&input_buffer);

    let Some(buffer) = fgets_like(&stdin, &mut cursor) else {
        let _ = io::stderr().write_all(b"Error reading decision string\n");
        std::process::exit(1);
    };
    input_buffer = buffer;

    let mut len = strlen_like(&input_buffer);
    if len > 0 && input_buffer[len - 1] == b'\n' {
        input_buffer[len - 1] = 0;
        len -= 1;
    }

    let result = process_decisions(&mut input_buffer, len, operation, param);
    println!("{}", result);
}

fn fgets_like(input: &[u8], cursor: &mut usize) -> Option<Vec<u8>> {
    if *cursor >= input.len() {
        return None;
    }

    let mut buffer = Vec::with_capacity(MAX_INPUT_SIZE);
    let limit = MAX_INPUT_SIZE - 1;
    while *cursor < input.len() && buffer.len() < limit {
        let byte = input[*cursor];
        *cursor += 1;
        buffer.push(byte);
        if byte == b'\n' {
            break;
        }
    }
    buffer.push(0);
    Some(buffer)
}

fn strlen_like(buffer: &[u8]) -> usize {
    buffer.iter().position(|&byte| byte == 0).unwrap_or(buffer.len())
}

fn atoi_like(buffer: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < buffer.len() {
        match buffer[i] {
            0 => return 0,
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => i += 1,
            _ => break,
        }
    }

    let mut sign = 1i32;
    if i < buffer.len() {
        if buffer[i] == b'-' {
            sign = -1;
            i += 1;
        } else if buffer[i] == b'+' {
            i += 1;
        }
    }

    let mut value = 0i32;
    while i < buffer.len() {
        let byte = buffer[i];
        if !byte.is_ascii_digit() {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add((byte - b'0') as i32);
        i += 1;
    }

    value.wrapping_mul(sign)
}

fn process_decisions(
    decision_string: &mut [u8],
    length: usize,
    operation: i32,
    param: i32,
) -> i32 {
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

            for i in 0..count {
                decisions[i] = parse_bool(decision_string[i]);
            }

            configure_flags(&decisions, count)
        }
        3 => validate_sequence(decision_string, length),
        _ => -3,
    }
}

fn parse_bool(c: u8) -> bool {
    c == b'y' || c == b'Y'
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
        0
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

fn configure_flags(decisions: &[bool; 32], count: usize) -> i32 {
    let mut _flags = 0u32;
    let mut special_count = 0i32;

    for i in 0..count.min(32) {
        if decisions[i] {
            _flags |= 1u32 << i;
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
    } else if special_count == count as i32 - 1 {
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

    let mut max_consecutive = 0;
    let mut current_consecutive = 0;
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

    for i in 0..len {
        let val = parse_bool(sequence[i]);
        sequence[i] = u8::from(val);
    }

    if sequence[0] == 0 {
        return -10;
    }

    if len > 1 && sequence[len - 1] != 0 {
        return -11;
    }

    let mut consecutive = 1;
    for i in 1..len {
        if sequence[i] == sequence[i - 1] {
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
        if sequence[i] != sequence[i - 1] {
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
