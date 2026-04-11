extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn snprintf(
        __s: *mut libc::c_char,
        __maxlen: size_t,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn strncmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
        __n: size_t,
    ) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn cleanup(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
    mut d: libc::c_int,
) -> libc::c_int {
    let mut numbers: [libc::c_int; 4] = [a, b, c, d];
    let mut dynamic_str: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut expected_str: *const libc::c_char =
        b"VALID\0" as *const u8 as *const libc::c_char;
    let mut input_str: *const libc::c_char =
        b"VALID\0" as *const u8 as *const libc::c_char;
    if strncmp(input_str, expected_str, strlen(expected_str)) != 0 as libc::c_int {
        printf(b"Input string validation failed.\n\0" as *const u8 as *const libc::c_char);
    } else {
        let mut i: libc::c_int = 0 as libc::c_int;
        while i < 4 as libc::c_int {
            let mut current_block_5: u64;
            match numbers[i as usize] {
                10 => {
                    result += 10 as libc::c_int;
                    current_block_5 = 8645023485768097611;
                }
                20 => {
                    current_block_5 = 8645023485768097611;
                }
                30 => {
                    result += 30 as libc::c_int;
                    current_block_5 = 14909307599308983100;
                }
                40 => {
                    current_block_5 = 14909307599308983100;
                }
                _ => {
                    result += numbers[i as usize];
                    current_block_5 = 1841672684692190573;
                }
            }
            match current_block_5 {
                8645023485768097611 => {
                    result += 20 as libc::c_int;
                }
                14909307599308983100 => {
                    result += 40 as libc::c_int;
                }
                _ => {}
            }
            i += 1;
        }
        dynamic_str = malloc(
            (50 as size_t).wrapping_mul(std::mem::size_of::<libc::c_char>() as size_t),
        ) as *mut libc::c_char;
        if dynamic_str.is_null() {
            printf(b"Memory allocation failed.\n\0" as *const u8 as *const libc::c_char);
        } else {
            snprintf(
                dynamic_str,
                50 as size_t,
                b"Processed numbers: %s\0" as *const u8 as *const libc::c_char,
                b"numbers\0" as *const u8 as *const libc::c_char,
            );
            printf(
                b"%s\n\0" as *const u8 as *const libc::c_char,
                dynamic_str,
            );
        }
    }
    cleanup_resources(dynamic_str);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn print_result(
    mut label: *const libc::c_char,
    mut result: libc::c_int,
) {
    printf(
        b"%s: %d\n\0" as *const u8 as *const libc::c_char,
        label,
        result,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cleanup_resources(mut dynamic_str: *mut libc::c_char) {
    if !dynamic_str.is_null() {
        free(dynamic_str as *mut libc::c_void);
        dynamic_str = std::ptr::null_mut::<libc::c_char>();
    }
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

