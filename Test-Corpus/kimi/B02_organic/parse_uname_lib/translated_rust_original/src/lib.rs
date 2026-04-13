use std::ffi::{c_char, CStr, CString};
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

impl os_data {
    fn new() -> Self {
        os_data {
            os_name: ptr::null_mut(),
            os_version: ptr::null_mut(),
            os_major: ptr::null_mut(),
            os_minor: ptr::null_mut(),
            os_codename: ptr::null_mut(),
            os_platform: ptr::null_mut(),
            os_build: ptr::null_mut(),
            os_uname: ptr::null_mut(),
            os_arch: ptr::null_mut(),
        }
    }
}

fn get_os_arch(os_header: &str) -> Option<String> {
    const ARCHS: &[&str] = &[
        "x86_64", "i386", "i686", "sparc", "amd64", "i86pc",
        "ia64", "AIX", "armv6", "armv7", "aarch64", "arm64",
    ];
    
    for arch in ARCHS {
        if os_header.contains(arch) {
            return Some(arch.to_string());
        }
    }
    None
}

fn w_regexec(pattern: &str, string: &str) -> Option<(usize, usize)> {
    let regex = regex::Regex::new(pattern).ok()?;
    regex.captures(string).and_then(|caps| {
        caps.get(1).map(|m| (m.start(), m.end()))
    })
}

fn strdup_rust(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_uname_string(uname: *mut c_char, osd: *mut os_data) {
    if osd.is_null() {
        return;
    }
    
    let uname_str = unsafe {
        match CStr::from_ptr(uname).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return,
        }
    };
    
    let mut osd_ref = unsafe { &mut *osd };
    
    if let Some(pos) = uname_str.find(" [Ver: ") {
        let (name_part, rest) = uname_str.split_at(pos);
        let mut str_tmp = rest[7..].to_string();
        if str_tmp.ends_with(']') {
            str_tmp.pop();
        }
        
        osd_ref.os_name = strdup_rust(name_part);
        
        if let Some((start, end)) = w_regexec(r"^([0-9]+)\.*", &str_tmp) {
            let major = &str_tmp[start..end];
            osd_ref.os_major = strdup_rust(major);
        }
        
        if let Some((start, end)) = w_regexec(r"^[0-9]+\.([0-9]+)\.*", &str_tmp) {
            let minor = &str_tmp[start..end];
            osd_ref.os_minor = strdup_rust(minor);
        }
        
        if let Some((start, end)) = w_regexec(r"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*", &str_tmp) {
            let build = &str_tmp[start..end];
            osd_ref.os_build = strdup_rust(build);
        }
        
        osd_ref.os_version = strdup_rust(&str_tmp);
        osd_ref.os_platform = strdup_rust("windows");
    } else {
        let mut remaining = uname_str.clone();
        
        if let Some(pos) = remaining.find(" [") {
            let name_part = &remaining[..pos];
            let mut str_tmp = remaining[pos + 2..].to_string();
            
            osd_ref.os_name = strdup_rust(&str_tmp);
            
            if let Some(cstr) = unsafe { CStr::from_ptr(osd_ref.os_name).to_str().ok() } {
                let name_owned = cstr.to_string();
                
                if let Some(colon_pos) = name_owned.find(": ") {
                    let name_only = &name_owned[..colon_pos];
                    let version_start = colon_pos + 2;
                    let mut version = name_owned[version_start..].to_string();
                    
                    unsafe {
                        if !osd_ref.os_name.is_null() {
                            let _ = CString::from_raw(osd_ref.os_name);
                        }
                    }
                    osd_ref.os_name = strdup_rust(name_only);
                    
                    if version.ends_with(']') {
                        version.pop();
                    }
                    
                    if let Some(paren_pos) = version.find(" (") {
                        let version_only = &version[..paren_pos];
                        let mut codename = version[paren_pos + 2..].to_string();
                        if codename.ends_with(')') {
                            codename.pop();
                        }
                        osd_ref.os_codename = strdup_rust(&codename);
                        version = version_only.to_string();
                    }
                    
                    if let Some((start, end)) = w_regexec(r"^([0-9]+)\.*", &version) {
                        let major = &version[start..end];
                        osd_ref.os_major = strdup_rust(major);
                    }
                    
                    if let Some((start, end)) = w_regexec(r"^[0-9]+\.([0-9]+)\.*", &version) {
                        let minor = &version[start..end];
                        osd_ref.os_minor = strdup_rust(minor);
                    }
                    
                    osd_ref.os_version = strdup_rust(&version);
                } else {
                    let mut name_trimmed = name_owned;
                    if name_trimmed.ends_with(']') {
                        name_trimmed.pop();
                    }
                    unsafe {
                        if !osd_ref.os_name.is_null() {
                            let _ = CString::from_raw(osd_ref.os_name);
                        }
                    }
                    osd_ref.os_name = strdup_rust(&name_trimmed);
                }
                
                if let Some(pipe_pos) = name_owned.find('|') {
                    let platform = &name_owned[pipe_pos + 1..];
                    let name_only = &name_owned[..pipe_pos];
                    
                    unsafe {
                        if !osd_ref.os_name.is_null() {
                            let _ = CString::from_raw(osd_ref.os_name);
                        }
                    }
                    osd_ref.os_name = strdup_rust(name_only);
                    osd_ref.os_platform = strdup_rust(platform);
                }
            }
        }
        
        if let Some(arch) = get_os_arch(&uname_str) {
            osd_ref.os_arch = strdup_rust(&arch);
        }
    }
}
