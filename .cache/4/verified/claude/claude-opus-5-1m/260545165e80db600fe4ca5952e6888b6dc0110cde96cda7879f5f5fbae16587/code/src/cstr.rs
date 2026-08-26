//! C string primitives (`strlen`, `strcmp`, `strncmp`, `strncpy`, `strncat`)
//! operating on an abstract byte addressable memory.  They are intentionally as
//! lazy as the libc versions: bytes are only touched until the first difference
//! or the terminating NUL, so an out-of-bounds access happens exactly when the C
//! code would perform one.

use crate::mem::MemAccess;

/// `strlen(&mem[off])`
pub fn strlen_mem<M: MemAccess + ?Sized>(mem: &M, off: usize) -> usize {
    let mut i = 0usize;
    while mem.get(off.wrapping_add(i)) != 0 {
        i += 1;
    }
    i
}

/// `strcmp(&mem[a], &mem[b])`
pub fn strcmp_mem_mem<M: MemAccess + ?Sized>(mem: &M, a: usize, b: usize) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = mem.get(a.wrapping_add(i));
        let cb = mem.get(b.wrapping_add(i));
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
pub fn strcmp_mem_bytes<M: MemAccess + ?Sized>(mem: &M, a: usize, lit: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = mem.get(a.wrapping_add(i));
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
pub fn strncmp_mem_mem<M: MemAccess + ?Sized>(mem: &M, a: usize, b: usize, n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let ca = mem.get(a.wrapping_add(i));
        let cb = mem.get(b.wrapping_add(i));
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
pub fn strncmp_mem_bytes<M: MemAccess + ?Sized>(mem: &M, a: usize, lit: &[u8], n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let ca = mem.get(a.wrapping_add(i));
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
pub fn strncpy_from_mem<M: MemAccess + ?Sized>(dst: &mut [u8], mem: &M, src: usize, n: usize) {
    let mut i = 0usize;
    while i < n {
        let c = mem.get(src.wrapping_add(i));
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

/// `snprintf(dst, dst.len(), fmt, &mem[src])` for the three fixed formats used
/// by the C code (`"*%s*"`, `"%s*"` and `"*%s"`).
///
/// Like `snprintf`, at most `dst.len() - 1` characters are stored followed by a
/// NUL, and - because the conversion has to compute the length it *would* have
/// produced - the whole source string is read even when the output is
/// truncated.
pub fn snprintf_star<M: MemAccess + ?Sized>(
    dst: &mut [u8],
    mem: &M,
    src: usize,
    lead_star: bool,
    trail_star: bool,
) {
    let cap = dst.len() - 1;
    let mut written = 0usize;
    if lead_star {
        if written < cap {
            dst[written] = b'*';
            written += 1;
        }
    }
    let mut i = 0usize;
    loop {
        let c = mem.get(src.wrapping_add(i));
        if c == 0 {
            break;
        }
        if written < cap {
            dst[written] = c;
            written += 1;
        }
        i += 1;
    }
    if trail_star {
        if written < cap {
            dst[written] = b'*';
            written += 1;
        }
    }
    dst[written] = 0;
}
