use std::ffi::c_int;

fn parse_bool(value: u8) -> bool {
    value == b'y' || value == b'Y'
}

fn apply_permissions(read: bool, write: bool, execute: bool) -> c_int {
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

fn evaluate_conditions(cond1: bool, cond2: bool, cond3: bool, logic_op: c_int) -> c_int {
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

fn configure_flags(decisions: &[bool]) -> c_int {
    let count = decisions.len();
    let mut flags = 0_u32;
    let mut special_count = 0;

    for (index, &decision) in decisions.iter().enumerate() {
        if decision {
            flags |= 1_u32 << index;
            special_count += 1;
        }
    }
    let _ = flags;

    if special_count == 0 {
        return 0;
    } else if special_count as usize == count {
        return 1000 + count as c_int;
    } else if special_count == 1 {
        for (index, &decision) in decisions.iter().enumerate() {
            if decision {
                return 100 + index as c_int;
            }
        }
    } else if special_count as usize == count - 1 {
        for (index, &decision) in decisions.iter().enumerate() {
            if !decision {
                return 200 + index as c_int;
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

fn validate_sequence(sequence: &mut [u8]) -> c_int {
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
        10 + transitions as c_int
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

/// C ABI entry point matching `c_src/src/lib.c`.
///
/// # Safety
///
/// A non-null `decision_string` must point to storage valid for the bytes read
/// or written by the selected operation, under the same contract as the C API.
#[no_mangle]
pub unsafe extern "C" fn process_decisions(
    decision_string: *mut u8,
    length: usize,
    operation: c_int,
    param: c_int,
) -> c_int {
    if decision_string.is_null() || length == 0 {
        return -1;
    }

    match operation {
        0 => {
            if length < 3 {
                return -2;
            }

            let decisions = std::slice::from_raw_parts(decision_string, 3);
            apply_permissions(
                parse_bool(decisions[0]),
                parse_bool(decisions[1]),
                parse_bool(decisions[2]),
            )
        }
        1 => {
            if length < 3 {
                return -2;
            }

            let decisions = std::slice::from_raw_parts(decision_string, 3);
            evaluate_conditions(
                parse_bool(decisions[0]),
                parse_bool(decisions[1]),
                parse_bool(decisions[2]),
                param,
            )
        }
        2 => {
            let count = length.min(32);
            let bytes = std::slice::from_raw_parts(decision_string, count);
            let mut decisions = [false; 32];
            for index in 0..count {
                decisions[index] = parse_bool(bytes[index]);
            }
            configure_flags(&decisions[..count])
        }
        3 => {
            let sequence = std::slice::from_raw_parts_mut(decision_string, length);
            validate_sequence(sequence)
        }
        _ => -3,
    }
}
