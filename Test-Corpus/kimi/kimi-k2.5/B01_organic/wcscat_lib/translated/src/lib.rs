use std::ffi::{c_int, c_void};
use std::os::raw::c_char;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn wcscat(dst: *mut u32, num_elem: usize, src: *const u32) -> c_int {
    if dst.is_null() || num_elem == 0 {
        return 22;
    }
    if src.is_null() {
        unsafe {
            ptr::write(dst, 0);
        }
        return 22;
    }
    let dst_end = unsafe { dst.add(num_elem) };
    let mut ptr = dst;
    unsafe {
        while ptr < dst_end && ptr::read(ptr) != 0 {
            ptr = ptr.add(1);
        }
        let mut src_ptr = src;
        while ptr < dst_end {
            let ch = ptr::read(src_ptr);
            ptr::write(ptr, ch);
            ptr = ptr.add(1);
            src_ptr = src_ptr.add(1);
            if ch == 0 {
                return 0;
            }
        }
        ptr::write(dst, 0);
    }
    34
}
