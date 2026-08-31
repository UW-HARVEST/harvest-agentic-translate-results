//! Translation of the parts of `c_src/src/pcre2_jit_compile.c` that are compiled
//! when `SUPPORT_JIT` is **not** defined.
//!
//! In that configuration the file reduces to:
//!   * the public `pcre2_jit_compile` function (whose body is largely shared with
//!     the JIT build, but whose tail returns `PCRE2_ERROR_JIT_BADOPTION`),
//!   * the stub `pcre2_jit_match` (from `pcre2_jit_match_inc.h`), and
//!   * the stubs in `pcre2_jit_misc_inc.h`:
//!     `PRIV(jit_free_rodata)`, `PRIV(jit_free)`, `pcre2_jit_free_unused_memory`,
//!     `pcre2_jit_stack_create`, `pcre2_jit_stack_assign`, `pcre2_jit_stack_free`,
//!     `PRIV(jit_get_target)`, `PRIV(jit_get_size)`.
//!
//! Every `#ifdef SUPPORT_JIT` branch (including `jit_machine_stack_exec`, the
//! executable-allocator probe, and the real compile/match paths) is omitted, as
//! it is not compiled in this build.

#![allow(non_snake_case, non_upper_case_globals, unused_parens)]

use core::ffi::{c_char, c_int, c_void};

use crate::internal::*;

/* `PUBLIC_JIT_COMPILE_OPTIONS` from pcre2_jit_compile.c */
const PUBLIC_JIT_COMPILE_OPTIONS: u32 =
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD | PCRE2_JIT_INVALID_UTF;

/* The public JIT callback type: `pcre2_jit_stack *(*)(void *)`. Only ever stored,
never called, in the non-JIT build. */
pub type pcre2_jit_callback =
    Option<unsafe extern "C" fn(*mut c_void) -> *mut pcre2_real_jit_stack>;

/*************************************************
*       JIT compile a Regular Expression         *
*************************************************/

/* Under `#ifndef SUPPORT_JIT`, all the SUPPORT_JIT-only locals and blocks vanish,
leaving the option validation, the PCRE2_MATCH_INVALID_UTF propagation, and a
final `return PCRE2_ERROR_JIT_BADOPTION`. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_compile_8(code: *mut pcre2_real_code, options: u32) -> c_int {
    unsafe {
        let re = code;

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

        /* PCRE2_JIT_INVALID_UTF propagates back into the regex options (ensuring
        interpreter support) even in the absence of JIT. */
        if (options & PCRE2_JIT_INVALID_UTF) != 0 {
            if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) == 0 {
                (*re).overall_options |= PCRE2_MATCH_INVALID_UTF;
            }
        }

        /* No JIT support: give an error return. */
        PCRE2_ERROR_JIT_BADOPTION
    }
}

/*************************************************
*              Do a JIT pattern match            *
*************************************************/

/* Stub from pcre2_jit_match_inc.h: sets and returns match_data->rc. */

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
        (*match_data).rc = PCRE2_ERROR_JIT_BADOPTION;
        (*match_data).rc
    }
}

/*************************************************
*           Free JIT read-only data              *
*************************************************/

/* PRIV(jit_free_rodata): no-op without JIT. */

pub unsafe fn jit_free_rodata(_current: *mut c_void, _allocator_data: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_rodata_8(
    current: *mut c_void,
    allocator_data: *mut c_void,
) {
    unsafe { jit_free_rodata(current, allocator_data) }
}

/*************************************************
*           Free JIT compiled code               *
*************************************************/

/* PRIV(jit_free): no-op without JIT. */

pub unsafe fn jit_free(_executable_jit: *mut c_void, _memctl: *mut pcre2_memctl) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_8(
    executable_jit: *mut c_void,
    memctl: *mut pcre2_memctl,
) {
    unsafe { jit_free(executable_jit, memctl) }
}

/*************************************************
*            Free unused JIT memory              *
*************************************************/

/* No-op without JIT. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_free_unused_memory_8(
    _gcontext: *mut pcre2_real_general_context,
) {
}

/*************************************************
*            Allocate a JIT stack                *
*************************************************/

/* Without JIT, always returns NULL. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_create_8(
    _startsize: PCRE2_SIZE,
    _maxsize: PCRE2_SIZE,
    _gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_jit_stack {
    core::ptr::null_mut()
}

/*************************************************
*         Assign a JIT stack to a pattern        *
*************************************************/

/* No-op without JIT. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_assign_8(
    _mcontext: *mut pcre2_real_match_context,
    _callback: pcre2_jit_callback,
    _callback_data: *mut c_void,
) {
}

/*************************************************
*               Free a JIT stack                 *
*************************************************/

/* No-op without JIT. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_free_8(_jit_stack: *mut pcre2_real_jit_stack) {}

/*************************************************
*               Get target CPU type              *
*************************************************/

/* PRIV(jit_get_target): returns the fixed string without JIT. */

static JIT_NOT_SUPPORTED: &[u8] = b"JIT is not supported\0";

pub unsafe fn jit_get_target() -> *const c_char {
    JIT_NOT_SUPPORTED.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_target_8() -> *const c_char {
    unsafe { jit_get_target() }
}

/*************************************************
*              Get size of JIT code              *
*************************************************/

/* PRIV(jit_get_size): returns 0 without JIT. */

pub unsafe fn jit_get_size(_executable_jit: *mut c_void) -> PCRE2_SIZE {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_size_8(executable_jit: *mut c_void) -> PCRE2_SIZE {
    unsafe { jit_get_size(executable_jit) }
}
