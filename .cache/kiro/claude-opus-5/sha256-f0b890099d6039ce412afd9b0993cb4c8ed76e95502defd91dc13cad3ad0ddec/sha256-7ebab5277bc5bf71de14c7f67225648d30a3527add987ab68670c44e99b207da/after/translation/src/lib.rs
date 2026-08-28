// Rust translation of c_src/src/lib.c
//
// Original copyright header from the C source:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::{c_char, c_int, c_long, c_uint};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C library writes its diagnostics with printf/fprintf on the C `stdout`
// and `stderr` streams. Reusing those same streams (instead of Rust's
// `std::io` wrappers) keeps formatting *and* buffering behaviour identical to
// the original, which is what byte-identical output requires when a caller
// mixes C and Rust output or redirects the streams to a file.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    static mut stderr: *mut FILE;

    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    fn getenv(name: *const c_char) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
}

#[allow(non_camel_case_types)]
pub enum FILE {}

const BUFFER_SIZE: usize = 256;

// ---------------------------------------------------------------------------
// struct ConfigFlags {
//     unsigned int verbose : 1;
//     unsigned int debug : 1;
//     unsigned int optimize : 1;
//     unsigned int cache_enabled : 1;
//     unsigned int log_level : 3;
//     unsigned int reserved : 1;
// };
//
// Modelled as the single 4-byte storage unit the System V x86-64 ABI uses,
// with the fields allocated from the least significant bit upwards.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ConfigFlags {
    bits: c_uint,
}

impl ConfigFlags {
    const VERBOSE_SHIFT: u32 = 0;
    const DEBUG_SHIFT: u32 = 1;
    const OPTIMIZE_SHIFT: u32 = 2;
    const CACHE_ENABLED_SHIFT: u32 = 3;
    const LOG_LEVEL_SHIFT: u32 = 4;
    const RESERVED_SHIFT: u32 = 7;

    #[inline]
    fn get(&self, shift: u32, width: u32) -> c_uint {
        (self.bits >> shift) & ((1u32 << width) - 1)
    }

    #[inline]
    fn set(&mut self, shift: u32, width: u32, value: c_uint) {
        let mask = ((1u32 << width) - 1) << shift;
        self.bits = (self.bits & !mask) | ((value << shift) & mask);
    }

    #[inline]
    fn verbose(&self) -> c_uint {
        self.get(Self::VERBOSE_SHIFT, 1)
    }
    #[inline]
    fn set_verbose(&mut self, v: c_uint) {
        self.set(Self::VERBOSE_SHIFT, 1, v)
    }

    #[inline]
    fn debug(&self) -> c_uint {
        self.get(Self::DEBUG_SHIFT, 1)
    }
    #[inline]
    fn set_debug(&mut self, v: c_uint) {
        self.set(Self::DEBUG_SHIFT, 1, v)
    }

    #[inline]
    fn optimize(&self) -> c_uint {
        self.get(Self::OPTIMIZE_SHIFT, 1)
    }
    #[inline]
    fn set_optimize(&mut self, v: c_uint) {
        self.set(Self::OPTIMIZE_SHIFT, 1, v)
    }

    #[inline]
    fn cache_enabled(&self) -> c_uint {
        self.get(Self::CACHE_ENABLED_SHIFT, 1)
    }
    #[inline]
    fn set_cache_enabled(&mut self, v: c_uint) {
        self.set(Self::CACHE_ENABLED_SHIFT, 1, v)
    }

    #[inline]
    fn log_level(&self) -> c_uint {
        self.get(Self::LOG_LEVEL_SHIFT, 3)
    }
    #[inline]
    fn set_log_level(&mut self, v: c_uint) {
        self.set(Self::LOG_LEVEL_SHIFT, 3, v)
    }

    #[inline]
    fn set_reserved(&mut self, v: c_uint) {
        self.set(Self::RESERVED_SHIFT, 1, v)
    }
}

// struct ProcessState { struct ConfigFlags flags; int base_value; int multiplier; char operation; };
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
}

// ---------------------------------------------------------------------------
// int parse_env_numeric(const char* env_name, int default_val)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    unsafe {
        let env_value = getenv(env_name);

        if env_value.is_null() {
            return default_val;
        }

        let mut invalid_char = strchr(env_value, b',' as c_int);
        if !invalid_char.is_null() {
            fprintf(
                stderr,
                c"Warning: Invalid character in %s\n".as_ptr(),
                env_name,
            );
            return default_val;
        }

        invalid_char = strchr(env_value, b';' as c_int);
        if !invalid_char.is_null() {
            fprintf(stderr, c"Warning: Semicolon found in %s\n".as_ptr(), env_name);
            return default_val;
        }

        atoi(env_value)
    }
}

