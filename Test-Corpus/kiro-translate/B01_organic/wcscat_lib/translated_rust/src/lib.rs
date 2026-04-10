use std::ffi::c_int;

// wchar_t is i32 on Linux, u16 on Windows
#[cfg(not(target_os = "windows"))]
type WcharT = i32;
#[cfg(target_os = "windows")]
type WcharT = u16;

#[unsafe(no_mangle)]
pub extern "C" fn wcscat(dst: *mut WcharT, num_elem: usize, src: *const WcharT) -> c_int {
    unsafe {
        if dst.is_null() || num_elem == 0 {
            return 22;
        }
        if src.is_null() {
            *dst = 0;
            return 22;
        }

        let mut ptr = dst;
        let end = dst.add(num_elem);

        // Find end of existing string in dst
        while ptr < end && *ptr != 0 {
            ptr = ptr.add(1);
        }

        // Copy src into dst
        let mut s = src;
        while ptr < end {
            *ptr = *s;
            let ch = *s;
            ptr = ptr.add(1);
            s = s.add(1);
            if ch == 0 {
                return 0;
            }
        }

        // Overflow: clear dst and return ERANGE
        *dst = 0;
        34
    }
}
