use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
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
pub extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    if os_header.is_null() {
        return ptr::null_mut();
    }
    let archs = [
        b"x86_64\0", b"i386\0", b"i686\0", b"sparc\0", b"amd64\0", b"i86pc\0", b"ia64\0", b"AIX\0",
        b"armv6\0", b"armv7\0", b"aarch64\0", b"arm64\0",
    ];
    unsafe {
        for arch in archs.iter() {
            if !libc::strstr(os_header, arch.as_ptr() as *const c_char).is_null() {
                return libc::strdup(arch.as_ptr() as *const c_char);
            }
        }
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut libc::regmatch_t,
) -> c_int {
    if pattern.is_null() || string.is_null() {
        return 0;
    }
    unsafe {
        let mut regex: libc::regex_t = std::mem::zeroed();
        if libc::regcomp(&mut regex, pattern, libc::REG_EXTENDED) != 0 {
            let pat_str = CStr::from_ptr(pattern).to_string_lossy();
            eprintln!("Couldn't compile regular expression '{}'", pat_str);
            return 0;
        }
        let result = libc::regexec(&regex, string, nmatch, pmatch, 0);
        libc::regfree(&mut regex);
        if result == 0 { 1 } else { 0 }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() || uname.is_null() {
        return;
    }

    unsafe {
        let ver_tag = b" [Ver: \0".as_ptr() as *const c_char;
        let mut str_tmp = libc::strstr(uname, ver_tag);

        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = str_tmp.add(7);
            (*osd).os_name = libc::strdup(uname);
            
            let len = libc::strlen(str_tmp);
            if len > 0 {
                *str_tmp.add(len - 1) = 0;
            }

            let mut match_arr: [libc::regmatch_t; 2] = std::mem::zeroed();
            
            // Get os_major
            let pat_major = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
            if w_regexec(pat_major, str_tmp, 2, match_arr.as_mut_ptr()) != 0 {
                let match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as usize;
                (*osd).os_major = libc::malloc(match_size + 1) as *mut c_char;
                libc::snprintf(
                    (*osd).os_major,
                    match_size + 1,
                    b"%.*s\0".as_ptr() as *const c_char,
                    match_size as c_int,
                    str_tmp.offset(match_arr[1].rm_so as isize),
                );
            }

            // Get os_minor
            let pat_minor = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
            if w_regexec(pat_minor, str_tmp, 2, match_arr.as_mut_ptr()) != 0 {
                let match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as usize;
                (*osd).os_minor = libc::malloc(match_size + 1) as *mut c_char;
                libc::snprintf(
                    (*osd).os_minor,
                    match_size + 1,
                    b"%.*s\0".as_ptr() as *const c_char,
                    match_size as c_int,
                    str_tmp.offset(match_arr[1].rm_so as isize),
                );
            }

            // Get os_build
            let pat_build = b"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*\0".as_ptr() as *const c_char;
            if w_regexec(pat_build, str_tmp, 2, match_arr.as_mut_ptr()) != 0 {
                let match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as usize;
                (*osd).os_build = libc::malloc(match_size + 1) as *mut c_char;
                libc::snprintf(
                    (*osd).os_build,
                    match_size + 1,
                    b"%.*s\0".as_ptr() as *const c_char,
                    match_size as c_int,
                    str_tmp.offset(match_arr[1].rm_so as isize),
                );
            }

            (*osd).os_version = libc::strdup(str_tmp);
            (*osd).os_platform = libc::strdup(b"windows\0".as_ptr() as *const c_char);
        } else {
            let bracket_tag = b" [\0".as_ptr() as *const c_char;
            str_tmp = libc::strstr(uname, bracket_tag);
            if !str_tmp.is_null() {
                *str_tmp = 0;
                str_tmp = str_tmp.add(2);
                (*osd).os_name = libc::strdup(str_tmp);
                
                let colon_tag = b": \0".as_ptr() as *const c_char;
                str_tmp = libc::strstr((*osd).os_name, colon_tag);
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(2);
                    (*osd).os_version = libc::strdup(str_tmp);
                    
                    let v_len = libc::strlen((*osd).os_version);
                    if v_len > 0 {
                        *(*osd).os_version.add(v_len - 1) = 0;
                    }

                    let paren_tag = b" (\0".as_ptr() as *const c_char;
                    str_tmp = libc::strstr((*osd).os_version, paren_tag);
                    if !str_tmp.is_null() {
                        *str_tmp = 0;
                        str_tmp = str_tmp.add(2);
                        (*osd).os_codename = libc::strdup(str_tmp);
                        
                        let c_len = libc::strlen((*osd).os_codename);
                        if c_len > 0 {
                            *(*osd).os_codename.add(c_len - 1) = 0;
                        }
                    }

                    let mut match_arr: [libc::regmatch_t; 2] = std::mem::zeroed();

                    // Get os_major
                    let pat_major = b"^([0-9]+)\\.*\0".as_ptr() as *const c_char;
                    if w_regexec(pat_major, (*osd).os_version, 2, match_arr.as_mut_ptr()) != 0 {
                        let match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as usize;
                        (*osd).os_major = libc::malloc(match_size + 1) as *mut c_char;
                        libc::snprintf(
                            (*osd).os_major,
                            match_size + 1,
                            b"%.*s\0".as_ptr() as *const c_char,
                            match_size as c_int,
                            (*osd).os_version.offset(match_arr[1].rm_so as isize),
                        );
                    }

                    // Get os_minor
                    let pat_minor = b"^[0-9]+\\.([0-9]+)\\.*\0".as_ptr() as *const c_char;
                    if w_regexec(pat_minor, (*osd).os_version, 2, match_arr.as_mut_ptr()) != 0 {
                        let match_size = (match_arr[1].rm_eo - match_arr[1].rm_so) as usize;
                        (*osd).os_minor = libc::malloc(match_size + 1) as *mut c_char;
                        libc::snprintf(
                            (*osd).os_minor,
                            match_size + 1,
                            b"%.*s\0".as_ptr() as *const c_char,
                            match_size as c_int,
                            (*osd).os_version.offset(match_arr[1].rm_so as isize),
                        );
                    }

                } else {
                    let n_len = libc::strlen((*osd).os_name);
                    if n_len > 0 {
                        *(*osd).os_name.add(n_len - 1) = 0;
                    }
                }

                let pipe_tag = b"|\0".as_ptr() as *const c_char;
                str_tmp = libc::strstr((*osd).os_name, pipe_tag);
                if !str_tmp.is_null() {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(1);
                    (*osd).os_platform = libc::strdup(str_tmp);
                }
            }

            let arch = get_os_arch(uname);
            if !arch.is_null() {
                (*osd).os_arch = libc::strdup(arch);
                libc::free(arch as *mut libc::c_void);
            }
        }
    }
}
