use std::ffi::c_char;

// Re-export linker-generated symbols so they appear in the dynamic symbol
// table of the Rust .so, matching the C build. Using global_asm! we declare
// these symbols as global aliases. The linker still generates the actual
// addresses; we just promote their visibility.
#[cfg(target_os = "linux")]
core::arch::global_asm!(
    ".globl _init",
    ".globl _fini",
    ".globl __bss_start",
    ".globl _edata",
    ".globl _end",
);

/// Decode a base64 character
fn decode(c: c_char) -> u8 {
    let c = c as u8;
    if c >= b'A' && c <= b'Z' {
        return c - b'A';
    }
    if c >= b'a' && c <= b'z' {
        return c - b'a' + 26;
    }
    if c >= b'0' && c <= b'9' {
        return c - b'0' + 52;
    }
    if c == b'+' {
        return 62;
    }

    63
}

/// Returns TRUE if 'c' is a valid base64 character, otherwise FALSE
fn is_base64(c: c_char) -> bool {
    let c = c as u8;
    (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'+'
        || c == b'/'
        || c == b'='
}

/// Decode the base64 encoded string 'src' into the memory pointed to by
/// 'dest'. The dest buffer is NUL terminated.
/// Returns NULL in case of error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if !src.is_null() && *src != 0 {
        // Compute strlen(src) + 1
        let mut src_len: usize = 0;
        while *src.add(src_len) != 0 {
            src_len += 1;
        }
        let mut l: usize = src_len + 1;

        // The size of the dest will always be less than the source.
        // dest = (char *)calloc(sizeof(char), l + 13);
        let dest = libc::calloc(l + 13, std::mem::size_of::<c_char>()) as *mut c_char;
        if dest.is_null() {
            return std::ptr::null_mut();
        }

        let mut p = dest as *mut u8;

        let buf = libc::malloc(l) as *mut u8;
        if buf.is_null() {
            libc::free(dest as *mut libc::c_void);
            return std::ptr::null_mut();
        }

        // Ignore non base64 chars as per the POSIX standard
        let mut k: usize = 0;
        l = 0;
        loop {
            let ch = *src.add(k);
            if ch == 0 {
                break;
            }
            if is_base64(ch) {
                *buf.add(l) = ch as u8;
                l += 1;
            }
            k += 1;
        }

        let mut k: usize = 0;
        while k < l {
            let mut c1: c_char = b'A' as c_char;
            let mut c2: c_char = b'A' as c_char;
            let mut c3: c_char = b'A' as c_char;
            let mut c4: c_char = b'A' as c_char;

            c1 = *buf.add(k) as c_char;

            if k + 1 < l {
                c2 = *buf.add(k + 1) as c_char;
            }

            if k + 2 < l {
                c3 = *buf.add(k + 2) as c_char;
            }

            if k + 3 < l {
                c4 = *buf.add(k + 3) as c_char;
            }

            let b1 = decode(c1);
            let b2 = decode(c2);
            let b3 = decode(c3);
            let b4 = decode(c4);

            *p = (b1 << 2) | (b2 >> 4);
            p = p.add(1);

            if c3 != b'=' as c_char {
                *p = ((b2 & 0xf) << 4) | (b3 >> 2);
                p = p.add(1);
            }

            if c4 != b'=' as c_char {
                *p = ((b3 & 0x3) << 6) | b4;
                p = p.add(1);
            }

            k += 4;
        }

        libc::free(buf as *mut libc::c_void);

        return dest;
    }
    std::ptr::null_mut()
}
