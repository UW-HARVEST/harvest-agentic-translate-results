// Rust translation of c_src/src/lib.c
//
// Original copyright notice from the C source:
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

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_long, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings
//
// The C library performs all of its formatted output through <stdio.h> and all
// of its string / environment handling through <stdlib.h> and <string.h>.  We
// call straight through to the very same libc entry points so that the emitted
// bytes (and the stdout/stderr buffering behaviour that determines their
// interleaving) are bit-for-bit identical to the original.
// ---------------------------------------------------------------------------

type FILE = c_void;

extern "C" {
    static mut stderr: *mut FILE;

    fn getenv(name: *const c_char) -> *mut c_char;
    fn atoi(nptr: *const c_char) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    fn snprintf(str_: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// struct ConfigFlags {
//     unsigned int verbose       : 1;
//     unsigned int debug         : 1;
//     unsigned int optimize      : 1;
//     unsigned int cache_enabled : 1;
//     unsigned int log_level     : 3;
//     unsigned int reserved      : 1;
// };
//
// On the System V x86-64 ABI (little endian) all eight bits live in the first
// byte of a single 4-byte `unsigned int` allocation unit, laid out from the
// least significant bit upwards:
//
//   bit 0      -> verbose
//   bit 1      -> debug
//   bit 2      -> optimize
//   bit 3      -> cache_enabled
//   bits 4..6  -> log_level
//   bit 7      -> reserved
//   bits 8..31 -> unnamed padding (never touched by the C code)
//
// `sizeof(struct ConfigFlags) == 4`, `_Alignof(struct ConfigFlags) == 4`.
// ---------------------------------------------------------------------------

#[repr(C, align(4))]
pub struct ConfigFlags {
    /// The 4-byte `unsigned int` allocation unit holding the bit-fields.
    storage: [u8; 4],
}

const VERBOSE_SHIFT: u32 = 0;
const DEBUG_SHIFT: u32 = 1;
const OPTIMIZE_SHIFT: u32 = 2;
const CACHE_ENABLED_SHIFT: u32 = 3;
const LOG_LEVEL_SHIFT: u32 = 4;
const RESERVED_SHIFT: u32 = 7;

impl ConfigFlags {
    #[inline]
    fn get(&self, shift: u32, width: u32) -> c_uint {
        ((self.storage[0] >> shift) as c_uint) & ((1u32 << width) - 1)
    }

    /// Read-modify-write of byte 0 only, exactly as gcc codegens a bit-field
    /// store for this layout.
    #[inline]
    fn set(&mut self, shift: u32, width: u32, value: c_uint) {
        let mask = ((1u32 << width) - 1) as u8;
        let byte = self.storage[0];
        self.storage[0] = (byte & !(mask << shift)) | (((value as u8) & mask) << shift);
    }

    #[inline]
    fn verbose(&self) -> c_uint {
        self.get(VERBOSE_SHIFT, 1)
    }
    #[inline]
    fn debug(&self) -> c_uint {
        self.get(DEBUG_SHIFT, 1)
    }
    #[inline]
    fn optimize(&self) -> c_uint {
        self.get(OPTIMIZE_SHIFT, 1)
    }
    #[inline]
    fn cache_enabled(&self) -> c_uint {
        self.get(CACHE_ENABLED_SHIFT, 1)
    }
    #[inline]
    fn log_level(&self) -> c_uint {
        self.get(LOG_LEVEL_SHIFT, 3)
    }

    #[inline]
    fn set_verbose(&mut self, v: c_uint) {
        self.set(VERBOSE_SHIFT, 1, v)
    }
    #[inline]
    fn set_debug(&mut self, v: c_uint) {
        self.set(DEBUG_SHIFT, 1, v)
    }
    #[inline]
    fn set_optimize(&mut self, v: c_uint) {
        self.set(OPTIMIZE_SHIFT, 1, v)
    }
    #[inline]
    fn set_cache_enabled(&mut self, v: c_uint) {
        self.set(CACHE_ENABLED_SHIFT, 1, v)
    }
    #[inline]
    fn set_log_level(&mut self, v: c_uint) {
        self.set(LOG_LEVEL_SHIFT, 3, v)
    }
    #[inline]
    fn set_reserved(&mut self, v: c_uint) {
        self.set(RESERVED_SHIFT, 1, v)
    }
}

// ---------------------------------------------------------------------------
// struct ProcessState {
//     struct ConfigFlags flags;
//     int  base_value;
//     int  multiplier;
//     char operation;
// };
//
// sizeof == 16 (3 bytes of tail padding after `operation`), alignment 4.
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
}

impl ProcessState {
    #[inline]
    fn zeroed() -> ProcessState {
        ProcessState {
            flags: ConfigFlags { storage: [0; 4] },
            base_value: 0,
            multiplier: 0,
            operation: 0,
        }
    }
}

/// `#define BUFFER_SIZE 256`
const BUFFER_SIZE: usize = 256;

// ---------------------------------------------------------------------------
// String literals (NUL terminated, exactly as they appear in the C source).
// ---------------------------------------------------------------------------

const S_WARN_INVALID_CHAR: &[u8] = b"Warning: Invalid character in %s\n\0";
const S_WARN_SEMICOLON: &[u8] = b"Warning: Semicolon found in %s\n\0";

const S_PROG_VERBOSE: &[u8] = b"PROG_VERBOSE\0";
const S_PROG_DEBUG: &[u8] = b"PROG_DEBUG\0";
const S_PROG_OPTIMIZE: &[u8] = b"PROG_OPTIMIZE\0";
const S_PROG_BASE_OFFSET: &[u8] = b"PROG_BASE_OFFSET\0";
const S_PROG_MULTIPLIER: &[u8] = b"PROG_MULTIPLIER\0";

const S_DBG_OPERATION_MODE: &[u8] = b"Debug: operation_mode = %o (octal)\n\0";
const S_DBG_RESULT_BEFORE: &[u8] = b"Debug: result before adjustment = %d\n\0";

const S_VERBOSE_ENABLED: &[u8] = b"Verbose mode enabled\n\0";
const S_BASE_OFFSET: &[u8] = b"Base offset: %d (from octal 0100)\n\0";
const S_MULTIPLIER: &[u8] = b"Multiplier: %d (from octal 012)\n\0";

const S_DBG_CREATED_BACKUP: &[u8] = b"Debug: Created state backup using memcpy\n\0";
const S_DBG_BACKUP_BASE: &[u8] = b"Debug: Backup base_value = %d\n\0";

const S_RESULT_FMT: &[u8] = b"Result:%d:Complete\0";
const S_FOUND_COLON: &[u8] = b"Found colon at position: %ld\n\0";
const S_DBG_FORMAT_VALIDATED: &[u8] = b"Debug: Result string format validated\n\0";
const S_RESTORED_STATE: &[u8] = b"Restored state from backup\n\0";
const S_FINAL_RESULT: &[u8] = b"Final result: %d\n\0";
const S_CONFIGURATION: &[u8] =
    b"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n\0";

#[inline]
fn cstr(s: &'static [u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// int parse_env_numeric(const char* env_name, int default_val)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_env_numeric(
    env_name: *const c_char,
    default_val: c_int,
) -> c_int {
    let env_value: *mut c_char = getenv(env_name);

    if env_value.is_null() {
        return default_val;
    }

    let mut invalid_char: *mut c_char = strchr(env_value, b',' as c_int);
    if !invalid_char.is_null() {
        fprintf(stderr, cstr(S_WARN_INVALID_CHAR), env_name);
        return default_val;
    }

    invalid_char = strchr(env_value, b';' as c_int);
    if !invalid_char.is_null() {
        fprintf(stderr, cstr(S_WARN_SEMICOLON), env_name);
        return default_val;
    }

    atoi(env_value)
}

// ---------------------------------------------------------------------------
// void init_config_from_env(struct ConfigFlags* flags)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    let verbose_env: *mut c_char = getenv(cstr(S_PROG_VERBOSE));
    let debug_env: *mut c_char = getenv(cstr(S_PROG_DEBUG));
    let optimize_env: *mut c_char = getenv(cstr(S_PROG_OPTIMIZE));

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

// ---------------------------------------------------------------------------
// int perform_operation(int val1, int val2, struct ConfigFlags* flags)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    // `int result = 0;` — immediately overwritten on both branches below, kept
    // for fidelity with the C source.
    let mut result: c_int = 0;

    let operation_mode: c_int = 0o755;

    let flags = &*flags;

    if flags.optimize() != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // The bit-field `log_level` (unsigned int : 3) undergoes the integer
        // promotions and becomes a plain `int` here.
        result = val1
            .wrapping_mul(flags.log_level() as c_int)
            .wrapping_add(val2.wrapping_div(2));
    }

    if flags.debug() != 0 {
        printf(cstr(S_DBG_OPERATION_MODE), operation_mode);
        printf(cstr(S_DBG_RESULT_BEFORE), result);
    }

    result
}

