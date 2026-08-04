// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust preserving byte-identical output.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

const BUFFER_SIZE: usize = 256;

#[derive(Copy, Clone, Default)]
struct ConfigFlags {
    verbose: u32,       // 1 bit
    debug: u32,         // 1 bit
    optimize: u32,      // 1 bit
    cache_enabled: u32, // 1 bit
    log_level: u32,     // 3 bits
    #[allow(dead_code)]
    reserved: u32, // 1 bit
}

#[derive(Copy, Clone, Default)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    #[allow(dead_code)]
    operation: c_char,
}

unsafe extern "C" {
    static stderr: *mut libc::FILE;
}

/// Lookup environment variable using libc::getenv to mirror C semantics
/// (returns NULL when unset; returns empty C string when set to empty value).
fn c_getenv(name: &[u8]) -> Option<&'static CStr> {
    // name is expected to be null-terminated
    debug_assert!(name.last() == Some(&0));
    unsafe {
        let ptr = libc::getenv(name.as_ptr() as *const c_char);
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr))
        }
    }
}

fn c_strchr<'a>(s: &'a CStr, ch: u8) -> Option<usize> {
    let bytes = s.to_bytes();
    bytes.iter().position(|&b| b == ch)
}

/// Replicate atoi: parse leading optional whitespace, optional sign, decimal digits.
fn c_atoi(s: &CStr) -> c_int {
    let bytes = s.to_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let mut sign: i64 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    (val * sign) as c_int
}

fn parse_env_numeric(env_name: &[u8], default_val: c_int) -> c_int {
    let env_value = match c_getenv(env_name) {
        None => return default_val,
        Some(v) => v,
    };

    if c_strchr(env_value, b',').is_some() {
        // Strip null terminator from env_name for the printf format
        let name_str = unsafe { CStr::from_ptr(env_name.as_ptr() as *const c_char) };
        unsafe {
            libc::fprintf(
                stderr,
                b"Warning: Invalid character in %s\n\0".as_ptr() as *const c_char,
                name_str.as_ptr(),
            );
        }
        return default_val;
    }

    if c_strchr(env_value, b';').is_some() {
        let name_str = unsafe { CStr::from_ptr(env_name.as_ptr() as *const c_char) };
        unsafe {
            libc::fprintf(
                stderr,
                b"Warning: Semicolon found in %s\n\0".as_ptr() as *const c_char,
                name_str.as_ptr(),
            );
        }
        return default_val;
    }

    c_atoi(env_value)
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = c_getenv(b"PROG_VERBOSE\0");
    let debug_env = c_getenv(b"PROG_DEBUG\0");
    let optimize_env = c_getenv(b"PROG_OPTIMIZE\0");

    flags.verbose = match verbose_env {
        Some(v) if c_strchr(v, b'1').is_some() => 1,
        _ => 0,
    };
    flags.debug = match debug_env {
        Some(v) if c_strchr(v, b'1').is_some() => 1,
        _ => 0,
    };
    flags.optimize = if optimize_env.is_some() { 1 } else { 0 };
    flags.cache_enabled = 1;
    flags.log_level = 0o3 & 0x7;
    flags.reserved = 0;
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let result: c_int;

    let operation_mode: c_int = 0o755;

    if flags.optimize != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // log_level is a 3-bit unsigned bit-field which is promoted to signed int
        // when used in expressions, mirroring C's integer promotion.
        result = val1
            .wrapping_mul(flags.log_level as c_int)
            .wrapping_add(val2 / 2);
    }

    if flags.debug != 0 {
        unsafe {
            libc::printf(
                b"Debug: operation_mode = %o (octal)\n\0".as_ptr() as *const c_char,
                operation_mode,
            );
            libc::printf(
                b"Debug: result before adjustment = %d\n\0".as_ptr() as *const c_char,
                result,
            );
        }
    }

    result
}

fn apply_bit_operations(value: c_int, flags: &ConfigFlags) -> c_int {
    let mut adjusted = value;

    if flags.verbose != 0 {
        adjusted = ((adjusted as u32) << 1) as c_int;
    }

    if flags.cache_enabled != 0 {
        adjusted |= 0x0F;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState::default();
    let state_backup: ProcessState;
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut result: c_int;

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric(b"PROG_BASE_OFFSET\0", 0o100);
    let multiplier = parse_env_numeric(b"PROG_MULTIPLIER\0", 0o12);

    if state.flags.verbose != 0 {
        unsafe {
            libc::printf(b"Verbose mode enabled\n\0".as_ptr() as *const c_char);
            libc::printf(
                b"Base offset: %d (from octal 0100)\n\0".as_ptr() as *const c_char,
                base_offset,
            );
            libc::printf(
                b"Multiplier: %d (from octal 012)\n\0".as_ptr() as *const c_char,
                multiplier,
            );
        }
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    // memcpy(&state_backup, &state, sizeof(struct ProcessState))
    state_backup = state;

    if state.flags.debug != 0 {
        unsafe {
            libc::printf(b"Debug: Created state backup using memcpy\n\0".as_ptr() as *const c_char);
            libc::printf(
                b"Debug: Backup base_value = %d\n\0".as_ptr() as *const c_char,
                state_backup.base_value,
            );
        }
    }

    result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        // Arithmetic right shift on signed int (matches C on gcc/clang).
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &state.flags);

    result = result.wrapping_add(base_offset);

    // snprintf(buffer, BUFFER_SIZE, "Result:%d:Complete", result);
    unsafe {
        libc::snprintf(
            buffer.as_mut_ptr() as *mut c_char,
            BUFFER_SIZE,
            b"Result:%d:Complete\0".as_ptr() as *const c_char,
            result,
        );
    }

    // Find first ':' in buffer
    let buffer_cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    if let Some(first_pos) = c_strchr(buffer_cstr, b':') {
        if state.flags.verbose != 0 {
            unsafe {
                libc::printf(
                    b"Found colon at position: %ld\n\0".as_ptr() as *const c_char,
                    first_pos as libc::c_long,
                );
            }
        }

        // Search for second colon starting after the first.
        let after_first = &buffer_cstr.to_bytes()[first_pos + 1..];
        let second_colon = after_first.iter().position(|&b| b == b':');
        if second_colon.is_some() && state.flags.debug != 0 {
            unsafe {
                libc::printf(
                    b"Debug: Result string format validated\n\0".as_ptr() as *const c_char,
                );
            }
        }
    }

    if result < 0 {
        // memcpy(&state, &state_backup, sizeof(struct ProcessState));
        state = state_backup;
        result = state.base_value;

        if state.flags.verbose != 0 {
            unsafe {
                libc::printf(b"Restored state from backup\n\0".as_ptr() as *const c_char);
            }
        }
    }

    if state.flags.verbose != 0 {
        unsafe {
            libc::printf(
                b"Final result: %d\n\0".as_ptr() as *const c_char,
                result,
            );
            libc::printf(
                b"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n\0".as_ptr()
                    as *const c_char,
                state.flags.debug as c_int,
                state.flags.optimize as c_int,
                state.flags.log_level as c_int,
            );
        }
    }

    result
}
