use std::ffi::{c_char, c_int, c_void};
use std::mem::MaybeUninit;

type SizeT = usize;

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
#[derive(Clone, Copy, Default)]
pub struct RegMatch {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

#[repr(C, align(8))]
struct Regex {
    bytes: [u8; 64],
}

unsafe extern "C" {
    fn malloc(size: SizeT) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(string: *const c_char) -> *mut c_char;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn strlen(string: *const c_char) -> SizeT;
    fn snprintf(buffer: *mut c_char, size: SizeT, format: *const c_char, ...) -> c_int;

    fn regcomp(regex: *mut Regex, pattern: *const c_char, flags: c_int) -> c_int;
    fn regexec(
        regex: *const Regex,
        string: *const c_char,
        nmatch: SizeT,
        matches: *mut RegMatch,
        flags: c_int,
    ) -> c_int;
    fn regfree(regex: *mut Regex);

    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    static mut stderr: *mut c_void;
}

const REG_EXTENDED: c_int = 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
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

    for arch in ARCHS {
        let arch = arch.as_ptr().cast::<c_char>();
        if unsafe { !strstr(os_header, arch).is_null() } {
            return unsafe { strdup(arch) };
        }
    }

    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_regexec(
    pattern: *const c_char,
    string: *const c_char,
    nmatch: SizeT,
    matches: *mut RegMatch,
) -> c_int {
    if pattern.is_null() || string.is_null() {
        return 0;
    }

    let mut regex = MaybeUninit::<Regex>::uninit();
    if unsafe { regcomp(regex.as_mut_ptr(), pattern, REG_EXTENDED) } != 0 {
        unsafe {
            fprintf(
                stderr,
                c"Couldn't compile regular expression '%s'\n".as_ptr(),
                pattern,
            );
        }
        return 0;
    }

    let result = unsafe { regexec(regex.as_ptr(), string, nmatch, matches, 0) };
    unsafe { regfree(regex.as_mut_ptr()) };
    c_int::from(result == 0)
}

unsafe fn copy_match(destination: *mut *mut c_char, source: *const c_char, matched: RegMatch) {
    let match_size = matched.rm_eo - matched.rm_so;
    let output = unsafe { malloc((match_size + 1) as SizeT).cast::<c_char>() };
    unsafe {
        *destination = output;
        snprintf(
            output,
            (match_size + 1) as SizeT,
            c"%.*s".as_ptr(),
            match_size,
            source.offset(matched.rm_so as isize),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut OsData) {
    if osd.is_null() {
        return;
    }

    let mut matches = [RegMatch::default(); 2];

    let mut temporary = unsafe { strstr(uname, c" [Ver: ".as_ptr()) };
    if !temporary.is_null() {
        unsafe {
            *temporary = 0;
            temporary = temporary.add(7);
            (*osd).os_name = strdup(uname);
            *temporary.add(strlen(temporary) - 1) = 0;
        }

        if unsafe {
            w_regexec(
                c"^([0-9]+)\\.*".as_ptr(),
                temporary,
                matches.len(),
                matches.as_mut_ptr(),
            )
        } != 0
        {
            unsafe { copy_match(&raw mut (*osd).os_major, temporary, matches[1]) };
        }

        if unsafe {
            w_regexec(
                c"^[0-9]+\\.([0-9]+)\\.*".as_ptr(),
                temporary,
                matches.len(),
                matches.as_mut_ptr(),
            )
        } != 0
        {
            unsafe { copy_match(&raw mut (*osd).os_minor, temporary, matches[1]) };
        }

        if unsafe {
            w_regexec(
                c"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*".as_ptr(),
                temporary,
                matches.len(),
                matches.as_mut_ptr(),
            )
        } != 0
        {
            unsafe { copy_match(&raw mut (*osd).os_build, temporary, matches[1]) };
        }

        unsafe {
            (*osd).os_version = strdup(temporary);
            (*osd).os_platform = strdup(c"windows".as_ptr());
        }
    } else {
        temporary = unsafe { strstr(uname, c" [".as_ptr()) };
        if !temporary.is_null() {
            unsafe {
                *temporary = 0;
                temporary = temporary.add(2);
                (*osd).os_name = strdup(temporary);
            }

            temporary = unsafe { strstr((*osd).os_name, c": ".as_ptr()) };
            if !temporary.is_null() {
                unsafe {
                    *temporary = 0;
                    temporary = temporary.add(2);
                    (*osd).os_version = strdup(temporary);
                    *(*osd).os_version.add(strlen((*osd).os_version) - 1) = 0;
                }

                temporary = unsafe { strstr((*osd).os_version, c" (".as_ptr()) };
                if !temporary.is_null() {
                    unsafe {
                        *temporary = 0;
                        temporary = temporary.add(2);
                        (*osd).os_codename = strdup(temporary);
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
                    unsafe { copy_match(&raw mut (*osd).os_major, (*osd).os_version, matches[1]) };
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
                    unsafe { copy_match(&raw mut (*osd).os_minor, (*osd).os_version, matches[1]) };
                }
            } else {
                unsafe {
                    *(*osd).os_name.add(strlen((*osd).os_name) - 1) = 0;
                }
            }

            temporary = unsafe { strstr((*osd).os_name, c"|".as_ptr()) };
            if !temporary.is_null() {
                unsafe {
                    *temporary = 0;
                    temporary = temporary.add(1);
                    (*osd).os_platform = strdup(temporary);
                }
            }
        }

        temporary = unsafe { get_os_arch(uname) };
        if !temporary.is_null() {
            unsafe {
                (*osd).os_arch = strdup(temporary);
                free(temporary.cast());
            }
        }
    }
}
