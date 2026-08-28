//! Emulation of the C string primitives (`strlen`, `strcmp`, `strncmp`,
//! `strncpy`, `strncat`, `snprintf`) used by `c_src/src/lib.c`.
//!
//! Every "C string" is modelled as a byte slice that starts at the first
//! character of the string and extends to the end of the modelled object.  The
//! NUL terminator is searched for inside that slice, exactly like the C code
//! does.  If a routine would have to read past the end of the modelled memory
//! the original program reads unmapped memory and dies from SIGSEGV, so the
//! translation reproduces that crash (see [`segfault`]).

/// Reproduce the crash of the original C program.
///
/// `match_pattern()` in the C source computes `text_len - pattern_len` with
/// `size_t` arithmetic.  When the pattern is longer than the text this
/// underflows to a huge value and the following `strncmp()` loop walks off the
/// end of the stack buffer until it hits an unmapped page, which kills the
/// process with SIGSEGV before anything is written to stdout.
#[allow(clippy::manual_dangling_ptr)]
pub fn segfault() -> ! {
    // A volatile read of an invalid address is not optimised away, so this
    // terminates the process with SIGSEGV just like the C program does.
    unsafe {
        let p = 1usize as *const u8;
        std::ptr::read_volatile(p);
    }
    // Should never be reached; abort as a fallback so that nothing is printed.
    std::process::abort();
}

/// Read one byte of the modelled memory, crashing on an out-of-bounds access.
#[inline]
fn byte_at(mem: &[u8], idx: usize) -> u8 {
    match mem.get(idx) {
        Some(&b) => b,
        None => segfault(),
    }
}

/// Sub-slice starting at `off`, i.e. the C expression `&mem[off]`.
#[inline]
pub fn slice_from(mem: &[u8], off: usize) -> &[u8] {
    if off > mem.len() {
        segfault()
    } else {
        &mem[off..]
    }
}

/// C `strlen()`.
pub fn strlen(s: &[u8]) -> usize {
    let mut i = 0usize;
    loop {
        if byte_at(s, i) == 0 {
            return i;
        }
        i += 1;
    }
}

/// C `strcmp()` (bytes are compared as `unsigned char`).
pub fn strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let x = byte_at(a, i);
        let y = byte_at(b, i);
        if x != y {
            return x as i32 - y as i32;
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
}

/// C `strncmp()`.
pub fn strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let x = byte_at(a, i);
        let y = byte_at(b, i);
        if x != y {
            return x as i32 - y as i32;
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/// C `strncpy(dst, src, n)`: copies at most `n` bytes and pads the remainder of
/// the `n` bytes with NULs.  No terminator is added when `src` is longer.
pub fn strncpy(dst: &mut [u8], src: &[u8], n: usize) {
    let mut i = 0usize;
    while i < n {
        let c = byte_at(src, i);
        dst[i] = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    // Pad the rest of the n bytes with NUL bytes.
    while i < n {
        dst[i] = 0;
        i += 1;
    }
}

/// C `strncat(dst, src, n)`: appends at most `n` bytes of `src` plus a NUL.
pub fn strncat(dst: &mut [u8], src: &[u8], n: usize) {
    let dlen = strlen(dst);
    let mut k = 0usize;
    while k < n {
        let c = byte_at(src, k);
        if c == 0 {
            break;
        }
        dst[dlen + k] = c;
        k += 1;
    }
    dst[dlen + k] = 0;
}

/// C `snprintf(dst, size, ...)` restricted to the plain concatenation of
/// literal text and `%s` conversions used by the original code.  At most
/// `size - 1` bytes are written followed by a NUL terminator.
pub fn snprintf_concat(dst: &mut [u8], size: usize, parts: &[&[u8]]) {
    let mut full: Vec<u8> = Vec::new();
    for part in parts {
        // `%s` arguments stop at their NUL terminator.
        let len = strlen(part);
        full.extend_from_slice(&part[..len]);
    }
    if size == 0 {
        return;
    }
    let n = if full.len() > size - 1 {
        size - 1
    } else {
        full.len()
    };
    dst[..n].copy_from_slice(&full[..n]);
    dst[n] = 0;
}
