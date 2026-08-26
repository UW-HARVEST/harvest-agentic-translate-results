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
//
// Rust translation of c_src/src/lib.c.  The translation is a 1:1 port of the C
// implementation: the same libc primitives (`getenv`, `atoi`, `strchr`,
// `printf`, `fprintf`, `snprintf`, `memcpy`) are used so that both the returned
// values and every byte written to stdout/stderr (including stream buffering
// behaviour) are identical to the original C shared library.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_long, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings
// ---------------------------------------------------------------------------

/// Opaque stand-in for `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    static mut stderr: *mut FILE;

    fn getenv(name: *const c_char) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// ```c
/// struct ConfigFlags {
///     unsigned int verbose : 1;
///     unsigned int debug : 1;
///     unsigned int optimize : 1;
///     unsigned int cache_enabled : 1;
///     unsigned int log_level : 3;
///     unsigned int reserved : 1;
/// };
/// ```
///
/// GCC on x86-64 (little endian) allocates these bit-fields inside a single
/// 4-byte storage unit, from the least significant bit upwards:
///
/// | bit(s) | field         |
/// |--------|---------------|
/// | 0      | verbose       |
/// | 1      | debug         |
/// | 2      | optimize      |
/// | 3      | cache_enabled |
/// | 4..6   | log_level     |
/// | 7      | reserved      |
/// | 8..31  | padding       |
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ConfigFlags {
    /// The single 4-byte bit-field storage unit.
    bits: u32,
}

const VERBOSE_SHIFT: u32 = 0;
const DEBUG_SHIFT: u32 = 1;
const OPTIMIZE_SHIFT: u32 = 2;
const CACHE_ENABLED_SHIFT: u32 = 3;
const LOG_LEVEL_SHIFT: u32 = 4;
const LOG_LEVEL_MASK: u32 = 0x7;
const RESERVED_SHIFT: u32 = 7;

impl ConfigFlags {
    #[inline]
    fn verbose(&self) -> c_uint {
        (self.bits >> VERBOSE_SHIFT) & 1
    }

    #[inline]
    fn debug(&self) -> c_uint {
        (self.bits >> DEBUG_SHIFT) & 1
    }

    #[inline]
    fn optimize(&self) -> c_uint {
        (self.bits >> OPTIMIZE_SHIFT) & 1
    }

    #[inline]
    fn cache_enabled(&self) -> c_uint {
        (self.bits >> CACHE_ENABLED_SHIFT) & 1
    }

    #[inline]
    fn log_level(&self) -> c_uint {
        (self.bits >> LOG_LEVEL_SHIFT) & LOG_LEVEL_MASK
    }

    #[inline]
    fn set_verbose(&mut self, v: c_uint) {
        self.bits = (self.bits & !(1 << VERBOSE_SHIFT)) | ((v & 1) << VERBOSE_SHIFT);
    }

    #[inline]
    fn set_debug(&mut self, v: c_uint) {
        self.bits = (self.bits & !(1 << DEBUG_SHIFT)) | ((v & 1) << DEBUG_SHIFT);
    }

    #[inline]
    fn set_optimize(&mut self, v: c_uint) {
        self.bits = (self.bits & !(1 << OPTIMIZE_SHIFT)) | ((v & 1) << OPTIMIZE_SHIFT);
    }

    #[inline]
    fn set_cache_enabled(&mut self, v: c_uint) {
        self.bits = (self.bits & !(1 << CACHE_ENABLED_SHIFT)) | ((v & 1) << CACHE_ENABLED_SHIFT);
    }

    #[inline]
    fn set_log_level(&mut self, v: c_uint) {
        self.bits = (self.bits & !(LOG_LEVEL_MASK << LOG_LEVEL_SHIFT))
            | ((v & LOG_LEVEL_MASK) << LOG_LEVEL_SHIFT);
    }

