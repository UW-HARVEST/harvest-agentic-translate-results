// Translated from c_src/src/pcre2_context.c
use crate::internal::*;

/*************************************************
*          Default malloc/free functions         *
*************************************************/

/* Ignore the "user data" argument in each case. */

unsafe extern "C" fn default_malloc(size: usize, _data: *mut c_void) -> *mut c_void {
    malloc(size)
}

unsafe extern "C" fn default_free(block: *mut c_void, _data: *mut c_void) {
    free(block);
}

/*************************************************
*        Get a block and save memory control     *
*************************************************/

/* This internal function is called to get a block of memory in which the
memory control data is to be stored at the start for future use. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_memctl_malloc_8(
    size: usize,
    memctl: *mut pcre2_memctl,
) -> *mut c_void {
    let newmemctl: *mut pcre2_memctl;
    let yield_: *mut c_void = if memctl.is_null() {
        malloc(size)
    } else {
        ((*memctl).malloc.unwrap())(size, (*memctl).memory_data)
    };
    if yield_.is_null() {
        return core::ptr::null_mut();
    }
    newmemctl = yield_ as *mut pcre2_memctl;
    if memctl.is_null() {
        (*newmemctl).malloc = Some(default_malloc);
        (*newmemctl).free = Some(default_free);
        (*newmemctl).memory_data = core::ptr::null_mut();
    } else {
        *newmemctl = *memctl;
    }
    yield_
}

/*************************************************
*          Create and initialize contexts        *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_create_8(
    private_malloc: pcre2_malloc_fn,
    private_free: pcre2_free_fn,
    memory_data: *mut c_void,
) -> *mut pcre2_real_general_context {
    let mut private_malloc = private_malloc;
    let mut private_free = private_free;
    if private_malloc.is_none() {
        private_malloc = Some(default_malloc);
    }
    if private_free.is_none() {
        private_free = Some(default_free);
    }
    let gcontext = (private_malloc.unwrap())(size_of::<pcre2_real_general_context>(), memory_data)
        as *mut pcre2_real_general_context;
    if gcontext.is_null() {
        return core::ptr::null_mut();
    }
    (*gcontext).memctl.malloc = private_malloc;
    (*gcontext).memctl.free = private_free;
    (*gcontext).memctl.memory_data = memory_data;
    gcontext
}

/* A default compile context is set up to save having to initialize at run time
when no context is supplied to the compile function. */

#[unsafe(no_mangle)]
pub static mut _pcre2_default_compile_context_8: pcre2_real_compile_context =
    pcre2_real_compile_context {
        memctl: pcre2_memctl {
            malloc: Some(default_malloc),
            free: Some(default_free),
            memory_data: core::ptr::null_mut(),
        },
        stack_guard: None,
        stack_guard_data: core::ptr::null_mut(),
        tables: unsafe { _pcre2_default_tables_8.as_ptr() },
        max_pattern_length: PCRE2_UNSET,
        max_pattern_compiled_length: PCRE2_UNSET,
        bsr_convention: BSR_DEFAULT as u16,
        newline_convention: NEWLINE_DEFAULT as u16,
        parens_nest_limit: PARENS_NEST_LIMIT,
        extra_options: 0,
        max_varlookbehind: MAX_VARLOOKBEHIND,
        optimization_flags: PCRE2_OPTIMIZATION_ALL,
    };

/* The create function copies the default into the new memory, but must
override the default memory handling functions if a gcontext was provided. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_compile_context {
    let ccontext = _pcre2_memctl_malloc_8(
        size_of::<pcre2_real_compile_context>(),
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_real_compile_context;
    if ccontext.is_null() {
        return core::ptr::null_mut();
    }
    *ccontext = _pcre2_default_compile_context_8;
    if !gcontext.is_null() {
        *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    ccontext
}

/* A default match context is set up to save having to initialize at run time
when no context is supplied to a match function. */

