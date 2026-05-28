// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust to produce byte-identical output.
//
// This module mirrors the C library in c_src/src/lib.c. The signatures
// have been adapted slightly to fit Rust's borrow checker, but the
// observable behavior — including return codes — matches the C
// implementation exactly.

/// Main entrance function - processes boolean decisions
///
/// * `decision_string` - byte slice of 'y'/'n' characters representing decisions
///   (note: must be mutable because validate_sequence in C reuses the buffer).
/// * `length` - length of the decision string. The C code passes this in
///   independently of the buffer length; we keep that pattern so the same
///   error checks fire in the same order.
/// * `operation` - operation to perform (0..=3).
/// * `param` - operation-specific parameter.
pub fn process_decisions(
    decision_string: &mut [u8],
    length: usize,
    operation: i32,
    param: i32,
) -> i32 {
    // The C code checks `decision_string == NULL || length == 0`.
    // In Rust the slice is always non-null, so we only mirror the length check.
    if length == 0 {
        return -1;
    }

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
            // Configure flags from all decisions (cap at 32)
            let mut decisions = [false; 32];
            let count = if length < 32 { length } else { 32 };
            for i in 0..count {
                decisions[i] = parse_bool(decision_string[i]);
            }
            configure_flags(&decisions, count)
        }
        3 => {
            // Validate decision sequence
            validate_sequence(decision_string, length)
        }
        _ => -3,
    }
}

/// Parse character to boolean.
/// 'y'/'Y' -> true, anything else -> false.
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

    // Match C control flow exactly (note: not all branches return).
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

/// Evaluate three conditions with different logic operators.
fn evaluate_conditions(cond1: bool, cond2: bool, cond3: bool, logic_op: i32) -> i32 {
    match logic_op {
        0 => {
            // AND - all must be true
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
            // OR - at least one must be true
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
            // XOR - odd number must be true
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
            // NAND - not all true
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
    } else if count > 0 && special_count as usize == count - 1 {
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

    // Longest consecutive run of `true`
    let mut max_consecutive: i32 = 0;
    let mut current: i32 = 0;
    for i in 0..count {
        if decisions[i] {
            current += 1;
            if current > max_consecutive {
                max_consecutive = current;
            }
        } else {
            current = 0;
        }
    }
    if max_consecutive >= 3 {
        return 300 + max_consecutive;
    }

    special_count
}

/// Validate sequence has proper pattern.
///
/// The C version reinterprets the input buffer as `bool*` and overwrites
/// each byte in place with 0 or 1 before reading subsequent bytes. Each
/// iteration reads index `i` *first*, so the in-place rewrite is safe.
/// We replicate this byte-write so we don't accidentally diverge if the
/// caller observed the buffer afterwards.
fn validate_sequence(sequence: &mut [u8], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    // Convert all to bools, overwriting each byte (sizeof(bool) == 1 in C).
    for i in 0..len {
        let val = parse_bool(sequence[i]);
        sequence[i] = if val { 1 } else { 0 };
    }
    let bools = |i: usize| -> bool { sequence[i] != 0 };

    if !bools(0) {
        return -10;
    }
    if len > 1 && bools(len - 1) {
        return -11;
    }

    let mut consecutive = 1;
    for i in 1..len {
        if bools(i) == bools(i - 1) {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    let mut transitions: i32 = 0;
    for i in 1..len {
        if bools(i) != bools(i - 1) {
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
