extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
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
pub type operation_func = Option<unsafe extern "C"  fn(_: libc::unix::c_int,) -> libc::unix::c_int>;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const UINT16_MAX: libc::c_int = 65535 as libc::c_int;
static mut counter: libc::c_int = 0 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn increment_counter(mut value: libc::c_int) -> libc::c_int {
    counter += value;
    return counter;
}
#[no_mangle]
pub unsafe extern "C" fn decrement_counter(mut value: libc::c_int) -> libc::c_int {
    counter -= value;
    return counter;
}
#[no_mangle]
pub unsafe extern "C" fn multiply_counter(mut value: libc::c_int) -> libc::c_int {
    counter *= value;
    return counter;
}
#[no_mangle]
pub unsafe extern "C" fn reset_counter(mut value: libc::c_int) -> libc::c_int {
    counter = value;
    return counter;
}
#[no_mangle]
pub unsafe extern "C" fn is_string_empty(
    mut str: *const libc::c_char,
) -> libc::c_int {
    if str.is_null() {
        return 1 as libc::c_int;
    }
    if *str != 0 {
        return 0 as libc::c_int;
    }
    return 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn find_char_in_buffer(
    mut buffer: *const libc::c_char,
    mut size: size_t,
    mut target: libc::c_char,
) -> *mut libc::c_char {
    if buffer.is_null() {
        return std::ptr::null_mut::<libc::c_char>();
    }
    return memchr(
        buffer as *const libc::c_void,
        target as libc::c_int,
        size,
    ) as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn create_buffer(
    mut initial: *const libc::c_char,
) -> *mut libc::c_char {
    if initial.is_null() {
        return std::ptr::null_mut::<libc::c_char>();
    }
    let mut len: size_t = strlen(initial);
    let mut buffer: *mut libc::c_char =
        malloc(len.wrapping_add(1 as size_t)) as *mut libc::c_char;
    if !buffer.is_null() {
        strcpy(buffer, initial);
    }
    return buffer;
}
#[no_mangle]
pub extern "C" fn validate_uint16_range(
    mut value: libc::c_int,
) -> libc::c_int {
    if value < 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    if value > UINT16_MAX {
        return 0 as libc::c_int;
    }
    return 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn apply_operation(
    mut op: operation_func,
    mut value: libc::c_int,
) -> libc::c_int {
    if op.is_none() {
        return -(1 as libc::c_int);
    }
    return op.expect("non-null function pointer")(value);
}
#[no_mangle]
pub unsafe extern "C" fn charinbuf(
    mut mode: libc::c_int,
    mut value: libc::c_int,
    mut opt1: libc::c_int,
    mut opt2: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut buffer: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut found_pos: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut test_string: *const libc::c_char =
        b"\0" as *const u8 as *const libc::c_char;
    let mut non_empty_string: *const libc::c_char =
        b"Hello, World!\0" as *const u8 as *const libc::c_char;
    let mut current_op: operation_func = None;
    counter = 0 as libc::c_int;
    match mode {
        0 => {
            printf(b"Mode 0: UINT16_MAX validation\n\0" as *const u8 as *const libc::c_char);
            printf(
                b"Checking if value %d is within uint16_t range...\n\0" as *const u8
                    as *const libc::c_char,
                value,
            );
            if validate_uint16_range(value) != 0 {
                printf(
                    b"Value %d is valid (0 <= value <= %u)\n\0" as *const u8
                        as *const libc::c_char,
                    value,
                    UINT16_MAX,
                );
                result = value;
            } else {
                printf(
                    b"Value %d is out of range for uint16_t\n\0" as *const u8
                        as *const libc::c_char,
                    value,
                );
                result = -(1 as libc::c_int);
            }
            printf(
                b"UINT16_MAX constant value: %u\n\0" as *const u8 as *const libc::c_char,
                UINT16_MAX,
            );
        }
        1 => {
            printf(
                b"Mode 1: String empty check by dereference\n\0" as *const u8
                    as *const libc::c_char,
            );
            if is_string_empty(test_string) != 0 {
                printf(
                    b"Test string is empty (checked with *string)\n\0" as *const u8
                        as *const libc::c_char,
                );
                result = 0 as libc::c_int;
            } else {
                printf(b"Test string is not empty\n\0" as *const u8 as *const libc::c_char);
                result = 1 as libc::c_int;
            }
            if is_string_empty(non_empty_string) != 0 {
                printf(
                    b"Non-empty string check failed!\n\0" as *const u8
                        as *const libc::c_char,
                );
            } else {
                printf(
                    b"Non-empty string correctly identified\n\0" as *const u8
                        as *const libc::c_char,
                );
                result += 10 as libc::c_int;
            }
        }
        2 => {
            printf(
                b"Mode 2: Dynamic memory allocation and free\n\0" as *const u8
                    as *const libc::c_char,
            );
            buffer = create_buffer(
                b"Testing malloc and free\0" as *const u8 as *const libc::c_char,
            );
            if !buffer.is_null() {
                printf(
                    b"Buffer allocated: '%s'\n\0" as *const u8 as *const libc::c_char,
                    buffer,
                );
                printf(
                    b"Buffer length: %zu\n\0" as *const u8 as *const libc::c_char,
                    strlen(buffer),
                );
                result = strlen(buffer) as libc::c_int;
                free(buffer as *mut libc::c_void);
                printf(b"Buffer freed successfully\n\0" as *const u8 as *const libc::c_char);
                buffer = std::ptr::null_mut::<libc::c_char>();
            } else {
                printf(b"Failed to allocate buffer\n\0" as *const u8 as *const libc::c_char);
                result = -(1 as libc::c_int);
            }
        }
        3 => {
            printf(
                b"Mode 3: Function pointers with static counter\n\0" as *const u8
                    as *const libc::c_char,
            );
            current_op = Some(
                reset_counter as unsafe extern "C" fn(libc::c_int) -> libc::c_int,
            ) as operation_func;
            result = apply_operation(current_op, value);
            printf(
                b"Counter reset to: %d\n\0" as *const u8 as *const libc::c_char,
                result,
            );
            current_op = Some(
                increment_counter as unsafe extern "C" fn(libc::c_int) -> libc::c_int,
            ) as operation_func;
            result = apply_operation(current_op, opt1);
            printf(
                b"Counter after increment by %d: %d\n\0" as *const u8 as *const libc::c_char,
                opt1,
                result,
            );
            current_op = Some(
                multiply_counter as unsafe extern "C" fn(libc::c_int) -> libc::c_int,
            ) as operation_func;
            result = apply_operation(current_op, opt2);
            printf(
                b"Counter after multiply by %d: %d\n\0" as *const u8 as *const libc::c_char,
                opt2,
                result,
            );
            current_op = Some(
                decrement_counter as unsafe extern "C" fn(libc::c_int) -> libc::c_int,
            ) as operation_func;
            result = apply_operation(current_op, 5 as libc::c_int);
            printf(
                b"Counter after decrement by 5: %d\n\0" as *const u8 as *const libc::c_char,
                result,
            );
            printf(
                b"Final static counter value: %d\n\0" as *const u8 as *const libc::c_char,
                counter,
            );
        }
        4 => {
            printf(
                b"Mode 4: Using memchr to find character\n\0" as *const u8
                    as *const libc::c_char,
            );
            buffer = create_buffer(
                b"Search for character X in this buffer\0" as *const u8
                    as *const libc::c_char,
            );
            if !buffer.is_null() {
                let mut buf_size: size_t = strlen(buffer);
                let mut search_char: libc::c_char = 'X' as i32 as libc::c_char;
                printf(
                    b"Searching for '%c' in: '%s'\n\0" as *const u8 as *const libc::c_char,
                    search_char as libc::c_int,
                    buffer,
                );
                found_pos = find_char_in_buffer(buffer, buf_size, search_char);
                if !found_pos.is_null() {
                    result =
                        found_pos.offset_from(buffer) as libc::c_long as libc::c_int;
                    printf(
                        b"Found '%c' at position: %d\n\0" as *const u8
                            as *const libc::c_char,
                        search_char as libc::c_int,
                        result,
                    );
                } else {
                    printf(
                        b"Character '%c' not found\n\0" as *const u8 as *const libc::c_char,
                        search_char as libc::c_int,
                    );
                    result = -(1 as libc::c_int);
                }
                free(buffer as *mut libc::c_void);
                buffer = std::ptr::null_mut::<libc::c_char>();
            }
        }
        _ => {
            printf(
                b"Invalid mode: %d\n\0" as *const u8 as *const libc::c_char,
                mode,
            );
            result = -(1 as libc::c_int);
        }
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

