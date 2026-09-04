//! Translation of `pcre2_context.c`.

use crate::internal::*;
use crate::tables;
use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// Default malloc/free functions
// ---------------------------------------------------------------------------

unsafe extern "C" fn ctx_default_malloc(size: usize, _data: *mut c_void) -> *mut c_void {
    unsafe { malloc(size) }
}

unsafe extern "C" fn ctx_default_free(block: *mut c_void, _data: *mut c_void) {
    unsafe { free(block) }
}

// ---------------------------------------------------------------------------
// Get a block and save memory control
// ---------------------------------------------------------------------------

/// `PRIV(memctl_malloc)` — allocate a block whose first bytes hold a copy of the
/// memory control data.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_memctl_malloc_8(
    size: usize,
    memctl: *mut pcre2_memctl,
) -> *mut c_void {
    unsafe {
        let yield_ = if memctl.is_null() {
            malloc(size)
        } else {
            ((*memctl).malloc.unwrap())(size, (*memctl).memory_data)
        };
        if yield_.is_null() {
            return ptr::null_mut();
        }
        let newmemctl = yield_ as *mut pcre2_memctl;
        if memctl.is_null() {
            (*newmemctl).malloc = Some(ctx_default_malloc);
            (*newmemctl).free = Some(ctx_default_free);
            (*newmemctl).memory_data = ptr::null_mut();
        } else {
            *newmemctl = *memctl;
        }
        yield_
    }
}

// ---------------------------------------------------------------------------
// Create and initialize contexts
// ---------------------------------------------------------------------------

/// `pcre2_general_context_create()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_create_8(
    private_malloc: MallocFn,
    private_free: FreeFn,
    memory_data: *mut c_void,
) -> *mut pcre2_real_general_context {
    unsafe {
        let private_malloc = private_malloc.unwrap_or(ctx_default_malloc);
        let private_free = private_free.unwrap_or(ctx_default_free);
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

/// `PRIV(default_compile_context)`.
#[unsafe(no_mangle)]
pub static mut _pcre2_default_compile_context_8: pcre2_real_compile_context =
    pcre2_real_compile_context {
        memctl: pcre2_memctl {
            malloc: Some(ctx_default_malloc),
            free: Some(ctx_default_free),
            memory_data: ptr::null_mut(),
        },
        stack_guard: None,
        stack_guard_data: ptr::null_mut(),
        tables: unsafe { tables::_pcre2_default_tables_8.as_ptr() },
        max_pattern_length: PCRE2_UNSET,
        max_pattern_compiled_length: PCRE2_UNSET,
        bsr_convention: BSR_DEFAULT as u16,
        newline_convention: NEWLINE_DEFAULT as u16,
        parens_nest_limit: PARENS_NEST_LIMIT as u32,
        extra_options: 0,
        max_varlookbehind: MAX_VARLOOKBEHIND as u32,
        optimization_flags: PCRE2_OPTIMIZATION_ALL as u32,
    };

/// `pcre2_compile_context_create()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_compile_context {
    unsafe {
        let ccontext = _pcre2_memctl_malloc_8(
            core::mem::size_of::<pcre2_real_compile_context>(),
            gcontext as *mut pcre2_memctl,
        ) as *mut pcre2_real_compile_context;
        if ccontext.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(
            &raw const _pcre2_default_compile_context_8,
            ccontext,
            1,
        );
        if !gcontext.is_null() {
            *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
        }
        ccontext
    }
}

/// `PRIV(default_match_context)`.
#[unsafe(no_mangle)]
pub static mut _pcre2_default_match_context_8: pcre2_real_match_context =
    pcre2_real_match_context {
        memctl: pcre2_memctl {
            malloc: Some(ctx_default_malloc),
            free: Some(ctx_default_free),
            memory_data: ptr::null_mut(),
        },
        callout: None,
        callout_data: ptr::null_mut(),
        substitute_callout: None,
        substitute_callout_data: ptr::null_mut(),
        substitute_case_callout: None,
        substitute_case_callout_data: ptr::null_mut(),
        offset_limit: PCRE2_UNSET,
        heap_limit: HEAP_LIMIT as u32,
        match_limit: MATCH_LIMIT as u32,
        depth_limit: MATCH_LIMIT_DEPTH as u32,
    };

/// `pcre2_match_context_create()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_match_context {
    unsafe {
        let mcontext = _pcre2_memctl_malloc_8(
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

const CHAR_SLASH: u32 = 0x2f;
const CHAR_BACKSLASH: u32 = 0x5c;
const CHAR_DOT: u32 = 0x2e;

/// `PRIV(default_convert_context)`.
#[unsafe(no_mangle)]
pub static mut _pcre2_default_convert_context_8: pcre2_real_convert_context =
    pcre2_real_convert_context {
        memctl: pcre2_memctl {
            malloc: Some(ctx_default_malloc),
            free: Some(ctx_default_free),
            memory_data: ptr::null_mut(),
        },
        // Not Windows: '/' path separator, '\' escape character.
        glob_separator: CHAR_SLASH,
        glob_escape: CHAR_BACKSLASH,
    };

/// `pcre2_convert_context_create()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_convert_context {
    unsafe {
        let ccontext = _pcre2_memctl_malloc_8(
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

// ---------------------------------------------------------------------------
// Context copy functions
// ---------------------------------------------------------------------------

/// `pcre2_general_context_copy()`.
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
        c_memcpy(
            newcontext as *mut c_void,
            gcontext as *const c_void,
            core::mem::size_of::<pcre2_real_general_context>(),
        );
        newcontext
    }
}

/// `pcre2_compile_context_copy()`.
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
        c_memcpy(
            newcontext as *mut c_void,
            ccontext as *const c_void,
            core::mem::size_of::<pcre2_real_compile_context>(),
        );
        newcontext
    }
}

/// `pcre2_match_context_copy()`.
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
        c_memcpy(
            newcontext as *mut c_void,
            mcontext as *const c_void,
            core::mem::size_of::<pcre2_real_match_context>(),
        );
        newcontext
    }
}

