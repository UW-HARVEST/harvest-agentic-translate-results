// Rust translation of c_src/src/lib.c
//
// Original copyright header from the C source is reproduced below, since this
// file is a direct derivative translation of it.
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

use std::ffi::{c_char, c_int, c_long, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// All formatted output goes through the C standard library on purpose: the C
// original writes to the process-wide `stdout`/`stderr` FILE streams, so using
// them here preserves buffering behaviour and the exact interleaving of stdout
// and stderr bytes.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    static stderr: *mut c_void;

    fn getenv(name: *const c_char) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
}

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
// The System V / GCC layout on little-endian x86-64 packs all six fields into
// the low byte of a single 4-byte `unsigned int` storage unit:
//
//   bit 0      verbose
//   bit 1      debug
//   bit 2      optimize
//   bit 3      cache_enabled
//   bits 4..6  log_level
//   bit 7      reserved
//   bits 8..31 padding (left untouched by the C code)
//
// It is modelled as a single `c_uint` so that size (4) and alignment (4) match
// the C struct exactly, and only the low byte is ever touched so that the
// padding bits behave the same way they do in the compiled C.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ConfigFlags {
    storage: c_uint,
}

const VERBOSE_SHIFT: u32 = 0;
const DEBUG_SHIFT: u32 = 1;
const OPTIMIZE_SHIFT: u32 = 2;
const CACHE_ENABLED_SHIFT: u32 = 3;
const LOG_LEVEL_SHIFT: u32 = 4;
const LOG_LEVEL_MASK: u8 = 0x07;
const RESERVED_SHIFT: u32 = 7;

/// Decoded copy of the bitfields held in the low byte of a `ConfigFlags`.
#[derive(Copy, Clone)]
struct Flags {
    verbose: u8,
    debug: u8,
    optimize: u8,
    cache_enabled: u8,
    log_level: u8,
    #[allow(dead_code)]
    reserved: u8,
}

impl Flags {
    fn from_byte(byte: u8) -> Self {
        Flags {
            verbose: (byte >> VERBOSE_SHIFT) & 1,
            debug: (byte >> DEBUG_SHIFT) & 1,
            optimize: (byte >> OPTIMIZE_SHIFT) & 1,
            cache_enabled: (byte >> CACHE_ENABLED_SHIFT) & 1,
            log_level: (byte >> LOG_LEVEL_SHIFT) & LOG_LEVEL_MASK,
            reserved: (byte >> RESERVED_SHIFT) & 1,
        }
    }

    fn to_byte(self) -> u8 {
        ((self.verbose & 1) << VERBOSE_SHIFT)
            | ((self.debug & 1) << DEBUG_SHIFT)
            | ((self.optimize & 1) << OPTIMIZE_SHIFT)
            | ((self.cache_enabled & 1) << CACHE_ENABLED_SHIFT)
            | ((self.log_level & LOG_LEVEL_MASK) << LOG_LEVEL_SHIFT)
            | ((self.reserved & 1) << RESERVED_SHIFT)
    }
}

/// Read the bitfield byte out of a `struct ConfigFlags *` supplied by a caller.
unsafe fn read_flags(flags: *const ConfigFlags) -> Flags {
    Flags::from_byte(unsafe { *(flags as *const u8) })
}

/// Store the bitfield byte into a `struct ConfigFlags *`.
///
/// All six bitfields together cover the whole low byte, so writing that single
/// byte is equivalent to the sequence of bitfield assignments the C performs
/// and leaves the upper three padding bytes untouched.
unsafe fn write_flags(flags: *mut ConfigFlags, value: Flags) {
    unsafe { *(flags as *mut u8) = value.to_byte() };
}

// ---------------------------------------------------------------------------
// struct ProcessState {
//     struct ConfigFlags flags;
//     int base_value;
//     int multiplier;
//     char operation;
// };
//
// size 16, align 4, offsets 0 / 4 / 8 / 12.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
}

const BUFFER_SIZE: usize = 256;

// ---------------------------------------------------------------------------
// int parse_env_numeric(const char* env_name, int default_val)
// ---------------------------------------------------------------------------
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
            )
        };
        return default_val;
    }

    invalid_char = unsafe { strchr(env_value, b';' as c_int) };
    if !invalid_char.is_null() {
        unsafe { fprintf(stderr, c"Warning: Semicolon found in %s\n".as_ptr(), env_name) };
        return default_val;
    }

    unsafe { atoi(env_value) }
}

