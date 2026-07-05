extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn time(__timer: *mut time_t) -> time_t;
}
pub type size_t = usize;
pub type __time_t = ::core::ffi::c_long;
pub type time_t = __time_t;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn classify_mode(mut mode: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    if strcmp(
        mode,
        b"standard\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return 0x10 as ::core::ffi::c_int;
    } else if strcmp(
        mode,
        b"enhanced\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return 0x20 as ::core::ffi::c_int;
    } else if strcmp(mode, b"turbo\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
    {
        return 0x30 as ::core::ffi::c_int;
    } else if strcmp(
        mode,
        b"extreme\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return 0x40 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn apply_multiplier(
    mut base: ::core::ffi::c_int,
    mut level: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = base;
    let mut current_block_5: u64;
    match level {
        4 => {
            result += 0xff as ::core::ffi::c_int;
            current_block_5 = 2494339188419310558;
        }
        3 => {
            current_block_5 = 2494339188419310558;
        }
        2 => {
            current_block_5 = 5444232212525381844;
        }
        1 => {
            current_block_5 = 11631766481041640858;
        }
        0 => {
            current_block_5 = 2378624631198048488;
        }
        _ => {
            result = 0xdead as ::core::ffi::c_int;
            current_block_5 = 6937071982253665452;
        }
    }
    match current_block_5 {
        2494339188419310558 => {
            result += 0xab as ::core::ffi::c_int;
            current_block_5 = 5444232212525381844;
        }
        _ => {}
    }
    match current_block_5 {
        5444232212525381844 => {
            result += 0x7e as ::core::ffi::c_int;
            current_block_5 = 11631766481041640858;
        }
        _ => {}
    }
    match current_block_5 {
        11631766481041640858 => {
            result += 0x1c as ::core::ffi::c_int;
            current_block_5 = 2378624631198048488;
        }
        _ => {}
    }
    match current_block_5 {
        2378624631198048488 => {
            result += 0x5 as ::core::ffi::c_int;
        }
        _ => {}
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn convert_time_factor(
    mut factor: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    let mut scaled: ::core::ffi::c_double = factor * 1e12f64;
    let mut result: ::core::ffi::c_int = scaled as ::core::ffi::c_int;
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn convert_negative_overflow(
    mut value: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    let mut extreme: ::core::ffi::c_double = value * -1e15f64;
    let mut result: ::core::ffi::c_int = extreme as ::core::ffi::c_int;
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn get_modified_time(
    mut offset_days: ::core::ffi::c_int,
    mut offset_hours: ::core::ffi::c_int,
) -> time_t {
    let mut current: time_t = time(::core::ptr::null_mut::<time_t>());
    current = current >> 29 as ::core::ffi::c_int;
    let mut offset: time_t = (offset_days * 86400 as ::core::ffi::c_int
        + offset_hours * 3600 as ::core::ffi::c_int) as time_t;
    return current + offset;
}
#[no_mangle]
pub unsafe extern "C" fn hash_time_value(mut t: time_t) -> ::core::ffi::c_int {
    let mut hash: ::core::ffi::c_int = 0x5a5a5a5a as ::core::ffi::c_int;
    let mut bytes: *mut ::core::ffi::c_uchar = &raw mut t as *mut ::core::ffi::c_uchar;
    let mut i: size_t = 0 as size_t;
    while i < ::core::mem::size_of::<time_t>() as usize {
        hash ^= (*bytes.offset(i as isize) as ::core::ffi::c_int)
            << i.wrapping_rem(4 as size_t).wrapping_mul(8 as size_t);
        hash *= 0x1f as ::core::ffi::c_int;
        i = i.wrapping_add(1);
    }
    return hash & 0x7fffffff as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn modeselect(
    mut mode_selector: ::core::ffi::c_int,
    mut time_offset: ::core::ffi::c_int,
    mut complexity: ::core::ffi::c_int,
    mut seed: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut modes: [*const ::core::ffi::c_char; 4] = [
        b"standard\0" as *const u8 as *const ::core::ffi::c_char,
        b"enhanced\0" as *const u8 as *const ::core::ffi::c_char,
        b"turbo\0" as *const u8 as *const ::core::ffi::c_char,
        b"extreme\0" as *const u8 as *const ::core::ffi::c_char,
    ];
    let mut mode_index: ::core::ffi::c_int = mode_selector % 4 as ::core::ffi::c_int;
    let mut selected_mode: *const ::core::ffi::c_char = modes[mode_index as usize];
    let mut mode_value: ::core::ffi::c_int = classify_mode(selected_mode);
    printf(
        b"Selected mode: %s (0x%X)\n\0" as *const u8 as *const ::core::ffi::c_char,
        selected_mode,
        mode_value,
    );
    result += mode_value;
    let mut complexity_level: ::core::ffi::c_int = complexity % 5 as ::core::ffi::c_int;
    let mut multiplier: ::core::ffi::c_int =
        apply_multiplier(0xa0 as ::core::ffi::c_int, complexity_level);
    printf(
        b"Complexity level: %d, Multiplier: 0x%X\n\0" as *const u8 as *const ::core::ffi::c_char,
        complexity_level,
        multiplier,
    );
    result += multiplier;
    let mut modified_time: time_t = get_modified_time(time_offset, seed % 24 as ::core::ffi::c_int);
    let mut time_hash: ::core::ffi::c_int = hash_time_value(modified_time);
    printf(
        b"Modified time: %ld, Hash: 0x%X\n\0" as *const u8 as *const ::core::ffi::c_char,
        modified_time,
        time_hash,
    );
    result += time_hash % 0x1000 as ::core::ffi::c_int;
    let mut factor1: ::core::ffi::c_double = seed as ::core::ffi::c_double * 1e8f64;
    let mut factor2: ::core::ffi::c_double = time_offset as ::core::ffi::c_double * -1e7f64;
    printf(
        b"Converting double %.2e to int (may overflow)...\n\0" as *const u8
            as *const ::core::ffi::c_char,
        factor1,
    );
    let mut result1: ::core::ffi::c_int = convert_time_factor(factor1);
    printf(
        b"Result 1: %d (0x%X)\n\0" as *const u8 as *const ::core::ffi::c_char,
        result1,
        result1,
    );
    printf(
        b"Converting double %.2e to int (may underflow)...\n\0" as *const u8
            as *const ::core::ffi::c_char,
        factor2,
    );
    let mut result2: ::core::ffi::c_int = convert_negative_overflow(factor2);
    printf(
        b"Result 2: %d (0x%X)\n\0" as *const u8 as *const ::core::ffi::c_char,
        result2,
        result2,
    );
    result ^= result1 & 0xff as ::core::ffi::c_int;
    result ^= result2 & 0xff00 as ::core::ffi::c_int;
    result = result * 0x10 as ::core::ffi::c_int + 0xbeef as ::core::ffi::c_int;
    printf(
        b"\nFinal result: %d (0x%X)\n\0" as *const u8 as *const ::core::ffi::c_char,
        result,
        result,
    );
    return result;
}