    #[inline]
    fn set_reserved(&mut self, v: c_uint) {
        self.bits = (self.bits & !(1 << RESERVED_SHIFT)) | ((v & 1) << RESERVED_SHIFT);
    }
}

/// Loads the 4-byte bit-field storage unit the C code would load.
///
/// The access is done with *byte-wise volatile* reads, in increasing address
/// order, because that is the only form which reproduces the behaviour of the
/// plain machine load gcc emits for `flags->field`:
///
/// * a plain `*p`, `&*p` or `ptr::read` would trip Rust's "null pointer
///   dereference" debug assertion and `abort()` (`SIGABRT`) where the C code
///   faults (`SIGSEGV`);
/// * a 4-byte `ptr::read_volatile` would trip the "pointer must be aligned"
///   precondition and `abort()`, whereas the C code happily performs the
///   unaligned access on x86-64 (a caller may legitimately pass a
///   `struct ConfigFlags*` derived from a `char` buffer).
///
/// `wrapping_add` is used for the byte offsets so that no in-bounds assumption
/// is made about an invalid pointer either.
#[inline]
unsafe fn flags_load(p: *const ConfigFlags) -> ConfigFlags {
    let b = p.cast::<u8>();
    let bytes = [
        core::ptr::read_volatile(b),
        core::ptr::read_volatile(b.wrapping_add(1)),
        core::ptr::read_volatile(b.wrapping_add(2)),
        core::ptr::read_volatile(b.wrapping_add(3)),
    ];
    ConfigFlags {
        bits: c_uint::from_ne_bytes(bytes),
    }
}

/// Stores the 4-byte bit-field storage unit back (see `flags_load`).
#[inline]
unsafe fn flags_store(p: *mut ConfigFlags, v: ConfigFlags) {
    let bytes = v.bits.to_ne_bytes();
    let b = p.cast::<u8>();
    core::ptr::write_volatile(b, bytes[0]);
    core::ptr::write_volatile(b.wrapping_add(1), bytes[1]);
    core::ptr::write_volatile(b.wrapping_add(2), bytes[2]);
    core::ptr::write_volatile(b.wrapping_add(3), bytes[3]);
}

/// ```c
/// struct ProcessState {
///     struct ConfigFlags flags;
///     int base_value;
///     int multiplier;
///     char operation;
/// };
/// ```
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct ProcessState {
    flags: ConfigFlags,
    base_value: c_int,
    multiplier: c_int,
    operation: c_char,
}

/// `#define BUFFER_SIZE 256`
const BUFFER_SIZE: usize = 256;

// ---------------------------------------------------------------------------
// Format / literal strings (NUL terminated, exactly as in the C source)
// ---------------------------------------------------------------------------

const FMT_WARN_INVALID_CHAR: &[u8] = b"Warning: Invalid character in %s\n\0";
const FMT_WARN_SEMICOLON: &[u8] = b"Warning: Semicolon found in %s\n\0";

const ENV_PROG_VERBOSE: &[u8] = b"PROG_VERBOSE\0";
const ENV_PROG_DEBUG: &[u8] = b"PROG_DEBUG\0";
const ENV_PROG_OPTIMIZE: &[u8] = b"PROG_OPTIMIZE\0";
const ENV_PROG_BASE_OFFSET: &[u8] = b"PROG_BASE_OFFSET\0";
const ENV_PROG_MULTIPLIER: &[u8] = b"PROG_MULTIPLIER\0";

const FMT_DEBUG_OPERATION_MODE: &[u8] = b"Debug: operation_mode = %o (octal)\n\0";
const FMT_DEBUG_RESULT_BEFORE: &[u8] = b"Debug: result before adjustment = %d\n\0";

const FMT_VERBOSE_ENABLED: &[u8] = b"Verbose mode enabled\n\0";
const FMT_BASE_OFFSET: &[u8] = b"Base offset: %d (from octal 0100)\n\0";
const FMT_MULTIPLIER: &[u8] = b"Multiplier: %d (from octal 012)\n\0";

