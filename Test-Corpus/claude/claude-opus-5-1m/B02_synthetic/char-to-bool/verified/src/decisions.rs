/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Faithful Rust translation of `c_src/src/lib.c`.

/// Main entrance function - processes boolean decisions
///
/// * `decision_string` - String of 'y'/'n' characters representing decisions.
///   The C code takes a plain `char *` alongside an explicit `length`, so the
///   slice only has to cover the bytes the selected `operation` reads; it is
///   deliberately taken by `&mut` because operation 3 rewrites it in place,
///   exactly like the C original.
/// * `length` - Length of decision string
/// * `operation` - Operation to perform:
///   - 0: apply permissions (uses first 3 decisions)
///   - 1: evaluate conditions with logic (uses first 3 decisions)
///   - 2: configure flags (uses all decisions)
///   - 3: validate sequence (checks pattern)
/// * `param` - Operation-specific parameter (logic operator, mode, etc)
///
/// Returns the operation result or an error code.
pub fn process_decisions(
    decision_string: &mut [u8],
    length: usize,
    operation: i32,
    param: i32,
) -> i32 {
    // The C code checks `decision_string == NULL || length == 0`.  The NULL
    // half of that test lives in the `extern "C"` wrapper in `lib.rs`, since a
    // Rust slice reference can never be NULL.
    if length == 0 {
        return -1;
    }

    match operation {
        0 => {
            /* Apply permissions: read, write, execute */
            if length < 3 {
                return -2;
            }

            let read = parse_bool(decision_string[0]);
            let write = parse_bool(decision_string[1]);
            let execute = parse_bool(decision_string[2]);

            apply_permissions(read, write, execute)
        }

        1 => {
            /* Evaluate logical conditions */
            if length < 3 {
                return -2;
            }

            let cond1 = parse_bool(decision_string[0]);
            let cond2 = parse_bool(decision_string[1]);
            let cond3 = parse_bool(decision_string[2]);

            /* param determines logic operation: 0=AND, 1=OR, 2=XOR, 3=NAND */
            evaluate_conditions(cond1, cond2, cond3, param)
        }

        2 => {
            /* Configure flags from all decisions */
            let mut decisions = [false; 32];
            let count = if length < 32 { length } else { 32 };

            for i in 0..count {
                decisions[i] = parse_bool(decision_string[i]);
            }

            configure_flags(&decisions[..count], count)
        }

        3 => {
            /* Validate decision sequence */
            validate_sequence(decision_string, length)
        }

        _ => -3,
    }
}

/// Parse character to boolean
/// 'y' or 'Y' -> true
/// 'n' or 'N' -> false
/// anything else -> false
fn parse_bool(c: u8) -> bool {
    if c == b'y' || c == b'Y' {
        true
    } else if c == b'n' || c == b'N' {
        false
    } else {
        false /* Default to false for invalid input */
    }
}

/// Apply permissions based on read/write/execute flags
/// Complex branching based on boolean combinations
fn apply_permissions(read: bool, write: bool, execute: bool) -> i32 {
    let mut permission_value: i32 = 0;

    /* Base permission calculation */
    if read {
        permission_value += 4;
    }
    if write {
        permission_value += 2;
    }
    if execute {
        permission_value += 1;
    }

    /* Complex decision tree based on combinations */
    if read && write && execute {
        /* Full permissions - special handling */
        return 100 + permission_value; /* 107 */
    } else if read && write {
        /* Read/write but no execute */
        if permission_value == 6 {
            return 50 + permission_value; /* 56 */
        }
    } else if read && execute {
        /* Read/execute but no write */
        return 30 + permission_value; /* 35 */
    } else if write && execute {
        /* Write/execute but no read - unusual case */
        return 20 + permission_value; /* 23 */
    } else if read {
        /* Read only */
        return 10 + permission_value; /* 14 */
    } else if write {
        /* Write only - dangerous */
        return -10;
    } else if execute {
        /* Execute only - very unusual */
        return -20;
    }

    /* No permissions */
    0
}

