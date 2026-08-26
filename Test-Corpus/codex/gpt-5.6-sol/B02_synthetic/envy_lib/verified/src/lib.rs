use std::ffi::{c_char, c_int, c_long, c_uint, c_void};
use std::ptr;

#[repr(C)]
struct ConfigFlags {
    storage: c_uint,
}

impl ConfigFlags {
    const VERBOSE: c_uint = 1 << 0;
    const DEBUG: c_uint = 1 << 1;
    const OPTIMIZE: c_uint = 1 << 2;
    const CACHE_ENABLED: c_uint = 1 << 3;
    const LOG_LEVEL_SHIFT: u32 = 4;
    const LOG_LEVEL_MASK: c_uint = 0b111 << Self::LOG_LEVEL_SHIFT;

    fn is_set(&self, mask: c_uint) -> bool {
        self.storage & mask != 0
    }

    fn log_level(&self) -> c_int {
        ((self.storage & Self::LOG_LEVEL_MASK) >> Self::LOG_LEVEL_SHIFT) as c_int
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
}

impl Clone for ConfigFlags {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for ConfigFlags {}

unsafe extern "C" {
    static mut stderr: *mut c_void;

    fn atoi(value: *const c_char) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn getenv(name: *const c_char) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(value: *const c_char) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strchr(value: *const c_char, character: c_int) -> *mut c_char;
}

unsafe fn fault_on_null_like_c() -> ! {
    // Use a synchronous libc fault so debug UB checks do not change SIGSEGV to SIGABRT.
    unsafe {
        getenv(ptr::null());
        std::hint::unreachable_unchecked();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    let env_value = unsafe { getenv(env_name) };

    if env_value.is_null() {
        return default_val;
    }

    if !unsafe { strchr(env_value, c_int::from(b',')) }.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Warning: Invalid character in %s\n".as_ptr(),
                env_name,
            );
        }
        return default_val;
    }

    if !unsafe { strchr(env_value, c_int::from(b';')) }.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Warning: Semicolon found in %s\n".as_ptr(),
                env_name,
            );
        }
        return default_val;
    }

    unsafe { atoi(env_value) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    let verbose_env = unsafe { getenv(c"PROG_VERBOSE".as_ptr()) };
    let debug_env = unsafe { getenv(c"PROG_DEBUG".as_ptr()) };
    let optimize_env = unsafe { getenv(c"PROG_OPTIMIZE".as_ptr()) };

    let verbose =
        !verbose_env.is_null() && !unsafe { strchr(verbose_env, c_int::from(b'1')) }.is_null();
    let debug = !debug_env.is_null() && !unsafe { strchr(debug_env, c_int::from(b'1')) }.is_null();

    let mut low_byte = ConfigFlags::CACHE_ENABLED | (3 << ConfigFlags::LOG_LEVEL_SHIFT);
    if verbose {
        low_byte |= ConfigFlags::VERBOSE;
    }
    if debug {
        low_byte |= ConfigFlags::DEBUG;
    }
    if !optimize_env.is_null() {
        low_byte |= ConfigFlags::OPTIMIZE;
    }

    let previous = unsafe { ptr::read(flags) }.storage;
    unsafe {
        ptr::write(
            flags,
            ConfigFlags {
                storage: (previous & !0xff) | low_byte,
            },
        );
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn perform_operation(val1: c_int, val2: c_int, flags: *mut ConfigFlags) -> c_int {
    if flags.is_null() {
        unsafe { fault_on_null_like_c() }
    }
    let flags = unsafe { &*flags };
    let result = if flags.is_set(ConfigFlags::OPTIMIZE) {
        val1.wrapping_add(val2)
    } else {
        val1.wrapping_mul(flags.log_level()).wrapping_add(val2 / 2)
    };

    if flags.is_set(ConfigFlags::DEBUG) {
        unsafe {
            printf(
                c"Debug: operation_mode = %o (octal)\n".as_ptr(),
                0o755 as c_uint,
            );
            printf(c"Debug: result before adjustment = %d\n".as_ptr(), result);
        }
    }

    result
}

#[unsafe(no_mangle)]
unsafe extern "C" fn apply_bit_operations(value: c_int, flags: *mut ConfigFlags) -> c_int {
    if flags.is_null() {
        unsafe { fault_on_null_like_c() }
    }
    let flags = unsafe { &*flags };
    let mut adjusted = value;

    if flags.is_set(ConfigFlags::VERBOSE) {
        adjusted = adjusted.wrapping_shl(1);
    }

    if flags.is_set(ConfigFlags::CACHE_ENABLED) {
        adjusted |= 0x0f;
    }

    adjusted
}

#[unsafe(no_mangle)]
unsafe extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState {
        flags: ConfigFlags { storage: 0 },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let mut buffer = [0 as c_char; 256];

    unsafe { init_config_from_env(&mut state.flags) };

    let base_offset = unsafe { parse_env_numeric(c"PROG_BASE_OFFSET".as_ptr(), 0o100) };
    let multiplier = unsafe { parse_env_numeric(c"PROG_MULTIPLIER".as_ptr(), 0o12) };

    if state.flags.is_set(ConfigFlags::VERBOSE) {
        unsafe {
            puts(c"Verbose mode enabled".as_ptr());
            printf(c"Base offset: %d (from octal 0100)\n".as_ptr(), base_offset);
            printf(c"Multiplier: %d (from octal 012)\n".as_ptr(), multiplier);
        }
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    let state_backup = state;

    if state.flags.is_set(ConfigFlags::DEBUG) {
        unsafe {
            puts(c"Debug: Created state backup using memcpy".as_ptr());
            printf(
                c"Debug: Backup base_value = %d\n".as_ptr(),
                state_backup.base_value,
            );
        }
    }

    let mut result = unsafe { perform_operation(param1, param2, &mut state.flags) };

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        result = result.wrapping_add(param4 >> 2);
    }

    result = unsafe { apply_bit_operations(result, &mut state.flags) };
    result = result.wrapping_add(base_offset);

    unsafe {
        snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            c"Result:%d:Complete".as_ptr(),
            result,
        );
    }

    let colon_pos = unsafe { strchr(buffer.as_ptr(), c_int::from(b':')) };
    if !colon_pos.is_null() {
        if state.flags.is_set(ConfigFlags::VERBOSE) {
            let position = unsafe { colon_pos.offset_from(buffer.as_ptr()) } as c_long;
            unsafe {
                printf(c"Found colon at position: %ld\n".as_ptr(), position);
            }
        }

        let second_colon = unsafe { strchr(colon_pos.add(1), c_int::from(b':')) };
        if !second_colon.is_null() && state.flags.is_set(ConfigFlags::DEBUG) {
            unsafe {
                puts(c"Debug: Result string format validated".as_ptr());
            }
        }
    }

    if result < 0 {
        state = state_backup;
        result = state.base_value;

        if state.flags.is_set(ConfigFlags::VERBOSE) {
            unsafe {
                puts(c"Restored state from backup".as_ptr());
            }
        }
    }

    if state.flags.is_set(ConfigFlags::VERBOSE) {
        unsafe {
            printf(c"Final result: %d\n".as_ptr(), result);
            printf(
                c"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n".as_ptr(),
                c_int::from(state.flags.is_set(ConfigFlags::DEBUG)),
                c_int::from(state.flags.is_set(ConfigFlags::OPTIMIZE)),
                state.flags.log_level(),
            );
        }
    }

    result
}