const FMT_DEBUG_BACKUP: &[u8] = b"Debug: Created state backup using memcpy\n\0";
const FMT_DEBUG_BACKUP_BASE: &[u8] = b"Debug: Backup base_value = %d\n\0";

const FMT_RESULT_STRING: &[u8] = b"Result:%d:Complete\0";
const FMT_FOUND_COLON: &[u8] = b"Found colon at position: %ld\n\0";
const FMT_DEBUG_FORMAT_OK: &[u8] = b"Debug: Result string format validated\n\0";
const FMT_RESTORED: &[u8] = b"Restored state from backup\n\0";
const FMT_FINAL_RESULT: &[u8] = b"Final result: %d\n\0";
const FMT_CONFIGURATION: &[u8] =
    b"Configuration - Debug: %d, Optimize: %d, Log Level: %d\n\0";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// ```c
/// int parse_env_numeric(const char* env_name, int default_val);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_env_numeric(env_name: *const c_char, default_val: c_int) -> c_int {
    let env_value: *mut c_char = getenv(env_name);

    if env_value.is_null() {
        return default_val;
    }

    let mut invalid_char: *mut c_char = strchr(env_value, b',' as c_int);
    if !invalid_char.is_null() {
        fprintf(
            stderr,
            FMT_WARN_INVALID_CHAR.as_ptr() as *const c_char,
            env_name,
        );
        return default_val;
    }

    invalid_char = strchr(env_value, b';' as c_int);
    if !invalid_char.is_null() {
        fprintf(
            stderr,
            FMT_WARN_SEMICOLON.as_ptr() as *const c_char,
            env_name,
        );
        return default_val;
    }

    atoi(env_value)
}

/// ```c
/// void init_config_from_env(struct ConfigFlags* flags);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_config_from_env(flags: *mut ConfigFlags) {
    let verbose_env: *mut c_char = getenv(ENV_PROG_VERBOSE.as_ptr() as *const c_char);
    let debug_env: *mut c_char = getenv(ENV_PROG_DEBUG.as_ptr() as *const c_char);
    let optimize_env: *mut c_char = getenv(ENV_PROG_OPTIMIZE.as_ptr() as *const c_char);

    // The C code performs a read-modify-write of the single 4-byte bit-field
    // storage unit for every assignment below (bits 8..31 are left untouched).
    let mut tmp = flags_load(flags);
    let flags_out = flags;
    let flags = &mut tmp;

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
    // `03` is an octal literal in C == 3.
    flags.set_log_level(0o3);
    flags.set_reserved(0);

    // Store the modified storage unit back; bits 8..31 (padding) keep whatever
    // the caller's buffer contained, matching the C read-modify-write.
    flags_store(flags_out, tmp);
}

/// ```c
/// int perform_operation(int val1, int val2, struct ConfigFlags* flags);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    val1: c_int,
    val2: c_int,
    flags: *mut ConfigFlags,
) -> c_int {
    let result: c_int;

    // `0755` is an octal literal in C == 493.
    let operation_mode: c_int = 0o755;

    let flags = &flags_load(flags);

    if flags.optimize() != 0 {
        result = val1.wrapping_add(val2);
    } else {
        // A `unsigned int : 3` bit-field promotes to `int`, so this is signed
        // arithmetic; `val2 / 2` truncates towards zero, exactly like Rust's
        // integer division.
        result = val1
            .wrapping_mul(flags.log_level() as c_int)
            .wrapping_add(val2.wrapping_div(2));
    }

    if flags.debug() != 0 {
        printf(
            FMT_DEBUG_OPERATION_MODE.as_ptr() as *const c_char,
            operation_mode,
        );
        printf(FMT_DEBUG_RESULT_BEFORE.as_ptr() as *const c_char, result);
    }

    result
}

