//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is intentionally byte-for-byte identical to the C original,
//! including its quirks:
//!
//! * `parse_uname_string` mutates the caller's `uname` buffer in place.
//! * All returned strings are `malloc`'d so the caller can `free` them.
//! * `*(p + strlen(p) - 1) = '\0'` is reproduced verbatim, so an empty string
//!   writes one byte *before* the buffer, exactly like the C.
//! * `uname` is never NULL-checked (only `osd` is), matching the C.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Write;

// ---------------------------------------------------------------------------
// libc bindings
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);

    fn regcomp(preg: *mut c_void, pattern: *const c_char, cflags: c_int) -> c_int;
    fn regexec(
        preg: *const c_void,
        string: *const c_char,
        nmatch: usize,
        pmatch: *mut regmatch_t,
        eflags: c_int,
    ) -> c_int;
    fn regfree(preg: *mut c_void);
}

/// POSIX `REG_EXTENDED` on glibc/musl.
const REG_EXTENDED: c_int = 1;

/// POSIX `regmatch_t`. `regoff_t` is `int` on glibc and musl.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct regmatch_t {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

/// Opaque stand-in for `regex_t`.
///
/// The real layout is libc-private, so we reserve a generously sized,
/// over-aligned, zeroed block on the stack. `regcomp` initialises every field
/// it uses, and the value never escapes `w_regexec`.
#[repr(C, align(16))]
struct RegexStorage([u8; 512]);

impl RegexStorage {
    fn new() -> Self {
        RegexStorage([0u8; 512])
    }

    fn as_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
}

// ---------------------------------------------------------------------------
// C string helpers (mirroring <string.h>)
// ---------------------------------------------------------------------------

/// `strlen`
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while unsafe { *s.add(n) } != 0 {
        n += 1;
    }
    n
}

/// `strstr`. Returns the haystack for an empty needle, like C.
unsafe fn c_strstr(haystack: *const c_char, needle: &[u8]) -> *mut c_char {
    if needle.is_empty() {
        return haystack as *mut c_char;
    }
    let mut h = haystack;
    loop {
        let mut i = 0usize;
        loop {
            if i == needle.len() {
                return h as *mut c_char;
            }
            let c = unsafe { *h.add(i) } as u8;
            if c == 0 || c != needle[i] {
                break;
            }
            i += 1;
        }
        if unsafe { *h } == 0 {
            return std::ptr::null_mut();
        }
        h = unsafe { h.add(1) };
    }
}

/// `strdup` of a NUL-terminated C string.
unsafe fn c_strdup(s: *const c_char) -> *mut c_char {
    let len = unsafe { c_strlen(s) };
    let dst = unsafe { malloc(len + 1) } as *mut c_char;
    if dst.is_null() {
        return dst;
    }
    for i in 0..=len {
        unsafe { *dst.add(i) = *s.add(i) };
    }
    dst
}

/// `strdup` of a Rust byte slice that has no interior NULs.
unsafe fn c_strdup_bytes(s: &[u8]) -> *mut c_char {
    let dst = unsafe { malloc(s.len() + 1) } as *mut c_char;
    if dst.is_null() {
        return dst;
    }
    for (i, &b) in s.iter().enumerate() {
        unsafe { *dst.add(i) = b as c_char };
    }
    unsafe { *dst.add(s.len()) = 0 };
    dst
}

/// Equivalent of `snprintf(dst, n + 1, "%.*s", n, src)`: copy at most `n`
/// bytes, stopping early at a NUL, then always NUL-terminate.
unsafe fn copy_precision(dst: *mut c_char, src: *const c_char, n: usize) {
    let mut i = 0usize;
    while i < n {
        let c = unsafe { *src.add(i) };
        if c == 0 {
            break;
        }
        unsafe { *dst.add(i) = c };
        i += 1;
    }
    unsafe { *dst.add(i) = 0 };
}

/// `*(p + strlen(p) - 1) = '\0'` — including the underflow when `p` is empty.
unsafe fn strip_last_char(p: *mut c_char) {
    let len = unsafe { c_strlen(p) } as isize;
    unsafe { *p.offset(len - 1) = 0 };
}

/// Extract capture group 1 of `match` from `base` into a freshly `malloc`'d
/// buffer, exactly as the C does with `malloc` + `snprintf`.
unsafe fn dup_match(base: *const c_char, m: &regmatch_t) -> *mut c_char {
    let match_size = m.rm_eo - m.rm_so;
    let dst = unsafe { malloc(match_size as usize + 1) } as *mut c_char;
    if dst.is_null() {
        return dst;
    }
    unsafe {
        copy_precision(
            dst,
            base.offset(m.rm_so as isize),
            match_size as usize,
        )
    };
    dst
}

// ---------------------------------------------------------------------------
// os_data
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct os_data {
    pub os_name: *mut c_char,
    pub os_version: *mut c_char,
    pub os_major: *mut c_char,
    pub os_minor: *mut c_char,
    pub os_codename: *mut c_char,
    pub os_platform: *mut c_char,
    pub os_build: *mut c_char,
    pub os_uname: *mut c_char,
    pub os_arch: *mut c_char,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Looks for the OS architecture in a string.
///
/// Returns a `malloc`'d string that the caller must `free`, or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    const ARCHS: [&[u8]; 12] = [
        b"x86_64", b"i386", b"i686", b"sparc", b"amd64", b"i86pc", b"ia64", b"AIX", b"armv6",
        b"armv7", b"aarch64", b"arm64",
    ];

    let mut os_arch: *mut c_char = std::ptr::null_mut();

    for arch in ARCHS {
        if !unsafe { c_strstr(os_header, arch) }.is_null() {
            os_arch = unsafe { c_strdup_bytes(arch) };
            break;
        }
    }

    os_arch
}

