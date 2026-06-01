// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of c_src/src/lib.c — byte-identical output required.
//
// Notes on bit-field layout: GCC on x86_64-linux packs unsigned-int bit-fields
// LSB-first within a 4-byte storage unit. The Rust ConfigFlags struct mirrors
// this with a single repr(C) u32, accessed via shift/mask helpers.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_long};

const BUFFER_SIZE: usize = 256;

extern "C" {
    fn getenv(name: *const c_char) -> *mut c_char;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
}

unsafe fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    stderr
}

// ----- ConfigFlags (matches GCC's bit-field layout) -----

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ConfigFlags {
    pub bits: u32,
}

// Bit positions / widths for fields, in the order they appear in the C struct.
const VERBOSE_SHIFT: u32 = 0;
const VERBOSE_BITS: u32 = 1;
const DEBUG_SHIFT: u32 = 1;
const DEBUG_BITS: u32 = 1;
const OPTIMIZE_SHIFT: u32 = 2;
const OPTIMIZE_BITS: u32 = 1;
const CACHE_SHIFT: u32 = 3;
const CACHE_BITS: u32 = 1;
const LOG_LEVEL_SHIFT: u32 = 4;
const LOG_LEVEL_BITS: u32 = 3;
const RESERVED_SHIFT: u32 = 7;
const RESERVED_BITS: u32 = 1;

#[inline]
fn mask(bits: u32) -> u32 {
    (1u32 << bits) - 1
}

impl ConfigFlags {
    #[inline]
    fn get(&self, shift: u32, bits: u32) -> u32 {
        (self.bits >> shift) & mask(bits)
    }
    #[inline]
    fn set(&mut self, shift: u32, bits: u32, value: u32) {
        let m = mask(bits) << shift;
        self.bits = (self.bits & !m) | ((value & mask(bits)) << shift);
    }

    #[inline]
    fn verbose(&self) -> u32 {
        self.get(VERBOSE_SHIFT, VERBOSE_BITS)
    }
    #[inline]
    fn debug(&self) -> u32 {
        self.get(DEBUG_SHIFT, DEBUG_BITS)
    }
    #[inline]
    fn optimize(&self) -> u32 {
        self.get(OPTIMIZE_SHIFT, OPTIMIZE_BITS)
    }
    #[inline]
    fn cache_enabled(&self) -> u32 {
        self.get(CACHE_SHIFT, CACHE_BITS)
    }
    #[inline]
    fn log_level(&self) -> u32 {
        self.get(LOG_LEVEL_SHIFT, LOG_LEVEL_BITS)
    }

    #[inline]
    fn set_verbose(&mut self, v: u32) {
        self.set(VERBOSE_SHIFT, VERBOSE_BITS, v);
    }
    #[inline]
    fn set_debug(&mut self, v: u32) {
        self.set(DEBUG_SHIFT, DEBUG_BITS, v);
    }
    #[inline]
    fn set_optimize(&mut self, v: u32) {
        self.set(OPTIMIZE_SHIFT, OPTIMIZE_BITS, v);
    }
    #[inline]
    fn set_cache_enabled(&mut self, v: u32) {
        self.set(CACHE_SHIFT, CACHE_BITS, v);
    }
    #[inline]
    fn set_log_level(&mut self, v: u32) {
        self.set(LOG_LEVEL_SHIFT, LOG_LEVEL_BITS, v);
    }
    #[inline]
    fn set_reserved(&mut self, v: u32) {
        self.set(RESERVED_SHIFT, RESERVED_BITS, v);
    }
}

// ----- ProcessState (mirrors C struct layout) -----

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessState {
    pub flags: ConfigFlags,
    pub base_value: c_int,
    pub multiplier: c_int,
    pub operation: c_char,
    // Trailing padding to round size up to 4-byte alignment of int. Keep it
    // explicit so layout is deterministic.
    _pad: [u8; 3],
}

impl ProcessState {
    fn new() -> Self {
        Self {
            flags: ConfigFlags::default(),
            base_value: 0,
            multiplier: 0,
            operation: 0,
            _pad: [0; 3],
        }
    }
}

// ----- parse_env_numeric -----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    let env_value = getenv(env_name);
    if env_value.is_null() {
        return default_val;
    }

    let comma = b',' as c_int;
    let semi = b';' as c_int;

    let invalid_char = strchr(env_value, comma);
    if !invalid_char.is_null() {
        let fmt = b"Warning: Invalid character in %s\n\0".as_ptr() as *const c_char;
        fprintf(libc_stderr(), fmt, env_name);
        return default_val;
    }

    let invalid_char = strchr(env_value, semi);
    if !invalid_char.is_null() {
        let fmt = b"Warning: Semicolon found in %s\n\0".as_ptr() as *const c_char;
        fprintf(libc_stderr(), fmt, env_name);
        return default_val;
    }

    atoi(env_value)
}

