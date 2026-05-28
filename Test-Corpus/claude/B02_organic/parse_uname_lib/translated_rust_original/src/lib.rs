//! Translation of c_src/src/lib.c to Rust.
//!
//! This crate exposes `parse_uname_string` as a C ABI symbol matching the C
//! implementation. All allocations made for OUT pointers use libc::malloc /
//! strdup-equivalents so that the caller can free them with libc::free, exactly
//! like the original C code.

use libc::{c_char, c_int, c_void, size_t};

// We don't use the regex_t / regmatch_t / regcomp / regexec / regfree symbols
// from libc directly because those are not exposed on every platform. We
// declare what we need with extern "C" blocks. The original C uses POSIX
// regex with REG_EXTENDED.
//
// On glibc, REG_EXTENDED == 1.
const REG_EXTENDED: c_int = 1;

// Opaque-enough representation of regex_t. The actual size differs across
// libc implementations; we use a buffer that is large enough for glibc /
// musl / BSD. glibc's regex_t is 64 bytes on 64-bit; we use 256 to be very
// safe across libc implementations.
#[repr(C)]
struct RegexT {
    _opaque: [u8; 256],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct RegmatchT {
    rm_so: libc::regoff_t,
    rm_eo: libc::regoff_t,
}

extern "C" {
    fn regcomp(preg: *mut RegexT, pattern: *const c_char, cflags: c_int) -> c_int;
    fn regexec(
        preg: *const RegexT,
        string: *const c_char,
        nmatch: size_t,
        pmatch: *mut RegmatchT,
        eflags: c_int,
    ) -> c_int;
    fn regfree(preg: *mut RegexT);

    // FILE-handle-bearing stderr stream and fprintf. We mirror the original
    // C code which uses fprintf(stderr, ...).
    static stderr: *mut libc::FILE;
    fn fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
}

/// Mirrors the C `os_data` struct exactly.
#[repr(C)]
pub struct OsData {
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
// Internal helpers — reproduce libc behaviors used by the C source.
// ---------------------------------------------------------------------------

/// Equivalent to `strlen`.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// Equivalent to `strdup` — allocates with libc::malloc so caller can free().
unsafe fn c_strdup(s: *const c_char) -> *mut c_char {
    let len = c_strlen(s);
    let p = libc::malloc(len + 1) as *mut c_char;
    if p.is_null() {
        return p;
    }
    libc::memcpy(p as *mut c_void, s as *const c_void, len + 1);
    p
}

/// Equivalent to `strstr`.
unsafe fn c_strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    let nlen = c_strlen(needle);
    if nlen == 0 {
        return haystack as *mut c_char;
    }
    let hlen = c_strlen(haystack);
    if nlen > hlen {
        return std::ptr::null_mut();
    }
    for i in 0..=(hlen - nlen) {
        if libc::memcmp(
            haystack.add(i) as *const c_void,
            needle as *const c_void,
            nlen,
        ) == 0
        {
            return haystack.add(i) as *mut c_char;
        }
    }
    std::ptr::null_mut()
}

/// Implements the same `snprintf(dst, size+1, "%.*s", size, src)` pattern used
/// in the C source. Copies exactly `size` bytes (or until NUL inside src) and
/// always NUL-terminates within `size + 1` capacity.
unsafe fn copy_field(dst: *mut c_char, src: *const c_char, size: usize) {
    // %.*s prints up to `size` bytes from `src`, stopping at the first NUL.
    // snprintf with buffer of size+1 will write at most `size` chars, then NUL.
    let mut written = 0usize;
    while written < size {
        let b = *src.add(written);
        if b == 0 {
            break;
        }
        *dst.add(written) = b;
        written += 1;
    }
    *dst.add(written) = 0;
}

// ---------------------------------------------------------------------------
// Translated functions.
// ---------------------------------------------------------------------------

/// Looks for the OS architecture in a string. Possible architectures are
/// x86_64, i386, i686, sparc, amd64, ia64, AIX, armv6, armv7. The function
/// returns a pointer to allocated memory that must be freed by the caller.
///
/// Returns NULL if not found.
unsafe fn get_os_arch(os_header: *const c_char) -> *mut c_char {
    // NUL-terminated C strings, in the same order as the C source. Note that
    // the C array contains literal pointers so we mirror by referencing static
    // C strings.
    static ARCHS: &[&[u8]] = &[
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

    let mut os_arch: *mut c_char = std::ptr::null_mut();

    for arch in ARCHS.iter() {
        let arch_ptr = arch.as_ptr() as *const c_char;
        if !c_strstr(os_header, arch_ptr).is_null() {
            os_arch = c_strdup(arch_ptr);
            break;
        }
    }

    os_arch
}

/// Compile and run a POSIX extended regex; returns 1 on match, 0 otherwise.
/// Matches the C implementation including the stderr message on compile error.
unsafe fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: size_t,
    pmatch: *mut RegmatchT,
) -> c_int {
    if pattern.is_null() || string.is_null() {
        return 0;
    }

    let mut regex: RegexT = RegexT { _opaque: [0u8; 256] };

    if regcomp(&mut regex as *mut RegexT, pattern, REG_EXTENDED) != 0 {
        let fmt = b"Couldn't compile regular expression '%s'\n\0".as_ptr() as *const c_char;
        fprintf(stderr, fmt, pattern);
        return 0;
    }

    let result = regexec(&regex as *const RegexT, string, nmatch, pmatch, 0);
    regfree(&mut regex as *mut RegexT);

    if result == 0 {
        1
    } else {
        0
    }
}

/// Parses an OS uname string. All the OUT parameters are pointers to allocated
/// memory that must be de-allocated by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut OsData) {
    let mut str_tmp: *mut c_char;
    let mut match_arr: [RegmatchT; 2] = [RegmatchT { rm_so: 0, rm_eo: 0 }; 2];
    let mut match_size: c_int;

