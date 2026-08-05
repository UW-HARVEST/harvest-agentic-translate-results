use crate::pcre2_internal::*;
use crate::pcre2_context::_pcre2_memctl_malloc_8;
use core::ffi::c_void;
use core::ptr;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_data_create_8(
    mut oveccount: u32,
    gcontext: *mut pcre2_general_context,
) -> *mut pcre2_match_data {
    if oveccount < 1 {
        oveccount = 1;
    }
    if oveccount > u16::MAX as u32 {
        oveccount = u16::MAX as u32;
    }
    let sz = match_data_ovector_offset()
        + 2 * (oveccount as usize) * core::mem::size_of::<PCRE2_SIZE>();
    let yield_ = _pcre2_memctl_malloc_8(sz, gcontext as *mut pcre2_memctl) as *mut pcre2_match_data;
    if yield_.is_null() {
        return ptr::null_mut();
    }
    (*yield_).oveccount = oveccount as u16;
    (*yield_).flags = 0;
    (*yield_).heapframes = ptr::null_mut();
    (*yield_).heapframes_size = 0;
    yield_
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_data_create_from_pattern_8(
    code: *const pcre2_code,
    mut gcontext: *mut pcre2_general_context,
) -> *mut pcre2_match_data {
    if code.is_null() {
        return ptr::null_mut();
    }
    if gcontext.is_null() {
        gcontext = code as *mut pcre2_general_context;
    }
    pcre2_match_data_create_8((*(code as *const pcre2_real_code)).top_bracket as u32 + 1, gcontext)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_data_free_8(match_data: *mut pcre2_match_data) {
    if !match_data.is_null() {
        if !(*match_data).heapframes.is_null() {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).heapframes,
                (*match_data).memctl.memory_data,
            );
        }
        if ((*match_data).flags & PCRE2_MD_COPIED_SUBJECT) != 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_mark_8(match_data: *mut pcre2_match_data) -> PCRE2_SPTR {
    (*match_data).mark
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_ovector_pointer_8(
    match_data: *mut pcre2_match_data,
) -> *mut PCRE2_SIZE {
    (*match_data).ovector.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_ovector_count_8(match_data: *mut pcre2_match_data) -> u32 {
    (*match_data).oveccount as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_startchar_8(match_data: *mut pcre2_match_data) -> PCRE2_SIZE {
    (*match_data).startchar
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_match_data_size_8(
    match_data: *mut pcre2_match_data,
) -> PCRE2_SIZE {
    match_data_ovector_offset()
        + 2 * ((*match_data).oveccount as usize) * core::mem::size_of::<PCRE2_SIZE>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_match_data_heapframes_size_8(
    match_data: *mut pcre2_match_data,
) -> PCRE2_SIZE {
    (*match_data).heapframes_size
}
