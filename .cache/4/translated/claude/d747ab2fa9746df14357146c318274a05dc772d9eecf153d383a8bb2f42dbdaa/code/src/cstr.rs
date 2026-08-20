//! C string primitives (`strlen`, `strcmp`, `strncmp`, `strncpy`, `strncat`)
//! operating on the emulated stack frame.  They are intentionally as lazy as
//! the libc versions: bytes are only touched until the first difference or the
//! terminating NUL, so an out-of-bounds access happens exactly when the C code
//! would perform one.

use crate::mem::Mem;

/// `strlen(&mem[off])`
pub fn strlen_mem(mem: &Mem, off: usize) -> usize {
    let mut i = 0usize;
    while mem.get(off + i) != 0 {
        i += 1;
    }
    i
}

/// `strcmp(&mem[a], &mem[b])`
pub fn strcmp_mem_mem(mem: &Mem, a: usize, b: usize) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = mem.get(a + i);
        let cb = mem.get(b + i);
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

/// `strcmp(&mem[a], lit)` where `lit` holds the characters of a NUL terminated
/// C string (without its terminator, and containing no interior NUL).
pub fn strcmp_mem_bytes(mem: &Mem, a: usize, lit: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = mem.get(a + i);
        let cb = if i < lit.len() { lit[i] } else { 0 };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

/// `strncmp(&mem[a], &mem[b], n)`
pub fn strncmp_mem_mem(mem: &Mem, a: usize, b: usize, n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let ca = mem.get(a + i);
        let cb = mem.get(b + i);
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/// `strncmp(&mem[a], lit, n)`
pub fn strncmp_mem_bytes(mem: &Mem, a: usize, lit: &[u8], n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let ca = mem.get(a + i);
        let cb = if i < lit.len() { lit[i] } else { 0 };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/// `strlen(buf)` for a local C array.
pub fn strlen_local(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(p) => p,
        None => buf.len(),
    }
}

/// The characters of a local C array up to (excluding) its terminating NUL.
pub fn as_cstr_local(buf: &[u8]) -> &[u8] {
    &buf[..strlen_local(buf)]
}

/// `strncpy(dst, &mem[src], n)`: copy up to `n` bytes, zero padding the rest of
/// the first `n` bytes when the source string is shorter.
pub fn strncpy_from_mem(dst: &mut [u8], mem: &Mem, src: usize, n: usize) {
    let mut i = 0usize;
    while i < n {
        let c = mem.get(src + i);
        dst[i] = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    while i < n {
        dst[i] = 0;
        i += 1;
    }
}

/// `strncat(dst, src, n)`: append at most `n` characters of `src` plus a NUL.
pub fn strncat_local(dst: &mut [u8], src: &[u8], n: usize) {
    let start = strlen_local(dst);
    let mut i = 0usize;
    while i < n && i < src.len() && src[i] != 0 {
        dst[start + i] = src[i];
        i += 1;
    }
    dst[start + i] = 0;
}

/// `snprintf(dst, dst.len(), ...)` for the fixed formats used by the C code:
/// writes `parts` concatenated, truncated so that at most `dst.len() - 1`
/// characters plus a terminating NUL are stored.
pub fn snprintf_concat(dst: &mut [u8], parts: &[&[u8]]) {
    let cap = dst.len() - 1;
    let mut written = 0usize;
    for part in parts {
        for &b in part.iter() {
            if written < cap {
                dst[written] = b;
                written += 1;
            }
        }
    }
    dst[written] = 0;
}
