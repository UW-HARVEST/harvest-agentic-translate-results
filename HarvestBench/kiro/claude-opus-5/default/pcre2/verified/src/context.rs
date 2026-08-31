//! Translation of `c_src/src/pcre2_context.c`.

#![allow(non_snake_case)]

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::chars::*;
use crate::chartables::DEFAULT_TABLES;
use crate::internal::*;

/* The default character tables, exported as `_pcre2_default_tables_8`. */
#[unsafe(no_mangle)]
pub static _pcre2_default_tables_8: [u8; TABLES_LENGTH] = DEFAULT_TABLES;

/* Default contexts. These are mutable in C (non-const globals) but are only
ever read by the library. */

#[unsafe(no_mangle)]
pub static mut _pcre2_default_compile_context_8: pcre2_real_compile_context =
    pcre2_real_compile_context {
        memctl: pcre2_memctl {
            malloc: Some(default_malloc),
            free: Some(default_free),
            memory_data: ptr::null_mut(),
        },
        stack_guard: None,
        stack_guard_data: ptr::null_mut(),
        tables: &raw const _pcre2_default_tables_8 as *const u8,
        max_pattern_length: PCRE2_UNSET,
        max_pattern_compiled_length: PCRE2_UNSET,
        bsr_convention: BSR_DEFAULT as u16,
        newline_convention: NEWLINE_DEFAULT as u16,
        parens_nest_limit: PARENS_NEST_LIMIT,
        extra_options: 0,
        max_varlookbehind: MAX_VARLOOKBEHIND,
        optimization_flags: PCRE2_OPTIMIZATION_ALL,
    };

#[unsafe(no_mangle)]
pub static mut _pcre2_default_match_context_8: pcre2_real_match_context =
    pcre2_real_match_context {
        memctl: pcre2_memctl {
            malloc: Some(default_malloc),
            free: Some(default_free),
            memory_data: ptr::null_mut(),
        },
        callout: None,
        callout_data: ptr::null_mut(),
        substitute_callout: None,
        substitute_callout_data: ptr::null_mut(),
        substitute_case_callout: None,
        substitute_case_callout_data: ptr::null_mut(),
        offset_limit: PCRE2_UNSET,
        heap_limit: HEAP_LIMIT,
        match_limit: MATCH_LIMIT,
        depth_limit: MATCH_LIMIT_DEPTH,
    };

#[unsafe(no_mangle)]
pub static mut _pcre2_default_convert_context_8: pcre2_real_convert_context =
    pcre2_real_convert_context {
        memctl: pcre2_memctl {
            malloc: Some(default_malloc),
            free: Some(default_free),
            memory_data: ptr::null_mut(),
        },
        /* Not _WIN32 */
        glob_separator: CHAR_SLASH,
        glob_escape: CHAR_BACKSLASH,
    };

