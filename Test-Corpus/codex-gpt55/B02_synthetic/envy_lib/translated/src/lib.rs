use std::ffi::{c_char, c_int};
use std::ptr;

const BUFFER_SIZE: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConfigFlags {
    bits: libc::c_uint,
}

impl ConfigFlags {
    fn verbose(&self) -> bool {
        self.bits & 0x01 != 0
    }

    fn set_verbose(&mut self, value: bool) {
        self.set_bit(0, value);
    }

    fn debug(&self) -> bool {
        self.bits & 0x02 != 0
    }

    fn set_debug(&mut self, value: bool) {
        self.set_bit(1, value);
    }

    fn optimize(&self) -> bool {
        self.bits & 0x04 != 0
    }

    fn set_optimize(&mut self, value: bool) {
        self.set_bit(2, value);
    }

    fn cache_enabled(&self) -> bool {
        self.bits & 0x08 != 0
    }

    fn set_cache_enabled(&mut self, value: bool) {
        self.set_bit(3, value);
    }

    fn log_level(&self) -> c_int {
        ((self.bits >> 4) & 0x07) as c_int
    }

    fn set_log_level(&mut self, value: libc::c_uint) {
        self.bits = (self.bits & !(0x07 << 4)) | ((value & 0x07) << 4);
    }

    fn set_reserved(&mut self, value: bool) {
        self.set_bit(7, value);
    }

    fn set_bit(&mut self, bit: libc::c_uint, value: bool) {
        let mask = 1u32 << bit;
        if value {
            self.bits |= mask;
        } else {
            self.bits &= !mask;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
}

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;

    fn getenv(name: *const c_char) -> *mut c_char;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn atoi(nptr: *const c_char) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: libc::size_t, format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    let env_value = unsafe { getenv(env_name) };

    if env_value.is_null() {
        return default_val;
    }

    let mut invalid_char = unsafe { strchr(env_value, b',' as c_int) };
    if !invalid_char.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Warning: Invalid character in %s\n".as_ptr(),
                env_name,
            );
        }
        return default_val;
    }

    invalid_char = unsafe { strchr(env_value, b';' as c_int) };
    if !invalid_char.is_null() {
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
pub unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    let verbose_env = unsafe { getenv(c"PROG_VERBOSE".as_ptr()) };
    let debug_env = unsafe { getenv(c"PROG_DEBUG".as_ptr()) };
    let optimize_env = unsafe { getenv(c"PROG_OPTIMIZE".as_ptr()) };

    let flags = unsafe { &mut *flags };
    flags.set_verbose(
        !verbose_env.is_null() && unsafe { !strchr(verbose_env, b'1' as c_int).is_null() },
    );
    flags.set_debug(!debug_env.is_null() && unsafe { !strchr(debug_env, b'1' as c_int).is_null() });
    flags.set_optimize(!optimize_env.is_null());
    flags.set_cache_enabled(true);
    flags.set_log_level(0o3);
    flags.set_reserved(false);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let result: c_int;
    let operation_mode: c_int = 0o755;
    let flags_ref = unsafe { &mut *flags };

    if flags_ref.optimize() {
        result = val1.wrapping_add(val2);
    } else {
        result = val1
            .wrapping_mul(flags_ref.log_level())
            .wrapping_add(val2 / 2);
    }

    if flags_ref.debug() {
        unsafe {
            printf(
                c"Debug: operation_mode = %o (octal)\n".as_ptr(),
                operation_mode as libc::c_uint,
            );
            printf(c"Debug: result before adjustment = %d\n".as_ptr(), result);
        }
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(value: c_int, flags: *mut ConfigFlags) -> c_int {
    let mut adjusted = value;
    let flags_ref = unsafe { &mut *flags };

    if flags_ref.verbose() {
        adjusted = adjusted.wrapping_shl(1);
    }

    if flags_ref.cache_enabled() {
        adjusted |= 0x0F;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = ProcessState {
        flags: ConfigFlags { bits: 0 },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let mut buffer = [0 as c_char; BUFFER_SIZE];
    let mut result: c_int;

    unsafe { init_config_from_env(ptr::addr_of_mut!(state.flags)) };

    let base_offset = unsafe { parse_env_numeric(c"PROG_BASE_OFFSET".as_ptr(), 0o100) };
    let multiplier = unsafe { parse_env_numeric(c"PROG_MULTIPLIER".as_ptr(), 0o12) };

    if state.flags.verbose() {
        unsafe {
            printf(c"Verbose mode enabled\n".as_ptr());
            printf(c"Base offset: %d (from octal 0100)\n".as_ptr(), base_offset);
            printf(c"Multiplier: %d (from octal 012)\n".as_ptr(), multiplier);
        }
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    let state_backup = state;

    if state.flags.debug() {
        unsafe {
            printf(c"Debug: Created state backup using memcpy\n".as_ptr());
            printf(
                c"Debug: Backup base_value = %d\n".as_ptr(),
                state_backup.base_value,
            );
        }
    }

    result = unsafe { perform_operation(param1, param2, ptr::addr_of_mut!(state.flags)) };

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        result = result.wrapping_add(param4 >> 2);
    }

    result = unsafe { apply_bit_operations(result, ptr::addr_of_mut!(state.flags)) };
    result = result.wrapping_add(base_offset);

    unsafe {
        snprintf(
            buffer.as_mut_ptr(),
            BUFFER_SIZE,
            c"Result:%d:Complete".as_ptr(),
            result,
        );
    }

    let colon_pos = unsafe { strchr(buffer.as_ptr(), b':' as c_int) };
    if !colon_pos.is_null() {
        if state.flags.verbose() {
            let position = unsafe { colon_pos.offset_from(buffer.as_ptr()) } as libc::c_long;
            unsafe {
                printf(c"Found colon at position: %ld\n".as_ptr(), position);
            }
        }

        let second_colon = unsafe { strchr(colon_pos.add(1), b':' as c_int) };
        if !second_colon.is_null() && state.flags.debug() {
            unsafe {
                printf(c"Debug: Result string format validated\n".as_ptr());
            }
        }
    }

    if result < 0 {
        state = state_backup;
        result = state.base_value;

        if state.flags.verbose() {
            unsafe {
                printf(c"Restored state from backup\n".as_ptr());
            }
        }
    }

    if state.flags.verbose() {
        unsafe {
            printf(c"Final result: %d\n".as_ptr(), result);
            printf(
                c"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n".as_ptr(),
                state.flags.debug() as c_int,
                state.flags.optimize() as c_int,
                state.flags.log_level(),
            );
        }
    }

    result
}
