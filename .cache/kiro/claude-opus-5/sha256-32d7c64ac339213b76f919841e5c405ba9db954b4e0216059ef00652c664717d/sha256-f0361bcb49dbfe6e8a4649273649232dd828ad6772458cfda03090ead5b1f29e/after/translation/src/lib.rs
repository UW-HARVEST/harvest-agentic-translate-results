//! Rust translation of `c_src/src/lib.c` (OS uname string parser).
//!
//! The translation is intentionally literal: it keeps the original evaluation
//! order, the in-place mutation of the caller's buffer, the C allocator
//! (`malloc`/`strdup`, so callers can `free()` the results) and the original
//! out-of-bounds / undefined-behaviour quirks of the C code.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::MaybeUninit;

// ---------------------------------------------------------------------------
// Public C types (from include/lib.h)
// ---------------------------------------------------------------------------

/// Mirror of `typedef struct os_data { ... } os_data;`
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
// POSIX <regex.h> / libc bindings
// ---------------------------------------------------------------------------

/// `regoff_t` is `int` in glibc's default (non large-offset) configuration.
pub type regoff_t = c_int;

/// Mirror of `regmatch_t` (two `regoff_t`, 8 bytes total on this platform).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct regmatch_t {
    pub rm_so: regoff_t,
    pub rm_eo: regoff_t,
}

/// Opaque stand-in for glibc's `regex_t` (64 bytes, 8 byte alignment).
#[repr(C)]
struct regex_t {
    _opaque: [u64; 8],
}

/// `REG_EXTENDED` on glibc / Linux.
const REG_EXTENDED: c_int = 1;

extern "C" {
    fn regcomp(preg: *mut regex_t, pattern: *const c_char, cflags: c_int) -> c_int;
    fn regexec(
        preg: *const regex_t,
        string: *const c_char,
        nmatch: usize,
        pmatch: *mut regmatch_t,
        eflags: c_int,
    ) -> c_int;
    fn regfree(preg: *mut regex_t);

    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;

    /// glibc's `extern FILE *stderr;`
    static mut stderr: *mut c_void;
}

// ---------------------------------------------------------------------------
// get_os_arch
// ---------------------------------------------------------------------------

/// Architectures probed, in the exact order of the C `ARCHS` array.
static ARCHS: [&[u8]; 12] = [
    b"x86_64\0",
    b"i386\0",
    b"i686\0",
    b"sparc\0",
    b"amd64\0",
    b"i86pc\0",
    b"ia64\0",
    b"AIX\0",
    b"armv6\0",
    b"armv7\0",
    b"aarch64\0",
    b"arm64\0",
];

/// Looks for the OS architecture in a string.
///
/// Returns a `malloc`'d copy of the matched architecture name, or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    let mut os_arch: *mut c_char = std::ptr::null_mut();

    for arch in ARCHS.iter() {
        let needle = arch.as_ptr() as *const c_char;
        if !strstr(os_header, needle).is_null() {
            os_arch = strdup(needle);
            break;
        }
    }

    os_arch
}

// ---------------------------------------------------------------------------
// w_regexec
// ---------------------------------------------------------------------------

/// Compiles `pattern` as a POSIX extended regex and matches it against
/// `string`. Returns non-zero on match, 0 otherwise (including on error).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut regmatch_t,
) -> c_int {
    let mut regex: MaybeUninit<regex_t> = MaybeUninit::uninit();

    if !(!pattern.is_null() && !string.is_null()) {
        return 0;
    }

    if regcomp(regex.as_mut_ptr(), pattern, REG_EXTENDED) != 0 {
        // NOTE: matches the C code, which does not call regfree() here.
        fprintf(
            stderr,
            b"Couldn't compile regular expression '%s'\n\0".as_ptr() as *const c_char,
            pattern,
        );
        return 0;
    }

    let result = regexec(regex.as_ptr(), string, nmatch, pmatch, 0);
    regfree(regex.as_mut_ptr());

    (result == 0) as c_int
}

// ---------------------------------------------------------------------------
// parse_uname_string
// ---------------------------------------------------------------------------

