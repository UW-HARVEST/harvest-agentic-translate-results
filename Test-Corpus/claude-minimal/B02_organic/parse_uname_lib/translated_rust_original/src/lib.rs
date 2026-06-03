//! Rust translation of the C library in `c_src/`.
//!
//! Provides functionality to parse an OS uname string and fill in an
//! `OsData` structure (and a C-compatible `os_data` structure for FFI).

use regex::Regex;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Idiomatic Rust representation of OS data parsed from a uname string.
#[derive(Default, Debug, Clone)]
pub struct OsData {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_major: Option<String>,
    pub os_minor: Option<String>,
    pub os_codename: Option<String>,
    pub os_platform: Option<String>,
    pub os_build: Option<String>,
    pub os_uname: Option<String>,
    pub os_arch: Option<String>,
}

/// C-ABI compatible version of `os_data`, mirrors the struct in `lib.h`.
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

const ARCHS: &[&str] = &[
    "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
    "aarch64", "arm64",
];

/// Looks for the OS architecture in a string. Possible architectures are
/// x86_64, i386, i686, sparc, amd64, ia64, AIX, armv6, armv7, etc.
/// Returns the matching architecture, or `None` if not found.
pub fn get_os_arch(os_header: &str) -> Option<String> {
    for arch in ARCHS {
        if os_header.contains(arch) {
            return Some((*arch).to_string());
        }
    }
    None
}

/// Returns the first capture group (group 1) of `pattern` applied to `string`,
/// or `None` if no match.
fn regex_capture(pattern: &str, string: &str) -> Option<String> {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Couldn't compile regular expression '{}'", pattern);
            return None;
        }
    };
    re.captures(string)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Parses an OS uname string and returns the parsed data.
pub fn parse_uname(uname: &str) -> OsData {
    let mut osd = OsData::default();

    // [Ver: os_major.os_minor.os_build]
    if let Some(idx) = uname.find(" [Ver: ") {
        let name_part = &uname[..idx];
        let after = &uname[idx + 7..]; // " [Ver: ".len() == 7
        // Strip trailing character (the closing ']')
        let ver_part = if !after.is_empty() {
            &after[..after.len() - 1]
        } else {
            after
        };

        osd.os_name = Some(name_part.to_string());

        // Get os_major
        if let Some(m) = regex_capture(r"^([0-9]+)\.*", ver_part) {
            osd.os_major = Some(m);
        }

        // Get os_minor
        if let Some(m) = regex_capture(r"^[0-9]+\.([0-9]+)\.*", ver_part) {
            osd.os_minor = Some(m);
        }

        // Get os_build
        if let Some(m) = regex_capture(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", ver_part) {
            osd.os_build = Some(m);
        }

        osd.os_version = Some(ver_part.to_string());
        osd.os_platform = Some("windows".to_string());
    } else if let Some(idx) = uname.find(" [") {
        let name_start = &uname[idx + 2..]; // skip " ["
        // os_name initially is everything after " ["
        let mut os_name_str = name_start.to_string();

        if let Some(colon_idx) = os_name_str.find(": ") {
            let name_only = os_name_str[..colon_idx].to_string();
            let after_colon = &os_name_str[colon_idx + 2..];
            // os_version is `after_colon` with the trailing ']' removed
            let mut version_str = if !after_colon.is_empty() {
                after_colon[..after_colon.len() - 1].to_string()
            } else {
                after_colon.to_string()
            };

            // os_major.os_minor (os_codename)
            if let Some(paren_idx) = version_str.find(" (") {
                let after_paren = &version_str[paren_idx + 2..];
                // os_codename is `after_paren` with the trailing ')' removed
                let codename = if !after_paren.is_empty() {
                    after_paren[..after_paren.len() - 1].to_string()
                } else {
                    after_paren.to_string()
                };
                osd.os_codename = Some(codename);
                version_str.truncate(paren_idx);
            }

            // Get os_major
            if let Some(m) = regex_capture(r"^([0-9]+)\.*", &version_str) {
                osd.os_major = Some(m);
            }

            // Get os_minor
            if let Some(m) = regex_capture(r"^[0-9]+\.([0-9]+)\.*", &version_str) {
                osd.os_minor = Some(m);
            }

            osd.os_version = Some(version_str);
            os_name_str = name_only;
        } else {
            // Trim trailing ']' from os_name
            if !os_name_str.is_empty() {
                os_name_str.truncate(os_name_str.len() - 1);
            }
        }

        // os_name|os_platform
        if let Some(pipe_idx) = os_name_str.find('|') {
            let platform = os_name_str[pipe_idx + 1..].to_string();
            osd.os_platform = Some(platform);
            os_name_str.truncate(pipe_idx);
        }

        osd.os_name = Some(os_name_str);

        if let Some(arch) = get_os_arch(uname) {
            osd.os_arch = Some(arch);
        }
    }

    osd
}

