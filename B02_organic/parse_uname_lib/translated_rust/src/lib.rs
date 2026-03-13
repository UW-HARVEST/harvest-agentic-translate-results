use libc::{c_char, c_int, free, malloc, regcomp, regexec, regfree, regmatch_t, regex_t, size_t, strdup, strlen, strstr, REG_EXTENDED};
use std::ffi::CStr;
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

/// Looks for the OS architecture in a string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    static ARCHS: &[&[u8]] = &[
        b"x86_64\0", b"i386\0", b"i686\0", b"sparc\0", b"amd64\0", b"i86pc\0",
        b"ia64\0", b"AIX\0", b"armv6\0", b"armv7\0", b"aarch64\0", b"arm64\0",
    ];

    for arch in ARCHS {
        let arch_ptr = arch.as_ptr() as *const c_char;
        if !unsafe { strstr(os_header, arch_ptr) }.is_null() {
            return unsafe { strdup(arch_ptr) };
        }
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: size_t,
    pmatch: *mut regmatch_t,
) -> c_int {
    if pattern.is_null() || string.is_null() {
        return 0;
    }

    let mut regex: regex_t = unsafe { std::mem::zeroed() };
    if unsafe { regcomp(&mut regex, pattern, REG_EXTENDED) } != 0 {
        let pat = unsafe { CStr::from_ptr(pattern) }.to_string_lossy();
        eprintln!("Couldn't compile regular expression '{}'", pat);
        return 0;
    }

    let result = unsafe { regexec(&regex, string, nmatch, pmatch, 0) };
    unsafe { regfree(&mut regex) };
    if result == 0 { 1 } else { 0 }
}

unsafe fn snprintf_match(dst: *mut c_char, size: c_int, src: *const c_char, len: c_int) {
    // Replicates: snprintf(dst, match_size + 1, "%.*s", match_size, src + offset)
    let count = len.min(size - 1);
    if count > 0 {
        unsafe { ptr::copy_nonoverlapping(src, dst, count as usize) };
    }
    unsafe { *dst.add(count as usize) = 0 };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() {
        return;
    }

    let mut match_arr: [regmatch_t; 2] = unsafe { std::mem::zeroed() };
    let mut match_size: c_int;

    // [Ver: os_major.os_minor.os_build]
    let ver_needle = b" [Ver: \0".as_ptr() as *const c_char;
    let str_tmp = unsafe { strstr(uname, ver_needle) };
    if !str_tmp.is_null() {
        unsafe { *str_tmp = 0 };
        let str_tmp = unsafe { str_tmp.add(7) };
        unsafe { (*osd).os_name = strdup(uname) };
        // Remove trailing ']'
        let end = unsafe { str_tmp.add(strlen(str_tmp) as usize - 1) };
        unsafe { *end = 0 };

        // Get os_major
        let pat = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
        if unsafe { w_regexec(pat, str_tmp, 2, match_arr.as_mut_ptr()) } != 0 {
            match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
            unsafe {
                (*osd).os_major = malloc((match_size + 1) as size_t) as *mut c_char;
                snprintf_match((*osd).os_major, match_size + 1, str_tmp.offset(match_arr[1].rm_so as isize), match_size);
            }
        }

        // Get os_minor
        let pat = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
        if unsafe { w_regexec(pat, str_tmp, 2, match_arr.as_mut_ptr()) } != 0 {
            match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
            unsafe {
                (*osd).os_minor = malloc((match_size + 1) as size_t) as *mut c_char;
                snprintf_match((*osd).os_minor, match_size + 1, str_tmp.offset(match_arr[1].rm_so as isize), match_size);
            }
        }

        // Get os_build
        let pat = b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*\0".as_ptr() as *const c_char;
        if unsafe { w_regexec(pat, str_tmp, 2, match_arr.as_mut_ptr()) } != 0 {
            match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
            unsafe {
                (*osd).os_build = malloc((match_size + 1) as size_t) as *mut c_char;
                snprintf_match((*osd).os_build, match_size + 1, str_tmp.offset(match_arr[1].rm_so as isize), match_size);
            }
        }

        unsafe {
            (*osd).os_version = strdup(str_tmp);
            (*osd).os_platform = strdup(b"windows\0".as_ptr() as *const c_char);
        }
    } else {
        // Non-windows path
        let bracket_needle = b" [\0".as_ptr() as *const c_char;
        let str_tmp = unsafe { strstr(uname, bracket_needle) };
        if !str_tmp.is_null() {
            unsafe { *str_tmp = 0 };
            let str_tmp = unsafe { str_tmp.add(2) };
            unsafe { (*osd).os_name = strdup(str_tmp) };

            let colon_needle = b": \0".as_ptr() as *const c_char;
            let str_tmp2 = unsafe { strstr((*osd).os_name, colon_needle) };
            if !str_tmp2.is_null() {
                unsafe { *str_tmp2 = 0 };
                let str_tmp2 = unsafe { str_tmp2.add(2) };
                unsafe {
                    (*osd).os_version = strdup(str_tmp2);
                    // Remove trailing ']'
                    let ver = (*osd).os_version;
                    *ver.add(strlen(ver) as usize - 1) = 0;
                }

                // os_major.os_minor (os_codename)
                let paren_needle = b" (\0".as_ptr() as *const c_char;
                let str_tmp3 = unsafe { strstr((*osd).os_version, paren_needle) };
                if !str_tmp3.is_null() {
                    unsafe {
                        *str_tmp3 = 0;
                        let str_tmp3 = str_tmp3.add(2);
                        (*osd).os_codename = strdup(str_tmp3);
                        let cn = (*osd).os_codename;
                        *cn.add(strlen(cn) as usize - 1) = 0;
                    }
                }

                // Get os_major
                let pat = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
                if unsafe { w_regexec(pat, (*osd).os_version, 2, match_arr.as_mut_ptr()) } != 0 {
                    match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
                    unsafe {
                        (*osd).os_major = malloc((match_size + 1) as size_t) as *mut c_char;
                        snprintf_match((*osd).os_major, match_size + 1, (*osd).os_version.offset(match_arr[1].rm_so as isize), match_size);
                    }
                }

                // Get os_minor
                let pat = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
                if unsafe { w_regexec(pat, (*osd).os_version, 2, match_arr.as_mut_ptr()) } != 0 {
                    match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
                    unsafe {
                        (*osd).os_minor = malloc((match_size + 1) as size_t) as *mut c_char;
                        snprintf_match((*osd).os_minor, match_size + 1, (*osd).os_version.offset(match_arr[1].rm_so as isize), match_size);
                    }
                }
            } else {
                // No ": " found — remove trailing ']'
                unsafe {
                    let name = (*osd).os_name;
                    *name.add(strlen(name) as usize - 1) = 0;
                }
            }

            // os_name|os_platform
            let pipe_needle = b"|\0".as_ptr() as *const c_char;
            let str_pipe = unsafe { strstr((*osd).os_name, pipe_needle) };
            if !str_pipe.is_null() {
                unsafe {
                    *str_pipe = 0;
                    (*osd).os_platform = strdup(str_pipe.add(1));
                }
            }
        }

        // get_os_arch on original uname
        let arch = unsafe { get_os_arch(uname) };
        if !arch.is_null() {
            unsafe {
                (*osd).os_arch = strdup(arch);
                free(arch as *mut libc::c_void);
            }
        }
    }
}
