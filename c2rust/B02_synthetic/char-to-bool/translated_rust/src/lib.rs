pub type size_t = usize;
pub type __uint32_t = u32;
pub type uint32_t = __uint32_t;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn process_decisions(
    mut decision_string: *mut ::core::ffi::c_char,
    mut length: size_t,
    mut operation: ::core::ffi::c_int,
    mut param: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if decision_string.is_null() || length == 0 as size_t {
        return -(1 as ::core::ffi::c_int);
    }
    match operation {
        0 => {
            if length < 3 as size_t {
                return -(2 as ::core::ffi::c_int);
            }
            let mut read: bool =
                parse_bool(*decision_string.offset(0 as ::core::ffi::c_int as isize));
            let mut write: bool =
                parse_bool(*decision_string.offset(1 as ::core::ffi::c_int as isize));
            let mut execute: bool =
                parse_bool(*decision_string.offset(2 as ::core::ffi::c_int as isize));
            return apply_permissions(read, write, execute);
        }
        1 => {
            if length < 3 as size_t {
                return -(2 as ::core::ffi::c_int);
            }
            let mut cond1: bool =
                parse_bool(*decision_string.offset(0 as ::core::ffi::c_int as isize));
            let mut cond2: bool =
                parse_bool(*decision_string.offset(1 as ::core::ffi::c_int as isize));
            let mut cond3: bool =
                parse_bool(*decision_string.offset(2 as ::core::ffi::c_int as isize));
            return evaluate_conditions(cond1, cond2, cond3, param);
        }
        2 => {
            let mut decisions: [bool; 32] = [false; 32];
            let mut count: size_t = if length < 32 as size_t {
                length
            } else {
                32 as size_t
            };
            let mut i: size_t = 0 as size_t;
            while i < count {
                decisions[i as usize] = parse_bool(*decision_string.offset(i as isize));
                i = i.wrapping_add(1);
            }
            return configure_flags(&raw mut decisions as *mut bool, count);
        }
        3 => return validate_sequence(decision_string, length),
        _ => return -(3 as ::core::ffi::c_int),
    };
}
unsafe extern "C" fn parse_bool(mut c: ::core::ffi::c_char) -> bool {
    if c as ::core::ffi::c_int == 'y' as i32 || c as ::core::ffi::c_int == 'Y' as i32 {
        return true_0 != 0;
    } else if c as ::core::ffi::c_int == 'n' as i32 || c as ::core::ffi::c_int == 'N' as i32 {
        return false_0 != 0;
    }
    return false_0 != 0;
}
unsafe extern "C" fn apply_permissions(
    mut read: bool,
    mut write: bool,
    mut execute: bool,
) -> ::core::ffi::c_int {
    let mut permission_value: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if read {
        permission_value += 4 as ::core::ffi::c_int;
    }
    if write {
        permission_value += 2 as ::core::ffi::c_int;
    }
    if execute {
        permission_value += 1 as ::core::ffi::c_int;
    }
    if read as ::core::ffi::c_int != 0
        && write as ::core::ffi::c_int != 0
        && execute as ::core::ffi::c_int != 0
    {
        return 100 as ::core::ffi::c_int + permission_value;
    } else if read as ::core::ffi::c_int != 0 && write as ::core::ffi::c_int != 0 {
        if permission_value == 6 as ::core::ffi::c_int {
            return 50 as ::core::ffi::c_int + permission_value;
        }
    } else if read as ::core::ffi::c_int != 0 && execute as ::core::ffi::c_int != 0 {
        return 30 as ::core::ffi::c_int + permission_value;
    } else if write as ::core::ffi::c_int != 0 && execute as ::core::ffi::c_int != 0 {
        return 20 as ::core::ffi::c_int + permission_value;
    } else if read {
        return 10 as ::core::ffi::c_int + permission_value;
    } else if write {
        return -(10 as ::core::ffi::c_int);
    } else if execute {
        return -(20 as ::core::ffi::c_int);
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn evaluate_conditions(
    mut cond1: bool,
    mut cond2: bool,
    mut cond3: bool,
    mut logic_op: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: bool = false;
    match logic_op {
        0 => {
            result = cond1 as ::core::ffi::c_int != 0
                && cond2 as ::core::ffi::c_int != 0
                && cond3 as ::core::ffi::c_int != 0;
            if result {
                return 100 as ::core::ffi::c_int;
            } else {
                if cond1 as ::core::ffi::c_int != 0 && cond2 as ::core::ffi::c_int != 0 {
                    return 50 as ::core::ffi::c_int;
                }
                if cond1 as ::core::ffi::c_int != 0 && cond3 as ::core::ffi::c_int != 0 {
                    return 51 as ::core::ffi::c_int;
                }
                if cond2 as ::core::ffi::c_int != 0 && cond3 as ::core::ffi::c_int != 0 {
                    return 52 as ::core::ffi::c_int;
                }
                if cond1 {
                    return 10 as ::core::ffi::c_int;
                }
                if cond2 {
                    return 11 as ::core::ffi::c_int;
                }
                if cond3 {
                    return 12 as ::core::ffi::c_int;
                }
                return 0 as ::core::ffi::c_int;
            }
        }
        1 => {
            result = cond1 as ::core::ffi::c_int != 0
                || cond2 as ::core::ffi::c_int != 0
                || cond3 as ::core::ffi::c_int != 0;
            if result {
                let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                if cond1 {
                    count += 1;
                }
                if cond2 {
                    count += 1;
                }
                if cond3 {
                    count += 1;
                }
                return 100 as ::core::ffi::c_int + count;
            }
            return 0 as ::core::ffi::c_int;
        }
        2 => {
            result = cond1 as ::core::ffi::c_int
                ^ cond2 as ::core::ffi::c_int
                ^ cond3 as ::core::ffi::c_int
                != 0;
            if result {
                if cond1 as ::core::ffi::c_int != 0 && !cond2 && !cond3 {
                    return 1 as ::core::ffi::c_int;
                }
                if !cond1 && cond2 as ::core::ffi::c_int != 0 && !cond3 {
                    return 2 as ::core::ffi::c_int;
                }
                if !cond1 && !cond2 && cond3 as ::core::ffi::c_int != 0 {
                    return 3 as ::core::ffi::c_int;
                }
                if cond1 as ::core::ffi::c_int != 0
                    && cond2 as ::core::ffi::c_int != 0
                    && cond3 as ::core::ffi::c_int != 0
                {
                    return 7 as ::core::ffi::c_int;
                }
                return 90 as ::core::ffi::c_int;
            }
            return 0 as ::core::ffi::c_int;
        }
        3 => {
            result = !(cond1 as ::core::ffi::c_int != 0
                && cond2 as ::core::ffi::c_int != 0
                && cond3 as ::core::ffi::c_int != 0);
            if result {
                if !cond1 && !cond2 && !cond3 {
                    return 200 as ::core::ffi::c_int;
                }
                if !cond1 {
                    return 150 as ::core::ffi::c_int;
                }
                if !cond2 {
                    return 151 as ::core::ffi::c_int;
                }
                if !cond3 {
                    return 152 as ::core::ffi::c_int;
                }
                return 100 as ::core::ffi::c_int;
            }
            return 0 as ::core::ffi::c_int;
        }
        _ => return -(1 as ::core::ffi::c_int),
    };
}
unsafe extern "C" fn configure_flags(
    mut decisions: *mut bool,
    mut count: size_t,
) -> ::core::ffi::c_int {
    let mut flags: uint32_t = 0 as uint32_t;
    let mut special_count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: size_t = 0 as size_t;
    while i < count && i < 32 as size_t {
        if *decisions.offset(i as isize) {
            flags = (flags as ::core::ffi::c_uint | (1 as ::core::ffi::c_uint) << i) as uint32_t;
            special_count += 1;
        }
        i = i.wrapping_add(1);
    }
    if special_count == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    } else if special_count as size_t == count {
        return 1000 as ::core::ffi::c_int + count as ::core::ffi::c_int;
    } else if special_count == 1 as ::core::ffi::c_int {
        let mut i_0: size_t = 0 as size_t;
        while i_0 < count {
            if *decisions.offset(i_0 as isize) {
                return 100 as ::core::ffi::c_int + i_0 as ::core::ffi::c_int;
            }
            i_0 = i_0.wrapping_add(1);
        }
    } else if special_count as size_t == count.wrapping_sub(1 as size_t) {
        let mut i_1: size_t = 0 as size_t;
        while i_1 < count {
            if !*decisions.offset(i_1 as isize) {
                return 200 as ::core::ffi::c_int + i_1 as ::core::ffi::c_int;
            }
            i_1 = i_1.wrapping_add(1);
        }
    }
    let mut alternating: bool = true_0 != 0;
    let mut i_2: size_t = 1 as size_t;
    while i_2 < count {
        if *decisions.offset(i_2 as isize) as ::core::ffi::c_int
            == *decisions.offset(i_2.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
        {
            alternating = false_0 != 0;
            break;
        } else {
            i_2 = i_2.wrapping_add(1);
        }
    }
    if alternating {
        return 500 as ::core::ffi::c_int + special_count;
    }
    let mut max_consecutive: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut current_consecutive: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i_3: size_t = 0 as size_t;
    while i_3 < count {
        if *decisions.offset(i_3 as isize) {
            current_consecutive += 1;
            if current_consecutive > max_consecutive {
                max_consecutive = current_consecutive;
            }
        } else {
            current_consecutive = 0 as ::core::ffi::c_int;
        }
        i_3 = i_3.wrapping_add(1);
    }
    if max_consecutive >= 3 as ::core::ffi::c_int {
        return 300 as ::core::ffi::c_int + max_consecutive;
    }
    return special_count;
}
unsafe extern "C" fn validate_sequence(
    mut sequence: *mut ::core::ffi::c_char,
    mut len: size_t,
) -> ::core::ffi::c_int {
    if len == 0 as size_t {
        return 0 as ::core::ffi::c_int;
    }
    let mut bools: *mut bool = sequence as *mut bool;
    let mut i: size_t = 0 as size_t;
    while i < len {
        let mut val: bool = parse_bool(*sequence.offset(i as isize));
        *bools.offset(i as isize) = val;
        i = i.wrapping_add(1);
    }
    if !*bools.offset(0 as ::core::ffi::c_int as isize) {
        return -(10 as ::core::ffi::c_int);
    }
    if len > 1 as size_t
        && *bools.offset(len.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int != 0
    {
        return -(11 as ::core::ffi::c_int);
    }
    let mut consecutive: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut i_0: size_t = 1 as size_t;
    while i_0 < len {
        if *bools.offset(i_0 as isize) as ::core::ffi::c_int
            == *bools.offset(i_0.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
        {
            consecutive += 1;
            if consecutive > 3 as ::core::ffi::c_int {
                return -(12 as ::core::ffi::c_int);
            }
        } else {
            consecutive = 1 as ::core::ffi::c_int;
        }
        i_0 = i_0.wrapping_add(1);
    }
    let mut transitions: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i_1: size_t = 1 as size_t;
    while i_1 < len {
        if *bools.offset(i_1 as isize) as ::core::ffi::c_int
            != *bools.offset(i_1.wrapping_sub(1 as size_t) as isize) as ::core::ffi::c_int
        {
            transitions += 1;
        }
        i_1 = i_1.wrapping_add(1);
    }
    if len <= 3 as size_t {
        if transitions == 0 as ::core::ffi::c_int {
            return 1 as ::core::ffi::c_int;
        }
        if transitions as size_t == len.wrapping_sub(1 as size_t) {
            return 2 as ::core::ffi::c_int;
        }
        return 10 as ::core::ffi::c_int + transitions;
    } else if len <= 10 as size_t {
        if (transitions as size_t) < len.wrapping_div(3 as size_t) {
            return 20 as ::core::ffi::c_int;
        }
        if transitions as size_t > len.wrapping_div(2 as size_t) {
            return 30 as ::core::ffi::c_int;
        }
        return 25 as ::core::ffi::c_int;
    } else {
        if transitions < 3 as ::core::ffi::c_int {
            return 40 as ::core::ffi::c_int;
        }
        if transitions as size_t > len.wrapping_sub(3 as size_t) {
            return 50 as ::core::ffi::c_int;
        }
        return 45 as ::core::ffi::c_int;
    };
}
