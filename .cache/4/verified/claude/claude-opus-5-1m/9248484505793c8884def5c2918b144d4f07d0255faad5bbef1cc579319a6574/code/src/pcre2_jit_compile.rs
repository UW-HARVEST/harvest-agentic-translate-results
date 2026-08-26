// Translated from c_src/src/pcre2_jit_compile.c, pcre2_jit_match_inc.h, pcre2_jit_misc_inc.h
// (SUPPORT_JIT is not defined, so only the non-JIT stubs are compiled)
use crate::internal::*;

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
pub unsafe extern "C" fn pcre2_jit_compile_8(code: *mut pcre2_real_code, options: u32) -> c_int {
    let re: *mut pcre2_real_code = code;

    if options & PCRE2_JIT_TEST_ALLOC != 0 {
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

    /* Support for invalid UTF was first introduced in JIT, with the option
    PCRE2_JIT_INVALID_UTF. Later, support was added to the interpreter, and the
    compile-time option PCRE2_MATCH_INVALID_UTF was created. This is now the
    preferred feature, with the earlier option deprecated. However, for backward
    compatibility, if the earlier option is set, it forces the new option so that
    if JIT matching falls back to the interpreter, there is still support for
    invalid UTF. However, if this function has already been successfully called
    without PCRE2_JIT_INVALID_UTF and without PCRE2_MATCH_INVALID_UTF (meaning that
    non-invalid-supporting JIT code was compiled), give an error. */

    if (options & PCRE2_JIT_INVALID_UTF) != 0 {
        if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) == 0 {
            (*re).overall_options |= PCRE2_MATCH_INVALID_UTF;
        }
    }

    /* The above tests are run with and without JIT support. This means that
    PCRE2_JIT_INVALID_UTF propagates back into the regex options (ensuring
    interpreter support) even in the absence of JIT. But now, if there is no JIT
    support, give an error return. */

    PCRE2_ERROR_JIT_BADOPTION
}

/*************************************************
*              Do a JIT pattern match            *
*************************************************/

/* This function runs a JIT pattern match.

Arguments:
  code            points to the compiled expression
  subject         points to the subject string
  length          length of subject string (may contain binary zeros)
  start_offset    where to start in the subject string
  options         option bits
  match_data      points to a match_data block
  mcontext        points to a match context

Returns:          > 0 => success; value is the number of ovector pairs filled
                  = 0 => success, but ovector is not big enough
                   -1 => failed to match (PCRE2_ERROR_NOMATCH)
                 < -1 => some kind of unexpected problem
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_match_8(
    code: *const pcre2_real_code,
    subject: PCRE2_SPTR,
    length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
) -> c_int {
    let _ = code;
    let _ = subject;
    let _ = length;
    let _ = start_offset;
    let _ = options;
    let _ = mcontext;
    (*match_data).rc = PCRE2_ERROR_JIT_BADOPTION;
    (*match_data).rc
}

/*************************************************
*           Free JIT read-only data              *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_rodata_8(current: *mut c_void, allocator_data: *mut c_void) {
    let _ = current;
    let _ = allocator_data;
}

/*************************************************
*           Free JIT compiled code               *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_free_8(executable_jit: *mut c_void, memctl: *mut pcre2_memctl) {
    let _ = executable_jit;
    let _ = memctl;
}

/*************************************************
*            Free unused JIT memory              *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_free_unused_memory_8(gcontext: *mut pcre2_real_general_context) {
    let _ = gcontext; /* Suppress warning */
}

/*************************************************
*            Allocate a JIT stack                *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_create_8(
    startsize: usize,
    maxsize: usize,
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_jit_stack {
    let _ = gcontext;
    let _ = startsize;
    let _ = maxsize;
    std::ptr::null_mut()
}

/*************************************************
*         Assign a JIT stack to a pattern        *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_assign_8(
    mcontext: *mut pcre2_real_match_context,
    callback: pcre2_jit_callback,
    callback_data: *mut c_void,
) {
    let _ = mcontext;
    let _ = callback;
    let _ = callback_data;
}

/*************************************************
*               Free a JIT stack                 *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_jit_stack_free_8(jit_stack: *mut pcre2_real_jit_stack) {
    let _ = jit_stack;
}

/*************************************************
*               Get target CPU type              *
*************************************************/

static JIT_TARGET_NOT_SUPPORTED: [u8; 21] = *b"JIT is not supported\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_target_8() -> *const c_char {
    JIT_TARGET_NOT_SUPPORTED.as_ptr() as *const c_char
}

/*************************************************
*              Get size of JIT code              *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_jit_get_size_8(executable_jit: *mut c_void) -> usize {
    let _ = executable_jit;
    0
}

/* End of pcre2_jit_compile.c */
