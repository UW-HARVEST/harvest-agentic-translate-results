//! Translation of c_src/src/lib.c — preserves original behavior exactly.

/// Parse a single ASCII character into a bool, matching C's parse_bool.
fn parse_bool(c: u8) -> bool {
    matches!(c, b'y' | b'Y')
}

/// Apply permissions based on read/write/execute flags.
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
        // falls through to return 0 below
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

/// Evaluate three conditions with different logic operators.
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
            // XOR (matches C's bitwise ^ on bools, which yields 0/1)
            let result = (cond1 as i32) ^ (cond2 as i32) ^ (cond3 as i32) != 0;
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

/// Configure system flags based on array of decisions.
fn configure_flags(decisions: &[bool], count: usize) -> i32 {
    let mut _flags: u32 = 0;
    let mut special_count: i32 = 0;

    let cap = count.min(32);
    for i in 0..cap {
        if decisions[i] {
            _flags |= 1u32 << i;
            special_count += 1;
        }
    }

    if special_count == 0 {
        return 0;
    } else if special_count as usize == count {
        return 1000 + count as i32;
    } else if special_count == 1 {
        for i in 0..count {
            if decisions[i] {
                return 100 + i as i32;
            }
        }
    } else if count >= 1 && special_count as usize == count - 1 {
        for i in 0..count {
            if !decisions[i] {
                return 200 + i as i32;
            }
        }
    }

    // Check for alternating pattern
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

    // Check for consecutive true values
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

/// Validate sequence has proper pattern.
///
/// Mirrors the C version which reuses the input buffer to store bool values.
fn validate_sequence(sequence: &mut [u8], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    // Convert all characters to bools in-place (just like the C code).
    for i in 0..len {
        let v = parse_bool(sequence[i]);
        sequence[i] = if v { 1 } else { 0 };
    }
    let bools = &sequence[..len];

    // Rule 1: must start with 'y'
    if bools[0] == 0 {
        return -10;
    }

    // Rule 2: if length > 1, must end with 'n'
    if len > 1 && bools[len - 1] != 0 {
        return -11;
    }

    // Rule 3: cannot have more than 3 consecutive same values
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

    // Rule 4: count transitions
    let mut transitions: i32 = 0;
    for i in 1..len {
        if bools[i] != bools[i - 1] {
            transitions += 1;
        }
    }

    if len <= 3 {
        if transitions == 0 {
            return 1;
        }
        if transitions == (len as i32) - 1 {
            return 2;
        }
        10 + transitions
    } else if len <= 10 {
        if transitions < (len as i32) / 3 {
            return 20;
        }
        if transitions > (len as i32) / 2 {
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

/// Main entry point — process a decision string with the given operation/param.
pub fn process_decisions(
    decision_string: &mut [u8],
    length: usize,
    operation: i32,
    param: i32,
) -> i32 {
    if decision_string.is_empty() || length == 0 {
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