// ---------------------------------------------------------------------------
// void init_config_from_env(struct ConfigFlags* flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    let verbose_env = unsafe { getenv(c"PROG_VERBOSE".as_ptr()) };
    let debug_env = unsafe { getenv(c"PROG_DEBUG".as_ptr()) };
    let optimize_env = unsafe { getenv(c"PROG_OPTIMIZE".as_ptr()) };

    let verbose = if !verbose_env.is_null() && !unsafe { strchr(verbose_env, b'1' as c_int) }.is_null()
    {
        1
    } else {
        0
    };
    let debug =
        if !debug_env.is_null() && !unsafe { strchr(debug_env, b'1' as c_int) }.is_null() {
            1
        } else {
            0
        };
    let optimize = if !optimize_env.is_null() { 1 } else { 0 };

    unsafe {
        write_flags(
            flags,
            Flags {
                verbose,
                debug,
                optimize,
                cache_enabled: 1,
                log_level: 0o3,
                reserved: 0,
            },
        )
    };
}

// ---------------------------------------------------------------------------
// int perform_operation(int val1, int val2, struct ConfigFlags* flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
#[allow(unused_assignments)] // mirrors the C's `int result = 0;` initialiser
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let mut result: c_int = 0;

    let operation_mode: c_int = 0o755;

    let f = unsafe { read_flags(flags) };

    if f.optimize != 0 {
        result = val1.wrapping_add(val2);
    } else {
        result = val1
            .wrapping_mul(f.log_level as c_int)
            .wrapping_add(val2.wrapping_div(2));
    }

    if f.debug != 0 {
        unsafe {
            printf(
                c"Debug: operation_mode = %o (octal)\n".as_ptr(),
                operation_mode,
            );
            printf(
                c"Debug: result before adjustment = %d\n".as_ptr(),
                result,
            );
        }
    }

    result
}

// ---------------------------------------------------------------------------
// int apply_bit_operations(int value, struct ConfigFlags* flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(value: c_int, flags: *mut ConfigFlags) -> c_int {
    let mut adjusted = value;

    let f = unsafe { read_flags(flags) };

    if f.verbose != 0 {
        adjusted = ((adjusted as u32) << 1) as c_int;
    }

    if f.cache_enabled != 0 {
        adjusted |= 0x0F;
    }

    adjusted
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
    // `struct ProcessState state;` / `state_backup;` are uninitialised in C.
    // Every field that is read below is assigned first, so zero-filling here is
    // not observable.
    let mut state = ProcessState {
        flags: ConfigFlags { storage: 0 },
        base_value: 0,
        multiplier: 0,
        operation: 0,
    };
    let mut buffer = [0 as c_char; BUFFER_SIZE];
    let mut result: c_int;

    unsafe { init_config_from_env(&mut state.flags) };

    let base_offset = unsafe { parse_env_numeric(c"PROG_BASE_OFFSET".as_ptr(), 0o100) };
    let multiplier = unsafe { parse_env_numeric(c"PROG_MULTIPLIER".as_ptr(), 0o12) };

    let f = unsafe { read_flags(&state.flags) };

    if f.verbose != 0 {
        unsafe {
            printf(c"Verbose mode enabled\n".as_ptr());
            printf(c"Base offset: %d (from octal 0100)\n".as_ptr(), base_offset);
            printf(c"Multiplier: %d (from octal 012)\n".as_ptr(), multiplier);
        }
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    // memcpy(&state_backup, &state, sizeof(struct ProcessState));
    let state_backup = state;

    if f.debug != 0 {
        unsafe {
            printf(c"Debug: Created state backup using memcpy\n".as_ptr());
            printf(
                c"Debug: Backup base_value = %d\n".as_ptr(),
                state_backup.base_value,
            );
        }
    }

    result = unsafe { perform_operation(param1, param2, &mut state.flags) };

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
            BUFFER_SIZE,
            c"Result:%d:Complete".as_ptr(),
            result,
        )
    };

    let colon_pos = unsafe { strchr(buffer.as_ptr(), b':' as c_int) };
    if !colon_pos.is_null() {
        if f.verbose != 0 {
            let offset = (colon_pos as usize).wrapping_sub(buffer.as_ptr() as usize) as c_long;
            unsafe { printf(c"Found colon at position: %ld\n".as_ptr(), offset) };
        }

        let second_colon = unsafe { strchr(colon_pos.add(1), b':' as c_int) };
        if !second_colon.is_null() && f.debug != 0 {
            unsafe { printf(c"Debug: Result string format validated\n".as_ptr()) };
        }
    }

    if result < 0 {
        // memcpy(&state, &state_backup, sizeof(struct ProcessState));
        state = state_backup;
        result = state.base_value; /* Use original base value */

        let f_restored = unsafe { read_flags(&state.flags) };
        if f_restored.verbose != 0 {
            unsafe { printf(c"Restored state from backup\n".as_ptr()) };
        }
    }

    let f_final = unsafe { read_flags(&state.flags) };
    if f_final.verbose != 0 {
        unsafe {
            printf(c"Final result: %d\n".as_ptr(), result);
            printf(
                c"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n".as_ptr(),
                f_final.debug as c_int,
                f_final.optimize as c_int,
                f_final.log_level as c_int,
            );
        }
    }

    result
}
