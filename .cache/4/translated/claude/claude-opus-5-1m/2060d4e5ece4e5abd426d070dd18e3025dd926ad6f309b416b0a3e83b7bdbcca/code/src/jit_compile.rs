// Translated from pcre2_jit_compile.c + pcre2_jit_match_inc.h + pcre2_jit_misc_inc.h
// SUPPORT_JIT is not defined, so only the stub paths are present.
use crate::internal::*;
use crate::pcre2_pub::*;
use core::ffi::{c_char, c_int, c_void};

const PUBLIC_JIT_COMPILE_OPTIONS: u32 =
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD | PCRE2_JIT_INVALID_UTF;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_compile_8(
    code: *mut pcre2_real_code,
    options: u32,
) -> c_int {
    let re: *mut pcre2_real_code = code;

    if (options & PCRE2_JIT_TEST_ALLOC) != 0 {
        if options != PCRE2_JIT_TEST_ALLOC {
            return PCRE2_ERROR_JIT_BADOPTION;
        }
        return PCRE2_ERROR_JIT_UNSUPPORTED;
    }

    if code.is_null() {
        return PCRE2_ERROR_NULL;
    }

    if (options & !PUBLIC_JIT_COMPILE_OPTIONS) != 0 {
        return PCRE2_ERROR_JIT_BADOPTION;
    }

    if (options & PCRE2_JIT_INVALID_UTF) != 0 {
        if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) == 0 {
            (*re).overall_options |= PCRE2_MATCH_INVALID_UTF;
        }
    }

    PCRE2_ERROR_JIT_BADOPTION
}

/*************************************************
*              Do a JIT pattern match            *
*************************************************/

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
    (*match_data).rc = PCRE2_ERROR_JIT_BADOPTION;
    (*match_data).rc
}

/*************************************************
*           Free JIT read-only data              *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_rodata_8(_current: *mut c_void, _allocator_data: *mut c_void) {
}

/*************************************************
*           Free JIT compiled code               *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_8(_executable_jit: *mut c_void, _memctl: *mut pcre2_memctl) {
}

/*************************************************
*            Free unused JIT memory              *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_free_unused_memory_8(
    _gcontext: *mut pcre2_real_general_context,
) {
}

/*************************************************
*            Allocate a JIT stack                *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_create_8(
    _startsize: usize,
    _maxsize: usize,
    _gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_jit_stack {
    core::ptr::null_mut()
}

/*************************************************
*         Assign a JIT stack to a pattern        *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_assign_8(
    _mcontext: *mut pcre2_real_match_context,
    _callback: Option<unsafe extern "C" fn(*mut c_void) -> *mut pcre2_real_jit_stack>,
    _callback_data: *mut c_void,
) {
}

/*************************************************
*               Free a JIT stack                 *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_free_8(_jit_stack: *mut pcre2_real_jit_stack) {}

/*************************************************
*               Get target CPU type              *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_target_8() -> *const c_char {
    b"JIT is not supported\0".as_ptr() as *const c_char
}

/*************************************************
*              Get size of JIT code              *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_size_8(_executable_jit: *mut c_void) -> usize {
    0
}
