use libc::{c_char, free, malloc};
use regex::Regex;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_void;
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

fn strdup_rust(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

fn malloc_copy_bytes(bytes: &[u8]) -> *mut c_char {
    let len = bytes.len();
    let ptr_out = unsafe { malloc(len + 1) as *mut c_char };
    if ptr_out.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), ptr_out as *mut u8, len);
        *ptr_out.add(len) = 0;
    }
    ptr_out
}

fn get_os_arch_rs(os_header: &str) -> Option<&'static str> {
    const ARCHS: [&str; 12] = [
        "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7", "aarch64", "arm64",
    ];
    ARCHS.into_iter().find(|arch| os_header.contains(*arch))
}

fn w_regexec_rs(pattern: &str, string: &str) -> Option<Vec<String>> {
    let regex = Regex::new(pattern).ok()?;
    let caps = regex.captures(string)?;
    let mut out = Vec::new();
    for i in 0..caps.len() {
        out.push(caps.get(i).map(|m| m.as_str().to_string()).unwrap_or_default());
    }
    Some(out)
}

#[unsafe(no_mangle)]
pub extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() {
        return;
    }

    let uname_string = if uname.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(uname) }.to_string_lossy().into_owned()
    };

    let osd_ref = unsafe { &mut *osd };

    let mut work = uname_string.clone();

    if let Some(ver_pos) = work.find(" [Ver: ") {
        let name = work[..ver_pos].to_string();
        let mut ver = work[ver_pos + 7..].to_string();
        if ver.ends_with(']') {
            ver.pop();
        }

        osd_ref.os_name = strdup_rust(&name);

        if let Some(caps) = w_regexec_rs(r"^([0-9]+)\.*", &ver) {
            if let Some(m) = caps.get(1) {
                osd_ref.os_major = malloc_copy_bytes(m.as_bytes());
            }
        }

        if let Some(caps) = w_regexec_rs(r"^[0-9]+\.([0-9]+)\.*", &ver) {
            if let Some(m) = caps.get(1) {
                osd_ref.os_minor = malloc_copy_bytes(m.as_bytes());
            }
        }

        if let Some(caps) = w_regexec_rs(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", &ver) {
            if let Some(m) = caps.get(1) {
                osd_ref.os_build = malloc_copy_bytes(m.as_bytes());
            }
        }

        osd_ref.os_version = strdup_rust(&ver);
        osd_ref.os_platform = strdup_rust("windows");
    } else {
        if let Some(bracket_pos) = work.find(" [") {
            let mut inner = work[bracket_pos + 2..].to_string();
            osd_ref.os_name = strdup_rust(&inner);

            if let Some(colon_pos) = inner.find(": ") {
                inner.replace_range(colon_pos..colon_pos + 2, "\0\0");
                let parts: Vec<&str> = inner.split("\0\0").collect();
                let name_part = parts.first().copied().unwrap_or("");
                let mut version_part = parts.get(1).copied().unwrap_or("").to_string();
                if version_part.ends_with(']') {
                    version_part.pop();
                }

                osd_ref.os_name = strdup_rust(name_part);
                osd_ref.os_version = strdup_rust(&version_part);

                let mut version_only = version_part.clone();
                if let Some(code_pos) = version_only.find(" (") {
                    let codename = version_only[code_pos + 2..].strip_suffix(')').unwrap_or(&version_only[code_pos + 2..]).to_string();
                    version_only.truncate(code_pos);
                    osd_ref.os_version = strdup_rust(&version_only);
                    osd_ref.os_codename = strdup_rust(&codename);
                }

                if let Some(caps) = w_regexec_rs(r"^([0-9]+)\.*", unsafe { CStr::from_ptr(osd_ref.os_version) }.to_string_lossy().as_ref()) {
                    if let Some(m) = caps.get(1) {
                        osd_ref.os_major = malloc_copy_bytes(m.as_bytes());
                    }
                }

                if let Some(caps) = w_regexec_rs(r"^[0-9]+\.([0-9]+)\.*", unsafe { CStr::from_ptr(osd_ref.os_version) }.to_string_lossy().as_ref()) {
                    if let Some(m) = caps.get(1) {
                        osd_ref.os_minor = malloc_copy_bytes(m.as_bytes());
                    }
                }
            } else {
                if inner.ends_with(']') {
                    inner.pop();
                }
                osd_ref.os_name = strdup_rust(&inner);
            }

            let current_name = if osd_ref.os_name.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(osd_ref.os_name) }.to_string_lossy().into_owned()
            };
            if let Some(pipe_pos) = current_name.find('|') {
                let left = &current_name[..pipe_pos];
                let right = &current_name[pipe_pos + 1..];
                unsafe {
                    let _ = CString::from_raw(osd_ref.os_name);
                }
                osd_ref.os_name = strdup_rust(left);
                osd_ref.os_platform = strdup_rust(right);
            }
        }

        if let Some(arch) = get_os_arch_rs(&uname_string) {
            let temp = strdup_rust(arch);
            if !temp.is_null() {
                let arch_str = unsafe { CStr::from_ptr(temp) }.to_string_lossy().into_owned();
                osd_ref.os_arch = strdup_rust(&arch_str);
                unsafe {
                    free(temp as *mut c_void);
                }
            }
        }
    }
}
