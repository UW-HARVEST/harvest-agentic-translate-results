// Translated from c_src/src/pcre2_match_data.c
use crate::internal::*;

/*************************************************
*  Create a match data block given ovector size  *
*************************************************/

/* A minimum of 1 is imposed on the number of ovector pairs. A maximum is also
imposed because the oveccount field in a match data block is uintt6_t. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_data_create_8(
    mut oveccount: u32,
    gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_match_data {
    let yield_: *mut pcre2_real_match_data;
    if oveccount < 1 {
        oveccount = 1;
    }
    if oveccount > 0xffff
    /* UINT16_MAX */
    {
        oveccount = 0xffff;
    }
    yield_ = _pcre2_memctl_malloc_8(
        offset_of!(pcre2_real_match_data, ovector) + 2 * (oveccount as usize) * size_of::<PCRE2_SIZE>(),
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_real_match_data;
    if yield_.is_null() {
        return std::ptr::null_mut();
    }
    (*yield_).oveccount = oveccount as u16;
    (*yield_).flags = 0;
    (*yield_).heapframes = std::ptr::null_mut();
    (*yield_).heapframes_size = 0;
    yield_
}

/*************************************************
*  Create a match data block using pattern data  *
*************************************************/

/* If no context is supplied, use the memory allocator from the code. This code
assumes that a general context contains nothing other than a memory allocator.
If that ever changes, this code will need fixing. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_data_create_from_pattern_8(
    code: *const pcre2_real_code,
    mut gcontext: *mut pcre2_real_general_context,
) -> *mut pcre2_real_match_data {
    if code.is_null() {
        return std::ptr::null_mut();
    }
    if gcontext.is_null() {
        gcontext = code as *mut pcre2_real_general_context;
    }
    pcre2_match_data_create_8((*code).top_bracket as u32 + 1, gcontext)
}

/*************************************************
*            Free a match data block             *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_data_free_8(match_data: *mut pcre2_real_match_data) {
    if !match_data.is_null() {
        if !(*match_data).heapframes.is_null() {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).heapframes as *mut c_void,
                (*match_data).memctl.memory_data,
            );
        }
        if ((*match_data).flags as u32 & PCRE2_MD_COPIED_SUBJECT) != 0 {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).subject as *mut c_void,
                (*match_data).memctl.memory_data,
            );
        }
        ((*match_data).memctl.free.unwrap())(
            match_data as *mut c_void,
            (*match_data).memctl.memory_data,
        );
    }
}

/*************************************************
*         Get last mark in match                 *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_mark_8(match_data: *mut pcre2_real_match_data) -> PCRE2_SPTR {
    (*match_data).mark
}

/*************************************************
*          Get pointer to ovector                *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_ovector_pointer_8(
    match_data: *mut pcre2_real_match_data,
) -> *mut PCRE2_SIZE {
    (*match_data).ovector.as_mut_ptr()
}

/*************************************************
*          Get number of ovector slots           *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_ovector_count_8(match_data: *mut pcre2_real_match_data) -> u32 {
    (*match_data).oveccount as u32
}

/*************************************************
*         Get starting code unit in match        *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_startchar_8(
    match_data: *mut pcre2_real_match_data,
) -> PCRE2_SIZE {
    (*match_data).startchar
}

/*************************************************
*         Get size of match data block           *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_match_data_size_8(
    match_data: *mut pcre2_real_match_data,
) -> PCRE2_SIZE {
    offset_of!(pcre2_real_match_data, ovector)
        + 2 * ((*match_data).oveccount as usize) * size_of::<PCRE2_SIZE>()
}

/*************************************************
*             Get heapframes size                *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_match_data_heapframes_size_8(
    match_data: *mut pcre2_real_match_data,
) -> PCRE2_SIZE {
    (*match_data).heapframes_size
}

/* End of pcre2_match_data.c */
