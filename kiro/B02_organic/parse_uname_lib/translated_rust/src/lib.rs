use libc::{
    c_char, c_int, free, malloc, regcomp, regex_t, regexec, regfree, regmatch_t, size_t, snprintf,
    strdup, strlen, strstr, REG_EXTENDED,
};
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
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

    for arch in ARCHS {
        if !strstr(os_header, arch.as_ptr() as *const c_char).is_null() {
            return strdup(arch.as_ptr() as *const c_char);
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

    let mut regex: regex_t = std::mem::zeroed();
    if regcomp(&mut regex, pattern, REG_EXTENDED) != 0 {
        libc::fprintf(
            libc::fdopen(2, b"w\0".as_ptr() as *const c_char),
            b"Couldn't compile regular expression '%s'\n\0".as_ptr() as *const c_char,
            pattern,
        );
        return 0;
    }

    let result = regexec(&regex, string, nmatch, pmatch, 0);
    regfree(&mut regex);
    if result == 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() {
        return;
    }

    let mut match_arr: [regmatch_t; 2] = std::mem::zeroed();
    let mut match_size: c_int;

    // [Ver: os_major.os_minor.os_build]
    let mut str_tmp = strstr(uname, b" [Ver: \0".as_ptr() as *const c_char);
    if !str_tmp.is_null() {
        *str_tmp = 0;
        str_tmp = str_tmp.add(7);
        (*osd).os_name = strdup(uname);
        // Remove trailing ']'
        *str_tmp.add(strlen(str_tmp) as usize - 1) = 0;

        // Get os_major
        if w_regexec(
            b"^([0-9]+)\\.*\0".as_ptr() as *const c_char,
            str_tmp,
            2,
            match_arr.as_mut_ptr(),
        ) != 0
        {
            match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
            (*osd).os_major = malloc((match_size + 1) as size_t) as *mut c_char;
            snprintf(
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
            match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
            (*osd).os_minor = malloc((match_size + 1) as size_t) as *mut c_char;
            snprintf(
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
            match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
            (*osd).os_build = malloc((match_size + 1) as size_t) as *mut c_char;
            snprintf(
                (*osd).os_build,
                (match_size + 1) as size_t,
                b"%.*s\0".as_ptr() as *const c_char,
                match_size,
                str_tmp.offset(match_arr[1].rm_so as isize),
            );
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
                // Remove trailing ']'
                *(*osd).os_version.add(strlen((*osd).os_version) as usize - 1) = 0;

                // os_major.os_minor (os_codename)
                str_tmp = strstr((*osd).os_version, b" (\0".as_ptr() as *const c_char);
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(2);
                    (*osd).os_codename = strdup(str_tmp);
                    // Remove trailing ')'
                    *(*osd).os_codename.add(strlen((*osd).os_codename) as usize - 1) = 0;
                }

                // Get os_major
                if w_regexec(
                    b"^([0-9]+)\\.*\0".as_ptr() as *const c_char,
                    (*osd).os_version,
                    2,
                    match_arr.as_mut_ptr(),
                ) != 0
                {
                    match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
                    (*osd).os_major = malloc((match_size + 1) as size_t) as *mut c_char;
                    snprintf(
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
                    match_size = match_arr[1].rm_eo - match_arr[1].rm_so;
                    (*osd).os_minor = malloc((match_size + 1) as size_t) as *mut c_char;
                    snprintf(
                        (*osd).os_minor,
                        (match_size + 1) as size_t,
                        b"%.*s\0".as_ptr() as *const c_char,
                        match_size,
                        (*osd).os_version.offset(match_arr[1].rm_so as isize),
                    );
                }
            } else {
                // No ": " found — remove trailing ']' from os_name
                *(*osd).os_name.add(strlen((*osd).os_name) as usize - 1) = 0;
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
            free(str_tmp as *mut libc::c_void);
        }
    }
}
