// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of lib.c

use libc::{c_char, c_double, c_int, time_t};

unsafe fn c_strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0isize;
    loop {
        let ca = *a.offset(i) as u8;
        let cb = *b.offset(i) as u8;
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    let standard = b"standard\0".as_ptr() as *const c_char;
    let enhanced = b"enhanced\0".as_ptr() as *const c_char;
    let turbo = b"turbo\0".as_ptr() as *const c_char;
    let extreme = b"extreme\0".as_ptr() as *const c_char;

    if c_strcmp(mode, standard) == 0 {
        0x10
    } else if c_strcmp(mode, enhanced) == 0 {
        0x20
    } else if c_strcmp(mode, turbo) == 0 {
        0x30
    } else if c_strcmp(mode, extreme) == 0 {
        0x40
    } else {
        0x00
    }
}

#[no_mangle]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result: c_int = base;
    // Replicate the C switch with fall-through
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
            result = 0xDEAD;
        }
    }
    result
}

#[no_mangle]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    let scaled = factor * 1e12;
    // Use unchecked conversion to match C semantics (x86 cvttsd2si).
    // For in-range values this matches; out-of-range is undefined in C
    // and will produce platform-dependent values via this intrinsic.
    unsafe { scaled.to_int_unchecked::<c_int>() }
}

#[no_mangle]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    let extreme = value * -1e15;
    unsafe { extreme.to_int_unchecked::<c_int>() }
}

#[no_mangle]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> time_t {
    let mut current: time_t = unsafe { libc::time(std::ptr::null_mut()) };
    current = current >> 29;
    let offset: time_t =
        (offset_days as time_t) * 86400 + (offset_hours as time_t) * 3600;
    current.wrapping_add(offset)
}

#[no_mangle]
pub extern "C" fn hash_time_value(t: time_t) -> c_int {
    let mut hash: c_int = 0x5A5A5A5Ai32;
    let bytes_ptr = &t as *const time_t as *const u8;
    let size = std::mem::size_of::<time_t>();
    for i in 0..size {
        let byte = unsafe { *bytes_ptr.add(i) } as c_int;
        let shift = ((i % 4) * 8) as u32;
        hash ^= byte.wrapping_shl(shift);
        hash = hash.wrapping_mul(0x1F);
    }
    hash & 0x7FFFFFFFi32
}

#[no_mangle]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let modes: [&[u8]; 4] = [
        b"standard\0",
        b"enhanced\0",
        b"turbo\0",
        b"extreme\0",
    ];

    // C: mode_selector % 4 — note: C % can be negative for negative inputs.
    let mode_index = mode_selector % 4;
    let selected_mode = modes[mode_index as usize].as_ptr() as *const c_char;
    let mode_value = unsafe { classify_mode(selected_mode) };

    // Replicate printf
    let selected_str = unsafe {
        std::ffi::CStr::from_ptr(selected_mode).to_string_lossy()
    };
    print!("Selected mode: {} (0x{:X})\n", selected_str, mode_value);
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    print!(
        "Complexity level: {}, Multiplier: 0x{:X}\n",
        complexity_level, multiplier
    );
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    print!(
        "Modified time: {}, Hash: 0x{:X}\n",
        modified_time as i64, time_hash
    );
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1: c_double = (seed as c_double) * 1e8;
    let factor2: c_double = (time_offset as c_double) * -1e7;

    print!("Converting double {:.2e} to int (may overflow)...\n", factor1);

    let result1 = convert_time_factor(factor1);
    print!("Result 1: {} (0x{:X})\n", result1, result1);

    print!("Converting double {:.2e} to int (may underflow)...\n", factor2);
    let result2 = convert_negative_overflow(factor2);
    print!("Result 2: {} (0x{:X})\n", result2, result2);

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEFi32);

    print!("\nFinal result: {} (0x{:X})\n", result, result);

    result
}