/// ```c
/// int apply_bit_operations(int value, struct ConfigFlags* flags);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_bit_operations(value: c_int, flags: *mut ConfigFlags) -> c_int {
    let mut adjusted: c_int = value;

    let flags = &flags_load(flags);

    if flags.verbose() != 0 {
        adjusted = ((adjusted as u32) << 1) as c_int;
    }

    if flags.cache_enabled() != 0 {
        adjusted |= 0x0F;
    }

    adjusted
}

/// ```c
/// int envy(int param1, int param2, int param3, int param4);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn envy(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut state: ProcessState = ProcessState::default();
    let mut state_backup: ProcessState = ProcessState::default();
    let mut buffer: [c_char; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut result: c_int;

    init_config_from_env(&mut state.flags as *mut ConfigFlags);

    // `0100` == 64, `012` == 10 (octal literals).
    let base_offset: c_int =
        parse_env_numeric(ENV_PROG_BASE_OFFSET.as_ptr() as *const c_char, 0o100);
    let multiplier: c_int =
        parse_env_numeric(ENV_PROG_MULTIPLIER.as_ptr() as *const c_char, 0o12);

    if state.flags.verbose() != 0 {
        printf(FMT_VERBOSE_ENABLED.as_ptr() as *const c_char);
        printf(FMT_BASE_OFFSET.as_ptr() as *const c_char, base_offset);
        printf(FMT_MULTIPLIER.as_ptr() as *const c_char, multiplier);
    }

    state.base_value = param1;
    state.multiplier = multiplier;
    state.operation = b'+' as c_char;

    memcpy(
        &mut state_backup as *mut ProcessState as *mut c_void,
        &state as *const ProcessState as *const c_void,
        core::mem::size_of::<ProcessState>(),
    );

    if state.flags.debug() != 0 {
        printf(FMT_DEBUG_BACKUP.as_ptr() as *const c_char);
        printf(
            FMT_DEBUG_BACKUP_BASE.as_ptr() as *const c_char,
            state_backup.base_value,
        );
    }

    result = perform_operation(param1, param2, &mut state.flags as *mut ConfigFlags);

    if param3 != 0 {
        result = result.wrapping_add(param3.wrapping_mul(state.multiplier));
    }

    if param4 != 0 {
        // Signed right shift is arithmetic on GCC/x86-64, matching Rust's `>>`
        // on `i32`.
        result = result.wrapping_add(param4 >> 2);
    }

    result = apply_bit_operations(result, &mut state.flags as *mut ConfigFlags);

    result = result.wrapping_add(base_offset);

    snprintf(
        buffer.as_mut_ptr(),
        BUFFER_SIZE,
        FMT_RESULT_STRING.as_ptr() as *const c_char,
        result,
    );

    let colon_pos: *mut c_char = strchr(buffer.as_ptr(), b':' as c_int);
    if !colon_pos.is_null() {
        if state.flags.verbose() != 0 {
            printf(
                FMT_FOUND_COLON.as_ptr() as *const c_char,
                (colon_pos as isize - buffer.as_ptr() as isize) as c_long,
            );
        }

        let second_colon: *mut c_char = strchr(colon_pos.add(1), b':' as c_int);
        if !second_colon.is_null() && state.flags.debug() != 0 {
            printf(FMT_DEBUG_FORMAT_OK.as_ptr() as *const c_char);
        }
    }

    if result < 0 {
        memcpy(
            &mut state as *mut ProcessState as *mut c_void,
            &state_backup as *const ProcessState as *const c_void,
            core::mem::size_of::<ProcessState>(),
        );
        result = state.base_value; /* Use original base value */

        if state.flags.verbose() != 0 {
            printf(FMT_RESTORED.as_ptr() as *const c_char);
        }
    }

    if state.flags.verbose() != 0 {
        printf(FMT_FINAL_RESULT.as_ptr() as *const c_char, result);
        printf(
            FMT_CONFIGURATION.as_ptr() as *const c_char,
            state.flags.debug() as c_int,
            state.flags.optimize() as c_int,
            state.flags.log_level() as c_int,
        );
    }

    result
}
