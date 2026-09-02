//! x86-64 System V `va_list` handling.
//!
//! On this ABI `va_list` is `__va_list_tag *`, so a C caller that passes a
//! `va_list` actually passes a pointer to the structure below.

use core::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VaListTag {
    pub gp_offset: u32,
    pub fp_offset: u32,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}

/// The type a C `va_list` parameter degenerates to.
pub type VaList = *mut VaListTag;

#[inline]
unsafe fn arg_gp(ap: VaList) -> *const u8 {
    unsafe {
        let a = &mut *ap;
        if a.gp_offset < 48 {
            let p = (a.reg_save_area as *const u8).add(a.gp_offset as usize);
            a.gp_offset += 8;
            p
        } else {
            let p = a.overflow_arg_area as *const u8;
            a.overflow_arg_area = (a.overflow_arg_area as *mut u8).add(8) as *mut c_void;
            p
        }
    }
}

#[inline]
unsafe fn arg_fp(ap: VaList) -> *const u8 {
    unsafe {
        let a = &mut *ap;
        if a.fp_offset < 176 {
            let p = (a.reg_save_area as *const u8).add(a.fp_offset as usize);
            a.fp_offset += 16;
            p
        } else {
            let p = a.overflow_arg_area as *const u8;
            a.overflow_arg_area = (a.overflow_arg_area as *mut u8).add(8) as *mut c_void;
            p
        }
    }
}

/// `va_arg(ap, int)`
#[inline]
pub unsafe fn va_int(ap: VaList) -> c_int {
    unsafe { (arg_gp(ap) as *const c_int).read_unaligned() }
}

/// `va_arg(ap, size_t)`
#[inline]
pub unsafe fn va_size(ap: VaList) -> usize {
    unsafe { (arg_gp(ap) as *const usize).read_unaligned() }
}

/// `va_arg(ap, json_int_t)` (long long)
#[inline]
pub unsafe fn va_longlong(ap: VaList) -> i64 {
    unsafe { (arg_gp(ap) as *const i64).read_unaligned() }
}

/// `va_arg(ap, T*)`
#[inline]
pub unsafe fn va_ptr<T>(ap: VaList) -> *mut T {
    unsafe { (arg_gp(ap) as *const *mut T).read_unaligned() }
}

/// `va_arg(ap, const char *)`
#[inline]
pub unsafe fn va_str(ap: VaList) -> *const c_char {
    unsafe { (arg_gp(ap) as *const *const c_char).read_unaligned() }
}

/// `va_arg(ap, double)`
#[inline]
pub unsafe fn va_double(ap: VaList) -> f64 {
    unsafe { (arg_fp(ap) as *const f64).read_unaligned() }
}

/// `va_copy(dst, src)`
#[inline]
pub unsafe fn va_copy(dst: *mut VaListTag, src: VaList) {
    unsafe { *dst = *src };
}