/// Evaluate three conditions with different logic operators
fn evaluate_conditions(cond1: bool, cond2: bool, cond3: bool, logic_op: i32) -> i32 {
    match logic_op {
        0 => {
            /* AND - all must be true */
            let result = cond1 && cond2 && cond3;

            if result {
                100
            } else {
                /* Partial matches */
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
            /* OR - at least one must be true */
            let result = cond1 || cond2 || cond3;

            if result {
                /* Count how many are true */
                let mut count: i32 = 0;
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
            /* XOR - odd number must be true */
            let result = cond1 ^ cond2 ^ cond3;

            if result {
                /* Determine which combination */
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
            /* NAND - not all true */
            let result = !(cond1 && cond2 && cond3);

            if result {
                /* Various false combinations */
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

/// Configure system flags based on array of decisions
/// Creates bitmask and applies various rules
fn configure_flags(decisions: &[bool], count: usize) -> i32 {
    let mut flags: u32 = 0;
    let mut special_count: i32 = 0;

    /* Build flag bitmask */
    let mut i = 0usize;
    while i < count && i < 32 {
        if decisions[i] {
            flags |= 1u32 << i;
            special_count += 1;
        }
        i += 1;
    }
    /* `flags` is computed but never used by the C code either. */
    let _ = flags;

    /* Apply rules based on flag patterns */
    if special_count == 0 {
        /* All false */
        return 0;
    } else if special_count as usize == count {
        /* All true */
        return 1000 + count as i32;
    } else if special_count == 1 {
        /* Exactly one true - find which one */
        for i in 0..count {
            if decisions[i] {
                return 100 + i as i32;
            }
        }
    } else if special_count as usize == count.wrapping_sub(1) {
        /* Exactly one false - find which one */
        for i in 0..count {
            if !decisions[i] {
                return 200 + i as i32;
            }
        }
    }

    /* Check for alternating pattern */
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

    /* Check for consecutive true values */
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

    /* Return count of true values */
    special_count
}

/// Validate sequence has proper pattern
/// Checks various decision sequence rules
fn validate_sequence(sequence: &mut [u8], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }

    /* Convert all to bools.  The C code does
     *     bool *bools = (bool*)sequence;  /* Reuse buffer */
     * i.e. it aliases the caller's `char` buffer with a `_Bool` pointer and
     * rewrites every byte in place (`_Bool` is one byte wide, storing 0 or 1).
     * That mutation is observable by the caller, so it is reproduced here
     * verbatim: byte `i` becomes 1 when `parse_bool` yields true, else 0.  All
     * later reads then come back out of the rewritten buffer, exactly as in C. */
    for i in 0..len {
        let val = parse_bool(sequence[i]);
        sequence[i] = u8::from(val);
    }
    let bools = |i: usize| -> bool { sequence[i] != 0 };

    /* Rule 1: Must start with 'y' */
    if !bools(0) {
        return -10;
    }

    /* Rule 2: Must end with 'n' if length > 1 */
    if len > 1 && bools(len - 1) {
        return -11;
    }

    /* Rule 3: Cannot have more than 3 consecutive same values */
    let mut consecutive: i32 = 1;
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

    /* Rule 4: Count transitions */
    let mut transitions: i32 = 0;
    for i in 1..len {
        if bools(i) != bools(i - 1) {
            transitions += 1;
        }
    }

    /* Validate based on length */
    if len <= 3 {
        /* Short sequences - simple rules */
        if transitions == 0 {
            return 1; /* All same (but passes other rules) */
        }
        if transitions as usize == len - 1 {
            return 2; /* All different */
        }
        10 + transitions
    } else if len <= 10 {
        /* Medium sequences */
        if (transitions as usize) < len / 3 {
            return 20; /* Few transitions */
        }
        if transitions as usize > len / 2 {
            return 30; /* Many transitions */
        }
        25
    } else {
        /* Long sequences */
        if transitions < 3 {
            return 40;
        }
        if transitions as usize > len - 3 {
            return 50;
        }
        45
    }
}
