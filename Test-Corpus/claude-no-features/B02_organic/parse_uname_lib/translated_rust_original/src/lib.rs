#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;
use std::ptr;

use libc::{c_void, regex_t, regmatch_t, size_t, REG_EXTENDED};

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
    // Most platforms expose `stderr` as a real symbol; this is what
    // glibc/musl provide on Linux.
    static stderr: *mut libc::FILE;
}

/// Looks for the OS architecture in a string. Possible architectures are
/// x86_64, i386, i686, sparc, amd64, ia64, AIX, armv6, armv7, etc.
/// Returns a pointer to allocated memory that must be deallocated by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    let archs: [*const c_char; 13] = [
        b"x86_64\0".as_ptr() as *const c_char,
        b"i386\0".as_ptr() as *const c_char,
        b"i686\0".as_ptr() as *const c_char,
        b"sparc\0".as_ptr() as *const c_char,
        b"amd64\0".as_ptr() as *const c_char,
        b"i86pc\0".as_ptr() as *const c_char,
        b"ia64\0".as_ptr() as *const c_char,
        b"AIX\0".as_ptr() as *const c_char,
        b"armv6\0".as_ptr() as *const c_char,
        b"armv7\0".as_ptr() as *const c_char,
        b"aarch64\0".as_ptr() as *const c_char,
        b"arm64\0".as_ptr() as *const c_char,
        ptr::null(),
    ];

    let mut os_arch: *mut c_char = ptr::null_mut();

    let mut i = 0usize;
    while !archs[i].is_null() {
        if !libc::strstr(os_header, archs[i]).is_null() {
            os_arch = libc::strdup(archs[i]);
            break;
        }
        i += 1;
    }

    os_arch
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

    let mut regex: MaybeUninit<regex_t> = MaybeUninit::uninit();

    if libc::regcomp(regex.as_mut_ptr(), pattern, REG_EXTENDED) != 0 {
        let fmt = b"Couldn't compile regular expression '%s'\n\0".as_ptr() as *const c_char;
        libc::fprintf(stderr, fmt, pattern);
        return 0;
    }

    let mut regex = regex.assume_init();
    let result = libc::regexec(&regex, string, nmatch, pmatch, 0);
    libc::regfree(&mut regex);
    if result == 0 {
        1
    } else {
        0
    }
}

/// Parses an OS uname string. All the OUT parameters are pointers to allocated
/// memory that must be deallocated by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    let mut str_tmp: *mut c_char;
    let mut match_arr: [regmatch_t; 2] = [regmatch_t { rm_so: 0, rm_eo: 0 }; 2];
    let mut match_size: c_int;

    if osd.is_null() {
        return;
    }

    // [Ver: os_major.os_minor.os_build]
    str_tmp = libc::strstr(uname, b" [Ver: \0".as_ptr() as *const c_char);
    if !str_tmp.is_null() {
        *str_tmp = 0;
        str_tmp = str_tmp.add(7);
        (*osd).os_name = libc::strdup(uname);
        let len = libc::strlen(str_tmp);
        *str_tmp.add(len - 1) = 0;

        // Get os_major
        if w_regexec(
            b"^([0-9]+)\\.*\0".as_ptr() as *const c_char,
            str_tmp,
            2,
            match_arr.as_mut_ptr(),
        ) != 0
        {
            match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
            (*osd).os_major = libc::malloc((match_size + 1) as size_t) as *mut c_char;
            libc::snprintf(
                (*osd).os_major,
                (match_size + 1) as size_t,
                b"%.*s\0".as_ptr() as *const c_char,
                match_size,
                str_tmp.offset(match_arr[1].rm_so as isize),
            );
        }

        // Get os_minor
        if w_regexec(
            b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char,
            str_tmp,
            2,
            match_arr.as_mut_ptr(),
        ) != 0
        {
            match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
            (*osd).os_minor = libc::malloc((match_size + 1) as size_t) as *mut c_char;
            libc::snprintf(
                (*osd).os_minor,
                (match_size + 1) as size_t,
                b"%.*s\0".as_ptr() as *const c_char,
                match_size,
                str_tmp.offset(match_arr[1].rm_so as isize),
            );
        }

        // Get os_build
        if w_regexec(
            b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*\0".as_ptr() as *const c_char,
            str_tmp,
            2,
            match_arr.as_mut_ptr(),
        ) != 0
        {
            match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
            (*osd).os_build = libc::malloc((match_size + 1) as size_t) as *mut c_char;
            libc::snprintf(
                (*osd).os_build,
                (match_size + 1) as size_t,
                b"%.*s\0".as_ptr() as *const c_char,
                match_size,
                str_tmp.offset(match_arr[1].rm_so as isize),
            );
        }

        (*osd).os_version = libc::strdup(str_tmp);
        (*osd).os_platform = libc::strdup(b"windows\0".as_ptr() as *const c_char);
    } else {
        str_tmp = libc::strstr(uname, b" [\0".as_ptr() as *const c_char);
        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = str_tmp.add(2);
            (*osd).os_name = libc::strdup(str_tmp);

            str_tmp = libc::strstr((*osd).os_name, b": \0".as_ptr() as *const c_char);
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(2);
                (*osd).os_version = libc::strdup(str_tmp);
                let vlen = libc::strlen((*osd).os_version);
                *(*osd).os_version.add(vlen - 1) = 0;

                // os_major.os_minor (os_codename)
                str_tmp = libc::strstr((*osd).os_version, b" (\0".as_ptr() as *const c_char);
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(2);
                    (*osd).os_codename = libc::strdup(str_tmp);
                    let clen = libc::strlen((*osd).os_codename);
                    *(*osd).os_codename.add(clen - 1) = 0;
                }

                // Get os_major
                if w_regexec(
                    b"^([0-9]+)\\.*\0".as_ptr() as *const c_char,
                    (*osd).os_version,
                    2,
                    match_arr.as_mut_ptr(),
                ) != 0
                {
                    match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
                    (*osd).os_major = libc::malloc((match_size + 1) as size_t) as *mut c_char;
                    libc::snprintf(
                        (*osd).os_major,
                        (match_size + 1) as size_t,
                        b"%.*s\0".as_ptr() as *const c_char,
                        match_size,
                        (*osd).os_version.offset(match_arr[1].rm_so as isize),
                    );
                }

                // Get os_minor
                if w_regexec(
                    b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char,
                    (*osd).os_version,
                    2,
                    match_arr.as_mut_ptr(),
                ) != 0
                {
                    match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as c_int;
                    (*osd).os_minor = libc::malloc((match_size + 1) as size_t) as *mut c_char;
                    libc::snprintf(
                        (*osd).os_minor,
                        (match_size + 1) as size_t,
                        b"%.*s\0".as_ptr() as *const c_char,
                        match_size,
                        (*osd).os_version.offset(match_arr[1].rm_so as isize),
                    );
                }
            } else {
                let nlen = libc::strlen((*osd).os_name);
                *(*osd).os_name.add(nlen - 1) = 0;
            }

            // os_name|os_platform
            str_tmp = libc::strstr((*osd).os_name, b"|\0".as_ptr() as *const c_char);
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(1);
                (*osd).os_platform = libc::strdup(str_tmp);
            }
        }

        str_tmp = get_os_arch(uname);
        if !str_tmp.is_null() {
            (*osd).os_arch = libc::strdup(str_tmp);
            libc::free(str_tmp as *mut c_void);
        }
    }
}
