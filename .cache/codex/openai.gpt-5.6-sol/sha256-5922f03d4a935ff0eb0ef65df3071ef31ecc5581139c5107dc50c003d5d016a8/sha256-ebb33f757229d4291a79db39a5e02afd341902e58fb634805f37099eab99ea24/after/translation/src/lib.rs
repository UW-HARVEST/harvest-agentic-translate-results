use std::ffi::{c_char, c_int, c_long, c_uint, c_void};

const VERBOSE_MASK: c_uint = 1 << 0;
const DEBUG_MASK: c_uint = 1 << 1;
const OPTIMIZE_MASK: c_uint = 1 << 2;
const CACHE_ENABLED_MASK: c_uint = 1 << 3;
const LOG_LEVEL_MASK: c_uint = 0b111 << 4;
const RESERVED_MASK: c_uint = 1 << 7;
const BUFFER_SIZE: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConfigFlags {
    bits: c_uint,
}

impl ConfigFlags {
    fn field(self, mask: c_uint, shift: u32) -> c_uint {
        (self.bits & mask) >> shift
    }

    fn set_field(&mut self, mask: c_uint, shift: u32, value: c_uint) {
        self.bits = (self.bits & !mask) | ((value << shift) & mask);
    }

    fn verbose(self) -> bool {
        self.field(VERBOSE_MASK, 0) != 0
    }

    fn debug(self) -> bool {
        self.field(DEBUG_MASK, 1) != 0
    }

    fn optimize(self) -> bool {
        self.field(OPTIMIZE_MASK, 2) != 0
    }

    fn cache_enabled(self) -> bool {
        self.field(CACHE_ENABLED_MASK, 3) != 0
    }

    fn log_level(self) -> c_uint {
        self.field(LOG_LEVEL_MASK, 4)
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
    fn getenv(name: *const c_char) -> *mut c_char;
    fn atoi(value: *const c_char) -> c_int;
    fn strchr(value: *const c_char, character: c_int) -> *mut c_char;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;

    #[link_name = "stderr"]
    static mut C_STDERR: *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    let env_value = unsafe { getenv(env_name) };

    if env_value.is_null() {
        return default_val;
    }

    let mut invalid_char = unsafe { strchr(env_value, c_int::from(b',')) };
    if !invalid_char.is_null() {
        unsafe {
            fprintf(
                C_STDERR,
                c"Warning: Invalid character in %s\n".as_ptr(),
                env_name,
            );
        }
        return default_val;
    }

    invalid_char = unsafe { strchr(env_value, c_int::from(b';')) };
    if !invalid_char.is_null() {
        unsafe {
            fprintf(
                C_STDERR,
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

    let verbose =
        !verbose_env.is_null() && !unsafe { strchr(verbose_env, c_int::from(b'1')) }.is_null();
    let debug = !debug_env.is_null() && !unsafe { strchr(debug_env, c_int::from(b'1')) }.is_null();

    unsafe {
        (*flags).set_field(VERBOSE_MASK, 0, c_uint::from(verbose));
        (*flags).set_field(DEBUG_MASK, 1, c_uint::from(debug));
        (*flags).set_field(OPTIMIZE_MASK, 2, c_uint::from(!optimize_env.is_null()));
        (*flags).set_field(CACHE_ENABLED_MASK, 3, 1);
        (*flags).set_field(LOG_LEVEL_MASK, 4, 0o3);
        (*flags).set_field(RESERVED_MASK, 7, 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let flags_value = unsafe { *flags };
    let result = if flags_value.optimize() {
        val1.wrapping_add(val2)
    } else {
        val1.wrapping_mul(flags_value.log_level() as c_int)
            .wrapping_add(val2 / 2)
    };

    if flags_value.debug() {
        let operation_mode: c_int = 0o755;
        unsafe {
            printf(
                c"Debug: operation_mode = %o (octal)\n".as_ptr(),
                operation_mode,
            );
            printf(c"Debug: result before adjustment = %d\n".as_ptr(), result);
        }
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(value: c_int, flags: *mut ConfigFlags) -> c_int {
    let flags_value = unsafe { *flags };
    let mut adjusted = value;

    if flags_value.verbose() {
        adjusted = adjusted.wrapping_shl(1);
    }

    if flags_value.cache_enabled() {
        adjusted |= 0x0f;
    }

    adjusted
}

#[unsafe(no_mangle)]
pub extern "C" fn envy(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        let mut state = ProcessState {
            flags: ConfigFlags { bits: 0 },
            base_value: 0,
            multiplier: 0,
            operation: 0,
        };
        init_config_from_env(&mut state.flags);

        let base_offset = parse_env_numeric(c"PROG_BASE_OFFSET".as_ptr(), 0o100);
        let multiplier = parse_env_numeric(c"PROG_MULTIPLIER".as_ptr(), 0o12);

        if state.flags.verbose() {
            printf(c"Verbose mode enabled\n".as_ptr());
            printf(c"Base offset: %d (from octal 0100)\n".as_ptr(), base_offset);
            printf(c"Multiplier: %d (from octal 012)\n".as_ptr(), multiplier);
        }

        state.base_value = param1;
        state.multiplier = multiplier;
        state.operation = b'+' as c_char;

        let state_backup = state;

        if state.flags.debug() {
            printf(c"Debug: Created state backup using memcpy\n".as_ptr());
            printf(
                c"Debug: Backup base_value = %d\n".as_ptr(),
                state_backup.base_value,
            );
        }

        let mut result = perform_operation(param1, param2, &mut state.flags);

        if param3 != 0 {
            result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
        }

        if param4 != 0 {
            result = result.wrapping_add(param4 >> 2);
        }

        result = apply_bit_operations(result, &mut state.flags);
        result = result.wrapping_add(base_offset);

        let mut buffer = [0 as c_char; BUFFER_SIZE];
        snprintf(
            buffer.as_mut_ptr(),
            BUFFER_SIZE,
            c"Result:%d:Complete".as_ptr(),
            result,
        );

        let colon_pos = strchr(buffer.as_ptr(), c_int::from(b':'));
        if !colon_pos.is_null() {
            if state.flags.verbose() {
                printf(
                    c"Found colon at position: %ld\n".as_ptr(),
                    colon_pos.offset_from(buffer.as_ptr()) as c_long,
                );
            }

            let second_colon = strchr(colon_pos.add(1), c_int::from(b':'));
            if !second_colon.is_null() && state.flags.debug() {
                printf(c"Debug: Result string format validated\n".as_ptr());
            }
        }

        if result < 0 {
            state = state_backup;
            result = state.base_value;

            if state.flags.verbose() {
                printf(c"Restored state from backup\n".as_ptr());
            }
        }

        if state.flags.verbose() {
            printf(c"Final result: %d\n".as_ptr(), result);
            printf(
                c"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n".as_ptr(),
                c_int::from(state.flags.debug()),
                c_int::from(state.flags.optimize()),
                state.flags.log_level() as c_int,
            );
        }

        result
    }
}
