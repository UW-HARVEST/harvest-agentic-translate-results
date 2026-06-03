// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior preserved byte-for-byte.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

// Bring in libc for getenv/atoi/printf/fprintf/snprintf so output matches C exactly.
extern "C" {
    fn getenv(name: *const c_char) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    static stderr: *mut FILE;
}

#[allow(non_camel_case_types)]
enum FILE {}

#[derive(Copy, Clone)]
struct ConfigFlags {
    verbose: u32,
    debug: u32,
    optimize: u32,
    cache_enabled: u32,
    log_level: u32,
    #[allow(dead_code)]
    reserved: u32,
}

#[derive(Copy, Clone)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    #[allow(dead_code)]
    operation: c_char,
}

const BUFFER_SIZE: usize = 256;

fn parse_env_numeric(env_name: &[u8], default_val: c_int) -> c_int {
    // env_name is a null-terminated byte slice
    unsafe {
        let env_value = getenv(env_name.as_ptr() as *const c_char);

        if env_value.is_null() {
            return default_val;
        }

        let mut invalid_char = strchr(env_value, b',' as c_int);
        if !invalid_char.is_null() {
            let fmt = b"Warning: Invalid character in %s\n\0";
            fprintf(
                stderr,
                fmt.as_ptr() as *const c_char,
                env_name.as_ptr() as *const c_char,
            );
            return default_val;
        }

        invalid_char = strchr(env_value, b';' as c_int);
        if !invalid_char.is_null() {
            let fmt = b"Warning: Semicolon found in %s\n\0";
            fprintf(
                stderr,
                fmt.as_ptr() as *const c_char,
                env_name.as_ptr() as *const c_char,
            );
            return default_val;
        }

        atoi(env_value)
    }
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    unsafe {
        let verbose_env = getenv(b"PROG_VERBOSE\0".as_ptr() as *const c_char);
        let debug_env = getenv(b"PROG_DEBUG\0".as_ptr() as *const c_char);
        let optimize_env = getenv(b"PROG_OPTIMIZE\0".as_ptr() as *const c_char);

        flags.verbose = if !verbose_env.is_null()
            && !strchr(verbose_env, b'1' as c_int).is_null()
        {
            1
        } else {
            0
        };
        flags.debug = if !debug_env.is_null()
            && !strchr(debug_env, b'1' as c_int).is_null()
        {
            1
        } else {
            0
        };
        flags.optimize = if !optimize_env.is_null() { 1 } else { 0 };
        flags.cache_enabled = 1;
        flags.log_level = 0o3;
        flags.reserved = 0;
    }
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let result: c_int;

    let operation_mode: c_int = 0o755;

    if flags.optimize != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // log_level is 3 bits unsigned (0..7); promoted to int in C.
        let ll = flags.log_level as c_int;
        result = (val1.wrapping_mul(ll)).wrapping_add(val2 / 2);
    }

    if flags.debug != 0 {
        unsafe {
            printf(
                b"Debug: operation_mode = %o (octal)\n\0".as_ptr() as *const c_char,
                operation_mode,
            );
            printf(
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
        adjusted = adjusted | 0x0F;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState {
        flags: ConfigFlags {
            verbose: 0,
            debug: 0,
            optimize: 0,
            cache_enabled: 0,
            log_level: 0,
            reserved: 0,
        },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };

    let mut buffer: [c_char; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut result: c_int;

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric(b"PROG_BASE_OFFSET\0", 0o100);
    let multiplier = parse_env_numeric(b"PROG_MULTIPLIER\0", 0o12);

    if state.flags.verbose != 0 {
        unsafe {
            printf(b"Verbose mode enabled\n\0".as_ptr() as *const c_char);
            printf(
                b"Base offset: %d (from octal 0100)\n\0".as_ptr() as *const c_char,
                base_offset,
            );
            printf(
                b"Multiplier: %d (from octal 012)\n\0".as_ptr() as *const c_char,
                multiplier,
            );
        }
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    // memcpy(&state_backup, &state, sizeof(struct ProcessState));
    let state_backup = state;

    if state.flags.debug != 0 {
        unsafe {
            printf(b"Debug: Created state backup using memcpy\n\0".as_ptr() as *const c_char);
            printf(
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
        // C: result += param4 >> 2;  (signed right shift, arithmetic)
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &state.flags);

    result = result.wrapping_add(base_offset);

    unsafe {
        snprintf(
            buffer.as_mut_ptr(),
            BUFFER_SIZE,
            b"Result:%d:Complete\0".as_ptr() as *const c_char,
            result,
        );
    }

    unsafe {
        let colon_pos = strchr(buffer.as_ptr(), b':' as c_int);
        if !colon_pos.is_null() {
            if state.flags.verbose != 0 {
                let offset = (colon_pos as isize) - (buffer.as_ptr() as isize);
                printf(
                    b"Found colon at position: %ld\n\0".as_ptr() as *const c_char,
                    offset as std::os::raw::c_long,
                );
            }

            let second_colon = strchr(colon_pos.offset(1), b':' as c_int);
            if !second_colon.is_null() && state.flags.debug != 0 {
                printf(
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
                printf(b"Restored state from backup\n\0".as_ptr() as *const c_char);
            }
        }
    }

    if state.flags.verbose != 0 {
        unsafe {
            printf(
                b"Final result: %d\n\0".as_ptr() as *const c_char,
                result,
            );
            printf(
                b"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n\0".as_ptr()
                    as *const c_char,
                state.flags.debug as c_int,
                state.flags.optimize as c_int,
                state.flags.log_level as c_int,
            );
        }
    }

    // Suppress unused-import warning when CStr is conditionally referenced.
    let _ = std::marker::PhantomData::<CStr>;

    result
}
