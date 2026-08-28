use core::ffi::c_char;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn a_place(p: *mut c_char) { unsafe { *p = 7 }; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn b_ptr_write(p: *mut c_char) { unsafe { core::ptr::write(p, 7) }; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn c_volatile(p: *mut c_char) { unsafe { core::ptr::write_volatile(p, 7) }; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn d_copy(p: *mut c_char) { let v: c_char = 7; unsafe { core::ptr::copy_nonoverlapping(&v, p, 1) }; }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn e_read_place(p: *const u8) -> u8 { unsafe { *p } }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn f_read_ptr(p: *const u8) -> u8 { unsafe { core::ptr::read(p) } }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn g_read_vol(p: *const u8) -> u8 { unsafe { core::ptr::read_volatile(p) } }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn h_read_copy(p: *const u8) -> u8 { let mut v = 0u8; unsafe { core::ptr::copy_nonoverlapping(p, &mut v, 1) }; v }