// ---------------------------------------------------------------------------
// void init_config_from_env(struct ConfigFlags* flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    unsafe {
        let verbose_env = getenv(c"PROG_VERBOSE".as_ptr());
        let debug_env = getenv(c"PROG_DEBUG".as_ptr());
        let optimize_env = getenv(c"PROG_OPTIMIZE".as_ptr());

        let flags = &mut *flags;

        flags.set_verbose(
            if !verbose_env.is_null() && !strchr(verbose_env, b'1' as c_int).is_null() {
                1
            } else {
                0
            },
        );
        flags.set_debug(
            if !debug_env.is_null() && !strchr(debug_env, b'1' as c_int).is_null() {
                1
            } else {
                0
            },
        );
        flags.set_optimize(if !optimize_env.is_null() { 1 } else { 0 });
        flags.set_cache_enabled(1);
        flags.set_log_level(0o3);
        flags.set_reserved(0);
    }
}

// ---------------------------------------------------------------------------
// int perform_operation(int val1, int val2, struct ConfigFlags* flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    unsafe {
        let flags = &*flags;
        let result: c_int;

        let operation_mode: c_int = 0o755;

        if flags.optimize() != 0 {
            result = val1.wrapping_add(val2);
        } else {
            // The 3-bit unsigned bit-field promotes to `int`, so this stays
            // signed integer arithmetic and `/ 2` truncates toward zero.
            result = val1
                .wrapping_mul(flags.log_level() as c_int)
                .wrapping_add(val2.wrapping_div(2));
        }

        if flags.debug() != 0 {
            printf(
                c"Debug: operation_mode = %o (octal)\n".as_ptr(),
                operation_mode,
            );
            printf(c"Debug: result before adjustment = %d\n".as_ptr(), result);
        }

        result
    }
}

// ---------------------------------------------------------------------------
// int apply_bit_operations(int value, struct ConfigFlags* flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(value: c_int, flags: *mut ConfigFlags) -> c_int {
    unsafe {
        let flags = &*flags;
        let mut adjusted = value;

        if flags.verbose() != 0 {
            adjusted = adjusted.wrapping_shl(1);
        }

        if flags.cache_enabled() != 0 {
            adjusted |= 0x0F;
        }

        adjusted
    }
}

// ---------------------------------------------------------------------------
// int envy(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn envy(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        let mut state = ProcessState::default();
        let state_backup: ProcessState;
        let mut buffer = [0 as c_char; BUFFER_SIZE];
        let mut result: c_int;

        init_config_from_env(&raw mut state.flags);

        let base_offset = parse_env_numeric(c"PROG_BASE_OFFSET".as_ptr(), 0o100);
        let multiplier = parse_env_numeric(c"PROG_MULTIPLIER".as_ptr(), 0o12);

        if state.flags.verbose() != 0 {
            printf(c"Verbose mode enabled\n".as_ptr());
            printf(c"Base offset: %d (from octal 0100)\n".as_ptr(), base_offset);
            printf(c"Multiplier: %d (from octal 012)\n".as_ptr(), multiplier);
        }

        state.base_value = param1;
        state.multiplier = multiplier;
        state.operation = b'+' as c_char;

        // memcpy(&state_backup, &state, sizeof(struct ProcessState));
        state_backup = state;

        if state.flags.debug() != 0 {
            printf(c"Debug: Created state backup using memcpy\n".as_ptr());
            printf(
                c"Debug: Backup base_value = %d\n".as_ptr(),
                state_backup.base_value,
            );
        }

        result = perform_operation(param1, param2, &raw mut state.flags);

        if param3 != 0 {
            result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
        }

        if param4 != 0 {
            result = result.wrapping_add(param4 >> 2);
        }

        result = apply_bit_operations(result, &raw mut state.flags);

        result = result.wrapping_add(base_offset);

        snprintf(
            buffer.as_mut_ptr(),
            BUFFER_SIZE,
            c"Result:%d:Complete".as_ptr(),
            result,
        );

        let colon_pos = strchr(buffer.as_ptr(), b':' as c_int);
        if !colon_pos.is_null() {
            if state.flags.verbose() != 0 {
                let offset = colon_pos.offset_from(buffer.as_ptr()) as c_long;
                printf(c"Found colon at position: %ld\n".as_ptr(), offset);
            }

            let second_colon = strchr(colon_pos.offset(1), b':' as c_int);
            if !second_colon.is_null() && state.flags.debug() != 0 {
                printf(c"Debug: Result string format validated\n".as_ptr());
            }
        }

        if result < 0 {
            // memcpy(&state, &state_backup, sizeof(struct ProcessState));
            state = state_backup;
            result = state.base_value; /* Use original base value */

            if state.flags.verbose() != 0 {
                printf(c"Restored state from backup\n".as_ptr());
            }
        }

        if state.flags.verbose() != 0 {
            printf(c"Final result: %d\n".as_ptr(), result);
            printf(
                c"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n".as_ptr(),
                state.flags.debug() as c_int,
                state.flags.optimize() as c_int,
                state.flags.log_level() as c_int,
            );
        }

        result
    }
}
