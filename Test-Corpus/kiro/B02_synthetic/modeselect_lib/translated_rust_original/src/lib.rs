use std::ffi::{c_char, c_int, c_long, CStr};

extern "C" {
    fn time(t: *mut i64) -> i64;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        let s = CStr::from_ptr(mode);
        match s.to_bytes() {
            b"standard" => 0x10,
            b"enhanced" => 0x20,
            b"turbo" => 0x30,
            b"extreme" => 0x40,
            _ => 0x00,
        }
    }
}

fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let mut result = base;
    match level {
        0 => { result = result.wrapping_add(0x05); }
        1 => { result = result.wrapping_add(0x1C).wrapping_add(0x05); }
        2 => { result = result.wrapping_add(0x7E).wrapping_add(0x1C).wrapping_add(0x05); }
        3 => { result = result.wrapping_add(0xAB).wrapping_add(0x7E).wrapping_add(0x1C).wrapping_add(0x05); }
        4 => { result = result.wrapping_add(0xFF).wrapping_add(0xAB).wrapping_add(0x7E).wrapping_add(0x1C).wrapping_add(0x05); }
        _ => { result = 0xDEADu32 as c_int; }
    }
    result
}

fn convert_time_factor(factor: f64) -> c_int {
    let scaled = factor * 1e12;
    scaled as c_int
}

fn convert_negative_overflow(value: f64) -> c_int {
    let extreme = value * -1e15;
    extreme as c_int
}

fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> i64 {
    let mut current: i64 = unsafe { time(std::ptr::null_mut()) };
    current >>= 29;
    let offset: i64 = (offset_days as i64 * 86400) + (offset_hours as i64 * 3600);
    current + offset
}

fn hash_time_value(t: i64) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A_u32 as c_int;
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(&t as *const i64 as *const u8, std::mem::size_of::<i64>())
    };
    for i in 0..bytes.len() {
        hash ^= (bytes[i] as c_int) << ((i % 4) * 8);
        hash = hash.wrapping_mul(0x1F);
    }
    hash & 0x7FFFFFFF
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(mode_selector: c_int, time_offset: c_int, complexity: c_int, seed: c_int) -> c_int {
    let mut result: c_int = 0;
    let modes: [&[u8]; 4] = [b"standard\0", b"enhanced\0", b"turbo\0", b"extreme\0"];

    let mode_index = mode_selector % 4;
    let selected_mode = modes[mode_index as usize].as_ptr() as *const c_char;
    let mode_value = classify_mode(selected_mode);

    unsafe {
        printf(b"Selected mode: %s (0x%X)\n\0".as_ptr() as *const c_char, selected_mode, mode_value);
    }
    result += mode_value;

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(b"Complexity level: %d, Multiplier: 0x%X\n\0".as_ptr() as *const c_char, complexity_level, multiplier);
    }
    result += multiplier;

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(b"Modified time: %ld, Hash: 0x%X\n\0".as_ptr() as *const c_char, modified_time as c_long, time_hash);
    }
    result += time_hash % 0x1000;

    let factor1: f64 = seed as f64 * 1e8;
    let factor2: f64 = time_offset as f64 * -1e7;

    unsafe {
        printf(b"Converting double %.2e to int (may overflow)...\n\0".as_ptr() as *const c_char, factor1);
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(b"Result 1: %d (0x%X)\n\0".as_ptr() as *const c_char, result1, result1);
    }

    unsafe {
        printf(b"Converting double %.2e to int (may underflow)...\n\0".as_ptr() as *const c_char, factor2);
    }
    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(b"Result 2: %d (0x%X)\n\0".as_ptr() as *const c_char, result2, result2);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00u32 as c_int;

    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(b"\nFinal result: %d (0x%X)\n\0".as_ptr() as *const c_char, result, result);
    }

    result
}