// ---------------------------------------------------------------------------
// int apply_bit_operations(int value, struct ConfigFlags* flags)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(
    value: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let mut adjusted: c_int = value;

    let flags = &*flags;

    if flags.verbose() != 0 {
        adjusted = ((adjusted as u32) << 1) as c_int;
    }

    if flags.cache_enabled() != 0 {
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
    // `struct ProcessState state, state_backup;` and `char buffer[256];` are
    // uninitialised automatic objects in the C original.  Reading uninitialised
    // memory is undefined behaviour in Rust (LLVM will happily fold it into an
    // `unreachable`), so the storage is zeroed here instead.  This is not
    // observable: every member and every buffer byte that the C code later
    // reads is unconditionally written first (`init_config_from_env` sets all
    // six bit-fields, `base_value`/`multiplier`/`operation` are assigned, and
    // `snprintf` NUL-terminates the buffer), so only never-read padding bytes
    // differ.
    let mut state_storage: ProcessState = ProcessState::zeroed();
    let mut backup_storage: ProcessState = ProcessState::zeroed();
    let state: *mut ProcessState = &mut state_storage;
    let state_backup: *mut ProcessState = &mut backup_storage;

    let mut buffer: [c_char; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let buffer_ptr: *mut c_char = buffer.as_mut_ptr();

    let mut result: c_int;

    init_config_from_env(core::ptr::addr_of_mut!((*state).flags));

    let base_offset: c_int = parse_env_numeric(cstr(S_PROG_BASE_OFFSET), 0o100);
    let multiplier: c_int = parse_env_numeric(cstr(S_PROG_MULTIPLIER), 0o12);

    if (*state).flags.verbose() != 0 {
        printf(cstr(S_VERBOSE_ENABLED));
        printf(cstr(S_BASE_OFFSET), base_offset);
        printf(cstr(S_MULTIPLIER), multiplier);
    }

    (*state).base_value = param1;
    (*state).multiplier = multiplier;
    (*state).operation = b'+' as c_char;

    memcpy(
        state_backup as *mut c_void,
        state as *const c_void,
        core::mem::size_of::<ProcessState>(),
    );

    if (*state).flags.debug() != 0 {
        printf(cstr(S_DBG_CREATED_BACKUP));
        printf(cstr(S_DBG_BACKUP_BASE), (*state_backup).base_value);
    }

    result = perform_operation(param1, param2, core::ptr::addr_of_mut!((*state).flags));

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul((*state).multiplier));
    }

    if param4 != 0 {
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, core::ptr::addr_of_mut!((*state).flags));

    result = result.wrapping_add(base_offset);

    snprintf(buffer_ptr, BUFFER_SIZE, cstr(S_RESULT_FMT), result);

    let colon_pos: *mut c_char = strchr(buffer_ptr, b':' as c_int);
    if !colon_pos.is_null() {
        if (*state).flags.verbose() != 0 {
            printf(
                cstr(S_FOUND_COLON),
                (colon_pos as isize - buffer_ptr as isize) as c_long,
            );
        }

        let second_colon: *mut c_char = strchr(colon_pos.add(1), b':' as c_int);
        if !second_colon.is_null() && (*state).flags.debug() != 0 {
            printf(cstr(S_DBG_FORMAT_VALIDATED));
        }
    }

    if result < 0 {
        memcpy(
            state as *mut c_void,
            state_backup as *const c_void,
            core::mem::size_of::<ProcessState>(),
        );
        result = (*state).base_value; /* Use original base value */

        if (*state).flags.verbose() != 0 {
            printf(cstr(S_RESTORED_STATE));
        }
    }

    if (*state).flags.verbose() != 0 {
        printf(cstr(S_FINAL_RESULT), result);
        printf(
            cstr(S_CONFIGURATION),
            (*state).flags.debug() as c_int,
            (*state).flags.optimize() as c_int,
            (*state).flags.log_level() as c_int,
        );
    }

    result
}
