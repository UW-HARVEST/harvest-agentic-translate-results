use std::ffi::c_int;
use std::ptr;

pub type WChar = i32;
pub type SizeT = usize;

#[unsafe(no_mangle)]
pub extern "C" fn wcscat(dst: *mut WChar, numElem: SizeT, src: *const WChar) -> c_int {
    if dst.is_null() || numElem == 0 {
        return 22;
    }

    unsafe {
        if src.is_null() {
            *dst = 0;
            return 22;
        }

        let mut ptr_cur = dst;
        let end = dst.add(numElem);

        while ptr_cur < end && *ptr_cur != 0 {
            ptr_cur = ptr_cur.add(1);
        }

        let mut src_cur = src;
        while ptr_cur < end {
            let ch = *src_cur;
            ptr::write(ptr_cur, ch);
            ptr_cur = ptr_cur.add(1);
            src_cur = src_cur.add(1);
            if ch == 0 {
                return 0;
            }
        }

        *dst = 0;
        34
    }
}
