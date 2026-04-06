extern "C" {
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub i: ::core::ffi::c_int,
    pub f: ::core::ffi::c_float,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
unsafe extern "C" fn memchra(
    mut str: *const ::core::ffi::c_char,
    mut c: ::core::ffi::c_int,
    mut n: size_t,
) -> ::core::ffi::c_int {
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: size_t = 0 as size_t;
    while i < n {
        if *str.offset(i as isize) as ::core::ffi::c_int
            == c as ::core::ffi::c_char as ::core::ffi::c_int
        {
            count += 1;
        }
        i = i.wrapping_add(1);
    }
    return count;
}
unsafe extern "C" fn process_buffer(
    mut buffer: *mut ::core::ffi::c_char,
    mut len: size_t,
) -> ::core::ffi::c_int {
    if buffer.is_null() || *buffer as ::core::ffi::c_int == '\0' as i32 {
        return -(1 as ::core::ffi::c_int);
    }
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: *mut ::core::ffi::c_char = buffer;
    while i < buffer.offset(len as isize) && *i as ::core::ffi::c_int != '\0' as i32 {
        result += *i as ::core::ffi::c_int;
        i = i.offset(1);
    }
    return result;
}
unsafe extern "C" fn int_to_float_bits(mut value: ::core::ffi::c_int) -> ::core::ffi::c_float {
    let mut converter: C2RustUnnamed = C2RustUnnamed { i: 0 };
    converter.i = value;
    return converter.f;
}
unsafe extern "C" fn process_strings(
    mut strings: *mut *mut ::core::ffi::c_char,
    mut count: ::core::ffi::c_int,
    mut target: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if strings.is_null() || count <= 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    let mut matches: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: *mut *mut ::core::ffi::c_char = strings;
    while i < strings.offset(count as isize) {
        if !((*i).is_null() || **i as ::core::ffi::c_int == '\0' as i32) {
            if strncmp(*i, target, strlen(target)) == 0 as ::core::ffi::c_int {
                matches += 1;
            }
        }
        i = i.offset(1);
    }
    return matches;
}
unsafe extern "C" fn safe_sum_array(
    mut arr: *mut ::core::ffi::c_int,
    mut size: size_t,
) -> ::core::ffi::c_int {
    if arr.is_null() || size == 0 as size_t {
        return 0 as ::core::ffi::c_int;
    }
    let mut sum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: *mut ::core::ffi::c_int = arr;
    while i < arr.offset(size as isize) {
        sum += *i;
        i = i.offset(1);
    }
    return sum;
}
unsafe extern "C" fn interpret_as_int(
    mut bytes: *mut ::core::ffi::c_uchar,
    mut len: size_t,
) -> ::core::ffi::c_int {
    if bytes.is_null() || len < ::core::mem::size_of::<::core::ffi::c_int>() as usize {
        return 0 as ::core::ffi::c_int;
    }
    let mut int_ptr: *mut ::core::ffi::c_int = bytes as *mut ::core::ffi::c_int;
    return *int_ptr;
}
unsafe extern "C" fn count_occurrences(
    mut text: *const ::core::ffi::c_char,
    mut ch: ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if text.is_null() || *text as ::core::ffi::c_int == '\0' as i32 {
        return 0 as ::core::ffi::c_int;
    }
    let mut len: size_t = strlen(text);
    return memchra(text, ch as ::core::ffi::c_int, len);
}
unsafe extern "C" fn complex_iteration(
    mut data: *mut ::core::ffi::c_int,
    mut count: size_t,
) -> ::core::ffi::c_int {
    if data.is_null() || count == 0 as size_t {
        return -(1 as ::core::ffi::c_int);
    }
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: *mut ::core::ffi::c_int = data;
    while i < data.offset(count as isize) {
        let mut u: ::core::ffi::c_uint = *i as ::core::ffi::c_uint;
        result ^= (u & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_int;
        i = i.offset(1);
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn memchra2(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut c: ::core::ffi::c_int,
    mut d: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut buffer: [::core::ffi::c_char; 64] = [0; 64];
    snprintf(
        &raw mut buffer as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
        b"test%d-%d-%d-%d\0" as *const u8 as *const ::core::ffi::c_char,
        a,
        b,
        c,
        d,
    );
    let mut dash_count: ::core::ffi::c_int = count_occurrences(
        &raw mut buffer as *mut ::core::ffi::c_char,
        '-' as i32 as ::core::ffi::c_char,
    );
    result += dash_count * 10 as ::core::ffi::c_int;
    let mut values: [::core::ffi::c_int; 4] = [a, b, c, d];
    let mut sum: ::core::ffi::c_int =
        safe_sum_array(&raw mut values as *mut ::core::ffi::c_int, 4 as size_t);
    result += sum;
    let mut test_strings: [*mut ::core::ffi::c_char; 4] = [
        b"test1\0" as *const u8 as *const ::core::ffi::c_char as *mut ::core::ffi::c_char,
        b"test2\0" as *const u8 as *const ::core::ffi::c_char as *mut ::core::ffi::c_char,
        b"testing\0" as *const u8 as *const ::core::ffi::c_char as *mut ::core::ffi::c_char,
        b"other\0" as *const u8 as *const ::core::ffi::c_char as *mut ::core::ffi::c_char,
    ];
    let mut matches: ::core::ffi::c_int = process_strings(
        &raw mut test_strings as *mut *mut ::core::ffi::c_char,
        4 as ::core::ffi::c_int,
        b"test\0" as *const u8 as *const ::core::ffi::c_char,
    );
    result += matches * 5 as ::core::ffi::c_int;
    let mut f: ::core::ffi::c_float = int_to_float_bits(a);
    if f > 0.0f32 && f < 1000.0f32 {
        result += f as ::core::ffi::c_int;
    }
    let mut buf_sum: ::core::ffi::c_int = process_buffer(
        &raw mut buffer as *mut ::core::ffi::c_char,
        strlen(&raw mut buffer as *mut ::core::ffi::c_char),
    );
    if buf_sum > 0 as ::core::ffi::c_int {
        result += buf_sum % 256 as ::core::ffi::c_int;
    }
    let mut bytes: [::core::ffi::c_uchar; 4] = [0; 4];
    bytes[0 as ::core::ffi::c_int as usize] =
        (b & 0xff as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    bytes[1 as ::core::ffi::c_int as usize] =
        (c & 0xff as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    bytes[2 as ::core::ffi::c_int as usize] =
        (d & 0xff as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    bytes[3 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_uchar;
    let mut interpreted: ::core::ffi::c_int =
        interpret_as_int(&raw mut bytes as *mut ::core::ffi::c_uchar, 4 as size_t);
    result ^= interpreted;
    let mut complex_result: ::core::ffi::c_int =
        complex_iteration(&raw mut values as *mut ::core::ffi::c_int, 4 as size_t);
    result += complex_result;
    return result;
}