/// Compiles `pattern` as a POSIX extended regex and matches it against
/// `string`. Returns non-zero on a match.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut regmatch_t,
) -> c_int {
    if !(!pattern.is_null() && !string.is_null()) {
        return 0;
    }

    let mut regex = RegexStorage::new();

    if unsafe { regcomp(regex.as_ptr(), pattern, REG_EXTENDED) } != 0 {
        // fprintf(stderr, "Couldn't compile regular expression '%s'\n", pattern)
        let mut msg: Vec<u8> = Vec::new();
        msg.extend_from_slice(b"Couldn't compile regular expression '");
        let len = unsafe { c_strlen(pattern) };
        for i in 0..len {
            msg.push(unsafe { *pattern.add(i) } as u8);
        }
        msg.extend_from_slice(b"'\n");
        let _ = std::io::stderr().write_all(&msg);
        return 0;
    }

    let result = unsafe { regexec(regex.as_ptr(), string, nmatch, pmatch, 0) };
    unsafe { regfree(regex.as_ptr()) };
    (result == 0) as c_int
}

/// Parses an OS uname string, filling `osd` with `malloc`'d fields.
///
/// Note: `uname` is modified in place.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    let mut str_tmp: *mut c_char;
    let mut m: [regmatch_t; 2] = [regmatch_t { rm_so: 0, rm_eo: 0 }; 2];

    if osd.is_null() {
        return;
    }
    let osd = unsafe { &mut *osd };

    // [Ver: os_major.os_minor.os_build]
    str_tmp = unsafe { c_strstr(uname, b" [Ver: ") };
    if !str_tmp.is_null() {
        unsafe { *str_tmp = 0 };
        str_tmp = unsafe { str_tmp.add(7) };
        osd.os_name = unsafe { c_strdup(uname) };
        unsafe { strip_last_char(str_tmp) };

        // Get os_major
        if unsafe { w_regexec(c"^([0-9]+)\\.*".as_ptr(), str_tmp, 2, m.as_mut_ptr()) } != 0 {
            osd.os_major = unsafe { dup_match(str_tmp, &m[1]) };
        }

        // Get os_minor
        if unsafe { w_regexec(c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(), str_tmp, 2, m.as_mut_ptr()) } != 0
        {
            osd.os_minor = unsafe { dup_match(str_tmp, &m[1]) };
        }

        // Get os_build
        if unsafe {
            w_regexec(
                c"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*".as_ptr(),
                str_tmp,
                2,
                m.as_mut_ptr(),
            )
        } != 0
        {
            osd.os_build = unsafe { dup_match(str_tmp, &m[1]) };
        }

        osd.os_version = unsafe { c_strdup(str_tmp) };
        osd.os_platform = unsafe { c_strdup_bytes(b"windows") };
    } else {
        str_tmp = unsafe { c_strstr(uname, b" [") };
        if !str_tmp.is_null() {
            unsafe { *str_tmp = 0 };
            str_tmp = unsafe { str_tmp.add(2) };
            osd.os_name = unsafe { c_strdup(str_tmp) };

            str_tmp = unsafe { c_strstr(osd.os_name, b": ") };
            if !str_tmp.is_null() {
                unsafe { *str_tmp = 0 };
                str_tmp = unsafe { str_tmp.add(2) };
                osd.os_version = unsafe { c_strdup(str_tmp) };
                unsafe { strip_last_char(osd.os_version) };

                // os_major.os_minor (os_codename)
                str_tmp = unsafe { c_strstr(osd.os_version, b" (") };
                if !str_tmp.is_null() {
                    unsafe { *str_tmp = 0 };
                    str_tmp = unsafe { str_tmp.add(2) };
                    osd.os_codename = unsafe { c_strdup(str_tmp) };
                    unsafe { strip_last_char(osd.os_codename) };
                }

                // Get os_major
                if unsafe {
                    w_regexec(c"^([0-9]+)\\.*".as_ptr(), osd.os_version, 2, m.as_mut_ptr())
                } != 0
                {
                    osd.os_major = unsafe { dup_match(osd.os_version, &m[1]) };
                }

                // Get os_minor
                if unsafe {
                    w_regexec(
                        c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
                        osd.os_version,
                        2,
                        m.as_mut_ptr(),
                    )
                } != 0
                {
                    osd.os_minor = unsafe { dup_match(osd.os_version, &m[1]) };
                }
            } else {
                unsafe { strip_last_char(osd.os_name) };
            }

            // os_name|os_platform
            str_tmp = unsafe { c_strstr(osd.os_name, b"|") };
            if !str_tmp.is_null() {
                unsafe { *str_tmp = 0 };
                str_tmp = unsafe { str_tmp.add(1) };
                osd.os_platform = unsafe { c_strdup(str_tmp) };
            }
        }

        str_tmp = unsafe { get_os_arch(uname) };
        if !str_tmp.is_null() {
            osd.os_arch = unsafe { c_strdup(str_tmp) };
            unsafe { free(str_tmp as *mut c_void) };
        }
    }
}
