extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn memchr(
        __s: *const libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn pow(__x: libc::c_double, __y: libc::c_double) -> libc::c_double;
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn convert_double_to_int(
    mut value: libc::c_double,
) -> libc::c_int {
    return value as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn find_value_in_buffer(
    mut buffer: *const libc::c_char,
    mut size: size_t,
    mut search_val: libc::c_int,
) -> libc::c_int {
    let mut target: libc::c_char = search_val as libc::c_char;
    let mut result: *mut libc::c_void = memchr(
        buffer as *const libc::c_void,
        target as libc::c_int,
        size,
    );
    if !result.is_null() {
        return (result as *mut libc::c_char).offset_from(buffer) as libc::c_long
            as libc::c_int;
    }
    return -(1 as libc::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn process_negation(mut var1: libc::c_int) -> libc::c_int {
    let mut var2: libc::c_int = 0;
    var2 = (var1 != 0) as libc::c_int;
    return var2;
}
#[no_mangle]
pub unsafe extern "C" fn create_numeric_buffer(
    mut buffer: *mut libc::c_char,
    mut size: libc::c_int,
    mut seed: libc::c_int,
) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < size {
        *buffer.offset(i as isize) = ((seed + i * 7 as libc::c_int)
            % 256 as libc::c_int)
            as libc::c_char;
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn calculate_with_doubles(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
) -> libc::c_double {
    let mut result: libc::c_double = 0.0f64;
    if b != 0 as libc::c_int {
        result = a as libc::c_double / b as libc::c_double;
    }
    result *= pow(
        10.0f64,
        (c % 10 as libc::c_int) as libc::c_double,
    );
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn doubleneg(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut buffer: [libc::c_char; 256] = [0; 256];
    let mut i: libc::c_int = 0;
    printf(b"=== Starting foo() execution ===\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"Parameters: %d, %d, %d, %d\n\0" as *const u8 as *const libc::c_char,
        param1,
        param2,
        param3,
        param4,
    );
    printf(b"\n--- Integer Negation Test ---\n\0" as *const u8 as *const libc::c_char);
    let mut negation_test: libc::c_int = param1;
    let mut negation_result: libc::c_int = (negation_test != 0) as libc::c_int;
    printf(
        b"Original value: %d\n\0" as *const u8 as *const libc::c_char,
        negation_test,
    );
    printf(
        b"After !!negation: %d\n\0" as *const u8 as *const libc::c_char,
        negation_result,
    );
    result += negation_result * 10 as libc::c_int;
    let mut neg_p2: libc::c_int = (param2 != 0) as libc::c_int;
    let mut neg_p3: libc::c_int = (param3 != 0) as libc::c_int;
    let mut neg_p4: libc::c_int = (param4 != 0) as libc::c_int;
    printf(
        b"Double negation results: %d, %d, %d\n\0" as *const u8 as *const libc::c_char,
        neg_p2,
        neg_p3,
        neg_p4,
    );
    result += neg_p2 + neg_p3 + neg_p4;
    printf(
        b"\n--- Double to Int Conversion Test ---\n\0" as *const u8 as *const libc::c_char,
    );
    let mut large_double: libc::c_double = calculate_with_doubles(param1, param2, param3);
    printf(
        b"Calculated double value: %e\n\0" as *const u8 as *const libc::c_char,
        large_double,
    );
    let mut converted_int: libc::c_int = convert_double_to_int(large_double);
    printf(
        b"Converted to int (may be UB): %d\n\0" as *const u8 as *const libc::c_char,
        converted_int,
    );
    let mut negative_large: libc::c_double =
        -1.0f64 * pow(2.0f64, 40 as libc::c_int as libc::c_double);
    printf(
        b"Very large negative double: %e\n\0" as *const u8 as *const libc::c_char,
        negative_large,
    );
    let mut converted_neg: libc::c_int = convert_double_to_int(negative_large);
    printf(
        b"Converted to int (UB likely): %d\n\0" as *const u8 as *const libc::c_char,
        converted_neg,
    );
    result +=
        converted_int % 1000 as libc::c_int + converted_neg % 1000 as libc::c_int;
    printf(b"\n--- Memchr Search Test ---\n\0" as *const u8 as *const libc::c_char);
    create_numeric_buffer(
        &raw mut buffer as *mut libc::c_char,
        256 as libc::c_int,
        param1,
    );
    let mut search_values: [libc::c_int; 4] = [
        param2 % 256 as libc::c_int,
        param3 % 256 as libc::c_int,
        param4 % 256 as libc::c_int,
        42 as libc::c_int,
    ];
    let mut num_searches: libc::c_int = (std::mem::size_of::<[libc::c_int; 4]>()
        as usize)
        .wrapping_div(std::mem::size_of::<libc::c_int>() as usize)
        as libc::c_int;
    printf(b"Searching buffer for values...\n\0" as *const u8 as *const libc::c_char);
    i = 0 as libc::c_int;
    while i < num_searches {
        let mut pos: libc::c_int = find_value_in_buffer(
            &raw mut buffer as *mut libc::c_char,
            256 as size_t,
            search_values[i as usize],
        );
        if pos >= 0 as libc::c_int {
            printf(
                b"Found value %d at position %d\n\0" as *const u8 as *const libc::c_char,
                search_values[i as usize],
                pos,
            );
            result += pos;
        } else {
            printf(
                b"Value %d not found\n\0" as *const u8 as *const libc::c_char,
                search_values[i as usize],
            );
        }
        i += 1;
    }
    let mut direct_search: *mut libc::c_char = memchr(
        &raw mut buffer as *mut libc::c_char as *const libc::c_void,
        100 as libc::c_int,
        256 as size_t,
    ) as *mut libc::c_char;
    if !direct_search.is_null() {
        printf(
            b"Direct memchr found byte 100 at offset: %ld\n\0" as *const u8
                as *const libc::c_char,
            direct_search.offset_from(&raw mut buffer as *mut libc::c_char)
                as libc::c_long,
        );
        result += direct_search.offset_from(&raw mut buffer as *mut libc::c_char)
            as libc::c_long as libc::c_int;
    }
    printf(b"\n--- Combined Feature Test ---\n\0" as *const u8 as *const libc::c_char);
    i = 0 as libc::c_int;
    while i < 10 as libc::c_int {
        let mut search_byte: libc::c_int = (param1 + i * param2) % 256 as libc::c_int;
        let mut found: *mut libc::c_void = memchr(
            &raw mut buffer as *mut libc::c_char as *const libc::c_void,
            search_byte,
            256 as size_t,
        );
        let mut found_flag: libc::c_int = !found.is_null() as libc::c_int;
        printf(
            b"Search %d: byte=%d, found=%d\n\0" as *const u8 as *const libc::c_char,
            i,
            search_byte,
            found_flag,
        );
        result += found_flag;
        i += 1;
    }
    let mut infinity_val: libc::c_double = ::core::f32::INFINITY as libc::c_double;
    let mut nan_val: libc::c_double = ::core::f32::NAN as libc::c_double;
    printf(b"\n--- Special Double Values ---\n\0" as *const u8 as *const libc::c_char);
    printf(b"Converting INFINITY to int: \0" as *const u8 as *const libc::c_char);
    let mut inf_as_int: libc::c_int = convert_double_to_int(infinity_val);
    printf(
        b"%d (undefined behavior)\n\0" as *const u8 as *const libc::c_char,
        inf_as_int,
    );
    printf(b"Converting NAN to int: \0" as *const u8 as *const libc::c_char);
    let mut nan_as_int: libc::c_int = convert_double_to_int(nan_val);
    printf(
        b"%d (undefined behavior)\n\0" as *const u8 as *const libc::c_char,
        nan_as_int,
    );
    printf(b"\n=== Final Result ===\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"Accumulated result: %d\n\0" as *const u8 as *const libc::c_char,
        result,
    );
    return result;
}