#[unsafe(no_mangle)]
pub static mut _pcre2_default_match_context_8: pcre2_real_match_context =
    pcre2_real_match_context {
        memctl: pcre2_memctl {
            malloc: Some(default_malloc),
            free: Some(default_free),
            memory_data: core::ptr::null_mut(),
        },
        callout: None,
        callout_data: core::ptr::null_mut(),
        substitute_callout: None,
        substitute_callout_data: core::ptr::null_mut(),
        substitute_case_callout: None,
        substitute_case_callout_data: core::ptr::null_mut(),
        offset_limit: PCRE2_UNSET,
        heap_limit: HEAP_LIMIT,
        match_limit: MATCH_LIMIT,
        depth_limit: MATCH_LIMIT_DEPTH,
    };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_match_context {
    let mcontext = _pcre2_memctl_malloc_8(
        size_of::<pcre2_real_match_context>(),
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_real_match_context;
    if mcontext.is_null() {
        return core::ptr::null_mut();
    }
    *mcontext = _pcre2_default_match_context_8;
    if !gcontext.is_null() {
        *(mcontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    mcontext
}

/* A default convert context is set up to save having to initialize at run time
when no context is supplied to the convert function. */

#[unsafe(no_mangle)]
pub static mut _pcre2_default_convert_context_8: pcre2_real_convert_context =
    pcre2_real_convert_context {
        memctl: pcre2_memctl {
            malloc: Some(default_malloc),
            free: Some(default_free),
            memory_data: core::ptr::null_mut(),
        },
        glob_separator: CHAR_SLASH,
        glob_escape: CHAR_BACKSLASH,
    };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_create_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_convert_context {
    let ccontext = _pcre2_memctl_malloc_8(
        size_of::<pcre2_real_convert_context>(),
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_real_convert_context;
    if ccontext.is_null() {
        return core::ptr::null_mut();
    }
    *ccontext = _pcre2_default_convert_context_8;
    if !gcontext.is_null() {
        *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    ccontext
}

/*************************************************
*              Context copy functions            *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_copy_8(
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_general_context {
    let newcontext = ((*gcontext).memctl.malloc.unwrap())(
        size_of::<pcre2_real_general_context>(),
        (*gcontext).memctl.memory_data,
    ) as *mut pcre2_real_general_context;
    if newcontext.is_null() {
        return core::ptr::null_mut();
    }
    memcpy(
        newcontext as *mut c_void,
        gcontext as *const c_void,
        size_of::<pcre2_real_general_context>(),
    );
    newcontext
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_copy_8(
    ccontext: *mut pcre2_real_compile_context,
) -> *mut pcre2_real_compile_context {
    let newcontext = ((*ccontext).memctl.malloc.unwrap())(
        size_of::<pcre2_real_compile_context>(),
        (*ccontext).memctl.memory_data,
    ) as *mut pcre2_real_compile_context;
    if newcontext.is_null() {
        return core::ptr::null_mut();
    }
    memcpy(
        newcontext as *mut c_void,
        ccontext as *const c_void,
        size_of::<pcre2_real_compile_context>(),
    );
    newcontext
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_copy_8(
    mcontext: *mut pcre2_real_match_context,
) -> *mut pcre2_real_match_context {
    let newcontext = ((*mcontext).memctl.malloc.unwrap())(
        size_of::<pcre2_real_match_context>(),
        (*mcontext).memctl.memory_data,
    ) as *mut pcre2_real_match_context;
    if newcontext.is_null() {
        return core::ptr::null_mut();
    }
    memcpy(
        newcontext as *mut c_void,
        mcontext as *const c_void,
        size_of::<pcre2_real_match_context>(),
    );
    newcontext
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_copy_8(
    ccontext: *mut pcre2_real_convert_context,
) -> *mut pcre2_real_convert_context {
    let newcontext = ((*ccontext).memctl.malloc.unwrap())(
        size_of::<pcre2_real_convert_context>(),
        (*ccontext).memctl.memory_data,
    ) as *mut pcre2_real_convert_context;
    if newcontext.is_null() {
        return core::ptr::null_mut();
    }
    memcpy(
        newcontext as *mut c_void,
        ccontext as *const c_void,
        size_of::<pcre2_real_convert_context>(),
    );
    newcontext
}

/*************************************************
*              Context free functions            *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_free_8(
    gcontext: *mut pcre2_real_general_context,
) {
    if !gcontext.is_null() {
        ((*gcontext).memctl.free.unwrap())(gcontext as *mut c_void, (*gcontext).memctl.memory_data);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_free_8(
    ccontext: *mut pcre2_real_compile_context,
) {
    if !ccontext.is_null() {
        ((*ccontext).memctl.free.unwrap())(ccontext as *mut c_void, (*ccontext).memctl.memory_data);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_free_8(mcontext: *mut pcre2_real_match_context) {
    if !mcontext.is_null() {
        ((*mcontext).memctl.free.unwrap())(mcontext as *mut c_void, (*mcontext).memctl.memory_data);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_free_8(
    ccontext: *mut pcre2_real_convert_context,
) {
    if !ccontext.is_null() {
        ((*ccontext).memctl.free.unwrap())(ccontext as *mut c_void, (*ccontext).memctl.memory_data);
    }
}

/*************************************************
*             Set values in contexts             *
*************************************************/

/* ------------ Compile context ------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_character_tables_8(
    ccontext: *mut pcre2_real_compile_context,
    tables: *const u8,
) -> c_int {
    (*ccontext).tables = tables;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_bsr_8(
    ccontext: *mut pcre2_real_compile_context,
    value: u32,
) -> c_int {
    match value {
        PCRE2_BSR_ANYCRLF | PCRE2_BSR_UNICODE => {
            (*ccontext).bsr_convention = value as u16;
            0
        }
        _ => PCRE2_ERROR_BADDATA,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_pattern_length_8(
    ccontext: *mut pcre2_real_compile_context,
    length: PCRE2_SIZE,
) -> c_int {
    (*ccontext).max_pattern_length = length;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_pattern_compiled_length_8(
    ccontext: *mut pcre2_real_compile_context,
    length: PCRE2_SIZE,
) -> c_int {
    (*ccontext).max_pattern_compiled_length = length;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_newline_8(
    ccontext: *mut pcre2_real_compile_context,
    newline: u32,
) -> c_int {
    match newline {
        PCRE2_NEWLINE_CR | PCRE2_NEWLINE_LF | PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY
        | PCRE2_NEWLINE_ANYCRLF | PCRE2_NEWLINE_NUL => {
            (*ccontext).newline_convention = newline as u16;
            0
        }
        _ => PCRE2_ERROR_BADDATA,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_varlookbehind_8(
    ccontext: *mut pcre2_real_compile_context,
    limit: u32,
) -> c_int {
    (*ccontext).max_varlookbehind = limit;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_parens_nest_limit_8(
    ccontext: *mut pcre2_real_compile_context,
    limit: u32,
) -> c_int {
    (*ccontext).parens_nest_limit = limit;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_compile_extra_options_8(
    ccontext: *mut pcre2_real_compile_context,
    options: u32,
) -> c_int {
    (*ccontext).extra_options = options;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_compile_recursion_guard_8(
    ccontext: *mut pcre2_real_compile_context,
    guard: pcre2_stack_guard_fn,
    user_data: *mut c_void,
) -> c_int {
    (*ccontext).stack_guard = guard;
    (*ccontext).stack_guard_data = user_data;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_optimize_8(
    ccontext: *mut pcre2_real_compile_context,
    directive: u32,
) -> c_int {
    if ccontext.is_null() {
        return PCRE2_ERROR_NULL;
    }

    match directive {
        PCRE2_OPTIMIZATION_NONE => {
            (*ccontext).optimization_flags = 0;
        }

        PCRE2_OPTIMIZATION_FULL => {
            (*ccontext).optimization_flags = PCRE2_OPTIMIZATION_ALL;
        }

        _ => {
            if directive >= PCRE2_AUTO_POSSESS && directive <= PCRE2_START_OPTIMIZE_OFF {
                /* Even directive numbers starting from 64 switch a bit on;
                 * Odd directive numbers starting from 65 switch a bit off */
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

/* ------------ Match context ------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_callout_8(
    mcontext: *mut pcre2_real_match_context,
    callout: pcre2_callout_fn,
    callout_data: *mut c_void,
) -> c_int {
    (*mcontext).callout = callout;
    (*mcontext).callout_data = callout_data;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_substitute_callout_8(
    mcontext: *mut pcre2_real_match_context,
    substitute_callout: pcre2_substitute_callout_fn,
    substitute_callout_data: *mut c_void,
) -> c_int {
    (*mcontext).substitute_callout = substitute_callout;
    (*mcontext).substitute_callout_data = substitute_callout_data;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_substitute_case_callout_8(
    mcontext: *mut pcre2_real_match_context,
    substitute_case_callout: pcre2_substitute_case_callout_fn,
    substitute_case_callout_data: *mut c_void,
) -> c_int {
    (*mcontext).substitute_case_callout = substitute_case_callout;
    (*mcontext).substitute_case_callout_data = substitute_case_callout_data;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_heap_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    (*mcontext).heap_limit = limit;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_match_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    (*mcontext).match_limit = limit;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_depth_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    (*mcontext).depth_limit = limit;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_offset_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: PCRE2_SIZE,
) -> c_int {
    (*mcontext).offset_limit = limit;
    0
}

/* These functions became obsolete at release 10.30. The first is kept as a
synonym for backwards compatibility. The second now does nothing. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_limit_8(
    mcontext: *mut pcre2_real_match_context,
    limit: u32,
) -> c_int {
    pcre2_set_depth_limit_8(mcontext, limit)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_memory_management_8(
    _mcontext: *mut pcre2_real_match_context,
    _mymalloc: pcre2_malloc_fn,
    _myfree: pcre2_free_fn,
    _mydata: *mut c_void,
) -> c_int {
    0
}

/* ------------ Convert context ------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_separator_8(
    ccontext: *mut pcre2_real_convert_context,
    separator: u32,
) -> c_int {
    if separator != CHAR_SLASH && separator != CHAR_BACKSLASH && separator != CHAR_DOT {
        return PCRE2_ERROR_BADDATA;
    }
    (*ccontext).glob_separator = separator;
    0
}

static globpunct: [u8; 33] = *b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_escape_8(
    ccontext: *mut pcre2_real_convert_context,
    escape: u32,
) -> c_int {
    if escape > 255
        || (escape != 0
            && strchr(globpunct.as_ptr() as *const c_char, escape as c_int).is_null())
    {
        return PCRE2_ERROR_BADDATA;
    }
    (*ccontext).glob_escape = escape;
    0
}
