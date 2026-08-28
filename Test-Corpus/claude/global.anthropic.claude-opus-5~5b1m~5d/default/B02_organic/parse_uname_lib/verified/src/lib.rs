//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI reproduced exactly (see `nm -D` on the C `libdriver.so`):
//!   * `get_os_arch`
//!   * `w_regexec`
//!   * `parse_uname_string`
//!
//! The original C code performs in-place mutation of caller-owned strings and
//! returns `malloc`-allocated buffers that the caller must `free`, so the libc
//! routines it used (`malloc`, `strdup`, `strstr`, `strlen`, `snprintf`,
//! `free`, `regcomp`, `regexec`, `regfree`, `fprintf`) are reused verbatim.
//! This guarantees byte-identical output (including POSIX regex semantics and
//! the exact allocator the caller frees with) and reproduces every quirk of the
//! original, bugs included.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc / POSIX regex bindings
// ---------------------------------------------------------------------------

/// `regmatch_t` from `<regex.h>`: two `regoff_t` (== `int` on glibc) fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct regmatch_t {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

/// Opaque, over-sized stand-in for glibc's `regex_t` (64 bytes on x86_64).
/// Extra tail padding is harmless: `regcomp` only touches the real fields.
#[repr(C, align(16))]
struct regex_t {
    _opaque: [u8; 128],
}

impl regex_t {
    #[inline]
    fn new() -> Self {
        regex_t { _opaque: [0u8; 128] }
    }
    #[inline]
    fn as_ptr(&mut self) -> *mut c_void {
        self as *mut regex_t as *mut c_void
    }
}

/// `REG_EXTENDED` from `<regex.h>`.
const REG_EXTENDED: c_int = 1;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;

    fn regcomp(preg: *mut c_void, pattern: *const c_char, cflags: c_int) -> c_int;
    fn regexec(
        preg: *const c_void,
        string: *const c_char,
        nmatch: usize,
        pmatch: *mut regmatch_t,
        eflags: c_int,
    ) -> c_int;
    fn regfree(preg: *mut c_void);

    static mut stderr: *mut c_void;
}

// ---------------------------------------------------------------------------
// Public types (from include/lib.h)
// ---------------------------------------------------------------------------

/// `typedef struct os_data { ... } os_data;` from `include/lib.h`.
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
// Helpers
// ---------------------------------------------------------------------------

/// Reproduces the C expression `*(p + strlen(p) - 1) = '\0';`.
///
/// Note: when `p` points at an empty string the C code writes one byte *before*
/// the buffer. That (buggy) behaviour is preserved bit-for-bit here, using
/// wrapping address arithmetic so the computed address matches C exactly
/// regardless of build profile.
#[inline]
unsafe fn trim_last_char(p: *mut c_char) {
    let len = strlen(p);
    let addr = (p as usize).wrapping_add(len).wrapping_sub(1);
    *(addr as *mut c_char) = 0;
}

/// Reproduces:
/// ```c
/// match_size = match[1].rm_eo - match[1].rm_so;
/// dst = malloc(match_size + 1);
/// snprintf(dst, match_size + 1, "%.*s", match_size, base + match[1].rm_so);
/// ```
#[inline]
unsafe fn dup_match(base: *const c_char, m: &regmatch_t) -> *mut c_char {
    let match_size: c_int = m.rm_eo - m.rm_so;
    let dst = malloc((match_size as isize + 1) as usize) as *mut c_char;
    snprintf(
        dst,
        (match_size as isize + 1) as usize,
        c"%.*s".as_ptr(),
        match_size,
        (base as usize).wrapping_add(m.rm_so as isize as usize) as *const c_char,
    );
    dst
}

// ---------------------------------------------------------------------------
// get_os_arch
// ---------------------------------------------------------------------------