/// `pcre2_convert_context_copy()`.
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
        c_memcpy(
            newcontext as *mut c_void,
            ccontext as *const c_void,
            core::mem::size_of::<pcre2_real_convert_context>(),
        );
        newcontext
    }
}

// ---------------------------------------------------------------------------
// Context free functions
// ---------------------------------------------------------------------------

/// `pcre2_general_context_free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_free_8(gcontext: *mut pcre2_real_general_context) {
    unsafe {
        if !gcontext.is_null() {
            ((*gcontext).memctl.free.unwrap())(
                gcontext as *mut c_void,
                (*gcontext).memctl.memory_data,
            );
        }
    }
}

/// `pcre2_compile_context_free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_free_8(ccontext: *mut pcre2_real_compile_context) {
    unsafe {
        if !ccontext.is_null() {
            ((*ccontext).memctl.free.unwrap())(
                ccontext as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }
    }
}

/// `pcre2_match_context_free()`.
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

/// `pcre2_convert_context_free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_free_8(ccontext: *mut pcre2_real_convert_context) {
    unsafe {
        if !ccontext.is_null() {
            ((*ccontext).memctl.free.unwrap())(
                ccontext as *mut c_void,
                (*ccontext).memctl.memory_data,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Set values in contexts
// ---------------------------------------------------------------------------

// ------------ Compile context ------------

/// `pcre2_set_character_tables()`.
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

/// `pcre2_set_bsr()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_bsr_8(
    ccontext: *mut pcre2_real_compile_context,
    value: u32,
) -> c_int {
    unsafe {
        if value == PCRE2_BSR_ANYCRLF as u32 || value == PCRE2_BSR_UNICODE as u32 {
            (*ccontext).bsr_convention = value as u16;
            0
        } else {
            PCRE2_ERROR_BADDATA as c_int
        }
    }
}

/// `pcre2_set_max_pattern_length()`.
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

/// `pcre2_set_max_pattern_compiled_length()`.
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

/// `pcre2_set_newline()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_newline_8(
    ccontext: *mut pcre2_real_compile_context,
    newline: u32,
) -> c_int {
    unsafe {
        match newline as i64 {
            PCRE2_NEWLINE_CR | PCRE2_NEWLINE_LF | PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY
            | PCRE2_NEWLINE_ANYCRLF | PCRE2_NEWLINE_NUL => {
                (*ccontext).newline_convention = newline as u16;
                0
            }
            _ => PCRE2_ERROR_BADDATA as c_int,
        }
    }
}

/// `pcre2_set_max_varlookbehind()`.
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

/// `pcre2_set_parens_nest_limit()`.
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

/// `pcre2_set_compile_extra_options()`.
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

/// `pcre2_set_compile_recursion_guard()`.
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

/// `pcre2_set_optimize()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_optimize_8(
    ccontext: *mut pcre2_real_compile_context,
    directive: u32,
) -> c_int {
    unsafe {
        if ccontext.is_null() {
            return PCRE2_ERROR_NULL as c_int;
        }

        if directive == PCRE2_OPTIMIZATION_NONE as u32 {
            (*ccontext).optimization_flags = 0;
        } else if directive == PCRE2_OPTIMIZATION_FULL as u32 {
            (*ccontext).optimization_flags = PCRE2_OPTIMIZATION_ALL as u32;
        } else if directive >= PCRE2_AUTO_POSSESS as u32
            && directive <= PCRE2_START_OPTIMIZE_OFF as u32
        {
            // Even directive numbers starting from 64 switch a bit on;
            // odd directive numbers starting from 65 switch a bit off.
            if (directive & 1) != 0 {
                (*ccontext).optimization_flags &= !(1u32 << ((directive >> 1) - 32));
            } else {
                (*ccontext).optimization_flags |= 1u32 << ((directive >> 1) - 32);
            }
            return 0;
        } else {
            return PCRE2_ERROR_BADOPTION as c_int;
        }

        0
    }
}

