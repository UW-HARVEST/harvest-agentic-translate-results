use std::ffi::{c_char, c_double, c_int, c_long};
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn time(timer: *mut c_long) -> c_long;
}

const STANDARD: &[u8] = b"standard\0";
const ENHANCED: &[u8] = b"enhanced\0";
const TURBO: &[u8] = b"turbo\0";
const EXTREME: &[u8] = b"extreme\0";

const SELECTED_MODE_FORMAT: &[u8] = b"Selected mode: %s (0x%X)\n\0";
const COMPLEXITY_FORMAT: &[u8] = b"Complexity level: %d, Multiplier: 0x%X\n\0";
const MODIFIED_TIME_FORMAT: &[u8] = b"Modified time: %ld, Hash: 0x%X\n\0";
const CONVERT_OVERFLOW_FORMAT: &[u8] = b"Converting double %.2e to int (may overflow)...\n\0";
const RESULT_ONE_FORMAT: &[u8] = b"Result 1: %d (0x%X)\n\0";
const CONVERT_UNDERFLOW_FORMAT: &[u8] = b"Converting double %.2e to int (may underflow)...\n\0";
const RESULT_TWO_FORMAT: &[u8] = b"Result 2: %d (0x%X)\n\0";
const FINAL_RESULT_FORMAT: &[u8] = b"\nFinal result: %d (0x%X)\n\0";

#[inline]
fn c_double_to_int(value: c_double) -> c_int {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let result: c_int;
        // Match the instruction GCC uses for C's double-to-int conversion,
        // including its result for NaN and out-of-range values.
        unsafe {
            core::arch::asm!(
                "cvttsd2si {result:e}, {value}",
                result = lateout(reg) result,
                value = in(xmm_reg) value,
                options(nomem, nostack, pure),
            );
        }
        result
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn classify_mode(mode: *const c_char) -> c_int {
    unsafe {
        if strcmp(mode, STANDARD.as_ptr().cast()) == 0 {
            0x10
        } else if strcmp(mode, ENHANCED.as_ptr().cast()) == 0 {
            0x20
        } else if strcmp(mode, TURBO.as_ptr().cast()) == 0 {
            0x30
        } else if strcmp(mode, EXTREME.as_ptr().cast()) == 0 {
            0x40
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_multiplier(base: c_int, level: c_int) -> c_int {
    let increment: c_int = match level {
        4 => 0xFF + 0xAB + 0x7E + 0x1C + 0x05,
        3 => 0xAB + 0x7E + 0x1C + 0x05,
        2 => 0x7E + 0x1C + 0x05,
        1 => 0x1C + 0x05,
        0 => 0x05,
        _ => return 0xDEAD,
    };

    base.wrapping_add(increment)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_time_factor(factor: c_double) -> c_int {
    c_double_to_int(factor * 1e12)
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_negative_overflow(value: c_double) -> c_int {
    c_double_to_int(value * -1e15)
}

#[unsafe(no_mangle)]
pub extern "C" fn get_modified_time(offset_days: c_int, offset_hours: c_int) -> c_long {
    let current = unsafe { time(ptr::null_mut()) };
    let offset = offset_days
        .wrapping_mul(86_400)
        .wrapping_add(offset_hours.wrapping_mul(3_600));

    (current >> 29).wrapping_add(c_long::from(offset))
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_time_value(t: c_long) -> c_int {
    let mut hash: c_int = 0x5A5A5A5A;

    for (index, byte) in t.to_ne_bytes().iter().copied().enumerate() {
        let shifted = (u32::from(byte) << ((index % 4) * 8)) as c_int;
        hash ^= shifted;
        hash = hash.wrapping_mul(0x1F);
    }

    hash & 0x7FFF_FFFF
}

#[unsafe(no_mangle)]
pub extern "C" fn modeselect(
    mode_selector: c_int,
    time_offset: c_int,
    complexity: c_int,
    seed: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let modes = [
        STANDARD.as_ptr().cast::<c_char>(),
        ENHANCED.as_ptr().cast::<c_char>(),
        TURBO.as_ptr().cast::<c_char>(),
        EXTREME.as_ptr().cast::<c_char>(),
    ];

    let mode_index = mode_selector % 4;
    let selected_mode = unsafe { *modes.as_ptr().offset(mode_index as isize) };
    let mode_value = classify_mode(selected_mode);

    unsafe {
        printf(
            SELECTED_MODE_FORMAT.as_ptr().cast(),
            selected_mode,
            mode_value,
        );
    }
    result = result.wrapping_add(mode_value);

    let complexity_level = complexity % 5;
    let multiplier = apply_multiplier(0xA0, complexity_level);

    unsafe {
        printf(
            COMPLEXITY_FORMAT.as_ptr().cast(),
            complexity_level,
            multiplier,
        );
    }
    result = result.wrapping_add(multiplier);

    let modified_time = get_modified_time(time_offset, seed % 24);
    let time_hash = hash_time_value(modified_time);

    unsafe {
        printf(
            MODIFIED_TIME_FORMAT.as_ptr().cast(),
            modified_time,
            time_hash,
        );
    }
    result = result.wrapping_add(time_hash % 0x1000);

    let factor1 = c_double::from(seed) * 1e8;
    let factor2 = c_double::from(time_offset) * -1e7;

    unsafe {
        printf(CONVERT_OVERFLOW_FORMAT.as_ptr().cast(), factor1);
    }

    let result1 = convert_time_factor(factor1);
    unsafe {
        printf(RESULT_ONE_FORMAT.as_ptr().cast(), result1, result1);
        printf(CONVERT_UNDERFLOW_FORMAT.as_ptr().cast(), factor2);
    }

    let result2 = convert_negative_overflow(factor2);
    unsafe {
        printf(RESULT_TWO_FORMAT.as_ptr().cast(), result2, result2);
    }

    result ^= result1 & 0xFF;
    result ^= result2 & 0xFF00;
    result = result.wrapping_mul(0x10).wrapping_add(0xBEEF);

    unsafe {
        printf(FINAL_RESULT_FORMAT.as_ptr().cast(), result, result);
    }

    result
}
