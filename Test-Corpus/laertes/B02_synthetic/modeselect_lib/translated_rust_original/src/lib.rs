extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
    fn time(__timer: *mut time_t) -> time_t;
}
pub type size_t = usize;
pub type __time_t = libc::linux_like::linux::gnu::b64::x86_64::not_x32::c_long;
pub type time_t = libc::linux_like::linux::gnu::b64::x86_64::not_x32::c_long;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn classify_mode(mut mode: *const libc::c_char) -> libc::c_int {
    if strcmp(
        mode,
        b"standard\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        return 0x10 as libc::c_int;
    } else if strcmp(
        mode,
        b"enhanced\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        return 0x20 as libc::c_int;
    } else if strcmp(mode, b"turbo\0" as *const u8 as *const libc::c_char)
        == 0 as libc::c_int
    {
        return 0x30 as libc::c_int;
    } else if strcmp(
        mode,
        b"extreme\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        return 0x40 as libc::c_int;
    }
    return 0 as libc::c_int;
}
#[no_mangle]
pub extern "C" fn apply_multiplier(
    mut base: libc::c_int,
    mut level: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = base;
    let mut current_block_5: u64;
    match level {
        4 => {
            result += 0xff as libc::c_int;
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
            result = 0xdead as libc::c_int;
            current_block_5 = 6937071982253665452;
        }
    }
    match current_block_5 {
        2494339188419310558 => {
            result += 0xab as libc::c_int;
            current_block_5 = 5444232212525381844;
        }
        _ => {}
    }
    match current_block_5 {
        5444232212525381844 => {
            result += 0x7e as libc::c_int;
            current_block_5 = 11631766481041640858;
        }
        _ => {}
    }
    match current_block_5 {
        11631766481041640858 => {
            result += 0x1c as libc::c_int;
            current_block_5 = 2378624631198048488;
        }
        _ => {}
    }
    match current_block_5 {
        2378624631198048488 => {
            result += 0x5 as libc::c_int;
        }
        _ => {}
    }
    return result;
}
#[no_mangle]
pub extern "C" fn convert_time_factor(
    mut factor: libc::c_double,
) -> libc::c_int {
    let mut scaled: libc::c_double = factor * 1e12f64;
    let mut result: libc::c_int = scaled as libc::c_int;
    return result;
}
#[no_mangle]
pub extern "C" fn convert_negative_overflow(
    mut value: libc::c_double,
) -> libc::c_int {
    let mut extreme: libc::c_double = value * -1e15f64;
    let mut result: libc::c_int = extreme as libc::c_int;
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn get_modified_time(
    mut offset_days: libc::c_int,
    mut offset_hours: libc::c_int,
) -> time_t {
    let mut current: time_t = time(std::ptr::null_mut::<time_t>());
    current = current >> 29 as libc::c_int;
    let mut offset: time_t = (offset_days * 86400 as libc::c_int
        + offset_hours * 3600 as libc::c_int) as time_t;
    return current + offset;
}
#[no_mangle]
pub unsafe extern "C" fn hash_time_value(mut t: time_t) -> libc::c_int {
    let mut hash: libc::c_int = 0x5a5a5a5a as libc::c_int;
    let mut bytes: *mut libc::c_uchar = &raw mut t as *mut libc::c_uchar;
    let mut i: size_t = 0 as size_t;
    while i < std::mem::size_of::<time_t>() as usize {
        hash ^= (*bytes.offset(i as isize) as libc::c_int)
            << i.wrapping_rem(4 as size_t).wrapping_mul(8 as size_t);
        hash *= 0x1f as libc::c_int;
        i = i.wrapping_add(1);
    }
    return hash & 0x7fffffff as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn modeselect(
    mut mode_selector: libc::c_int,
    mut time_offset: libc::c_int,
    mut complexity: libc::c_int,
    mut seed: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut modes: [*const libc::c_char; 4] = [
        b"standard\0" as *const u8 as *const libc::c_char,
        b"enhanced\0" as *const u8 as *const libc::c_char,
        b"turbo\0" as *const u8 as *const libc::c_char,
        b"extreme\0" as *const u8 as *const libc::c_char,
    ];
    let mut mode_index: libc::c_int = mode_selector % 4 as libc::c_int;
    let mut selected_mode: *const libc::c_char = modes[mode_index as usize];
    let mut mode_value: libc::c_int = classify_mode(selected_mode);
    printf(
        b"Selected mode: %s (0x%X)\n\0" as *const u8 as *const libc::c_char,
        selected_mode,
        mode_value,
    );
    result += mode_value;
    let mut complexity_level: libc::c_int = complexity % 5 as libc::c_int;
    let mut multiplier: libc::c_int =
        apply_multiplier(0xa0 as libc::c_int, complexity_level);
    printf(
        b"Complexity level: %d, Multiplier: 0x%X\n\0" as *const u8 as *const libc::c_char,
        complexity_level,
        multiplier,
    );
    result += multiplier;
    let mut modified_time: time_t = get_modified_time(time_offset, seed % 24 as libc::c_int);
    let mut time_hash: libc::c_int = hash_time_value(modified_time);
    printf(
        b"Modified time: %ld, Hash: 0x%X\n\0" as *const u8 as *const libc::c_char,
        modified_time,
        time_hash,
    );
    result += time_hash % 0x1000 as libc::c_int;
    let mut factor1: libc::c_double = seed as libc::c_double * 1e8f64;
    let mut factor2: libc::c_double = time_offset as libc::c_double * -1e7f64;
    printf(
        b"Converting double %.2e to int (may overflow)...\n\0" as *const u8
            as *const libc::c_char,
        factor1,
    );
    let mut result1: libc::c_int = convert_time_factor(factor1);
    printf(
        b"Result 1: %d (0x%X)\n\0" as *const u8 as *const libc::c_char,
        result1,
        result1,
    );
    printf(
        b"Converting double %.2e to int (may underflow)...\n\0" as *const u8
            as *const libc::c_char,
        factor2,
    );
    let mut result2: libc::c_int = convert_negative_overflow(factor2);
    printf(
        b"Result 2: %d (0x%X)\n\0" as *const u8 as *const libc::c_char,
        result2,
        result2,
    );
    result ^= result1 & 0xff as libc::c_int;
    result ^= result2 & 0xff00 as libc::c_int;
    result = result * 0x10 as libc::c_int + 0xbeef as libc::c_int;
    printf(
        b"\nFinal result: %d (0x%X)\n\0" as *const u8 as *const libc::c_char,
        result,
        result,
    );
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

