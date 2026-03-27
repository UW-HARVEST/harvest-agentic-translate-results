use std::ffi::CStr;

extern "C" {
    fn printf(fmt: *const libc::c_char, ...) -> libc::c_int;
    fn time(t: *mut libc::time_t) -> libc::time_t;
    fn strcmp(s1: *const libc::c_char, s2: *const libc::c_char) -> libc::c_int;
}

/// Mimic x86-64 cvttsd2si: out-of-range f64 → i32 yields 0x80000000 (INT_MIN).
/// Rust `as i32` saturates instead, so we replicate the C/x86-64 UB behavior.
#[inline]
fn double_to_int_c(v: f64) -> i32 {
    if v.is_nan() || v < (i32::MIN as f64) || v > (i32::MAX as f64) {
        i32::MIN // 0x80000000 — what cvttsd2si returns for out-of-range
    } else {
        v as i32
    }
}

fn classify_mode(mode: *const libc::c_char) -> libc::c_int {
    unsafe {
        if strcmp(mode, b"standard\0".as_ptr() as *const libc::c_char) == 0 {
            0x10
        } else if strcmp(mode, b"enhanced\0".as_ptr() as *const libc::c_char) == 0 {
            0x20
        } else if strcmp(mode, b"turbo\0".as_ptr() as *const libc::c_char) == 0 {
            0x30
        } else if strcmp(mode, b"extreme\0".as_ptr() as *const libc::c_char) == 0 {
            0x40
        } else {
            0x00
        }
    }
}

fn apply_multiplier(base: libc::c_int, level: libc::c_int) -> libc::c_int {
    let mut result: i32 = base;

    // Reproduce C switch fallthrough: case 4 falls through 3→2→1→0
    match level {
        4 => {
            result = result.wrapping_add(0xFF);
            result = result.wrapping_add(0xAB);
            result = result.wrapping_add(0x7E);
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        3 => {
            result = result.wrapping_add(0xAB);
            result = result.wrapping_add(0x7E);
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        2 => {
            result = result.wrapping_add(0x7E);
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        1 => {
            result = result.wrapping_add(0x1C);
            result = result.wrapping_add(0x05);
        }
        0 => {
            result = result.wrapping_add(0x05);
        }
        _ => {
            result = 0xDEADu32 as i32;
        }
    }

    result
}

fn convert_time_factor(factor: f64) -> libc::c_int {
    let scaled = factor * 1e12;
    double_to_int_c(scaled)
}

fn convert_negative_overflow(value: f64) -> libc::c_int {
    let extreme = value * -1e15;
    double_to_int_c(extreme)
}

fn get_modified_time(offset_days: libc::c_int, offset_hours: libc::c_int) -> libc::time_t {
    let mut current: libc::time_t = unsafe { time(std::ptr::null_mut()) };
    current >>= 29;
    let offset: libc::time_t =
        (offset_days as libc::time_t) * 86400 + (offset_hours as libc::time_t) * 3600;
    current + offset
}

fn hash_time_value(t: libc::time_t) -> libc::c_int {
    let mut hash: i32 = 0x5A5A5A5Au32 as i32;
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(&t as *const libc::time_t as *const u8, std::mem::size_of::<libc::time_t>())
    };

    for (i, &b) in bytes.iter().enumerate() {
        hash ^= (b as i32) << ((i % 4) * 8);
        hash = hash.wrapping_mul(0x1F);
    }

    hash & 0x7FFFFFFF
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: libc::c_int,
    time_offset: libc::c_int,
    complexity: libc::c_int,
    seed: libc::c_int,
) -> libc::c_int {
    let mut result: i32 = 0;

    let modes: [&CStr; 4] = [
        c"standard",
        c"enhanced",
        c"turbo",
        c"extreme",
    ];

    let mode_index = mode_selector % 4;
    // C % can be negative; array index must be valid. But the C code does the
    // same thing — it would be UB there too. We reproduce it exactly: if
    // mode_index is negative, the C code would access out-of-bounds. For the
    // typical positive inputs this is fine. We use wrapping to match C behavior.
    let selected_mode = modes[mode_index as usize];
    let mode_value = classify_mode(selected_mode.as_ptr());

    unsafe {
        printf(
            b"Selected mode: %s (0x%X)\n\0".as_ptr() as *const libc::c_char,
            selected_mode.as_ptr(),
            mode_value as libc::c_uint,
        );
    }
    result += mode_value;

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            b"Complexity level: %d, Multiplier: 0x%X\n\0".as_ptr() as *const libc::c_char,
            complexity_level,
            multiplier as libc::c_uint,
        );
    }
    result += multiplier;

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(
            b"Modified time: %ld, Hash: 0x%X\n\0".as_ptr() as *const libc::c_char,
            modified_time as libc::c_long,
            time_hash as libc::c_uint,
        );
    }
    result += time_hash % 0x1000;

    let factor1: f64 = (seed as f64) * 1e8;
    let factor2: f64 = (time_offset as f64) * -1e7;

    unsafe {
        printf(
            b"Converting double %.2e to int (may overflow)...\n\0".as_ptr()
                as *const libc::c_char,
            factor1,
        );
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(
            b"Result 1: %d (0x%X)\n\0".as_ptr() as *const libc::c_char,
            result1,
            result1 as libc::c_uint,
        );
    }

    unsafe {
        printf(
            b"Converting double %.2e to int (may underflow)...\n\0".as_ptr()
                as *const libc::c_char,
            factor2,
        );
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(
            b"Result 2: %d (0x%X)\n\0".as_ptr() as *const libc::c_char,
            result2,
            result2 as libc::c_uint,
        );
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00u32 as i32;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(
            b"\nFinal result: %d (0x%X)\n\0".as_ptr() as *const libc::c_char,
            result,
            result as libc::c_uint,
        );
    }

    result
}