/// Helper that converts an `Option<String>` into a `*mut c_char` allocated
/// with `malloc`-equivalent (CString::into_raw). Caller must free with
/// `CString::from_raw` (or equivalent).
fn opt_to_cstr_ptr(s: Option<String>) -> *mut c_char {
    match s {
        Some(v) => match CString::new(v) {
            Ok(cs) => cs.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

/// C-ABI compatible entry point matching the C `parse_uname_string` function.
///
/// # Safety
///
/// `uname` must be a valid pointer to a NUL-terminated C string, and `osd`
/// must be a valid pointer to a writable `os_data` structure.
#[no_mangle]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() || uname.is_null() {
        return;
    }
    let uname_str = match CStr::from_ptr(uname).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    let parsed = parse_uname(uname_str);

    let out = &mut *osd;
    out.os_name = opt_to_cstr_ptr(parsed.os_name);
    out.os_version = opt_to_cstr_ptr(parsed.os_version);
    out.os_major = opt_to_cstr_ptr(parsed.os_major);
    out.os_minor = opt_to_cstr_ptr(parsed.os_minor);
    out.os_codename = opt_to_cstr_ptr(parsed.os_codename);
    out.os_platform = opt_to_cstr_ptr(parsed.os_platform);
    out.os_build = opt_to_cstr_ptr(parsed.os_build);
    // os_uname and os_arch may be set as well
    out.os_uname = opt_to_cstr_ptr(parsed.os_uname);
    out.os_arch = opt_to_cstr_ptr(parsed.os_arch);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os_arch_found() {
        assert_eq!(get_os_arch("Linux x86_64 stuff"), Some("x86_64".to_string()));
        assert_eq!(get_os_arch("AIX whatever"), Some("AIX".to_string()));
    }

    #[test]
    fn test_get_os_arch_not_found() {
        assert_eq!(get_os_arch("nothing here"), None);
    }

    #[test]
    fn test_parse_windows_ver() {
        let osd = parse_uname("Microsoft Windows 10 [Ver: 10.0.19044.1234]");
        assert_eq!(osd.os_name.as_deref(), Some("Microsoft Windows 10"));
        assert_eq!(osd.os_major.as_deref(), Some("10"));
        assert_eq!(osd.os_minor.as_deref(), Some("0"));
        assert_eq!(osd.os_build.as_deref(), Some("19044.1234"));
        assert_eq!(osd.os_platform.as_deref(), Some("windows"));
    }

    #[test]
    fn test_parse_linux_bracketed() {
        let osd = parse_uname(
            "Linux x86_64 [Ubuntu|ubuntu: 20.04 (Focal Fossa)]",
        );
        assert_eq!(osd.os_name.as_deref(), Some("Ubuntu"));
        assert_eq!(osd.os_platform.as_deref(), Some("ubuntu"));
        assert_eq!(osd.os_version.as_deref(), Some("20.04"));
        assert_eq!(osd.os_codename.as_deref(), Some("Focal Fossa"));
        assert_eq!(osd.os_major.as_deref(), Some("20"));
        assert_eq!(osd.os_minor.as_deref(), Some("04"));
        assert_eq!(osd.os_arch.as_deref(), Some("x86_64"));
    }
}
