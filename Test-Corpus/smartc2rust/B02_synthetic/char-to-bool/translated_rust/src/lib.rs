
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[unsafe(no_mangle)]
pub extern "C" fn main_c_main() -> std::os::raw::c_int {
    // Stub implementation
    0
}



fn apply_permissions(read: bool, write: bool, execute: bool) -> i32 {
    let permission_value: i32 =
        i32::from(read) * 4 + i32::from(write) * 2 + i32::from(execute);

    match (read, write, execute) {
        (true, true, true) => 100 + permission_value,
        (true, true, false) if permission_value == 6 => 50 + permission_value,
        (true, true, false) => 0,
        (true, false, true) => 30 + permission_value,
        (false, true, true) => 20 + permission_value,
        (true, false, false) => 10 + permission_value,
        (false, true, false) => -10,
        (false, false, true) => -20,
        (false, false, false) => 0,
    }
}

fn configure_flags(decisions: &[bool]) -> i32 {
    let count = decisions.len();
    let special_count = decisions.iter().filter(|&&d| d).count();

    if special_count == 0 {
        return 0;
    }
    if special_count == count {
        return 1000 + count as i32;
    }
    if special_count == 1 {
        if let Some(idx) = decisions.iter().position(|&d| d) {
            return 100 + idx as i32;
        }
    }
    if count > 0 && special_count == count - 1 {
        if let Some(idx) = decisions.iter().position(|&d| !d) {
            return 200 + idx as i32;
        }
    }

    let alternating = decisions.windows(2).all(|w| w[0] != w[1]);
    if alternating && count > 1 {
        return 500 + special_count as i32;
    }
    // Preserve original behavior when count <= 1 (loop from 1..count doesn't execute,
    // so alternating remains true).
    if count <= 1 && alternating {
        return 500 + special_count as i32;
    }

    let mut max_consecutive: i32 = 0;
    let mut current_consecutive: i32 = 0;
    for &d in decisions {
        if d {
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

    special_count as i32
}

fn evaluate_conditions(cond1: bool, cond2: bool, cond3: bool, logic_op: i32) -> i32 {
    match logic_op {
        0 => {
            if cond1 && cond2 && cond3 {
                100
            } else if cond1 && cond2 {
                50
            } else if cond1 && cond3 {
                51
            } else if cond2 && cond3 {
                52
            } else if cond1 {
                10
            } else if cond2 {
                11
            } else if cond3 {
                12
            } else {
                0
            }
        }
        1 => {
            if cond1 || cond2 || cond3 {
                let count = i32::from(cond1) + i32::from(cond2) + i32::from(cond3);
                100 + count
            } else {
                0
            }
        }
        2 => {
            if cond1 ^ cond2 ^ cond3 {
                match (cond1, cond2, cond3) {
                    (true, false, false) => 1,
                    (false, true, false) => 2,
                    (false, false, true) => 3,
                    (true, true, true) => 7,
                    _ => 90,
                }
            } else {
                0
            }
        }
        3 => {
            if !(cond1 && cond2 && cond3) {
                if !cond1 && !cond2 && !cond3 {
                    200
                } else if !cond1 {
                    150
                } else if !cond2 {
                    151
                } else if !cond3 {
                    152
                } else {
                    100
                }
            } else {
                0
            }
        }
        _ => -1,
    }
}

fn parse_bool(c: u8) -> bool {
    matches!(c, b'y' | b'Y')
}

fn validate_sequence(sequence: &[u8]) -> i32 {
    let len = sequence.len();
    if len == 0 {
        return 0;
    }

    let bools: Vec<bool> = sequence.iter().map(|&c| parse_bool(c)).collect();

    if !bools[0] {
        return -10;
    }

    if len > 1 && bools[len - 1] {
        return -11;
    }

    let mut consecutive = 1;
    for w in bools.windows(2) {
        if w[0] == w[1] {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    let transitions = bools.windows(2).filter(|w| w[0] != w[1]).count() as i32;

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
    if decision_string.is_empty() {
        return -1;
    }

    match operation {
        0 => {
            if decision_string.len() < 3 {
                return -2;
            }
            let read = parse_bool(decision_string[0]);
            let write = parse_bool(decision_string[1]);
            let execute = parse_bool(decision_string[2]);
            apply_permissions(read, write, execute)
        }
        1 => {
            if decision_string.len() < 3 {
                return -2;
            }
            let cond1 = parse_bool(decision_string[0]);
            let cond2 = parse_bool(decision_string[1]);
            let cond3 = parse_bool(decision_string[2]);
            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            let count = decision_string.len().min(32);
            let decisions: Vec<bool> = decision_string[..count]
                .iter()
                .map(|&c| parse_bool(c))
                .collect();
            configure_flags(&decisions)
        }
        3 => validate_sequence(decision_string),
        _ => -3,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    use std::io::{BufRead, Write};

    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();

    let read_line = |lock: &mut std::io::StdinLock<'_>| -> Option<String> {
        let mut line = String::new();
        match lock.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line),
        }
    };

    let op_line = match read_line(&mut stdin_lock) {
        Some(l) => l,
        None => {
            eprintln!("Error reading operation");
            return 1;
        }
    };
    let operation: i32 = op_line.trim().parse().unwrap_or(0);

    let param_line = match read_line(&mut stdin_lock) {
        Some(l) => l,
        None => {
            eprintln!("Error reading parameter");
            return 1;
        }
    };
    let param: i32 = param_line.trim().parse().unwrap_or(0);

    let dec_line = match read_line(&mut stdin_lock) {
        Some(l) => l,
        None => {
            eprintln!("Error reading decision string");
            return 1;
        }
    };

    let trimmed = dec_line.trim_end_matches(|c| c == '\n' || c == '\r');
    let bytes = trimmed.as_bytes();

    let result = process_decisions(bytes, operation, param);

    println!("{}", result);
    let _ = std::io::stdout().flush();

    0
}