/// Equivalent of the repeated C block:
///
/// ```c
/// if (w_regexec(pattern, s, 2, match)) {
///     match_size = match[1].rm_eo - match[1].rm_so;
///     dst = malloc(match_size + 1);
///     snprintf(dst, match_size + 1, "%.*s", match_size, s + match[1].rm_so);
/// }
/// ```
///
/// Returns `Some(ptr)` when the regex matched, `None` otherwise, leaving the
/// destination field untouched in the latter case (as the C code does).
unsafe fn capture_group1(
    pattern: &[u8],
    s: *const c_char,
    m: *mut regmatch_t,
) -> Option<*mut c_char> {
    if w_regexec(pattern.as_ptr() as *const c_char, s, 2, m) != 0 {
        let group = *m.add(1);
        let match_size: c_int = group.rm_eo - group.rm_so;
        // `match_size + 1` is an `int` promoted to `size_t` in C: sign-extend.
        let size = (match_size + 1) as usize;
        let dst = malloc(size) as *mut c_char;
        snprintf(
            dst,
            size,
            b"%.*s\0".as_ptr() as *const c_char,
            match_size,
            s.offset(group.rm_so as isize),
        );
        Some(dst)
    } else {
        None
    }
}

/// Truncates the C string at `p` by one character:
/// `*(p + strlen(p) - 1) = '\0';`
///
/// Reproduces the C behaviour verbatim, including the out-of-bounds write when
/// the string is empty.
unsafe fn strip_last_char(p: *mut c_char) {
    let len = strlen(p);
    *p.offset(len as isize - 1) = 0;
}

/// Parses an OS uname string, filling `osd`.
///
/// All the OUT parameters are `malloc`'d and must be freed by the caller.
/// The input `uname` buffer is modified in place.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    let mut str_tmp: *mut c_char;
    // `regmatch_t match[2] = {{.rm_so = 0}};` -> fully zero initialised.
    let mut match_: [regmatch_t; 2] = [regmatch_t { rm_so: 0, rm_eo: 0 }; 2];
    let m = match_.as_mut_ptr();

    if osd.is_null() {
        return;
    }

    // [Ver: os_major.os_minor.os_build]
    str_tmp = strstr(uname, b" [Ver: \0".as_ptr() as *const c_char);
    if !str_tmp.is_null() {
        *str_tmp = 0;
        str_tmp = str_tmp.add(7);
        (*osd).os_name = strdup(uname);
        strip_last_char(str_tmp);

        // Get os_major
        if let Some(p) = capture_group1(b"^([0-9]+)\\.*\0", str_tmp, m) {
            (*osd).os_major = p;
        }

        // Get os_minor
        if let Some(p) = capture_group1(b"^[0-9]+\\.([0-9]+)\\.*\0", str_tmp, m) {
            (*osd).os_minor = p;
        }

        // Get os_build
        if let Some(p) = capture_group1(b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*\0", str_tmp, m)
        {
            (*osd).os_build = p;
        }

        (*osd).os_version = strdup(str_tmp);
        (*osd).os_platform = strdup(b"windows\0".as_ptr() as *const c_char);
    } else {
        str_tmp = strstr(uname, b" [\0".as_ptr() as *const c_char);
        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = str_tmp.add(2);
            (*osd).os_name = strdup(str_tmp);

            str_tmp = strstr((*osd).os_name, b": \0".as_ptr() as *const c_char);
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(2);
                (*osd).os_version = strdup(str_tmp);
                strip_last_char((*osd).os_version);

                // os_major.os_minor (os_codename)
                str_tmp = strstr((*osd).os_version, b" (\0".as_ptr() as *const c_char);
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(2);
                    (*osd).os_codename = strdup(str_tmp);
                    strip_last_char((*osd).os_codename);
                }

                // Get os_major
                if let Some(p) = capture_group1(b"^([0-9]+)\\.*\0", (*osd).os_version, m) {
                    (*osd).os_major = p;
                }

                // Get os_minor
                if let Some(p) = capture_group1(b"^[0-9]+\\.([0-9]+)\\.*\0", (*osd).os_version, m) {
                    (*osd).os_minor = p;
                }
            } else {
                strip_last_char((*osd).os_name);
            }

            // os_name|os_platform
            str_tmp = strstr((*osd).os_name, b"|\0".as_ptr() as *const c_char);
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(1);
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
