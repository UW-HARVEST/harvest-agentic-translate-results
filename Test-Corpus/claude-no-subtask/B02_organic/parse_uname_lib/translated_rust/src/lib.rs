use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

// Mirror of C's `os_data` struct from lib.h
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

/// Compute strlen of a NUL-terminated C string starting at `p`.
unsafe fn c_strlen(p: *const c_char) -> usize {
    let mut len = 0usize;
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    len
}

/// Equivalent to libc's strstr: searches `haystack` (NUL-terminated) for the
/// first occurrence of the substring `needle` (NUL-terminated). Returns a
/// pointer into `haystack` or NULL.
unsafe fn c_strstr(haystack: *mut c_char, needle: &[u8]) -> *mut c_char {
    if needle.is_empty() {
        return haystack;
    }
    let hlen = unsafe { c_strlen(haystack) };
    if hlen < needle.len() {
        return ptr::null_mut();
    }
    let max = hlen - needle.len();
    let mut i = 0usize;
    while i <= max {
        let mut matches = true;
        for j in 0..needle.len() {
            if unsafe { *haystack.add(i + j) } as u8 != needle[j] {
                matches = false;
                break;
            }
        }
        if matches {
            return unsafe { haystack.add(i) };
        }
        i += 1;
    }
    ptr::null_mut()
}

/// Allocates a libc-allocated NUL-terminated copy of the C-string at `s`.
/// Mirrors POSIX strdup semantics.
unsafe fn c_strdup(s: *const c_char) -> *mut c_char {
    let len = unsafe { c_strlen(s) };
    let buf = unsafe { libc::malloc(len + 1) } as *mut c_char;
    if buf.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        ptr::copy_nonoverlapping(s, buf, len);
        *buf.add(len) = 0;
    }
    buf
}

/// Equivalent to a C `malloc` call (returns a libc-allocated buffer of the
/// requested size).
unsafe fn c_malloc(n: usize) -> *mut c_char {
    unsafe { libc::malloc(n) as *mut c_char }
}

/// Looks for the OS architecture in a string. Possible architectures are
/// x86_64, i386, i686, sparc, amd64, i86pc, ia64, AIX, armv6, armv7, aarch64,
/// arm64. Returns a libc-allocated copy of the first match, or NULL.
unsafe fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    const ARCHS: &[&[u8]] = &[
        b"x86_64", b"i386", b"i686", b"sparc", b"amd64", b"i86pc", b"ia64",
        b"AIX", b"armv6", b"armv7", b"aarch64", b"arm64",
    ];
    let mut os_arch: *mut c_char = ptr::null_mut();

    for arch in ARCHS.iter() {
        if !unsafe { c_strstr(os_header, arch) }.is_null() {
            // strdup the literal architecture string
            let len = arch.len();
            let buf = unsafe { libc::malloc(len + 1) } as *mut c_char;
            if !buf.is_null() {
                unsafe {
                    ptr::copy_nonoverlapping(arch.as_ptr() as *const c_char, buf, len);
                    *buf.add(len) = 0;
                }
            }
            os_arch = buf;
            break;
        }
    }

    os_arch
}

