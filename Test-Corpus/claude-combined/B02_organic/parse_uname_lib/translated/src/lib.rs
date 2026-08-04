use std::ffi::{c_char, c_int};
use std::ptr;

use libc::{
    free, malloc, memcpy, regcomp, regexec, regfree, regex_t, regmatch_t, size_t, strdup, strlen,
    strstr, REG_EXTENDED,
};

/// Mirror of the C struct `os_data`.
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

/// Allocate `n + 1` bytes via libc::malloc, copy `n` bytes from `src`, and
/// null-terminate. Mirrors `snprintf(buf, n+1, "%.*s", n, src)` behavior.
unsafe fn alloc_and_copy(src: *const c_char, n: usize) -> *mut c_char {
    let buf = malloc(n + 1) as *mut c_char;
    if buf.is_null() {
        return ptr::null_mut();
    }
    if n > 0 {
        memcpy(buf as *mut _, src as *const _, n);
    }
    *buf.add(n) = 0;
    buf
}

/// Looks for the OS architecture in a string. Possible architectures are
/// x86_64, i386, i686, sparc, amd64, ia64, AIX, armv6, armv7. Returns a pointer
/// to memory allocated via libc that the caller must free.
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
        if !strstr(os_header, archs[i]).is_null() {
            os_arch = strdup(archs[i]);
            break;
        }
        i += 1;
    }

    os_arch
}

/// Compile + execute a POSIX extended regex. Returns 1 on match, 0 otherwise.
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
        // Match the C output exactly: "Couldn't compile regular expression '%s'\n"
        let prefix = b"Couldn't compile regular expression '\0";
        let suffix = b"'\n\0";
        libc::fprintf(
            libc_stderr(),
            b"%s%s%s\0".as_ptr() as *const c_char,
            prefix.as_ptr() as *const c_char,
            pattern,
            suffix.as_ptr() as *const c_char,
        );
        return 0;
    }

    let result = regexec(&regex, string, nmatch, pmatch, 0);
    regfree(&mut regex);
    if result == 0 {
        1
    } else {
        0
    }
}

/// Returns the libc stderr FILE* pointer.
fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        // On glibc, `stderr` is a real symbol of type `FILE *`.
        static stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

/// Parses an OS uname string. All the OUT parameters are pointers to allocated
/// memory that must be freed by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut OsData) {
    let mut str_tmp: *mut c_char;
    // regmatch_t[2] = {{.rm_so = 0}} — zero-initialized.
    let mut match_arr: [regmatch_t; 2] = [
        regmatch_t { rm_so: 0, rm_eo: 0 },
        regmatch_t { rm_so: 0, rm_eo: 0 },
    ];
    let mut match_size: c_int;

    if osd.is_null() {
        return;
    }

    // [Ver: os_major.os_minor.os_build]
    str_tmp = strstr(uname, b" [Ver: \0".as_ptr() as *const c_char);
    if !str_tmp.is_null() {
        *str_tmp = 0;
        str_tmp = str_tmp.add(7);
        (*osd).os_name = strdup(uname);
        // *(str_tmp + strlen(str_tmp) - 1) = '\0';
        let len = strlen(str_tmp);
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
            (*osd).os_major =
                alloc_and_copy(str_tmp.add(match_arr[1].rm_so as usize), match_size as usize);
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
            (*osd).os_minor =
                alloc_and_copy(str_tmp.add(match_arr[1].rm_so as usize), match_size as usize);
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
            (*osd).os_build =
                alloc_and_copy(str_tmp.add(match_arr[1].rm_so as usize), match_size as usize);
        }

        (*osd).os_version = strdup(str_tmp);
        (*osd).os_platform = strdup(b"windows\0".as_ptr() as *const c_char);
    } else {
        str_tmp = strstr(uname, b" [\0".as_ptr() as *const c_char);
        if !str_tmp.is_null() {
            *str_tmp = 0;
            str_tmp = str_tmp.add(2);
            (*osd).os_name = strdup(str_tmp);

            let inner_tmp = strstr((*osd).os_name, b": \0".as_ptr() as *const c_char);
            if !inner_tmp.is_null() {
                *inner_tmp = 0;
                let after = inner_tmp.add(2);
                (*osd).os_version = strdup(after);
                let ver_len = strlen((*osd).os_version);
                *(*osd).os_version.add(ver_len - 1) = 0;

                // os_major.os_minor (os_codename)
                let code_tmp = strstr((*osd).os_version, b" (\0".as_ptr() as *const c_char);
                if !code_tmp.is_null() {
                    *code_tmp = 0;
                    let after_code = code_tmp.add(2);
                    (*osd).os_codename = strdup(after_code);
                    let code_len = strlen((*osd).os_codename);
                    *(*osd).os_codename.add(code_len - 1) = 0;
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
                    (*osd).os_major = alloc_and_copy(
                        (*osd).os_version.add(match_arr[1].rm_so as usize),
                        match_size as usize,
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
                    (*osd).os_minor = alloc_and_copy(
                        (*osd).os_version.add(match_arr[1].rm_so as usize),
                        match_size as usize,
                    );
                }
            } else {
                let name_len = strlen((*osd).os_name);
                *(*osd).os_name.add(name_len - 1) = 0;
            }

            // os_name|os_platform
            let plat_tmp = strstr((*osd).os_name, b"|\0".as_ptr() as *const c_char);
            if !plat_tmp.is_null() {
                *plat_tmp = 0;
                let after_plat = plat_tmp.add(1);
                (*osd).os_platform = strdup(after_plat);
            }
        }

        let arch_tmp = get_os_arch(uname);
        if !arch_tmp.is_null() {
            (*osd).os_arch = strdup(arch_tmp);
            free(arch_tmp as *mut _);
        }
    }
}
