//! Rust translation of the C library in `c_src/`.
//!
//! The library exposes three public symbols (as verified with `nm -D` on the C
//! shared object):
//!
//! ```text
//! T get_os_arch
//! T w_regexec
//! T parse_uname_string
//! ```
//!
//! The translation is byte-for-byte behaviour preserving: it performs the very
//! same in-place mutations on the caller supplied buffers, allocates its result
//! strings with the platform `malloc`/`strdup` (so the caller can `free()`
//! them), and uses the very same POSIX regular expression engine (glibc
//! `regcomp`/`regexec`) as the original code. Buggy behaviour of the original
//! implementation (such as writing a NUL byte one octet *before* the start of
//! an empty string) is faithfully reproduced instead of being fixed.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void, CStr};
use core::mem::MaybeUninit;

// ---------------------------------------------------------------------------
// libc / POSIX declarations
// ---------------------------------------------------------------------------

/// Opaque stand-in for `FILE`.
pub type FILE = c_void;

/// POSIX `regoff_t` (a plain `int` in glibc).
pub type regoff_t = c_int;

/// POSIX `regmatch_t`, layout compatible with `<regex.h>`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct regmatch_t {
    pub rm_so: regoff_t,
    pub rm_eo: regoff_t,
}

/// Opaque, over-sized stand-in for `regex_t`.
///
/// glibc's `regex_t` is 64 bytes with an alignment of 8; the buffer below is
/// deliberately larger so that the storage is always sufficient, regardless of
/// the C library in use. `regcomp()` only ever touches `sizeof(regex_t)` bytes.
#[repr(C, align(16))]
struct regex_t {
    _opaque: [u8; 256],
}

/// `REG_EXTENDED` from `<regex.h>`.
const REG_EXTENDED: c_int = 1;

extern "C" {
    static mut stderr: *mut FILE;

    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;

    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;

    fn regcomp(preg: *mut regex_t, pattern: *const c_char, cflags: c_int) -> c_int;
    fn regexec(
        preg: *const regex_t,
        string: *const c_char,
        nmatch: usize,
        pmatch: *mut regmatch_t,
        eflags: c_int,
    ) -> c_int;
    fn regfree(preg: *mut regex_t);
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Mirrors `os_data` from `include/lib.h`.
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
// Helpers that mirror the C pointer idioms
// ---------------------------------------------------------------------------

/// `*(s + strlen(s) - 1) = '\0';`
///
/// Note that the original C code performs this unconditionally, so for an empty
/// string the NUL byte is written *one byte before* the buffer. That exact
/// (out-of-bounds) behaviour is reproduced here on purpose.
#[inline]
unsafe fn strip_last_char(s: *mut c_char) {
    let len = strlen(s);
    *s.wrapping_offset(len as isize).wrapping_offset(-1) = 0;
}

/// `dst = malloc(n + 1); snprintf(dst, n + 1, "%.*s", n, src);`
#[inline]
unsafe fn dup_match(match_size: c_int, src: *const c_char) -> *mut c_char {
    let buf = malloc((match_size + 1) as usize) as *mut c_char;
    snprintf(
        buf,
        (match_size + 1) as usize,
        c"%.*s".as_ptr(),
        match_size,
        src,
    );
    buf
}

// ---------------------------------------------------------------------------
// get_os_arch
// ---------------------------------------------------------------------------

/// Architectures recognised by [`get_os_arch`], in the exact order of the C
/// `ARCHS` table (the trailing `NULL` sentinel is implicit here).
const ARCHS: [&CStr; 12] = [
    c"x86_64",
    c"i386",
    c"i686",
    c"sparc",
    c"amd64",
    c"i86pc",
    c"ia64",
    c"AIX",
    c"armv6",
    c"armv7",
    c"aarch64",
    c"arm64",
];

/// Looks for the OS architecture in a string.
///
/// Returns a `strdup`'d copy of the first architecture found (searched in table
/// order) or `NULL` when none matches. The caller owns the returned memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    let mut os_arch: *mut c_char = core::ptr::null_mut();

    for arch in ARCHS.iter() {
        if !strstr(os_header, arch.as_ptr()).is_null() {
            os_arch = strdup(arch.as_ptr());
            break;
        }
    }

    os_arch
}

// ---------------------------------------------------------------------------
// w_regexec
// ---------------------------------------------------------------------------