// ----- init_config_from_env -----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    let verbose_env = getenv(b"PROG_VERBOSE\0".as_ptr() as *const c_char);
    let debug_env = getenv(b"PROG_DEBUG\0".as_ptr() as *const c_char);
    let optimize_env = getenv(b"PROG_OPTIMIZE\0".as_ptr() as *const c_char);

    let one = b'1' as c_int;

    let f = &mut *flags;
    f.set_verbose(if !verbose_env.is_null() && !strchr(verbose_env, one).is_null() {
        1
    } else {
        0
    });
    f.set_debug(if !debug_env.is_null() && !strchr(debug_env, one).is_null() {
        1
    } else {
        0
    });
    f.set_optimize(if !optimize_env.is_null() { 1 } else { 0 });
    f.set_cache_enabled(1);
    f.set_log_level(0o3);
    f.set_reserved(0);
}

// ----- perform_operation -----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let result: c_int;
    let operation_mode: c_int = 0o755;

    let f = &*flags;

    if f.optimize() != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // log_level is 3 bits; promote to int as in C.
        let log_level_i = f.log_level() as c_int;
        let mul = val1.wrapping_mul(log_level_i);
        let div = val2 / 2;
        result = mul.wrapping_add(div);
    }

    if f.debug() != 0 {
        printf(
            b"Debug: operation_mode = %o (octal)\n\0".as_ptr() as *const c_char,
            operation_mode,
        );
        printf(
            b"Debug: result before adjustment = %d\n\0".as_ptr() as *const c_char,
            result,
        );
    }

    result
}

// ----- apply_bit_operations -----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(value: c_int, flags: *mut ConfigFlags) -> c_int {
    let mut adjusted = value;
    let f = &*flags;

    if f.verbose() != 0 {
        adjusted = adjusted.wrapping_shl(1);
    }

    if f.cache_enabled() != 0 {
        adjusted |= 0x0F;
    }

    adjusted
}

// ----- envy -----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn envy(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut state = ProcessState::new();
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut result: c_int;

    init_config_from_env(&mut state.flags as *mut ConfigFlags);

    let base_offset = parse_env_numeric(
        b"PROG_BASE_OFFSET\0".as_ptr() as *const c_char,
        0o100,
    );
    let multiplier =
        parse_env_numeric(b"PROG_MULTIPLIER\0".as_ptr() as *const c_char, 0o12);

    if state.flags.verbose() != 0 {
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

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    // memcpy(&state_backup, &state, sizeof(...))
    let state_backup: ProcessState = state;

    if state.flags.debug() != 0 {
        printf(b"Debug: Created state backup using memcpy\n\0".as_ptr() as *const c_char);
        printf(
            b"Debug: Backup base_value = %d\n\0".as_ptr() as *const c_char,
            state_backup.base_value,
        );
    }

    result = perform_operation(param1, param2, &mut state.flags as *mut ConfigFlags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        // Signed right shift: in C this is implementation-defined for negative
        // values, but on x86_64 with gcc/clang it is arithmetic. Rust's i32 >>
        // is also arithmetic. Use direct shift to match.
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &mut state.flags as *mut ConfigFlags);

    result = result.wrapping_add(base_offset);

    snprintf(
        buffer.as_mut_ptr() as *mut c_char,
        BUFFER_SIZE,
        b"Result:%d:Complete\0".as_ptr() as *const c_char,
        result,
    );

    let buf_ptr = buffer.as_ptr() as *const c_char;
    let colon = b':' as c_int;
    let colon_pos = strchr(buf_ptr, colon);
    if !colon_pos.is_null() {
        if state.flags.verbose() != 0 {
            let diff: c_long = (colon_pos as isize - buf_ptr as isize) as c_long;
            printf(
                b"Found colon at position: %ld\n\0".as_ptr() as *const c_char,
                diff,
            );
        }

        let second_colon = strchr(colon_pos.add(1), colon);
        if !second_colon.is_null() && state.flags.debug() != 0 {
            printf(b"Debug: Result string format validated\n\0".as_ptr() as *const c_char);
        }
    }

    if result < 0 {
        // memcpy(&state, &state_backup, ...)
        state = state_backup;
        result = state.base_value;

        if state.flags.verbose() != 0 {
            printf(b"Restored state from backup\n\0".as_ptr() as *const c_char);
        }
    }

    if state.flags.verbose() != 0 {
        printf(
            b"Final result: %d\n\0".as_ptr() as *const c_char,
            result,
        );
        printf(
            b"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n\0".as_ptr() as *const c_char,
            state.flags.debug() as c_int,
            state.flags.optimize() as c_int,
            state.flags.log_level() as c_int,
        );
    }

    // Touch CStr to suppress unused-import warnings on minimal builds.
    let _ = CStr::from_bytes_with_nul(b"\0");

    result
}