/// Looks for the OS architecture in a string. Possibles architectures are
/// x86_64, i386, i686, sparc, amd64, ia64, AIX, armv6, armv7. The function will
/// return a pointer to allocated memory that must be de-allocated by the
/// caller.
///
/// Returns a string pointer to the architecture, NULL if not found.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    const ARCHS: [&core::ffi::CStr; 12] = [
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut regmatch_t,
) -> c_int {
    let mut regex = regex_t::new();

    if !(!pattern.is_null() && !string.is_null()) {
        return 0;
    }

    if regcomp(regex.as_ptr(), pattern, REG_EXTENDED) != 0 {
        fprintf(
            stderr,
            c"Couldn't compile regular expression '%s'\n".as_ptr(),
            pattern,
        );
        return 0;
    }

    let result = regexec(regex.as_ptr(), string, nmatch, pmatch, 0);
    regfree(regex.as_ptr());

    // C: `return !result;`
    (result == 0) as c_int
}

// ---------------------------------------------------------------------------
// parse_uname_string
// ---------------------------------------------------------------------------

/// Parses an OS uname string. All the OUT parameters are pointers to allocated
/// memory that must be de-allocated by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    let mut str_tmp: *mut c_char;
    let mut matches: [regmatch_t; 2] = [regmatch_t { rm_so: 0, rm_eo: 0 }; 2];

    if osd.is_null() {
        return;
    }

    // [Ver: os_major.os_minor.os_build]
    str_tmp = strstr(uname, c" [Ver: ".as_ptr());
    if !str_tmp.is_null() {
        *str_tmp = 0;
        str_tmp = (str_tmp as usize).wrapping_add(7) as *mut c_char;
        (*osd).os_name = strdup(uname);
        trim_last_char(str_tmp);

        // Get os_major
        if w_regexec(
            c"^([0-9]+)\\.*".as_ptr(),
            str_tmp,
            2,
            matches.as_mut_ptr(),
        ) != 0
        {
            (*osd).os_major = dup_match(str_tmp, &matches[1]);
        }

        // Get os_minor
        if w_regexec(
            c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
            str_tmp,
            2,
            matches.as_mut_ptr(),
        ) != 0
        {
            (*osd).os_minor = dup_match(str_tmp, &matches[1]);
        }

        // Get os_build
        if w_regexec(
            c"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*".as_ptr(),
            str_tmp,
            2,
            matches.as_mut_ptr(),
        ) != 0
        {
            (*osd).os_build = dup_match(str_tmp, &matches[1]);
        }

        (*osd).os_version = strdup(str_tmp);
        (*osd).os_platform = strdup(c"windows".as_ptr());
    } else {
        str_tmp = strstr(uname, c" [".as_ptr());
        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = (str_tmp as usize).wrapping_add(2) as *mut c_char;
            (*osd).os_name = strdup(str_tmp);

            str_tmp = strstr((*osd).os_name, c": ".as_ptr());
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = (str_tmp as usize).wrapping_add(2) as *mut c_char;
                (*osd).os_version = strdup(str_tmp);
                trim_last_char((*osd).os_version);

                // os_major.os_minor (os_codename)
                str_tmp = strstr((*osd).os_version, c" (".as_ptr());
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = (str_tmp as usize).wrapping_add(2) as *mut c_char;
                    (*osd).os_codename = strdup(str_tmp);
                    trim_last_char((*osd).os_codename);
                }

                // Get os_major
                if w_regexec(
                    c"^([0-9]+)\\.*".as_ptr(),
                    (*osd).os_version,
                    2,
                    matches.as_mut_ptr(),
                ) != 0
                {
                    (*osd).os_major = dup_match((*osd).os_version, &matches[1]);
                }

                // Get os_minor
                if w_regexec(
                    c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
                    (*osd).os_version,
                    2,
                    matches.as_mut_ptr(),
                ) != 0
                {
                    (*osd).os_minor = dup_match((*osd).os_version, &matches[1]);
                }
            } else {
                trim_last_char((*osd).os_name);
            }

            // os_name|os_platform
            str_tmp = strstr((*osd).os_name, c"|".as_ptr());
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = (str_tmp as usize).wrapping_add(1) as *mut c_char;
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
