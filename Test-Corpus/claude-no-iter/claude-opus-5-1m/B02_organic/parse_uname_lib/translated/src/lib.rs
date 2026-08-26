#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_void};
use std::ptr;

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

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

/// Architectures recognized by `get_os_arch`. Order must match the C source.
const ARCHS: &[&[u8]] = &[
    b"x86_64", b"i386", b"i686", b"sparc", b"amd64", b"i86pc", b"ia64", b"AIX", b"armv6", b"armv7",
    b"aarch64", b"arm64",
];

/// Allocates a NUL-terminated copy of `bytes` using libc `malloc` so the
/// returned pointer is compatible with `free()` from the C caller.
unsafe fn alloc_cstr_from_bytes(bytes: &[u8]) -> *mut c_char {
    let len = bytes.len();
    let p = malloc(len + 1) as *mut u8;
    if p.is_null() {
        return ptr::null_mut();
    }
    if len > 0 {
        ptr::copy_nonoverlapping(bytes.as_ptr(), p, len);
    }
    *p.add(len) = 0;
    p as *mut c_char
}

/// strdup of a NUL-terminated C string using libc malloc.
unsafe fn cstr_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    let len = strlen(s);
    let p = malloc(len + 1) as *mut u8;
    if p.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(s as *const u8, p, len + 1);
    p as *mut c_char
}

/// Equivalent of C `strstr` returning a pointer into `haystack`.
unsafe fn cstr_strstr(haystack: *mut c_char, needle: &[u8]) -> *mut c_char {
    if haystack.is_null() {
        return ptr::null_mut();
    }
    let h_len = strlen(haystack);
    if needle.is_empty() {
        return haystack;
    }
    if needle.len() > h_len {
        return ptr::null_mut();
    }
    let h_slice = std::slice::from_raw_parts(haystack as *const u8, h_len);
    for i in 0..=(h_len - needle.len()) {
        if &h_slice[i..i + needle.len()] == needle {
            return (haystack as *mut u8).add(i) as *mut c_char;
        }
    }
    ptr::null_mut()
}

/// View a NUL-terminated C string as a byte slice (without the NUL).
unsafe fn cstr_as_bytes<'a>(p: *const c_char) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    let len = strlen(p);
    std::slice::from_raw_parts(p as *const u8, len)
}