// ------------ Match context ------------

/// `pcre2_set_callout()`.
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

/// `pcre2_set_substitute_callout()`.
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

/// `pcre2_set_substitute_case_callout()`.
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

/// `pcre2_set_heap_limit()`.
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

/// `pcre2_set_match_limit()`.
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

/// `pcre2_set_depth_limit()`.
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

/// `pcre2_set_offset_limit()`.
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

/// `pcre2_set_recursion_limit()` — obsolete synonym for `pcre2_set_depth_limit()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    unsafe { pcre2_set_depth_limit_8(mcontext, limit) }
}

/// `pcre2_set_recursion_memory_management()` — obsolete, does nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_memory_management_8(
    _mcontext: *mut pcre2_real_match_context,
    _mymalloc: MallocFn,
    _myfree: FreeFn,
    _mydata: *mut c_void,
) -> c_int {
    0
}

// ------------ Convert context ------------

/// `pcre2_set_glob_separator()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_separator_8(
    ccontext: *mut pcre2_real_convert_context,
    separator: u32,
) -> c_int {
    unsafe {
        if separator != CHAR_SLASH && separator != CHAR_BACKSLASH && separator != CHAR_DOT {
            return PCRE2_ERROR_BADDATA as c_int;
        }
        (*ccontext).glob_separator = separator;
        0
    }
}

/// The set of punctuation characters allowed as a glob escape.
static GLOBPUNCT: &[u8; 33] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~\0";

/// `pcre2_set_glob_escape()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_escape_8(
    ccontext: *mut pcre2_real_convert_context,
    escape: u32,
) -> c_int {
    unsafe {
        // `strchr(globpunct, escape)`: a search for 0 finds the terminator, so
        // escape == 0 is allowed (and short-circuited in the C too).
        if escape > 255 || (escape != 0 && strchr_u8(GLOBPUNCT, escape as u8).is_none()) {
            return PCRE2_ERROR_BADDATA as c_int;
        }
        (*ccontext).glob_escape = escape;
        0
    }
}

fn strchr_u8(s: &[u8], c: u8) -> Option<usize> {
    // Mirrors strchr(): the terminating NUL is part of the searched string.
    for (i, &b) in s.iter().enumerate() {
        if b == c {
            return Some(i);
        }
        if b == 0 {
            return None;
        }
    }
    None
}

const _: () = {
    // `strchr` compares as `char`, so values above 127 are sign-extended before
    // comparison on platforms with a signed `char`. All members of `globpunct`
    // are ASCII, so an `escape` above 127 can never match; the `escape > 255`
    // guard plus the byte comparison above reproduces this.
};

/// Marker so that `c_char` stays referenced (the C file uses `const char *`).
type _Unused = c_char;