/* ------------------ Create and initialize contexts ------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_create_8(
    private_malloc: MallocFn,
    private_free: FreeFn,
    memory_data: *mut c_void,
) -> *mut pcre2_real_general_context {
    unsafe {
        let private_malloc = match private_malloc {
            None => default_malloc as unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void,
            Some(f) => f,
        };
        let private_free = match private_free {
            None => default_free as unsafe extern "C" fn(*mut c_void, *mut c_void),
            Some(f) => f,
        };
        let gcontext = private_malloc(
            core::mem::size_of::<pcre2_real_general_context>(),
            memory_data,
        ) as *mut pcre2_real_general_context;
        if gcontext.is_null() {
            return ptr::null_mut();
        }
        (*gcontext).memctl.malloc = Some(private_malloc);
        (*gcontext).memctl.free = Some(private_free);
        (*gcontext).memctl.memory_data = memory_data;
        gcontext
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_compile_context {
    unsafe {
        let ccontext = memctl_malloc(
            core::mem::size_of::<pcre2_real_compile_context>(),
            gcontext as *mut pcre2_memctl,
        ) as *mut pcre2_real_compile_context;
        if ccontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(&raw const _pcre2_default_compile_context_8, ccontext, 1);
        if !gcontext.is_null() {
            *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
        }
        ccontext
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_match_context {
    unsafe {
        let mcontext = memctl_malloc(
            core::mem::size_of::<pcre2_real_match_context>(),
            gcontext as *mut pcre2_memctl,
        ) as *mut pcre2_real_match_context;
        if mcontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(&raw const _pcre2_default_match_context_8, mcontext, 1);
        if !gcontext.is_null() {
            *(mcontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
        }
        mcontext
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_convert_context {
    unsafe {
        let ccontext = memctl_malloc(
            core::mem::size_of::<pcre2_real_convert_context>(),
            gcontext as *mut pcre2_memctl,
        ) as *mut pcre2_real_convert_context;
        if ccontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(&raw const _pcre2_default_convert_context_8, ccontext, 1);
        if !gcontext.is_null() {
            *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
        }
        ccontext
    }
}

/* ------------------ Context copy functions ------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_copy_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_general_context {
    unsafe {
        let newcontext = ((*gcontext).memctl.malloc.unwrap())(
            core::mem::size_of::<pcre2_real_general_context>(),
            (*gcontext).memctl.memory_data,
        ) as *mut pcre2_real_general_context;
        if newcontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(
            gcontext as *const u8,
            newcontext as *mut u8,
            core::mem::size_of::<pcre2_real_general_context>(),
        );
        newcontext
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_copy_8(
    ccontext: *mut pcre2_real_compile_context,
) -> *mut pcre2_real_compile_context {
    unsafe {
        let newcontext = ((*ccontext).memctl.malloc.unwrap())(
            core::mem::size_of::<pcre2_real_compile_context>(),
            (*ccontext).memctl.memory_data,
        ) as *mut pcre2_real_compile_context;
        if newcontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(
            ccontext as *const u8,
            newcontext as *mut u8,
            core::mem::size_of::<pcre2_real_compile_context>(),
        );
        newcontext
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_copy_8(
    mcontext: *mut pcre2_real_match_context,
) -> *mut pcre2_real_match_context {
    unsafe {
        let newcontext = ((*mcontext).memctl.malloc.unwrap())(
            core::mem::size_of::<pcre2_real_match_context>(),
            (*mcontext).memctl.memory_data,
        ) as *mut pcre2_real_match_context;
        if newcontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(
            mcontext as *const u8,
            newcontext as *mut u8,
            core::mem::size_of::<pcre2_real_match_context>(),
        );
        newcontext
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_copy_8(
    ccontext: *mut pcre2_real_convert_context,
) -> *mut pcre2_real_convert_context {
    unsafe {
        let newcontext = ((*ccontext).memctl.malloc.unwrap())(
            core::mem::size_of::<pcre2_real_convert_context>(),
            (*ccontext).memctl.memory_data,
        ) as *mut pcre2_real_convert_context;
        if newcontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(
            ccontext as *const u8,
            newcontext as *mut u8,
            core::mem::size_of::<pcre2_real_convert_context>(),
        );
        newcontext
    }
}

/* ------------------ Context free functions ------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_free_8(
    gcontext: *mut pcre2_real_general_context,
) {
    unsafe {
        if !gcontext.is_null() {
            ((*gcontext).memctl.free.unwrap())(
                gcontext as *mut c_void,
                (*gcontext).memctl.memory_data,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_free_8(
    ccontext: *mut pcre2_real_compile_context,
) {
    unsafe {
        if !ccontext.is_null() {
            ((*ccontext).memctl.free.unwrap())(
                ccontext as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_free_8(mcontext: *mut pcre2_real_match_context) {
    unsafe {
        if !mcontext.is_null() {
            ((*mcontext).memctl.free.unwrap())(
                mcontext as *mut c_void,
                (*mcontext).memctl.memory_data,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_free_8(
    ccontext: *mut pcre2_real_convert_context,
) {
    unsafe {
        if !ccontext.is_null() {
            ((*ccontext).memctl.free.unwrap())(
                ccontext as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }
    }
}

/* ------------------ Compile context setters ------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_character_tables_8(
    ccontext: *mut pcre2_real_compile_context,
    tables: *const u8,
) -> c_int {
    unsafe {
        (*ccontext).tables = tables;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_bsr_8(
    ccontext: *mut pcre2_real_compile_context,
    value: u32,
) -> c_int {
    unsafe {
        match value {
            PCRE2_BSR_ANYCRLF | PCRE2_BSR_UNICODE => {
                (*ccontext).bsr_convention = value as u16;
                0
            }
            _ => PCRE2_ERROR_BADDATA,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_pattern_length_8(
    ccontext: *mut pcre2_real_compile_context,
    length: PCRE2_SIZE,
) -> c_int {
    unsafe {
        (*ccontext).max_pattern_length = length;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_pattern_compiled_length_8(
    ccontext: *mut pcre2_real_compile_context,
    length: PCRE2_SIZE,
) -> c_int {
    unsafe {
        (*ccontext).max_pattern_compiled_length = length;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_newline_8(
    ccontext: *mut pcre2_real_compile_context,
    newline: u32,
) -> c_int {
    unsafe {
        match newline {
            PCRE2_NEWLINE_CR | PCRE2_NEWLINE_LF | PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY
            | PCRE2_NEWLINE_ANYCRLF | PCRE2_NEWLINE_NUL => {
                (*ccontext).newline_convention = newline as u16;
                0
            }
            _ => PCRE2_ERROR_BADDATA,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_varlookbehind_8(
    ccontext: *mut pcre2_real_compile_context,
    limit: u32,
) -> c_int {
    unsafe {
        (*ccontext).max_varlookbehind = limit;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_parens_nest_limit_8(
    ccontext: *mut pcre2_real_compile_context,
    limit: u32,
) -> c_int {
    unsafe {
        (*ccontext).parens_nest_limit = limit;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_compile_extra_options_8(
    ccontext: *mut pcre2_real_compile_context,
    options: u32,
) -> c_int {
    unsafe {
        (*ccontext).extra_options = options;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_compile_recursion_guard_8(
    ccontext: *mut pcre2_real_compile_context,
    guard: StackGuardFn,
    user_data: *mut c_void,
) -> c_int {
    unsafe {
        (*ccontext).stack_guard = guard;
        (*ccontext).stack_guard_data = user_data;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_optimize_8(
    ccontext: *mut pcre2_real_compile_context,
    directive: u32,
) -> c_int {
    unsafe {
        if ccontext.is_null() {
            return PCRE2_ERROR_NULL;
        }

        match directive {
            PCRE2_OPTIMIZATION_NONE => (*ccontext).optimization_flags = 0,
            PCRE2_OPTIMIZATION_FULL => (*ccontext).optimization_flags = PCRE2_OPTIMIZATION_ALL,
            _ => {
                if directive >= PCRE2_AUTO_POSSESS && directive <= PCRE2_START_OPTIMIZE_OFF {
                    /* Even directive numbers starting from 64 switch a bit on;
                    odd directive numbers starting from 65 switch a bit off. */
                    if (directive & 1) != 0 {
                        (*ccontext).optimization_flags &= !(1u32 << ((directive >> 1) - 32));
                    } else {
                        (*ccontext).optimization_flags |= 1u32 << ((directive >> 1) - 32);
                    }
                    return 0;
                }
                return PCRE2_ERROR_BADOPTION;
            }
        }

        0
    }
}