/// Compiles `pattern` as a POSIX extended regular expression and matches it
/// against `string`.
///
/// Returns non-zero (`1`) on a successful match, `0` otherwise (including when
/// either argument is `NULL` or the pattern fails to compile).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut regmatch_t,
) -> c_int {
    let mut regex: MaybeUninit<regex_t> = MaybeUninit::uninit();
    let result: c_int;

    if !(!pattern.is_null() && !string.is_null()) {
        return 0;
    }

    if regcomp(regex.as_mut_ptr(), pattern, REG_EXTENDED) != 0 {
        fprintf(
            stderr,
            c"Couldn't compile regular expression '%s'\n".as_ptr(),
            pattern,
        );
        return 0;
    }

    result = regexec(regex.as_ptr(), string, nmatch, pmatch, 0);
    regfree(regex.as_mut_ptr());

    // C: `return !result;`
    if result == 0 {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// parse_uname_string
// ---------------------------------------------------------------------------

/// Parses an OS uname string, filling `osd`.
///
/// All produced members of `osd` point to heap memory owned by the caller. The
/// `uname` buffer is modified in place, exactly as in the C original.
#[allow(unused_assignments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    let mut str_tmp: *mut c_char;
    // C: `regmatch_t match[2] = {{.rm_so = 0}};` -> fully zero initialised.
    let mut m: [regmatch_t; 2] = [regmatch_t { rm_so: 0, rm_eo: 0 }; 2];
    let mut match_size: c_int = 0;

    if osd.is_null() {
        return;
    }

    // [Ver: os_major.os_minor.os_build]
    str_tmp = strstr(uname, c" [Ver: ".as_ptr());
    if !str_tmp.is_null() {
        *str_tmp = 0;
        str_tmp = str_tmp.wrapping_add(7);
        (*osd).os_name = strdup(uname);
        strip_last_char(str_tmp);

        // Get os_major
        if w_regexec(c"^([0-9]+)\\.*".as_ptr(), str_tmp, 2, m.as_mut_ptr()) != 0 {
            match_size = m[1].rm_eo - m[1].rm_so;
            (*osd).os_major = dup_match(match_size, str_tmp.wrapping_offset(m[1].rm_so as isize));
        }

        // Get os_minor
        if w_regexec(
            c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
            str_tmp,
            2,
            m.as_mut_ptr(),
        ) != 0
        {
            match_size = m[1].rm_eo - m[1].rm_so;
            (*osd).os_minor = dup_match(match_size, str_tmp.wrapping_offset(m[1].rm_so as isize));
        }

        // Get os_build
        if w_regexec(
            c"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*".as_ptr(),
            str_tmp,
            2,
            m.as_mut_ptr(),
        ) != 0
        {
            match_size = m[1].rm_eo - m[1].rm_so;
            (*osd).os_build = dup_match(match_size, str_tmp.wrapping_offset(m[1].rm_so as isize));
        }

        (*osd).os_version = strdup(str_tmp);
        (*osd).os_platform = strdup(c"windows".as_ptr());
    } else {
        str_tmp = strstr(uname, c" [".as_ptr());
        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = str_tmp.wrapping_add(2);
            (*osd).os_name = strdup(str_tmp);

            str_tmp = strstr((*osd).os_name, c": ".as_ptr());
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.wrapping_add(2);
                (*osd).os_version = strdup(str_tmp);
                strip_last_char((*osd).os_version);

                // os_major.os_minor (os_codename)
                str_tmp = strstr((*osd).os_version, c" (".as_ptr());
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = str_tmp.wrapping_add(2);
                    (*osd).os_codename = strdup(str_tmp);
                    strip_last_char((*osd).os_codename);
                }

                // Get os_major
                if w_regexec(
                    c"^([0-9]+)\\.*".as_ptr(),
                    (*osd).os_version,
                    2,
                    m.as_mut_ptr(),
                ) != 0
                {
                    match_size = m[1].rm_eo - m[1].rm_so;
                    (*osd).os_major = dup_match(
                        match_size,
                        (*osd).os_version.wrapping_offset(m[1].rm_so as isize),
                    );
                }

                // Get os_minor
                if w_regexec(
                    c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
                    (*osd).os_version,
                    2,
                    m.as_mut_ptr(),
                ) != 0
                {
                    match_size = m[1].rm_eo - m[1].rm_so;
                    (*osd).os_minor = dup_match(
                        match_size,
                        (*osd).os_version.wrapping_offset(m[1].rm_so as isize),
                    );
                }
            } else {
                strip_last_char((*osd).os_name);
            }

            // os_name|os_platform
            str_tmp = strstr((*osd).os_name, c"|".as_ptr());
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.wrapping_add(1);
                (*osd).os_platform = strdup(str_tmp);
            }
        }

        str_tmp = get_os_arch(uname);
        if !str_tmp.is_null() {
            (*osd).os_arch = strdup(str_tmp);
            free(str_tmp as *mut c_void);
        }
    }
}
