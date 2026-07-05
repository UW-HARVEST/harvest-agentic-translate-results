





pub type size_t = usize;
pub type __uint32_t = u32;
pub type uint32_t = __uint32_t;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub fn process_decisions(
    decision_string: &mut [u8],
    operation: ::core::ffi::c_int,
    param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if decision_string.is_empty() {
        return -1;
    }

    match operation {
        0 => {
            if decision_string.len() < 3 {
                return -2;
            }
            let read = parse_bool(decision_string[0] as ::core::ffi::c_char);
            let write = parse_bool(decision_string[1] as ::core::ffi::c_char);
            let execute = parse_bool(decision_string[2] as ::core::ffi::c_char);
            apply_permissions(read, write, execute)
        }
        1 => {
            if decision_string.len() < 3 {
                return -2;
            }
            let cond1 = parse_bool(decision_string[0] as ::core::ffi::c_char);
            let cond2 = parse_bool(decision_string[1] as ::core::ffi::c_char);
            let cond3 = parse_bool(decision_string[2] as ::core::ffi::c_char);
            evaluate_conditions(cond1, cond2, cond3, param)
        }
        2 => {
            let count = decision_string.len().min(32);
            let mut decisions = [false; 32];

            for (i, &ch) in decision_string.iter().take(count).enumerate() {
                decisions[i] = parse_bool(ch as ::core::ffi::c_char);
            }

            configure_flags(&mut decisions, count)
        }
        3 => validate_sequence(decision_string),
        _ => -3,
    }
}

fn parse_bool(c: i8) -> bool {
    matches!(c as u8 as char, 'y' | 'Y')
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
            50 + permission_value
        } else {
            0
        }
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
            let result = cond1 ^ cond2 ^ cond3;
            if result {
                if cond1 && !cond2 && !cond3 {
                    1
                } else if !cond1 && cond2 && !cond3 {
                    2
                } else if !cond1 && !cond2 && cond3 {
                    3
                } else if cond1 && cond2 && cond3 {
                    7
                } else {
                    90
                }
            } else {
                0
            }
        }
        3 => {
            let result = !(cond1 && cond2 && cond3);
            if result {
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

fn configure_flags(decisions: &[bool], count: usize) -> i32 {
    let special_count = decisions.iter().take(count).take(32).filter(|&&d| d).count() as i32;

    if special_count == 0 {
        return 0;
    } else if special_count as usize == count {
        return 1000 + count as i32;
    } else if special_count == 1 {
        for (i, &decision) in decisions.iter().take(count).enumerate() {
            if decision {
                return 100 + i as i32;
            }
        }
    } else if special_count as usize == count.wrapping_sub(1) {
        for (i, &decision) in decisions.iter().take(count).enumerate() {
            if !decision {
                return 200 + i as i32;
            }
        }
    }

    let mut alternating = true;
    for i in 1..count.min(decisions.len()) {
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
    for &decision in decisions.iter().take(count) {
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

fn validate_sequence(sequence: &mut [u8]) -> i32 {
    let len = sequence.len();
    if len == 0 {
        return 0;
    }

    let mut bools = Vec::with_capacity(len);
    for &ch in sequence.iter() {
        bools.push(parse_bool(ch as ::core::ffi::c_char));
    }

    if !bools[0] {
        return -10;
    }

    if len > 1 && bools[len - 1] {
        return -11;
    }

    let mut consecutive: i32 = 1;
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

