extern "C" {
    fn sprintf(
        __s: *mut libc::c_char,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn memchr(
        __s: *const libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strcpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
pub type operation_func =
    Option<unsafe extern "C"  fn(_: libc::c_int,_: libc::c_int,) -> libc::c_int>;
static mut accumulator: libc::c_int = 0 as libc::c_int;
static mut multiplier: libc::c_int = 1 as libc::c_int;
static mut operation_count: libc::c_int = 0 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn add_to_accumulator(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    accumulator += a + b;
    operation_count += 1;
    return accumulator;
}
#[no_mangle]
pub unsafe extern "C" fn multiply_with_multiplier(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    multiplier *= a * b;
    operation_count += 1;
    return multiplier;
}
#[no_mangle]
pub unsafe extern "C" fn subtract_from_accumulator(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    accumulator -= a - b;
    operation_count += 1;
    return accumulator;
}
#[no_mangle]
pub unsafe extern "C" fn divide_multiplier(
    mut a: libc::c_int,
    mut b: libc::c_int,
) -> libc::c_int {
    if b != 0 as libc::c_int {
        multiplier /= b;
    }
    operation_count += 1;
    return multiplier;
}
#[no_mangle]
pub unsafe extern "C" fn process_octal_string(
    mut dest: *mut libc::c_char,
    mut octal_val: libc::c_int,
) {
    let mut buffer: [libc::c_char; 50] = [0; 50];
    sprintf(
        &raw mut buffer as *mut libc::c_char,
        b"Octal: 0%o, Decimal: %d\0" as *const u8 as *const libc::c_char,
        octal_val,
        octal_val,
    );
    strcpy(dest, &raw mut buffer as *mut libc::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn find_and_replace_char(
    mut str: *mut libc::c_char,
    mut search_char: libc::c_int,
) {
    let mut found: *mut libc::c_char =
        memchr(str as *const libc::c_void, search_char, strlen(str))
            as *mut libc::c_char;
    if !found.is_null() {
        *found = 'X' as i32 as libc::c_char;
    }
}
#[no_mangle]
pub extern "C" fn validate_and_normalize(
    mut value: libc::c_int,
) -> libc::c_int {
    let mut is_nonzero: libc::c_int = (value != 0) as libc::c_int;
    let mut is_zero: libc::c_int = (value == 0) as libc::c_int;
    let mut lower_threshold: libc::c_int = 0o100 as libc::c_int;
    let mut upper_threshold: libc::c_int = 0o777 as libc::c_int;
    if is_nonzero != 0 && value > 0 as libc::c_int {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }
    return value;
}
static mut operations: [operation_func; 4] = unsafe {
    [
        Some(
            add_to_accumulator
                as unsafe extern "C" fn(
                    libc::c_int,
                    libc::c_int,
                ) -> libc::c_int,
        ),
        Some(
            multiply_with_multiplier
                as unsafe extern "C" fn(
                    libc::c_int,
                    libc::c_int,
                ) -> libc::c_int,
        ),
        Some(
            subtract_from_accumulator
                as unsafe extern "C" fn(
                    libc::c_int,
                    libc::c_int,
                ) -> libc::c_int,
        ),
        Some(
            divide_multiplier
                as unsafe extern "C" fn(
                    libc::c_int,
                    libc::c_int,
                ) -> libc::c_int,
        ),
    ]
};
#[no_mangle]
pub unsafe extern "C" fn findrep(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut p1_valid: libc::c_int = (param1 != 0) as libc::c_int;
    let mut p2_valid: libc::c_int = (param2 != 0) as libc::c_int;
    let mut p3_valid: libc::c_int = (param3 != 0) as libc::c_int;
    let mut p4_valid: libc::c_int = (param4 != 0) as libc::c_int;
    let mut active_params: libc::c_int = p1_valid + p2_valid + p3_valid + p4_valid;
    let mut mode_add: libc::c_int = 0o1 as libc::c_int;
    let mut mode_multiply: libc::c_int = 0o2 as libc::c_int;
    let mut mode_subtract: libc::c_int = 0o3 as libc::c_int;
    let mut mode_divide: libc::c_int = 0o4 as libc::c_int;
    let mut normalized_p1: libc::c_int = validate_and_normalize(param1);
    let mut normalized_p2: libc::c_int = validate_and_normalize(param2);
    let mut normalized_p3: libc::c_int = validate_and_normalize(param3);
    let mut normalized_p4: libc::c_int = validate_and_normalize(param4);
    let mut message: [libc::c_char; 100] = [0; 100];
    let mut search_buffer: [libc::c_char; 100] = [0; 100];
    process_octal_string(
        &raw mut message as *mut libc::c_char,
        0o123 as libc::c_int,
    );
    strcpy(
        &raw mut search_buffer as *mut libc::c_char,
        b"Function pointer example with static vars\0" as *const u8 as *const libc::c_char,
    );
    let mut found_char: *mut libc::c_char = memchr(
        &raw mut search_buffer as *mut libc::c_char as *const libc::c_void,
        'p' as i32,
        strlen(&raw mut search_buffer as *mut libc::c_char),
    ) as *mut libc::c_char;
    if !found_char.is_null() {
        result += found_char.offset_from(&raw mut search_buffer as *mut libc::c_char)
            as libc::c_long as libc::c_int;
    }
    let mut selected_op: operation_func = None;
    if active_params >= mode_add {
        selected_op = operations[0 as libc::c_int as usize];
        result += selected_op.expect("non-null function pointer")(normalized_p1, normalized_p2);
    }
    if active_params >= mode_multiply {
        selected_op = operations[1 as libc::c_int as usize];
        result += selected_op.expect("non-null function pointer")(normalized_p3, normalized_p4);
    }
    if accumulator > 0o150 as libc::c_int {
        selected_op = operations[2 as libc::c_int as usize];
        let mut subtract_result: libc::c_int =
            selected_op.expect("non-null function pointer")(normalized_p1, normalized_p3);
        result += subtract_result;
    }
    find_and_replace_char(&raw mut message as *mut libc::c_char, 'O' as i32);
    let mut final_message: [libc::c_char; 100] = [0; 100];
    strcpy(
        &raw mut final_message as *mut libc::c_char,
        &raw mut message as *mut libc::c_char,
    );
    let mut has_accumulator: libc::c_int = (accumulator != 0) as libc::c_int;
    let mut has_multiplier: libc::c_int = (multiplier != 0) as libc::c_int;
    let mut both_active: libc::c_int =
        (has_accumulator != 0 && has_multiplier != 0) as libc::c_int;
    if both_active != 0 {
        result += accumulator + multiplier;
    }
    if multiplier > 0o100 as libc::c_int {
        selected_op = operations[3 as libc::c_int as usize];
        selected_op.expect("non-null function pointer")(multiplier, 2 as libc::c_int);
    }
    result += operation_count * 0o10 as libc::c_int;
    let mut result_exists: libc::c_int = (result != 0) as libc::c_int;
    if result_exists == 0 {
        result = 0o777 as libc::c_int;
    }
    return result;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

