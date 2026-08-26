// Translation of c_src/lib.c

/// Main entrance function - processes boolean decisions.
///
/// `decision_string` is a mutable byte slice (matching C's `char *`); the
/// `validate_sequence` operation overwrites the buffer with boolean values,
/// matching the behaviour of the original C code.
pub fn process_decisions(decision_string: &mut [u8], operation: i32, param: i32) -> i32 {
    // Original C: if (decision_string == NULL || length == 0) return -1;
    // We can't have a NULL slice in safe Rust; only check length.
    if decision_string.is_empty() {
        return -1;
    }
    let length = decision_string.len();

    match operation {
        0 => {
            // Apply permissions: read, write, execute
            if length < 3 {
                return -2;
            }
            let read = parse_bool(decision_string[0]);
            let write = parse_bool(decision_string[1]);
            let execute = parse_bool(decision_string[2]);

            apply_permissions(read, write, execute)
        }
        1 => {
            // Evaluate logical conditions
            if length < 3 {
                return -2;
            }
            let cond1 = parse_bool(decision_string[0]);
            let cond2 = parse_bool(decision_string[1]);
            let cond3 = parse_bool(decision_string[2]);

            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            // Configure flags from all decisions
            let mut decisions = [false; 32];
            let count = if length < 32 { length } else { 32 };
            for i in 0..count {
                decisions[i] = parse_bool(decision_string[i]);
            }
            configure_flags(&decisions[..count])
        }
        3 => {
            // Validate decision sequence
            validate_sequence(decision_string)
        }
        _ => -3,
    }
}

/// Parse character to boolean.
/// 'y' or 'Y' -> true; 'n' or 'N' -> false; anything else -> false.
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
            // XOR
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

/// Configure system flags based on array of decisions.
fn configure_flags(decisions: &[bool]) -> i32 {
    let count = decisions.len();
    let mut flags: u32 = 0;
    let mut special_count: i32 = 0;

    let limit = if count < 32 { count } else { 32 };
    for i in 0..limit {
        if decisions[i] {
            flags |= 1u32 << i;
            special_count += 1;
        }
    }
    // Suppress unused warning - flags is computed identically to C even if unused.
    let _ = flags;

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
/// The original C code reuses the input buffer as a `bool *` to store the
/// converted boolean values. We do the same conceptually here, mutating the
/// buffer in place so that bools[i] = 1 for true / 0 for false.
fn validate_sequence(sequence: &mut [u8]) -> i32 {
    let len = sequence.len();
    if len == 0 {
        return 0;
    }

    // Convert all to bools (stored as 0/1 bytes, matching C's bool layout).
    for i in 0..len {
        let val = parse_bool(sequence[i]);
        sequence[i] = if val { 1 } else { 0 };
    }

    // Read back as booleans for clarity.
    let bool_at = |i: usize| -> bool { sequence[i] != 0 };

    // Rule 1: Must start with 'y'
    if !bool_at(0) {
        return -10;
    }

    // Rule 2: Must end with 'n' if length > 1
    if len > 1 && bool_at(len - 1) {
        return -11;
    }

    // Rule 3: Cannot have more than 3 consecutive same values
    let mut consecutive: i32 = 1;
    for i in 1..len {
        if bool_at(i) == bool_at(i - 1) {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    // Rule 4: Count transitions
    let mut transitions: i32 = 0;
    for i in 1..len {
        if bool_at(i) != bool_at(i - 1) {
            transitions += 1;
        }
    }

    // Validate based on length
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
