use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;
}

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

const ARCHS: [&[u8]; 12] = [
    b"x86_64", b"i386", b"i686", b"sparc", b"amd64", b"i86pc", b"ia64", b"AIX", b"armv6",
    b"armv7", b"aarch64", b"arm64",
];

unsafe fn strlen(ptr: *const c_char) -> usize {
    unsafe { libc::strlen(ptr) }
}

unsafe fn strstr(haystack: *mut c_char, needle: &[u8]) -> *mut c_char {
    let hay = unsafe { c_bytes(haystack) };
    if needle.is_empty() {
        return haystack;
    }
    match hay.windows(needle.len()).position(|window| window == needle) {
        Some(pos) => unsafe { haystack.add(pos) },
        None => ptr::null_mut(),
    }
}

unsafe fn strdup(ptr: *const c_char) -> *mut c_char {
    unsafe { libc::strdup(ptr) }
}

unsafe fn malloc_copy(bytes: &[u8]) -> *mut c_char {
    let out = unsafe { libc::malloc(bytes.len() + 1).cast::<c_char>() };
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), out, bytes.len());
        *out.add(bytes.len()) = 0;
    }
    out
}

unsafe fn c_bytes(ptr: *const c_char) -> &'static [u8] {
    unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), strlen(ptr)) }
}

unsafe fn dup_match(ptr: *const c_char, start: usize, end: usize) -> *mut c_char {
    let bytes = unsafe { c_bytes(ptr) };
    unsafe { malloc_copy(&bytes[start..end]) }
}

fn major_match(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    (end > 0).then_some((0, end))
}

fn minor_match(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos == 0 || bytes.get(pos) != Some(&b'.') {
        return None;
    }
    let start = pos + 1;
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    (end > start).then_some((start, end))
}

fn build_match(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos == 0 || bytes.get(pos) != Some(&b'.') {
        return None;
    }
    pos += 1;
    let minor_start = pos;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos == minor_start || bytes.get(pos) != Some(&b'.') {
        return None;
    }
    let start = pos + 1;
    let mut end = start;
    let mut saw_digit = false;
    loop {
        let group_start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == group_start {
            break;
        }
        saw_digit = true;
        if bytes.get(end) == Some(&b'.') {
            let next = end + 1;
            if next < bytes.len() && bytes[next].is_ascii_digit() {
                end = next;
                continue;
            }
        }
        break;
    }
    saw_digit.then_some((start, end))
}

unsafe fn set_major(ptr: *const c_char, osd: *mut os_data) {
    let bytes = unsafe { c_bytes(ptr) };
    if let Some((start, end)) = major_match(bytes) {
        unsafe {
            (*osd).os_major = dup_match(ptr, start, end);
        }
    }
}

unsafe fn set_minor(ptr: *const c_char, osd: *mut os_data) {
    let bytes = unsafe { c_bytes(ptr) };
    if let Some((start, end)) = minor_match(bytes) {
        unsafe {
            (*osd).os_minor = dup_match(ptr, start, end);
        }
    }
}

unsafe fn set_build(ptr: *const c_char, osd: *mut os_data) {
    let bytes = unsafe { c_bytes(ptr) };
    if let Some((start, end)) = build_match(bytes) {
        unsafe {
            (*osd).os_build = dup_match(ptr, start, end);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    for arch in ARCHS {
        if unsafe { strstr(os_header, arch) }.is_null() {
            continue;
        }
        return unsafe { malloc_copy(arch) };
    }

    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut libc::regmatch_t,
) -> c_int {
    if pattern.is_null() || string.is_null() {
        return 0;
    }

    let mut regex = std::mem::MaybeUninit::<libc::regex_t>::uninit();
    if unsafe { libc::regcomp(regex.as_mut_ptr(), pattern, libc::REG_EXTENDED) } != 0 {
        unsafe {
                libc::fprintf(
                stderr,
                c"Couldn't compile regular expression '%s'\n".as_ptr(),
                pattern,
            );
        }
        return 0;
    }

    let mut regex = unsafe { regex.assume_init() };
    let result = unsafe { libc::regexec(&regex, string, nmatch, pmatch, 0) };
    unsafe {
        libc::regfree(&mut regex);
    }
    (result == 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() {
        return;
    }

    let mut str_tmp = unsafe { strstr(uname, b" [Ver: ") };
    if !str_tmp.is_null() {
        unsafe {
            *str_tmp = 0;
            str_tmp = str_tmp.add(7);
            (*osd).os_name = strdup(uname);
            *str_tmp.add(strlen(str_tmp) - 1) = 0;

            set_major(str_tmp, osd);
            set_minor(str_tmp, osd);
            set_build(str_tmp, osd);

            (*osd).os_version = strdup(str_tmp);
            (*osd).os_platform = strdup(c"windows".as_ptr());
        }
    } else {
        str_tmp = unsafe { strstr(uname, b" [") };
        if !str_tmp.is_null() {
            unsafe {
                *str_tmp = 0;
                str_tmp = str_tmp.add(2);
                (*osd).os_name = strdup(str_tmp);
            }

            str_tmp = unsafe { strstr((*osd).os_name, b": ") };
            if !str_tmp.is_null() {
                unsafe {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(2);
                    (*osd).os_version = strdup(str_tmp);
                    *(*osd).os_version.add(strlen((*osd).os_version) - 1) = 0;
                }

                str_tmp = unsafe { strstr((*osd).os_version, b" (") };
                if !str_tmp.is_null() {
                    unsafe {
                        *str_tmp = 0;
                        str_tmp = str_tmp.add(2);
                        (*osd).os_codename = strdup(str_tmp);
                        *(*osd).os_codename.add(strlen((*osd).os_codename) - 1) = 0;
                    }
                }

                unsafe {
                    set_major((*osd).os_version, osd);
                    set_minor((*osd).os_version, osd);
                }
            } else {
                unsafe {
                    *(*osd).os_name.add(strlen((*osd).os_name) - 1) = 0;
                }
            }

            str_tmp = unsafe { strstr((*osd).os_name, b"|") };
            if !str_tmp.is_null() {
                unsafe {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(1);
                    (*osd).os_platform = strdup(str_tmp);
                }
            }
        }

        str_tmp = unsafe { get_os_arch(uname) };
        if !str_tmp.is_null() {
            unsafe {
                (*osd).os_arch = strdup(str_tmp);
                libc::free(str_tmp.cast());
            }
        }
    }
}
