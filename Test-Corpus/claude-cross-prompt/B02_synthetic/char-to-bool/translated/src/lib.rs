use std::ffi::c_char;
use std::ffi::c_int;

/// Parse character to boolean
/// 'y' or 'Y' -> true
/// 'n' or 'N' -> false
/// anything else -> false
fn parse_bool(c: c_char) -> bool {
    let cu = c as u8;
    if cu == b'y' || cu == b'Y' {
        true
    } else if cu == b'n' || cu == b'N' {
        false
    } else {
        false
    }
}

/// Apply permissions based on read/write/execute flags
fn apply_permissions(read: bool, write: bool, execute: bool) -> c_int {
    let mut permission_value: c_int = 0;

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

/// Evaluate three conditions with different logic operators
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
                let mut count: c_int = 0;
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

/// Configure system flags based on array of decisions
fn configure_flags(decisions: &[bool], count: usize) -> c_int {
    let mut _flags: u32 = 0;
    let mut special_count: c_int = 0;

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
    } else if special_count as usize == count {
        return 1000 + count as c_int;
    } else if special_count == 1 {
        for i in 0..count {
            if decisions[i] {
                return 100 + i as c_int;
            }
        }
    } else if special_count as usize == count - 1 {
        for i in 0..count {
            if !decisions[i] {
                return 200 + i as c_int;
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
    let mut max_consecutive: c_int = 0;
    let mut current_consecutive: c_int = 0;
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

/// Validate sequence has proper pattern
/// NOTE: The original C code reuses the input buffer to store bools
/// (writing 1 or 0 byte values into the char array). We must reproduce this
/// behavior exactly to match byte-identical output, even though the side
/// effect is on the input buffer.
unsafe fn validate_sequence(sequence: *mut c_char, len: usize) -> c_int {
    if len == 0 {
        return 0;
    }

    /* Convert all to bools, reusing the input buffer (matching C semantics:
     * bool is typically 1 byte; the C code writes the bool value into the
     * char buffer at position i). */
    let bools = sequence as *mut u8;
    for i in 0..len {
        let val = parse_bool(*sequence.add(i));
        *bools.add(i) = if val { 1 } else { 0 };
    }

    let read_bool = |idx: usize| -> bool { unsafe { *bools.add(idx) != 0 } };

    /* Rule 1: Must start with 'y' */
    if !read_bool(0) {
        return -10;
    }

    /* Rule 2: Must end with 'n' if length > 1 */
    if len > 1 && read_bool(len - 1) {
        return -11;
    }

    /* Rule 3: Cannot have more than 3 consecutive same values */
    let mut consecutive: c_int = 1;
    for i in 1..len {
        if read_bool(i) == read_bool(i - 1) {
            consecutive += 1;
            if consecutive > 3 {
                return -12;
            }
        } else {
            consecutive = 1;
        }
    }

    /* Rule 4: Count transitions */
    let mut transitions: c_int = 0;
    for i in 1..len {
        if read_bool(i) != read_bool(i - 1) {
            transitions += 1;
        }
    }

    /* Validate based on length */
    if len <= 3 {
        if transitions == 0 {
            return 1;
        }
        if transitions == (len as c_int) - 1 {
            return 2;
        }
        return 10 + transitions;
    } else if len <= 10 {
        if transitions < (len as c_int) / 3 {
            return 20;
        }
        if transitions > (len as c_int) / 2 {
            return 30;
        }
        return 25;
    } else {
        if transitions < 3 {
            return 40;
        }
        if transitions > (len as c_int) - 3 {
            return 50;
        }
        return 45;
    }
}

/// Main entrance function - processes boolean decisions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_decisions(
    decision_string: *mut c_char,
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
            let read = parse_bool(*decision_string.add(0));
            let write = parse_bool(*decision_string.add(1));
            let execute = parse_bool(*decision_string.add(2));
            apply_permissions(read, write, execute)
        }
        1 => {
            if length < 3 {
                return -2;
            }
            let cond1 = parse_bool(*decision_string.add(0));
            let cond2 = parse_bool(*decision_string.add(1));
            let cond3 = parse_bool(*decision_string.add(2));
            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            let mut decisions = [false; 32];
            let count = if length < 32 { length } else { 32 };
            for i in 0..count {
                decisions[i] = parse_bool(*decision_string.add(i));
            }
            configure_flags(&decisions, count)
        }
        3 => validate_sequence(decision_string, length),
        _ => -3,
    }
}
