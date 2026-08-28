use std::ffi::{c_char, c_int, c_void};

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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RegMatch {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

#[repr(C, align(8))]
struct Regex {
    storage: [u8; 64],
}

unsafe extern "C" {
    static mut stderr: *mut c_void;

    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn regcomp(regex: *mut Regex, pattern: *const c_char, flags: c_int) -> c_int;
    fn regexec(
        regex: *const Regex,
        string: *const c_char,
        nmatch: usize,
        pmatch: *mut RegMatch,
        flags: c_int,
    ) -> c_int;
    fn regfree(regex: *mut Regex);
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strdup(string: *const c_char) -> *mut c_char;
    fn strlen(string: *const c_char) -> usize;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
}

const REG_EXTENDED: c_int = 1;
const ARCHS: [&[u8]; 12] = [
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    for arch in ARCHS {
        if !unsafe { strstr(os_header, arch.as_ptr().cast()) }.is_null() {
            return unsafe { strdup(arch.as_ptr().cast()) };
        }
    }

    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut RegMatch,
) -> c_int {
    let mut regex = Regex { storage: [0; 64] };

    if pattern.is_null() || string.is_null() {
        return 0;
    }

    if unsafe { regcomp(&mut regex, pattern, REG_EXTENDED) } != 0 {
        unsafe {
            fprintf(
                stderr,
                c"Couldn't compile regular expression '%s'\n".as_ptr(),
                pattern,
            );
        }
        return 0;
    }

    let result = unsafe { regexec(&regex, string, nmatch, pmatch, 0) };
    unsafe { regfree(&mut regex) };
    c_int::from(result == 0)
}

unsafe fn duplicate_capture(
    destination: *mut *mut c_char,
    source: *const c_char,
    capture: RegMatch,
) {
    let match_size = capture.rm_eo - capture.rm_so;
    let allocation = unsafe { malloc((match_size + 1) as usize) }.cast::<c_char>();
    unsafe {
        *destination = allocation;
        snprintf(
            allocation,
            (match_size + 1) as usize,
            c"%.*s".as_ptr(),
            match_size,
            source.offset(capture.rm_so as isize),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut OsData) {
    let mut matches = [RegMatch { rm_so: 0, rm_eo: 0 }; 2];

    if osd.is_null() {
        return;
    }

    let mut str_tmp = unsafe { strstr(uname, c" [Ver: ".as_ptr()) };
    if !str_tmp.is_null() {
        unsafe {
            *str_tmp = 0;
            str_tmp = str_tmp.add(7);
            (*osd).os_name = strdup(uname);
            *str_tmp.add(strlen(str_tmp) - 1) = 0;
        }

        if unsafe {
            w_regexec(
                c"^([0-9]+)\\.*".as_ptr(),
                str_tmp,
                matches.len(),
                matches.as_mut_ptr(),
            )
        } != 0
        {
            unsafe { duplicate_capture(&raw mut (*osd).os_major, str_tmp, matches[1]) };
        }

        if unsafe {
            w_regexec(
                c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
                str_tmp,
                matches.len(),
                matches.as_mut_ptr(),
            )
        } != 0
        {
            unsafe { duplicate_capture(&raw mut (*osd).os_minor, str_tmp, matches[1]) };
        }

        if unsafe {
            w_regexec(
                c"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*".as_ptr(),
                str_tmp,
                matches.len(),
                matches.as_mut_ptr(),
            )
        } != 0
        {
            unsafe { duplicate_capture(&raw mut (*osd).os_build, str_tmp, matches[1]) };
        }

        unsafe {
            (*osd).os_version = strdup(str_tmp);
            (*osd).os_platform = strdup(c"windows".as_ptr());
        }
    } else {
        str_tmp = unsafe { strstr(uname, c" [".as_ptr()) };
        if !str_tmp.is_null() {
            unsafe {
                *str_tmp = 0;
                str_tmp = str_tmp.add(2);
                (*osd).os_name = strdup(str_tmp);
            }

            str_tmp = unsafe { strstr((*osd).os_name, c": ".as_ptr()) };
            if !str_tmp.is_null() {
                unsafe {
                    *str_tmp = 0;
                    str_tmp = str_tmp.add(2);
                    (*osd).os_version = strdup(str_tmp);
                    *(*osd).os_version.add(strlen((*osd).os_version) - 1) = 0;
                }

                str_tmp = unsafe { strstr((*osd).os_version, c" (".as_ptr()) };
                if !str_tmp.is_null() {
                    unsafe {
                        *str_tmp = 0;
                        str_tmp = str_tmp.add(2);
                        (*osd).os_codename = strdup(str_tmp);
                        *(*osd).os_codename.add(strlen((*osd).os_codename) - 1) = 0;
                    }
                }

                if unsafe {
                    w_regexec(
                        c"^([0-9]+)\\.*".as_ptr(),
                        (*osd).os_version,
                        matches.len(),
                        matches.as_mut_ptr(),
                    )
                } != 0
                {
                    unsafe {
                        duplicate_capture(&raw mut (*osd).os_major, (*osd).os_version, matches[1])
                    };
                }

                if unsafe {
                    w_regexec(
                        c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
                        (*osd).os_version,
                        matches.len(),
                        matches.as_mut_ptr(),
                    )
                } != 0
                {
                    unsafe {
                        duplicate_capture(&raw mut (*osd).os_minor, (*osd).os_version, matches[1])
                    };
                }
            } else {
                unsafe {
                    *(*osd).os_name.add(strlen((*osd).os_name) - 1) = 0;
                }
            }

            str_tmp = unsafe { strstr((*osd).os_name, c"|".as_ptr()) };
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
                free(str_tmp.cast());
            }
        }
    }
}
