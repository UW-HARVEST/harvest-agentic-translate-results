use std::ffi::{c_int, CStr};
use std::os::raw::c_char;

const BUFFER_SIZE: usize = 256;

/// C-compatible bitfield layout for ConfigFlags.
/// In C: verbose:1, debug:1, optimize:1, cache_enabled:1, log_level:3, reserved:1
/// Packed into a single u32.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CConfigFlags {
    pub bits: u32,
}

#[derive(Clone, Copy)]
struct ConfigFlags {
    verbose: bool,
    debug: bool,
    optimize: bool,
    cache_enabled: bool,
    log_level: u8,
}

impl ConfigFlags {
    fn to_c(&self) -> CConfigFlags {
        let mut bits: u32 = 0;
        if self.verbose { bits |= 1 << 0; }
        if self.debug { bits |= 1 << 1; }
        if self.optimize { bits |= 1 << 2; }
        if self.cache_enabled { bits |= 1 << 3; }
        bits |= (self.log_level as u32 & 0x7) << 4;
        CConfigFlags { bits }
    }

    fn from_c(c: &CConfigFlags) -> Self {
        ConfigFlags {
            verbose: (c.bits & (1 << 0)) != 0,
            debug: (c.bits & (1 << 1)) != 0,
            optimize: (c.bits & (1 << 2)) != 0,
            cache_enabled: (c.bits & (1 << 3)) != 0,
            log_level: ((c.bits >> 4) & 0x7) as u8,
        }
    }
}

#[derive(Clone, Copy)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    _operation: u8,
}

unsafe fn c_getenv(name: &[u8]) -> *const c_char {
    unsafe { libc::getenv(name.as_ptr() as *const c_char) }
}

unsafe fn c_strchr(s: *const c_char, c: c_int) -> *const c_char {
    unsafe { libc::strchr(s, c) }
}

unsafe fn libc_stderr() -> *mut libc::FILE {
    unsafe { libc::fdopen(2, b"w\0".as_ptr() as *const c_char) }
}

fn parse_env_numeric_impl(env_name: &[u8], default_val: c_int) -> c_int {
    unsafe {
        let env_value = c_getenv(env_name);
        if env_value.is_null() {
            return default_val;
        }

        let invalid_char = c_strchr(env_value, b',' as c_int);
        if !invalid_char.is_null() {
            let name = CStr::from_ptr(env_name.as_ptr() as *const c_char);
            libc::fprintf(
                libc_stderr(),
                b"Warning: Invalid character in %s\n\0".as_ptr() as *const c_char,
                name.as_ptr(),
            );
            return default_val;
        }

        let invalid_char = c_strchr(env_value, b';' as c_int);
        if !invalid_char.is_null() {
            let name = CStr::from_ptr(env_name.as_ptr() as *const c_char);
            libc::fprintf(
                libc_stderr(),
                b"Warning: Semicolon found in %s\n\0".as_ptr() as *const c_char,
                name.as_ptr(),
            );
            return default_val;
        }

        libc::atoi(env_value)
    }
}

fn init_config_from_env_impl(flags: &mut ConfigFlags) {
    unsafe {
        let verbose_env = c_getenv(b"PROG_VERBOSE\0");
        let debug_env = c_getenv(b"PROG_DEBUG\0");
        let optimize_env = c_getenv(b"PROG_OPTIMIZE\0");

        flags.verbose =
            !verbose_env.is_null() && !c_strchr(verbose_env, b'1' as c_int).is_null();
        flags.debug =
            !debug_env.is_null() && !c_strchr(debug_env, b'1' as c_int).is_null();
        flags.optimize = !optimize_env.is_null();
        flags.cache_enabled = true;
        flags.log_level = 3;
    }
}

fn perform_operation_impl(val1: c_int, val2: c_int, flags: &ConfigFlags) -> c_int {
    let result;
    let operation_mode: c_int = 0o755;

    if flags.optimize {
        result = val1.wrapping_add(val2);
    } else {
        result = (val1.wrapping_mul(flags.log_level as c_int)).wrapping_add(val2 / 2);
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

fn apply_bit_operations_impl(value: c_int, flags: &ConfigFlags) -> c_int {
    let mut adjusted = value;

    if flags.verbose {
        adjusted <<= 1;
    }

    if flags.cache_enabled {
        adjusted |= 0x0F;
    }

    adjusted
}

// ---- Public C-ABI exports ----

#[unsafe(no_mangle)]
pub extern "C" fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    let name = unsafe { CStr::from_ptr(env_name) };
    parse_env_numeric_impl(name.to_bytes_with_nul(), default_val)
}

#[unsafe(no_mangle)]
pub extern "C" fn init_config_from_env(flags: *mut CConfigFlags) {
    let mut rf = ConfigFlags::from_c(unsafe { &*flags });
    init_config_from_env_impl(&mut rf);
    unsafe { *flags = rf.to_c(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn perform_operation(val1: c_int, val2: c_int, flags: *const CConfigFlags) -> c_int {
    let rf = ConfigFlags::from_c(unsafe { &*flags });
    perform_operation_impl(val1, val2, &rf)
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_bit_operations(value: c_int, flags: *const CConfigFlags) -> c_int {
    let rf = ConfigFlags::from_c(unsafe { &*flags });
    apply_bit_operations_impl(value, &rf)
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
        _operation: 0,
    };
    let mut buffer = [0u8; BUFFER_SIZE];

    init_config_from_env_impl(&mut state.flags);

    let base_offset = parse_env_numeric_impl(b"PROG_BASE_OFFSET\0", 0o100);
    let multiplier = parse_env_numeric_impl(b"PROG_MULTIPLIER\0", 0o12);

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
    state._operation = b'+';

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

    let mut result = perform_operation_impl(param1, param2, &state.flags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations_impl(result, &state.flags);

    result = result.wrapping_add(base_offset);

    unsafe {
        libc::snprintf(
            buffer.as_mut_ptr() as *mut c_char,
            BUFFER_SIZE,
            b"Result:%d:Complete\0".as_ptr() as *const c_char,
            result,
        );

        let colon_pos = c_strchr(buffer.as_ptr() as *const c_char, b':' as c_int);
        if !colon_pos.is_null() {
            if state.flags.verbose {
                let offset = colon_pos.offset_from(buffer.as_ptr() as *const c_char);
                libc::printf(
                    b"Found colon at position: %ld\n\0".as_ptr() as *const c_char,
                    offset as libc::c_long,
                );
            }

            let second_colon = c_strchr(colon_pos.add(1), b':' as c_int);
            if !second_colon.is_null() && state.flags.debug {
                libc::printf(
                    b"Debug: Result string format validated\n\0".as_ptr() as *const c_char,
                );
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

/// Public wrapper for testing parse_env_numeric from Rust tests
pub fn parse_env_numeric_wrapper(env_name: &[u8], default_val: c_int) -> c_int {
    parse_env_numeric_impl(env_name, default_val)
}
