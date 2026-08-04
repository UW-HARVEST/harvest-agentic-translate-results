use std::ffi::{c_int, CStr, CString};
use std::os::raw::c_char;

#[derive(Clone, Copy)]
struct ConfigFlags {
    verbose: bool,
    debug: bool,
    optimize: bool,
    cache_enabled: bool,
    log_level: u8,
}

#[derive(Clone, Copy)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    #[allow(dead_code)]
    operation: u8,
}

const BUFFER_SIZE: usize = 256;

unsafe fn getenv_str(name: &str) -> Option<&CStr> {
    let cname = CString::new(name).unwrap();
    let ptr = unsafe { libc::getenv(cname.as_ptr()) };
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(ptr) })
    }
}

fn parse_env_numeric(env_name: &str, default_val: c_int) -> c_int {
    let env_value = match unsafe { getenv_str(env_name) } {
        None => return default_val,
        Some(v) => v,
    };

    let bytes = env_value.to_bytes();

    if bytes.contains(&b',') {
        unsafe {
            libc::fprintf(
                libc_stderr(),
                b"Warning: Invalid character in %s\n\0".as_ptr() as *const c_char,
                CString::new(env_name).unwrap().as_ptr(),
            );
        }
        return default_val;
    }

    if bytes.contains(&b';') {
        unsafe {
            libc::fprintf(
                libc_stderr(),
                b"Warning: Semicolon found in %s\n\0".as_ptr() as *const c_char,
                CString::new(env_name).unwrap().as_ptr(),
            );
        }
        return default_val;
    }

    // Replicate atoi behavior
    let s = env_value.to_str().unwrap_or("");
    let s = s.trim_start();
    // atoi: parse optional sign then digits, stop at first non-digit
    if s.is_empty() {
        return 0;
    }
    let mut chars = s.chars().peekable();
    let negative = if chars.peek() == Some(&'-') {
        chars.next();
        true
    } else {
        if chars.peek() == Some(&'+') {
            chars.next();
        }
        false
    };
    let mut val: c_int = 0;
    for ch in chars {
        if ch.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((ch as u8 - b'0') as c_int);
        } else {
            break;
        }
    }
    if negative { val.wrapping_neg() } else { val }
}

fn init_config_from_env(flags: &mut ConfigFlags) {
    let verbose_env = unsafe { getenv_str("PROG_VERBOSE") };
    let debug_env = unsafe { getenv_str("PROG_DEBUG") };
    let optimize_env = unsafe { getenv_str("PROG_OPTIMIZE") };

    flags.verbose = verbose_env.is_some_and(|v| v.to_bytes().contains(&b'1'));
    flags.debug = debug_env.is_some_and(|v| v.to_bytes().contains(&b'1'));
    flags.optimize = optimize_env.is_some();
    flags.cache_enabled = true;
    flags.log_level = 3; // octal 03 = 3
}

fn perform_operation(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let result;
    let operation_mode: c_int = 0o755; // 493

    if flags.optimize {
        result = val1 + val2;
    } else {
        result = (val1 * flags.log_level as c_int) + (val2 / 2);
    }

    if flags.debug {
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

    if flags.verbose {
        adjusted <<= 1;
    }

    if flags.cache_enabled {
        adjusted |= 0x0F;
    }

    adjusted
}

unsafe fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        static stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState {
        flags: ConfigFlags {
            verbose: false,
            debug: false,
            optimize: false,
            cache_enabled: false,
            log_level: 0,
        },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };

    init_config_from_env(&mut state.flags);

    let base_offset = parse_env_numeric("PROG_BASE_OFFSET", 0o100); // 64
    let multiplier = parse_env_numeric("PROG_MULTIPLIER", 0o12);    // 10

    if state.flags.verbose {
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
    state.operation = b'+';

    let state_backup = state;

    if state.flags.debug {
        unsafe {
            libc::printf(
                b"Debug: Created state backup using memcpy\n\0".as_ptr() as *const c_char,
            );
            libc::printf(
                b"Debug: Backup base_value = %d\n\0".as_ptr() as *const c_char,
                state_backup.base_value,
            );
        }
    }

    let mut result = perform_operation(param1, param2, &state.flags);

    if param3 != 0 {
        result += param3 * state.multiplier;
    }

    if param4 != 0 {
        result += param4 >> 2;
    }

    result = apply_bit_operations(result, &state.flags);

    result += base_offset;

    let mut buffer = [0u8; BUFFER_SIZE];
    unsafe {
        libc::snprintf(
            buffer.as_mut_ptr() as *mut c_char,
            BUFFER_SIZE,
            b"Result:%d:Complete\0".as_ptr() as *const c_char,
            result,
        );
    }

    // strchr for ':'
    if let Some(colon_pos) = buffer.iter().position(|&b| b == b':') {
        if state.flags.verbose {
            unsafe {
                libc::printf(
                    b"Found colon at position: %ld\n\0".as_ptr() as *const c_char,
                    colon_pos as libc::c_long,
                );
            }
        }

        if let Some(_) = buffer[colon_pos + 1..].iter().position(|&b| b == b':') {
            if state.flags.debug {
                unsafe {
                    libc::printf(
                        b"Debug: Result string format validated\n\0".as_ptr() as *const c_char,
                    );
                }
            }
        }
    }

    if result < 0 {
        state = state_backup;
        result = state.base_value;

        if state.flags.verbose {
            unsafe {
                libc::printf(b"Restored state from backup\n\0".as_ptr() as *const c_char);
            }
        }
    }

    if state.flags.verbose {
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
