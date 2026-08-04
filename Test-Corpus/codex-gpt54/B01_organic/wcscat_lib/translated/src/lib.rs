use std::ffi::c_int;

#[allow(non_camel_case_types)]
type wchar_t = i32;

const EINVAL: c_int = 22;
const ERANGE: c_int = 34;

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(dst: *mut wchar_t, numElem: usize, src: *const wchar_t) -> c_int {
    let mut ptr = dst;

    if dst.is_null() || numElem == 0 {
        return EINVAL;
    }
    if src.is_null() {
        *dst = 0;
        return EINVAL;
    }

    let end = dst.add(numElem);
    while ptr < end && *ptr != 0 {
        ptr = ptr.add(1);
    }

    let mut src_ptr = src;
    while ptr < end {
        let ch = *src_ptr;
        *ptr = ch;
        ptr = ptr.add(1);
        src_ptr = src_ptr.add(1);
        if ch == 0 {
            return 0;
        }
    }

    *dst = 0;
    ERANGE
}
