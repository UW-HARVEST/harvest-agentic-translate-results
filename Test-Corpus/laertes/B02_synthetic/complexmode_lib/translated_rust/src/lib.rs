extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn snprintf(
        __s: *mut libc::c_char,
        __maxlen: size_t,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strcpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Result_0 {
    pub value: libc::c_int,
    pub operation: [libc::c_char; 32],
    pub permissions: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const READ_PERM: libc::c_int = 0o400 as libc::c_int;
pub const WRITE_PERM: libc::c_int = 0o200 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn create_result_string(
    mut op: *const libc::c_char,
    mut val: libc::c_int,
) -> *mut libc::c_char {
    let mut str: *mut libc::c_char = malloc(
        (64 as size_t).wrapping_mul(std::mem::size_of::<libc::c_char>() as size_t),
    ) as *mut libc::c_char;
    if str.is_null() {
        return std::ptr::null_mut::<libc::c_char>();
    }
    snprintf(
        str,
        64 as size_t,
        b"Operation: %s, Value: %d\0" as *const u8 as *const libc::c_char,
        op,
        val,
    );
    return str;
}
#[no_mangle]
pub extern "C" fn check_permissions(
    mut perms: libc::c_int,
    mut required: libc::c_int,
) -> libc::c_int {
    return (perms & required == required) as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn safe_add(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut perms: libc::c_int,
) -> libc::c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        printf(
            b"Insufficient permissions for addition\n\0" as *const u8 as *const libc::c_char,
        );
        return 0 as libc::c_int;
    }
    return a + b;
}
#[no_mangle]
pub unsafe extern "C" fn multiply_with_log(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut log_msg: *mut *mut libc::c_char,
) -> libc::c_int {
    *log_msg = create_result_string(
        b"multiply\0" as *const u8 as *const libc::c_char,
        a * b,
    );
    if (*log_msg).is_null() {
        return 0 as libc::c_int;
    }
    return a * b;
}
#[no_mangle]
pub unsafe extern "C" fn copy_and_sum(
    mut src: *mut libc::c_int,
    mut count: libc::c_int,
) -> libc::c_int {
    if src.is_null() {
        printf(b"Source pointer is NULL\n\0" as *const u8 as *const libc::c_char);
        return -(1 as libc::c_int);
    }
    let mut dest: *mut libc::c_int = malloc(
        (count as size_t).wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
    ) as *mut libc::c_int;
    if dest.is_null() {
        printf(b"Memory allocation failed\n\0" as *const u8 as *const libc::c_char);
        return -(1 as libc::c_int);
    }
    memcpy(
        dest as *mut libc::c_void,
        src as *const libc::c_void,
        (count as size_t).wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
    );
    let mut sum: libc::c_int = 0 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < count {
        sum += *dest.offset(i as isize);
        i += 1;
    }
    free(dest as *mut libc::c_void);
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn compare_operations(
    mut op1: *const libc::c_char,
    mut op2: *const libc::c_char,
) -> libc::c_int {
    if op1.is_null() || op2.is_null() {
        printf(
            b"One or both operation strings are NULL\n\0" as *const u8
                as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    return strcmp(op1, op2);
}
#[no_mangle]
pub unsafe extern "C" fn complexmode(
    mut mode: libc::c_int,
    mut value1: libc::c_int,
    mut value2: libc::c_int,
    mut value3: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut log_message: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut permissions: libc::c_int = 0o644 as libc::c_int;
    let mut res_tracker: *mut Result_0 =
        malloc(std::mem::size_of::<Result_0>() as size_t) as *mut Result_0;
    if res_tracker.is_null() {
        printf(b"Failed to allocate result tracker\n\0" as *const u8 as *const libc::c_char);
        return -(1 as libc::c_int);
    }
    (*res_tracker).value = 0 as libc::c_int;
    (*res_tracker).permissions = permissions;
    strcpy(
        &raw mut (*res_tracker).operation as *mut libc::c_char,
        b"none\0" as *const u8 as *const libc::c_char,
    );
    match mode {
        1 => {
            strcpy(
                &raw mut (*res_tracker).operation as *mut libc::c_char,
                b"addition\0" as *const u8 as *const libc::c_char,
            );
            result = safe_add(value1, value2, permissions);
            (*res_tracker).value = result;
            printf(b"Mode 1: Addition\n\0" as *const u8 as *const libc::c_char);
            printf(
                b"Result: %d\n\0" as *const u8 as *const libc::c_char,
                result,
            );
        }
        2 => {
            strcpy(
                &raw mut (*res_tracker).operation as *mut libc::c_char,
                b"multiplication\0" as *const u8 as *const libc::c_char,
            );
            result = multiply_with_log(value1, value2, &raw mut log_message);
            (*res_tracker).value = result;
            if log_message.is_null()
                || strcmp(
                    log_message,
                    b"\0" as *const u8 as *const libc::c_char,
                ) == 0 as libc::c_int
            {
                printf(
                    b"Log message creation failed\n\0" as *const u8 as *const libc::c_char,
                );
            } else {
                printf(
                    b"Mode 2: %s\n\0" as *const u8 as *const libc::c_char,
                    log_message,
                );
                free(log_message as *mut libc::c_void);
            }
        }
        3 => {
            strcpy(
                &raw mut (*res_tracker).operation as *mut libc::c_char,
                b"array_sum\0" as *const u8 as *const libc::c_char,
            );
            let mut values: [libc::c_int; 3] = [value1, value2, value3];
            result = copy_and_sum(
                &raw mut values as *mut libc::c_int,
                3 as libc::c_int,
            );
            (*res_tracker).value = result;
            printf(b"Mode 3: Array Sum\n\0" as *const u8 as *const libc::c_char);
            printf(
                b"Result: %d\n\0" as *const u8 as *const libc::c_char,
                result,
            );
        }
        4 => {
            strcpy(
                &raw mut (*res_tracker).operation as *mut libc::c_char,
                b"complex\0" as *const u8 as *const libc::c_char,
            );
            if check_permissions(permissions, 0o100 as libc::c_int) != 0 {
                result = value1 * value2 + value3;
            } else {
                result = value1 + value2 + value3;
            }
            (*res_tracker).value = result;
            printf(b"Mode 4: Complex Calculation\n\0" as *const u8 as *const libc::c_char);
            printf(
                b"Result: %d\n\0" as *const u8 as *const libc::c_char,
                result,
            );
        }
        _ => {
            printf(b"Invalid mode\n\0" as *const u8 as *const libc::c_char);
            result = -(1 as libc::c_int);
        }
    }
    if strcmp(
        &raw mut (*res_tracker).operation as *mut libc::c_char,
        b"none\0" as *const u8 as *const libc::c_char,
    ) != 0 as libc::c_int
    {
        printf(
            b"Operation performed: %s\n\0" as *const u8 as *const libc::c_char,
            &raw mut (*res_tracker).operation as *mut libc::c_char,
        );
    }
    free(res_tracker as *mut libc::c_void);
    return result;
}
