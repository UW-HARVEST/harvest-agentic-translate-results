//! Translated from pcre2_jit_compile.c (no JIT support).
//!
//! `SUPPORT_JIT` is not defined, so essentially the whole of pcre2_jit_compile.c is
//! compiled out. What remains is `pcre2_jit_compile()` (with its JIT-only blocks
//! removed) plus the non-JIT stubs from `pcre2_jit_match_inc.h` and
//! `pcre2_jit_misc_inc.h`, both of which are `#include`d unconditionally at the end
//! of pcre2_jit_compile.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/*************************************************
*        JIT compile a Regular Expression        *
*************************************************/

/* This function used JIT to convert a previously-compiled pattern into machine
code.

Arguments:
  code          a compiled pattern
  options       JIT option bits

Returns:        0: success or (*NOJIT) was used
               <0: an error code
*/

const PUBLIC_JIT_COMPILE_OPTIONS: u32 =
    PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD | PCRE2_JIT_INVALID_UTF;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_compile_8(code: *mut pcre2_real_code, options: u32) -> i32 {
let re: *mut pcre2_real_code = code;

if options & PCRE2_JIT_TEST_ALLOC != 0
  {
  if options != PCRE2_JIT_TEST_ALLOC
    { return PCRE2_ERROR_JIT_BADOPTION; }

  /* !SUPPORT_JIT */
  return PCRE2_ERROR_JIT_UNSUPPORTED;
  }

if code.is_null()
  { return PCRE2_ERROR_NULL; }

if (options & !PUBLIC_JIT_COMPILE_OPTIONS) != 0
  { return PCRE2_ERROR_JIT_BADOPTION; }

/* Support for invalid UTF was first introduced in JIT, with the option
PCRE2_JIT_INVALID_UTF. Later, support was added to the interpreter, and the
compile-time option PCRE2_MATCH_INVALID_UTF was created. This is now the
preferred feature, with the earlier option deprecated. However, for backward
compatibility, if the earlier option is set, it forces the new option so that
if JIT matching falls back to the interpreter, there is still support for
invalid UTF. */

if (options & PCRE2_JIT_INVALID_UTF) != 0
  {
  if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) == 0
    {
    (*re).overall_options |= PCRE2_MATCH_INVALID_UTF;
    }
  }

/* The above tests are run with and without JIT support. This means that
PCRE2_JIT_INVALID_UTF propagates back into the regex options (ensuring
interpreter support) even in the absence of JIT. But now, if there is no JIT
support, give an error return. */

return PCRE2_ERROR_JIT_BADOPTION;
}


/*************************************************
*              Do a JIT pattern match            *
*************************************************/

/* From pcre2_jit_match_inc.h, !SUPPORT_JIT branch. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_match_8(code: *const pcre2_real_code, subject: PCRE2_SPTR, length: PCRE2_SIZE, start_offset: PCRE2_SIZE, options: u32, match_data: *mut pcre2_real_match_data, mcontext: *mut pcre2_real_match_context) -> i32 {
(*match_data).rc = PCRE2_ERROR_JIT_BADOPTION;
return (*match_data).rc;
}


/*************************************************
*            Free unused JIT memory              *
*************************************************/

/* From pcre2_jit_misc_inc.h, !SUPPORT_JIT branch: does nothing. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_free_unused_memory_8(gcontext: *mut pcre2_real_general_context) {
/* (void)gcontext; */
}


/*************************************************
*            Allocate a JIT stack                *
*************************************************/

/* From pcre2_jit_misc_inc.h, !SUPPORT_JIT branch: always returns NULL. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_create_8(startsize: usize, maxsize: usize, gcontext: *mut pcre2_real_general_context) -> *mut pcre2_real_jit_stack {
return core::ptr::null_mut();
}


/*************************************************
*         Assign a JIT stack to a pattern        *
*************************************************/

/* From pcre2_jit_misc_inc.h, !SUPPORT_JIT branch: does nothing. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_assign_8(mcontext: *mut pcre2_real_match_context, callback: JitCallbackFn, callback_data: *mut c_void) {
/* (void)mcontext; (void)callback; (void)callback_data; */
}


/*************************************************
*               Free a JIT stack                 *
*************************************************/

/* From pcre2_jit_misc_inc.h, !SUPPORT_JIT branch: does nothing. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_free_8(jit_stack: *mut pcre2_real_jit_stack) {
/* (void)jit_stack; */
}


/*************************************************
*           Free JIT compiled code               *
*************************************************/

/* PRIV(jit_free) - from pcre2_jit_misc_inc.h, !SUPPORT_JIT branch: does nothing. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_8(executable_jit: *mut c_void, memctl: *mut pcre2_memctl) {
/* (void)executable_jit; (void)memctl; */
}


/*************************************************
*           Free JIT read-only data              *
*************************************************/

/* PRIV(jit_free_rodata) - from pcre2_jit_misc_inc.h, !SUPPORT_JIT branch: does
nothing. (The second parameter is the allocator data.) */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_rodata_8(current: *mut c_void, next: *mut c_void) {
/* (void)current; (void)allocator_data; */
}


/*************************************************
*              Get size of JIT code              *
*************************************************/

/* PRIV(jit_get_size) - from pcre2_jit_misc_inc.h, !SUPPORT_JIT branch. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_size_8(executable_jit: *mut c_void) -> usize {
/* (void)executable_jit; */
return 0;
}


/*************************************************
*               Get target CPU type              *
*************************************************/

/* PRIV(jit_get_target) - from pcre2_jit_misc_inc.h, !SUPPORT_JIT branch. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_target_8() -> *const c_char {
return b"JIT is not supported\0".as_ptr() as *const c_char;
}

/* End of pcre2_jit_compile.c */