/// Look for any architecture token from `ARCHS` as a substring of `os_header`.
/// Returns a freshly malloc'd copy of the first match (caller must free).
unsafe fn get_os_arch(os_header: *const c_char) -> *mut c_char {
    if os_header.is_null() {
        return ptr::null_mut();
    }
    let header = cstr_as_bytes(os_header);
    for arch in ARCHS {
        if find_subslice(header, arch).is_some() {
            return alloc_cstr_from_bytes(arch);
        }
    }
    ptr::null_mut()
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

// ---- Hand-rolled regex evaluators for the three POSIX ERE patterns used. ----
// All three are anchored with `^` and trail with `\.*` (zero-or-more literal
// dots) which never affects the captured substring; we reproduce only what
// is needed to extract capture group 1.

fn match_leading_digits(s: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == from {
        None
    } else {
        Some(i)
    }
}

/// `^([0-9]+)\.*` — capture group 1 is the leading digit run.
fn match_pattern1(s: &[u8]) -> Option<(usize, usize)> {
    let end = match_leading_digits(s, 0)?;
    Some((0, end))
}

/// `^[0-9]+\.([0-9]+)\.*` — capture is the digit run after the first dot.
fn match_pattern2(s: &[u8]) -> Option<(usize, usize)> {
    let first_end = match_leading_digits(s, 0)?;
    if s.get(first_end) != Some(&b'.') {
        return None;
    }
    let group_start = first_end + 1;
    let group_end = match_leading_digits(s, group_start)?;
    Some((group_start, group_end))
}

/// `^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*`
/// Capture group 1 is `[0-9]+(\.[0-9]+)*` after the second dot.
fn match_pattern3(s: &[u8]) -> Option<(usize, usize)> {
    let first_end = match_leading_digits(s, 0)?;
    if s.get(first_end) != Some(&b'.') {
        return None;
    }
    let second_start = first_end + 1;
    let second_end = match_leading_digits(s, second_start)?;
    if s.get(second_end) != Some(&b'.') {
        return None;
    }
    let group_start = second_end + 1;
    let mut i = match_leading_digits(s, group_start)?;
    // Greedy `(\.[0-9]+)*`
    loop {
        if i + 1 < s.len() && s[i] == b'.' && s[i + 1].is_ascii_digit() {
            i += 1;
            while i < s.len() && s[i].is_ascii_digit() {
                i += 1;
            }
        } else {
            break;
        }
    }
    Some((group_start, i))
}

/// Allocate a NUL-terminated copy of `s[range]` matching the C idiom of
/// `malloc(n+1); snprintf(p, n+1, "%.*s", n, src)`.
unsafe fn dup_substring(s: &[u8], start: usize, end: usize) -> *mut c_char {
    alloc_cstr_from_bytes(&s[start..end])
}

/// Truncate the C string at `p` by writing a NUL byte at the position of its
/// last character. Mirrors the (potentially undefined-on-empty) C idiom
/// `*(p + strlen(p) - 1) = '\0'`. We preserve the wrapping arithmetic.
unsafe fn drop_last_char(p: *mut c_char) {
    let len = strlen(p);
    let target = (p as *mut u8).wrapping_add(len.wrapping_sub(1));
    *target = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() {
        return;
    }

    // [Ver: os_major.os_minor.os_build]
    let ver_marker: &[u8] = b" [Ver: ";
    let str_tmp = cstr_strstr(uname, ver_marker);
    if !str_tmp.is_null() {
        // Terminate uname at the start of " [Ver: "
        *str_tmp = 0;
        // Advance past " [Ver: " (7 bytes)
        let after = (str_tmp as *mut u8).add(7) as *mut c_char;

        (*osd).os_name = cstr_strdup(uname);

        // Strip trailing ']' from `after`
        drop_last_char(after);

        let after_bytes = cstr_as_bytes(after);

        // os_major
        if let Some((s, e)) = match_pattern1(after_bytes) {
            (*osd).os_major = dup_substring(after_bytes, s, e);
        }
        // os_minor
        if let Some((s, e)) = match_pattern2(after_bytes) {
            (*osd).os_minor = dup_substring(after_bytes, s, e);
        }
        // os_build
        if let Some((s, e)) = match_pattern3(after_bytes) {
            (*osd).os_build = dup_substring(after_bytes, s, e);
        }

        (*osd).os_version = cstr_strdup(after);
        (*osd).os_platform = alloc_cstr_from_bytes(b"windows");
    } else {
        // " [" branch
        let bracket = cstr_strstr(uname, b" [");
        if !bracket.is_null() {
            *bracket = 0;
            let after_bracket = (bracket as *mut u8).add(2) as *mut c_char;
            (*osd).os_name = cstr_strdup(after_bracket);

            let colon_in_name = cstr_strstr((*osd).os_name, b": ");
            if !colon_in_name.is_null() {
                *colon_in_name = 0;
                let after_colon = (colon_in_name as *mut u8).add(2) as *mut c_char;
                (*osd).os_version = cstr_strdup(after_colon);
                // Trim trailing ']' (matches `*(os_version + strlen - 1) = '\0'`)
                drop_last_char((*osd).os_version);

                // os_major.os_minor (os_codename)
                let paren = cstr_strstr((*osd).os_version, b" (");
                if !paren.is_null() {
                    *paren = 0;
                    let after_paren = (paren as *mut u8).add(2) as *mut c_char;
                    (*osd).os_codename = cstr_strdup(after_paren);
                    drop_last_char((*osd).os_codename);
                }

                let version_bytes = cstr_as_bytes((*osd).os_version);
                if let Some((s, e)) = match_pattern1(version_bytes) {
                    (*osd).os_major = dup_substring(version_bytes, s, e);
                }
                if let Some((s, e)) = match_pattern2(version_bytes) {
                    (*osd).os_minor = dup_substring(version_bytes, s, e);
                }
            } else {
                drop_last_char((*osd).os_name);
            }

            // os_name|os_platform
            let pipe = cstr_strstr((*osd).os_name, b"|");
            if !pipe.is_null() {
                *pipe = 0;
                let after_pipe = (pipe as *mut u8).add(1) as *mut c_char;
                (*osd).os_platform = cstr_strdup(after_pipe);
            }
        }

        let arch_ptr = get_os_arch(uname);
        if !arch_ptr.is_null() {
            (*osd).os_arch = cstr_strdup(arch_ptr);
            libc_free(arch_ptr as *mut c_void);
        }
    }
}

extern "C" {
    #[link_name = "free"]
    fn libc_free(ptr: *mut c_void);
}
