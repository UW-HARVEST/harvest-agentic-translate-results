use libc::{c_char, c_void, free, malloc, strdup, strlen, strstr};
use regex::Regex;
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

unsafe fn get_os_arch(os_header: *mut c_char) -> *mut c_char {
    const ARCHS: &[&[u8]] = &[
        b"x86_64\0", b"i386\0", b"i686\0", b"sparc\0", b"amd64\0", b"i86pc\0",
        b"ia64\0", b"AIX\0", b"armv6\0", b"armv7\0", b"aarch64\0", b"arm64\0",
    ];
    for arch in ARCHS {
        let p = strstr(os_header, arch.as_ptr() as *const c_char);
        if !p.is_null() {
            return strdup(arch.as_ptr() as *const c_char);
        }
    }
    ptr::null_mut()
}

/// Mimics w_regexec: compiles pattern as POSIX ERE, runs against string,
/// returns whether it matched, filling pmatch with (start, end) of capture group 1.
/// We return (matched, start, end) for capture group 1.
fn w_regexec(pattern: &str, string: &str) -> Option<(usize, usize)> {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => {
            eprint!("Couldn't compile regular expression '{}'\n", pattern);
            return None;
        }
    };
    let caps = re.captures(string)?;
    let m = caps.get(1)?;
    Some((m.start(), m.end()))
}

/// Helper: snprintf-style copy of a substring (match_size bytes from src+offset)
unsafe fn alloc_match(src: *const c_char, offset: usize, match_size: usize) -> *mut c_char {
    let p = malloc(match_size + 1) as *mut c_char;
    ptr::copy_nonoverlapping(src.add(offset), p, match_size);
    *p.add(match_size) = 0;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() {
        return;
    }

    let osd = &mut *osd;

    // Check for " [Ver: " (Windows path)
    let ver_needle = b" [Ver: \0";
    let str_tmp = strstr(uname, ver_needle.as_ptr() as *const c_char);
    if !str_tmp.is_null() {
        // *str_tmp = '\0'
        *(str_tmp as *mut u8) = 0;
        let after_ver = str_tmp.add(7); // skip " [Ver: "

        osd.os_name = strdup(uname);

        // Remove trailing ']': *(str_tmp + strlen(str_tmp) - 1) = '\0'
        let len = strlen(after_ver);
        *after_ver.add(len - 1) = 0;

        // Extract version string for regex matching
        let ver_str = CStr::from_ptr(after_ver).to_str().unwrap_or("");

        // Get os_major
        if let Some((so, eo)) = w_regexec(r"^([0-9]+)\.*", ver_str) {
            osd.os_major = alloc_match(after_ver, so, eo - so);
        }

        // Get os_minor
        if let Some((so, eo)) = w_regexec(r"^[0-9]+\.([0-9]+)\.*", ver_str) {
            osd.os_minor = alloc_match(after_ver, so, eo - so);
        }

        // Get os_build
        if let Some((so, eo)) = w_regexec(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", ver_str) {
            osd.os_build = alloc_match(after_ver, so, eo - so);
        }

        osd.os_version = strdup(after_ver);
        osd.os_platform = strdup(b"windows\0".as_ptr() as *const c_char);
    } else {
        // Non-Windows path: look for " ["
        let bracket_needle = b" [\0";
        let str_tmp = strstr(uname, bracket_needle.as_ptr() as *const c_char);
        if !str_tmp.is_null() {
            // *str_tmp = '\0'
            *(str_tmp as *mut u8) = 0;
            let after_bracket = str_tmp.add(2); // skip " ["

            osd.os_name = strdup(after_bracket);

            // Look for ": " in os_name
            let colon_needle = b": \0";
            let str_tmp2 = strstr(osd.os_name, colon_needle.as_ptr() as *const c_char);
            if !str_tmp2.is_null() {
                *(str_tmp2 as *mut u8) = 0;
                let after_colon = str_tmp2.add(2);

                osd.os_version = strdup(after_colon);
                // Remove trailing ']'
                let vlen = strlen(osd.os_version);
                *osd.os_version.add(vlen - 1) = 0;

                // Look for " (" in os_version for codename
                let paren_needle = b" (\0";
                let str_tmp3 = strstr(osd.os_version, paren_needle.as_ptr() as *const c_char);
                if !str_tmp3.is_null() {
                    *(str_tmp3 as *mut u8) = 0;
                    let after_paren = str_tmp3.add(2);
                    osd.os_codename = strdup(after_paren);
                    let clen = strlen(osd.os_codename);
                    *osd.os_codename.add(clen - 1) = 0;
                }

                let ver_str = CStr::from_ptr(osd.os_version).to_str().unwrap_or("");

                // Get os_major
                if let Some((so, eo)) = w_regexec(r"^([0-9]+)\.*", ver_str) {
                    osd.os_major = alloc_match(osd.os_version, so, eo - so);
                }

                // Get os_minor
                if let Some((so, eo)) = w_regexec(r"^[0-9]+\.([0-9]+)\.*", ver_str) {
                    osd.os_minor = alloc_match(osd.os_version, so, eo - so);
                }
            } else {
                // No ": " found - remove trailing ']' from os_name
                let nlen = strlen(osd.os_name);
                *osd.os_name.add(nlen - 1) = 0;
            }

            // Look for "|" in os_name for platform
            let pipe_needle = b"|\0";
            let str_tmp4 = strstr(osd.os_name, pipe_needle.as_ptr() as *const c_char);
            if !str_tmp4.is_null() {
                *(str_tmp4 as *mut u8) = 0;
                osd.os_platform = strdup(str_tmp4.add(1));
            }
        }

        // Get architecture from the (now possibly truncated) uname
        let arch = get_os_arch(uname);
        if !arch.is_null() {
            osd.os_arch = strdup(arch);
            free(arch as *mut c_void);
        }
    }
}