    if osd.is_null() {
        return;
    }

    // [Ver: os_major.os_minor.os_build]
    let ver_marker = b" [Ver: \0".as_ptr() as *const c_char;
    str_tmp = c_strstr(uname, ver_marker);
    if !str_tmp.is_null() {
        *str_tmp = 0;
        str_tmp = str_tmp.add(7);
        (*osd).os_name = c_strdup(uname);
        // *(str_tmp + strlen(str_tmp) - 1) = '\0';
        let len = c_strlen(str_tmp);
        *str_tmp.add(len - 1) = 0;

        // Get os_major
        let pat_major = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
        if w_regexec(pat_major, str_tmp, 2, match_arr.as_mut_ptr()) != 0 {
            match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
            (*osd).os_major = libc::malloc((match_size as usize) + 1) as *mut c_char;
            copy_field(
                (*osd).os_major,
                str_tmp.offset(match_arr[1].rm_so as isize),
                match_size as usize,
            );
        }

        // Get os_minor
        let pat_minor = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
        if w_regexec(pat_minor, str_tmp, 2, match_arr.as_mut_ptr()) != 0 {
            match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
            (*osd).os_minor = libc::malloc((match_size as usize) + 1) as *mut c_char;
            copy_field(
                (*osd).os_minor,
                str_tmp.offset(match_arr[1].rm_so as isize),
                match_size as usize,
            );
        }

        // Get os_build
        let pat_build = b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*\0".as_ptr() as *const c_char;
        if w_regexec(pat_build, str_tmp, 2, match_arr.as_mut_ptr()) != 0 {
            match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
            (*osd).os_build = libc::malloc((match_size as usize) + 1) as *mut c_char;
            copy_field(
                (*osd).os_build,
                str_tmp.offset(match_arr[1].rm_so as isize),
                match_size as usize,
            );
        }

        (*osd).os_version = c_strdup(str_tmp);
        let win_lit = b"windows\0".as_ptr() as *const c_char;
        (*osd).os_platform = c_strdup(win_lit);
    } else {
        let bracket_marker = b" [\0".as_ptr() as *const c_char;
        str_tmp = c_strstr(uname, bracket_marker);
        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = str_tmp.add(2);
            (*osd).os_name = c_strdup(str_tmp);

            let colon_marker = b": \0".as_ptr() as *const c_char;
            str_tmp = c_strstr((*osd).os_name, colon_marker);
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(2);
                (*osd).os_version = c_strdup(str_tmp);
                let vlen = c_strlen((*osd).os_version);
                *(*osd).os_version.add(vlen - 1) = 0;

                // os_major.os_minor (os_codename)
                let paren_marker = b" (\0".as_ptr() as *const c_char;
                str_tmp = c_strstr((*osd).os_version, paren_marker);
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(2);
                    (*osd).os_codename = c_strdup(str_tmp);
                    let clen = c_strlen((*osd).os_codename);
                    *(*osd).os_codename.add(clen - 1) = 0;
                }

                // Get os_major
                let pat_major2 = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
                if w_regexec(pat_major2, (*osd).os_version, 2, match_arr.as_mut_ptr()) != 0 {
                    match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
                    (*osd).os_major = libc::malloc((match_size as usize) + 1) as *mut c_char;
                    copy_field(
                        (*osd).os_major,
                        (*osd).os_version.offset(match_arr[1].rm_so as isize),
                        match_size as usize,
                    );
                }

                // Get os_minor
                let pat_minor2 = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
                if w_regexec(pat_minor2, (*osd).os_version, 2, match_arr.as_mut_ptr()) != 0 {
                    match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
                    (*osd).os_minor = libc::malloc((match_size as usize) + 1) as *mut c_char;
                    copy_field(
                        (*osd).os_minor,
                        (*osd).os_version.offset(match_arr[1].rm_so as isize),
                        match_size as usize,
                    );
                }
            } else {
                let nlen = c_strlen((*osd).os_name);
                *(*osd).os_name.add(nlen - 1) = 0;
            }

            // os_name|os_platform
            let pipe_marker = b"|\0".as_ptr() as *const c_char;
            str_tmp = c_strstr((*osd).os_name, pipe_marker);
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(1);
                (*osd).os_platform = c_strdup(str_tmp);
            }
        }

        str_tmp = get_os_arch(uname);
        if !str_tmp.is_null() {
            (*osd).os_arch = c_strdup(str_tmp);
            libc::free(str_tmp as *mut c_void);
        }
    }
}
