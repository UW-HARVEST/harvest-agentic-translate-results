extern "C" {
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
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub i: libc::c_int,
    pub f: libc::c_float,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
unsafe extern "C" fn memchra(
    mut str: *const libc::c_char,
    mut c: libc::c_int,
    mut n: size_t,
) -> libc::c_int {
    let mut count: libc::c_int = 0 as libc::c_int;
    let mut i: size_t = 0 as size_t;
    while i < n {
        if *str.offset(i as isize) as libc::c_int
            == c as libc::c_char as libc::c_int
        {
            count += 1;
        }
        i = i.wrapping_add(1);
    }
    return count;
}
unsafe extern "C" fn process_buffer(
    mut buffer: *mut libc::c_char,
    mut len: size_t,
) -> libc::c_int {
    if buffer.is_null() || *buffer as libc::c_int == '\0' as i32 {
        return -(1 as libc::c_int);
    }
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut i: *mut libc::c_char = buffer;
    while i < buffer.offset(len as isize) && *i as libc::c_int != '\0' as i32 {
        result += *i as libc::c_int;
        i = i.offset(1);
    }
    return result;
}
unsafe extern "C" fn int_to_float_bits(mut value: libc::c_int) -> libc::c_float {
    let mut converter: C2RustUnnamed = C2RustUnnamed { i: 0 };
    converter.i = value;
    return converter.f;
}
unsafe extern "C" fn process_strings(
    mut strings: *mut *mut libc::c_char,
    mut count: libc::c_int,
    mut target: *const libc::c_char,
) -> libc::c_int {
    if strings.is_null() || count <= 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    let mut matches: libc::c_int = 0 as libc::c_int;
    let mut i: *mut *mut libc::c_char = strings;
    while i < strings.offset(count as isize) {
        if !((*i).is_null() || **i as libc::c_int == '\0' as i32) {
            if strncmp(*i, target, strlen(target)) == 0 as libc::c_int {
                matches += 1;
            }
        }
        i = i.offset(1);
    }
    return matches;
}
unsafe extern "C" fn safe_sum_array(
    mut arr: *mut libc::c_int,
    mut size: size_t,
) -> libc::c_int {
    if arr.is_null() || size == 0 as size_t {
        return 0 as libc::c_int;
    }
    let mut sum: libc::c_int = 0 as libc::c_int;
    let mut i: *mut libc::c_int = arr;
    while i < arr.offset(size as isize) {
        sum += *i;
        i = i.offset(1);
    }
    return sum;
}
unsafe extern "C" fn interpret_as_int(
    mut bytes: *mut libc::c_uchar,
    mut len: size_t,
) -> libc::c_int {
    if bytes.is_null() || len < std::mem::size_of::<libc::c_int>() as usize {
        return 0 as libc::c_int;
    }
    let mut int_ptr: *mut libc::c_int = bytes as *mut libc::c_int;
    return *int_ptr;
}
unsafe extern "C" fn count_occurrences(
    mut text: *const libc::c_char,
    mut ch: libc::c_char,
) -> libc::c_int {
    if text.is_null() || *text as libc::c_int == '\0' as i32 {
        return 0 as libc::c_int;
    }
    let mut len: size_t = strlen(text);
    return memchra(text, ch as libc::c_int, len);
}
unsafe extern "C" fn complex_iteration(
    mut data: *mut libc::c_int,
    mut count: size_t,
) -> libc::c_int {
    if data.is_null() || count == 0 as size_t {
        return -(1 as libc::c_int);
    }
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut i: *mut libc::c_int = data;
    while i < data.offset(count as isize) {
        let mut u: libc::c_uint = *i as libc::c_uint;
        result ^= (u & 0xff as libc::c_uint) as libc::c_int;
        i = i.offset(1);
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn memchra2(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
    mut d: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut buffer: [libc::c_char; 64] = [0; 64];
    snprintf(
        &raw mut buffer as *mut libc::c_char,
        std::mem::size_of::<[libc::c_char; 64]>() as size_t,
        b"test%d-%d-%d-%d\0" as *const u8 as *const libc::c_char,
        a,
        b,
        c,
        d,
    );
    let mut dash_count: libc::c_int = count_occurrences(
        &raw mut buffer as *mut libc::c_char,
        '-' as i32 as libc::c_char,
    );
    result += dash_count * 10 as libc::c_int;
    let mut values: [libc::c_int; 4] = [a, b, c, d];
    let mut sum: libc::c_int =
        safe_sum_array(&raw mut values as *mut libc::c_int, 4 as size_t);
    result += sum;
    let mut test_strings: [*mut libc::c_char; 4] = [
        b"test1\0" as *const u8 as *const libc::c_char as *mut libc::c_char,
        b"test2\0" as *const u8 as *const libc::c_char as *mut libc::c_char,
        b"testing\0" as *const u8 as *const libc::c_char as *mut libc::c_char,
        b"other\0" as *const u8 as *const libc::c_char as *mut libc::c_char,
    ];
    let mut matches: libc::c_int = process_strings(
        &raw mut test_strings as *mut *mut libc::c_char,
        4 as libc::c_int,
        b"test\0" as *const u8 as *const libc::c_char,
    );
    result += matches * 5 as libc::c_int;
    let mut f: libc::c_float = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result += f as libc::c_int;
    }
    let mut buf_sum: libc::c_int = process_buffer(
        &raw mut buffer as *mut libc::c_char,
        strlen(&raw mut buffer as *mut libc::c_char),
    );
    if buf_sum > 0 as libc::c_int {
        result += buf_sum % 256 as libc::c_int;
    }
    let mut bytes: [libc::c_uchar; 4] = [0; 4];
    bytes[0 as libc::c_int as usize] =
        (b & 0xff as libc::c_int) as libc::c_uchar;
    bytes[1 as libc::c_int as usize] =
        (c & 0xff as libc::c_int) as libc::c_uchar;
    bytes[2 as libc::c_int as usize] =
        (d & 0xff as libc::c_int) as libc::c_uchar;
    bytes[3 as libc::c_int as usize] = 0 as libc::c_uchar;
    let mut interpreted: libc::c_int =
        interpret_as_int(&raw mut bytes as *mut libc::c_uchar, 4 as size_t);
    result ^= interpreted;
    let mut complex_result: libc::c_int =
        complex_iteration(&raw mut values as *mut libc::c_int, 4 as size_t);
    result += complex_result;
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

