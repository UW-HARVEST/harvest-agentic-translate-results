// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust preserving byte-identical output.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

const BUFFER_SIZE: usize = 256;

/// Mirrors the C bit-field struct layout used by the GCC/Clang ABI:
///   verbose      : bit 0
///   debug        : bit 1
///   optimize     : bit 2
///   cache_enabled: bit 3
///   log_level    : bits 4..6 (3 bits)
///   reserved     : bit 7
/// The bit-field is stored in a single 32-bit unit (4-byte alignment),
/// matching `sizeof(struct ConfigFlags) == 4`.
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ConfigFlags {
    pub bits: u32,
}

impl ConfigFlags {
    #[inline]
    fn get_verbose(&self) -> u32 {
        self.bits & 0x1
    }
    #[inline]
    fn set_verbose(&mut self, v: u32) {
        self.bits = (self.bits & !0x1) | (v & 0x1);
    }
    #[inline]
    fn get_debug(&self) -> u32 {
        (self.bits >> 1) & 0x1
    }
    #[inline]
    fn set_debug(&mut self, v: u32) {
        self.bits = (self.bits & !0x2) | ((v & 0x1) << 1);
    }
    #[inline]
    fn get_optimize(&self) -> u32 {
        (self.bits >> 2) & 0x1
    }
    #[inline]
    fn set_optimize(&mut self, v: u32) {
        self.bits = (self.bits & !0x4) | ((v & 0x1) << 2);
    }
    #[inline]
    fn get_cache_enabled(&self) -> u32 {
        (self.bits >> 3) & 0x1
    }
    #[inline]
    fn set_cache_enabled(&mut self, v: u32) {
        self.bits = (self.bits & !0x8) | ((v & 0x1) << 3);
    }
    #[inline]
    fn get_log_level(&self) -> u32 {
        (self.bits >> 4) & 0x7
    }
    #[inline]
    fn set_log_level(&mut self, v: u32) {
        self.bits = (self.bits & !0x70) | ((v & 0x7) << 4);
    }
    #[inline]
    fn set_reserved(&mut self, v: u32) {
        self.bits = (self.bits & !0x80) | ((v & 0x1) << 7);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
    // C ABI pads ProcessState up to 16 bytes (3 bytes of trailing padding).
    _pad: [u8; 3],
}

unsafe extern "C" {
    static stderr: *mut libc::FILE;
}

/// Lookup environment variable using libc::getenv to mirror C semantics
/// (returns NULL when unset; returns empty C string when set to empty value).
fn c_getenv(name_with_nul: &[u8]) -> Option<&'static CStr> {
    debug_assert!(name_with_nul.last() == Some(&0));
    unsafe {
        let ptr = libc::getenv(name_with_nul.as_ptr() as *const c_char);
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr))
        }
    }
}

fn c_strchr(s: &CStr, ch: u8) -> Option<usize> {
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
    (val.wrapping_mul(sign)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_env_numeric(
    env_name: *const c_char,
    default_val: c_int,
) -> c_int {
    if env_name.is_null() {
        return default_val;
    }
    let env_value_ptr = unsafe { libc::getenv(env_name) };
    if env_value_ptr.is_null() {
        return default_val;
    }
    let env_value = unsafe { CStr::from_ptr(env_value_ptr) };

    if c_strchr(env_value, b',').is_some() {
        unsafe {
            libc::fprintf(
                stderr,
                b"Warning: Invalid character in %s\n\0".as_ptr() as *const c_char,
                env_name,
            );
        }
        return default_val;
    }

    if c_strchr(env_value, b';').is_some() {
        unsafe {
            libc::fprintf(
                stderr,
                b"Warning: Semicolon found in %s\n\0".as_ptr() as *const c_char,
                env_name,
            );
        }
        return default_val;
    }

    c_atoi(env_value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    let flags = unsafe { &mut *flags };
    let verbose_env = c_getenv(b"PROG_VERBOSE\0");
    let debug_env = c_getenv(b"PROG_DEBUG\0");
    let optimize_env = c_getenv(b"PROG_OPTIMIZE\0");

    flags.set_verbose(match verbose_env {
        Some(v) if c_strchr(v, b'1').is_some() => 1,
        _ => 0,
    });
    flags.set_debug(match debug_env {
        Some(v) if c_strchr(v, b'1').is_some() => 1,
        _ => 0,
    });
    flags.set_optimize(if optimize_env.is_some() { 1 } else { 0 });
    flags.set_cache_enabled(1);
    flags.set_log_level(0o3);
    flags.set_reserved(0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let flags = unsafe { &*flags };
    let result: c_int;

    let operation_mode: c_int = 0o755;

    if flags.get_optimize() != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // log_level is a 3-bit unsigned bit-field, promoted to signed int in C.
        result = val1
            .wrapping_mul(flags.get_log_level() as c_int)
            .wrapping_add(val2 / 2);
    }

    if flags.get_debug() != 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(
    value: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let flags = unsafe { &*flags };
    let mut adjusted = value;

    if flags.get_verbose() != 0 {
        adjusted = ((adjusted as u32) << 1) as c_int;
    }

    if flags.get_cache_enabled() != 0 {
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

    unsafe {
        init_config_from_env(&mut state.flags);
    }

    let base_offset = unsafe {
        parse_env_numeric(b"PROG_BASE_OFFSET\0".as_ptr() as *const c_char, 0o100)
    };
    let multiplier = unsafe {
        parse_env_numeric(b"PROG_MULTIPLIER\0".as_ptr() as *const c_char, 0o12)
    };

    if state.flags.get_verbose() != 0 {
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

    if state.flags.get_debug() != 0 {
        unsafe {
            libc::printf(b"Debug: Created state backup using memcpy\n\0".as_ptr() as *const c_char);
            libc::printf(
                b"Debug: Backup base_value = %d\n\0".as_ptr() as *const c_char,
                state_backup.base_value,
            );
        }
    }

    result = unsafe { perform_operation(param1, param2, &mut state.flags) };

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        // Arithmetic right shift on signed int (matches C on gcc/clang).
        result = result.wrapping_add(param4 >> 2);
    }

    result = unsafe { apply_bit_operations(result, &mut state.flags) };

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

    let buffer_cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    if let Some(first_pos) = c_strchr(buffer_cstr, b':') {
        if state.flags.get_verbose() != 0 {
            unsafe {
                libc::printf(
                    b"Found colon at position: %ld\n\0".as_ptr() as *const c_char,
                    first_pos as libc::c_long,
                );
            }
        }

        let after_first = &buffer_cstr.to_bytes()[first_pos + 1..];
        let second_colon = after_first.iter().position(|&b| b == b':');
        if second_colon.is_some() && state.flags.get_debug() != 0 {
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

        if state.flags.get_verbose() != 0 {
            unsafe {
                libc::printf(b"Restored state from backup\n\0".as_ptr() as *const c_char);
            }
        }
    }

    if state.flags.get_verbose() != 0 {
        unsafe {
            libc::printf(
                b"Final result: %d\n\0".as_ptr() as *const c_char,
                result,
            );
            libc::printf(
                b"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n\0".as_ptr()
                    as *const c_char,
                state.flags.get_debug() as c_int,
                state.flags.get_optimize() as c_int,
                state.flags.get_log_level() as c_int,
            );
        }
    }

    result
}