/// Wrapper around POSIX regcomp/regexec/regfree. Returns 1 on match, 0
/// otherwise (or on compile failure / NULL input). Mirrors the C function
/// of the same name in the original source.
unsafe fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: libc::size_t,
    pmatch: *mut libc::regmatch_t,
) -> c_int {
    if pattern.is_null() || string.is_null() {
        return 0;
    }

    let mut regex: libc::regex_t = unsafe { std::mem::zeroed() };
    if unsafe { libc::regcomp(&mut regex, pattern, libc::REG_EXTENDED) } != 0 {
        // Replicate the stderr message produced by the C version.
        unsafe {
            let fmt = b"Couldn't compile regular expression '%s'\n\0";
            libc::fprintf(
                libc_stderr(),
                fmt.as_ptr() as *const c_char,
                pattern,
            );
        }
        return 0;
    }

    let result = unsafe { libc::regexec(&regex, string, nmatch, pmatch, 0) };
    unsafe { libc::regfree(&mut regex) };
    if result == 0 {
        1
    } else {
        0
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn libc_stderr() -> *mut libc::FILE {
    unsafe extern "C" {
        // glibc/musl: stderr is an extern variable.
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd", target_os = "dragonfly"))]
fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        static mut __stderrp: *mut libc::FILE;
    }
    unsafe { __stderrp }
}

/// Helper that emulates C's `snprintf(buf, n, "%.*s", match_size, src)`.
/// Copies up to `match_size` bytes from `src` into `buf`, NUL-terminates,
/// and stops if `buf` is too small. `n` is the buffer size (including the
/// trailing NUL), `match_size` is the maximum number of bytes to copy from
/// `src` (regardless of `src`'s actual length, just like the C version).
unsafe fn snprintf_pct_dot_s(buf: *mut c_char, n: usize, match_size: usize, src: *const c_char) {
    if n == 0 {
        return;
    }
    let limit = if match_size < n - 1 { match_size } else { n - 1 };
    for i in 0..limit {
        unsafe { *buf.add(i) = *src.add(i) };
    }
    unsafe { *buf.add(limit) = 0 };
}

/// Parses an OS uname string. Output strings on `osd` are libc-allocated
/// (malloc/strdup) and must be freed by the caller. The input `uname`
/// buffer is modified in place (the C version writes NUL terminators
/// through it).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut OsData) {
    if osd.is_null() {
        return;
    }

    // Reborrow `osd` for safer field access.
    let osd_ref: &mut OsData = unsafe { &mut *osd };

    let mut match_arr: [libc::regmatch_t; 2] = [
        libc::regmatch_t { rm_so: 0, rm_eo: 0 },
        libc::regmatch_t { rm_so: 0, rm_eo: 0 },
    ];

    // [Ver: os_major.os_minor.os_build]
    let str_tmp_ver = unsafe { c_strstr(uname, b" [Ver: ") };
    if !str_tmp_ver.is_null() {
        // Truncate `uname` at the start of " [Ver: " and advance past it.
        unsafe { *str_tmp_ver = 0 };
        let str_tmp = unsafe { str_tmp_ver.add(7) };

        osd_ref.os_name = unsafe { c_strdup(uname) };

        // Strip the trailing ']' from the version segment.
        let st_len = unsafe { c_strlen(str_tmp) };
        if st_len > 0 {
            unsafe { *str_tmp.add(st_len - 1) = 0 };
        }

        // Get os_major
        let pat_major = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
        if unsafe { w_regexec(pat_major, str_tmp, 2, match_arr.as_mut_ptr()) } != 0 {
            let m = match_arr[1];
            let match_size = (m.rm_eo - m.rm_so) as usize;
            osd_ref.os_major = unsafe { c_malloc(match_size + 1) };
            unsafe {
                snprintf_pct_dot_s(
                    osd_ref.os_major,
                    match_size + 1,
                    match_size,
                    str_tmp.add(m.rm_so as usize),
                );
            }
        }

        // Get os_minor
        let pat_minor = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
        if unsafe { w_regexec(pat_minor, str_tmp, 2, match_arr.as_mut_ptr()) } != 0 {
            let m = match_arr[1];
            let match_size = (m.rm_eo - m.rm_so) as usize;
            osd_ref.os_minor = unsafe { c_malloc(match_size + 1) };
            unsafe {
                snprintf_pct_dot_s(
                    osd_ref.os_minor,
                    match_size + 1,
                    match_size,
                    str_tmp.add(m.rm_so as usize),
                );
            }
        }

        // Get os_build
        let pat_build = b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*\0".as_ptr() as *const c_char;
        if unsafe { w_regexec(pat_build, str_tmp, 2, match_arr.as_mut_ptr()) } != 0 {
            let m = match_arr[1];
            let match_size = (m.rm_eo - m.rm_so) as usize;
            osd_ref.os_build = unsafe { c_malloc(match_size + 1) };
            unsafe {
                snprintf_pct_dot_s(
                    osd_ref.os_build,
                    match_size + 1,
                    match_size,
                    str_tmp.add(m.rm_so as usize),
                );
            }
        }

        osd_ref.os_version = unsafe { c_strdup(str_tmp) };
        osd_ref.os_platform = unsafe { c_strdup(b"windows\0".as_ptr() as *const c_char) };
    } else {
        let str_tmp_bracket = unsafe { c_strstr(uname, b" [") };
        if !str_tmp_bracket.is_null() {
            unsafe { *str_tmp_bracket = 0 };
            let str_tmp = unsafe { str_tmp_bracket.add(2) };
            osd_ref.os_name = unsafe { c_strdup(str_tmp) };

            let str_tmp_colon = unsafe { c_strstr(osd_ref.os_name, b": ") };
            if !str_tmp_colon.is_null() {
                unsafe { *str_tmp_colon = 0 };
                let str_tmp2 = unsafe { str_tmp_colon.add(2) };
                osd_ref.os_version = unsafe { c_strdup(str_tmp2) };
                let v_len = unsafe { c_strlen(osd_ref.os_version) };
                if v_len > 0 {
                    unsafe { *osd_ref.os_version.add(v_len - 1) = 0 };
                }

                // os_major.os_minor (os_codename)
                let str_tmp_paren = unsafe { c_strstr(osd_ref.os_version, b" (") };
                if !str_tmp_paren.is_null() {
                    unsafe { *str_tmp_paren = 0 };
                    let str_tmp3 = unsafe { str_tmp_paren.add(2) };
                    osd_ref.os_codename = unsafe { c_strdup(str_tmp3) };
                    let c_len = unsafe { c_strlen(osd_ref.os_codename) };
                    if c_len > 0 {
                        unsafe { *osd_ref.os_codename.add(c_len - 1) = 0 };
                    }
                }

                // Get os_major
                let pat_major = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
                if unsafe {
                    w_regexec(pat_major, osd_ref.os_version, 2, match_arr.as_mut_ptr())
                } != 0
                {
                    let m = match_arr[1];
                    let match_size = (m.rm_eo - m.rm_so) as usize;
                    osd_ref.os_major = unsafe { c_malloc(match_size + 1) };
                    unsafe {
                        snprintf_pct_dot_s(
                            osd_ref.os_major,
                            match_size + 1,
                            match_size,
                            osd_ref.os_version.add(m.rm_so as usize),
                        );
                    }
                }

                // Get os_minor
                let pat_minor = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
                if unsafe {
                    w_regexec(pat_minor, osd_ref.os_version, 2, match_arr.as_mut_ptr())
                } != 0
                {
                    let m = match_arr[1];
                    let match_size = (m.rm_eo - m.rm_so) as usize;
                    osd_ref.os_minor = unsafe { c_malloc(match_size + 1) };
                    unsafe {
                        snprintf_pct_dot_s(
                            osd_ref.os_minor,
                            match_size + 1,
                            match_size,
                            osd_ref.os_version.add(m.rm_so as usize),
                        );
                    }
                }
            } else {
                let n_len = unsafe { c_strlen(osd_ref.os_name) };
                if n_len > 0 {
                    unsafe { *osd_ref.os_name.add(n_len - 1) = 0 };
                }
            }

            // os_name|os_platform
            let str_tmp_pipe = unsafe { c_strstr(osd_ref.os_name, b"|") };
            if !str_tmp_pipe.is_null() {
                unsafe { *str_tmp_pipe = 0 };
                let str_tmp4 = unsafe { str_tmp_pipe.add(1) };
                osd_ref.os_platform = unsafe { c_strdup(str_tmp4) };
            }
        }

        let arch = unsafe { get_os_arch(uname) };
        if !arch.is_null() {
            osd_ref.os_arch = unsafe { c_strdup(arch) };
            unsafe { libc::free(arch as *mut libc::c_void) };
        }
    }
}
