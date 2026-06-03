// Translation of c_src/src/main.c and c_src/src/lib.c to Rust.
// Goal: byte-identical output for the same inputs.

use std::io::{self, Read, Write};
use std::process::ExitCode;

const MAX_INPUT_SIZE: usize = 1024;

/// Mimic C's fgets: read up to (max_size - 1) bytes from stdin, stopping at
/// a newline (which is kept) or EOF. Returns None if EOF and nothing read,
/// otherwise returns the bytes read (not including a terminating NUL, but
/// including a trailing newline if encountered).
fn fgets(max_size: usize) -> Option<Vec<u8>> {
    let mut stdin = io::stdin();
    let mut buf: Vec<u8> = Vec::new();
    let limit = max_size.saturating_sub(1);
    let mut byte = [0u8; 1];
    while buf.len() < limit {
        match stdin.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Mimic C's atoi: skip leading whitespace, optional sign, parse digits,
/// stop at first non-digit. Return 0 if no digits.  Wraps on overflow
/// (matches typical C `atoi` undefined-but-common behavior — sufficient
/// for the inputs here).
fn atoi(bytes: &[u8]) -> i32 {
    let mut i = 0;
    // Skip leading whitespace (matching isspace: space, \t, \n, \v, \f, \r).
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
            _ => break,
        }
    }
    let mut sign: i32 = 1;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            sign = -1;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
        }
    }
    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i32;
        result = result.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    result.wrapping_mul(sign)
}

/// Parse character to boolean.
/// 'y'/'Y' -> true; 'n'/'N' -> false; anything else -> false.
fn parse_bool(c: u8) -> bool {
    matches!(c, b'y' | b'Y')
}

fn apply_permissions(read: bool, write: bool, execute: bool) -> i32 {
    let mut permission_value: i32 = 0;
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
        return 100 + permission_value; // 107
    } else if read && write {
        if permission_value == 6 {
            return 50 + permission_value; // 56
        }
    } else if read && execute {
        return 30 + permission_value; // 35
    } else if write && execute {
        return 20 + permission_value; // 23
    } else if read {
        return 10 + permission_value; // 14
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
            // AND
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
            // OR
            let result = cond1 || cond2 || cond3;
            if result {
                let mut count = 0i32;
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
            // XOR (odd count true)
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
                90
            } else {
                0
            }
        }
        3 => {
            // NAND
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

    let mut i = 0usize;
    while i < count && i < 32 {
        if decisions[i] {
            _flags |= 1u32 << i;
            special_count += 1;
        }
        i += 1;
    }

    if special_count == 0 {
        return 0;
    } else if (special_count as usize) == count {
        return 1000 + count as i32;
    } else if special_count == 1 {
        for i in 0..count {
            if decisions[i] {
                return 100 + i as i32;
            }
        }
    } else if (special_count as usize) == count - 1 {
        for i in 0..count {
            if !decisions[i] {
                return 200 + i as i32;
            }
        }
    }

    // Alternating pattern
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

    // Consecutive trues
    let mut max_consecutive: i32 = 0;
    let mut current_consecutive: i32 = 0;
    for i in 0..count {
        if decisions[i] {
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

    // Reuse buffer: convert each char to bool (0/1).
    // The C code aliases the char* as bool*; sizeof(bool) == 1 on common
    // platforms, so values are stored as raw 0/1 bytes.
    for i in 0..len {
        let val = parse_bool(sequence[i]);
        sequence[i] = if val { 1 } else { 0 };
    }
    let bools = &sequence[..len];

    // Rule 1: must start with 'y'
    if bools[0] == 0 {
        return -10;
    }

    // Rule 2: must end with 'n' if length > 1
    if len > 1 && bools[len - 1] != 0 {
        return -11;
    }

    // Rule 3: no more than 3 consecutive same values
    let mut consecutive = 1i32;
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

    // Rule 4: count transitions
    let mut transitions = 0i32;
    for i in 1..len {
        if bools[i] != bools[i - 1] {
            transitions += 1;
        }
    }

    let len_i = len as i32;
    if len <= 3 {
        if transitions == 0 {
            return 1;
        }
        if transitions == len_i - 1 {
            return 2;
        }
        10 + transitions
    } else if len <= 10 {
        if transitions < len_i / 3 {
            return 20;
        }
        if transitions > len_i / 2 {
            return 30;
        }
        25
    } else {
        if transitions < 3 {
            return 40;
        }
        if transitions > len_i - 3 {
            return 50;
        }
        45
    }
}

fn process_decisions(decision_string: &mut [u8], length: usize, operation: i32, param: i32) -> i32 {
    if length == 0 {
        return -1;
    }

    match operation {
        0 => {
            if length < 3 {
                return -2;
            }
            let r = parse_bool(decision_string[0]);
            let w = parse_bool(decision_string[1]);
            let x = parse_bool(decision_string[2]);
            apply_permissions(r, w, x)
        }
        1 => {
            if length < 3 {
                return -2;
            }
            let c1 = parse_bool(decision_string[0]);
            let c2 = parse_bool(decision_string[1]);
            let c3 = parse_bool(decision_string[2]);
            evaluate_conditions(c1, c2, c3, param)
        }
        2 => {
            let mut decisions = [false; 32];
            let count = if length < 32 { length } else { 32 };
            for i in 0..count {
                decisions[i] = parse_bool(decision_string[i]);
            }
            configure_flags(&decisions, count)
        }
        3 => validate_sequence(decision_string, length),
        _ => -3,
    }
}

fn main() -> ExitCode {
    // Read operation number
    let buf = match fgets(MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(io::stderr(), "Error reading operation");
            return ExitCode::from(1);
        }
    };
    let operation = atoi(&buf);

    // Read parameter
    let buf = match fgets(MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(io::stderr(), "Error reading parameter");
            return ExitCode::from(1);
        }
    };
    let param = atoi(&buf);

    // Read decision string
    let mut input = match fgets(MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            let _ = writeln!(io::stderr(), "Error reading decision string");
            return ExitCode::from(1);
        }
    };

    // Strip trailing newline if present
    let mut len = input.len();
    if len > 0 && input[len - 1] == b'\n' {
        input[len - 1] = 0;
        len -= 1;
    }

    let result = process_decisions(&mut input, len, operation, param);

    // Print result followed by newline
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", result);
    let _ = out.flush();

    ExitCode::from(0)
}