/* ------------------ Match context setters ------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_callout_8(
    mcontext: *mut pcre2_real_match_context,
    callout: CalloutFn,
    callout_data: *mut c_void,
) -> c_int {
    unsafe {
        (*mcontext).callout = callout;
        (*mcontext).callout_data = callout_data;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_substitute_callout_8(
    mcontext: *mut pcre2_real_match_context,
    substitute_callout: SubstituteCalloutFn,
    substitute_callout_data: *mut c_void,
) -> c_int {
    unsafe {
        (*mcontext).substitute_callout = substitute_callout;
        (*mcontext).substitute_callout_data = substitute_callout_data;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_substitute_case_callout_8(
    mcontext: *mut pcre2_real_match_context,
    substitute_case_callout: SubstituteCaseCalloutFn,
    substitute_case_callout_data: *mut c_void,
) -> c_int {
    unsafe {
        (*mcontext).substitute_case_callout = substitute_case_callout;
        (*mcontext).substitute_case_callout_data = substitute_case_callout_data;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_heap_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    unsafe {
        (*mcontext).heap_limit = limit;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_match_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    unsafe {
        (*mcontext).match_limit = limit;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_depth_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    unsafe {
        (*mcontext).depth_limit = limit;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_offset_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: PCRE2_SIZE,
) -> c_int {
    unsafe {
        (*mcontext).offset_limit = limit;
        0
    }
}

/* Obsolete since 10.30; kept for backwards compatibility. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    unsafe { pcre2_set_depth_limit_8(mcontext, limit) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_memory_management_8(
    _mcontext: *mut pcre2_real_match_context,
    _mymalloc: MallocFn,
    _myfree: FreeFn,
    _mydata: *mut c_void,
) -> c_int {
    0
}

/* ------------------ Convert context setters ------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_separator_8(
    ccontext: *mut pcre2_real_convert_context,
    separator: u32,
) -> c_int {
    unsafe {
        if separator != CHAR_SLASH && separator != CHAR_BACKSLASH && separator != CHAR_DOT {
            return PCRE2_ERROR_BADDATA;
        }
        (*ccontext).glob_separator = separator;
        0
    }
}

static GLOBPUNCT: &[u8] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_escape_8(
    ccontext: *mut pcre2_real_convert_context,
    escape: u32,
) -> c_int {
    unsafe {
        if escape > 255 || (escape != 0 && !GLOBPUNCT.contains(&(escape as u8))) {
            return PCRE2_ERROR_BADDATA;
        }
        (*ccontext).glob_escape = escape;
        0
    }
}
