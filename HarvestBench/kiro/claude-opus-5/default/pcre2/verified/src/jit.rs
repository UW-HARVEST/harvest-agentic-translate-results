//! Translation of the non-JIT parts of `pcre2_jit_compile.c`,
//! `pcre2_jit_match_inc.h` and `pcre2_jit_misc_inc.h`.
//!
//! `SUPPORT_JIT` is not defined in this configuration, so all of these are the
//! stub variants.

use crate::internal::*;
use core::ffi::{c_char, c_int, c_void};

const PUBLIC_JIT_COMPILE_OPTIONS: u32 = (PCRE2_JIT_COMPLETE
    | PCRE2_JIT_PARTIAL_SOFT
    | PCRE2_JIT_PARTIAL_HARD
    | PCRE2_JIT_INVALID_UTF) as u32;

/// `pcre2_jit_compile()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_compile_8(code: *mut pcre2_real_code, options: u32) -> c_int {
    unsafe {
        let re = code;

        if (options & PCRE2_JIT_TEST_ALLOC as u32) != 0 {
            if options != PCRE2_JIT_TEST_ALLOC as u32 {
                return PCRE2_ERROR_JIT_BADOPTION as c_int;
            }
            return PCRE2_ERROR_JIT_UNSUPPORTED as c_int;
        }

        if code.is_null() {
            return PCRE2_ERROR_NULL as c_int;
        }

        if (options & !PUBLIC_JIT_COMPILE_OPTIONS) != 0 {
            return PCRE2_ERROR_JIT_BADOPTION as c_int;
        }

        // PCRE2_JIT_INVALID_UTF propagates back into the pattern options even
        // without JIT support, so that the interpreter handles invalid UTF.
        if (options & PCRE2_JIT_INVALID_UTF as u32) != 0
            && ((*re).overall_options & PCRE2_MATCH_INVALID_UTF as u32) == 0
        {
            (*re).overall_options |= PCRE2_MATCH_INVALID_UTF as u32;
        }

        PCRE2_ERROR_JIT_BADOPTION as c_int
    }
}

/// `pcre2_jit_match()` — always reports that the JIT option is bad.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_match_8(
    _code: *const pcre2_real_code,
    _subject: PCRE2_SPTR,
    _length: PCRE2_SIZE,
    _start_offset: PCRE2_SIZE,
    _options: u32,
    match_data: *mut pcre2_real_match_data,
    _mcontext: *mut pcre2_real_match_context,
) -> c_int {
    unsafe {
        (*match_data).rc = PCRE2_ERROR_JIT_BADOPTION as c_int;
        (*match_data).rc
    }
}

/// `PRIV(jit_free_rodata)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_rodata_8(
    _current: *mut c_void,
    _allocator_data: *mut c_void,
) {
}

/// `PRIV(jit_free)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_8(
    _executable_jit: *mut c_void,
    _memctl: *mut pcre2_memctl,
) {
}

/// `pcre2_jit_free_unused_memory()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_free_unused_memory_8(
    _gcontext: *mut pcre2_real_general_context,
) {
}

/// `pcre2_jit_stack_create()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_create_8(
    _startsize: usize,
    _maxsize: usize,
    _gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_jit_stack {
    core::ptr::null_mut()
}

/// `pcre2_jit_stack_assign()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_assign_8(
    _mcontext: *mut pcre2_real_match_context,
    _callback: pcre2_jit_callback,
    _callback_data: *mut c_void,
) {
}

/// `pcre2_jit_stack_free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_free_8(_jit_stack: *mut pcre2_real_jit_stack) {}

/// `PRIV(jit_get_target)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_target_8() -> *const c_char {
    c"JIT is not supported".as_ptr()
}

/// `PRIV(jit_get_size)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_size_8(_executable_jit: *mut c_void) -> usize {
    0
}
