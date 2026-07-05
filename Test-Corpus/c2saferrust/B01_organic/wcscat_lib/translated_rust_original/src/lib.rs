
pub type size_t = usize;
pub type wchar_t = ::libc::wchar_t;
#[no_mangle]
pub fn wcscat(dst: &mut [wchar_t], numElem: size_t, src: &[wchar_t]) -> ::core::ffi::c_int {
    if numElem == 0 {
        return 22;
    }

    let limit = core::cmp::min(dst.len(), numElem);

    if src.is_empty() {
        if !dst.is_empty() {
            dst[0] = 0 as wchar_t;
        }
        return 22;
    }

    let dst_prefix = &mut dst[..limit];

    let start = match dst_prefix.iter().position(|&ch| ch == 0 as wchar_t) {
        Some(pos) => pos,
        None => {
            if !dst.is_empty() {
                dst[0] = 0 as wchar_t;
            }
            return 34;
        }
    };

    let mut write = start;
    for &ch in src {
        if write >= limit {
            if !dst.is_empty() {
                dst[0] = 0 as wchar_t;
            }
            return 34;
        }
        dst_prefix[write] = ch;
        write += 1;
        if ch == 0 as wchar_t {
            return 0;
        }
    }

    if !dst.is_empty() {
        dst[0] = 0 as wchar_t;
    }
    34
}

