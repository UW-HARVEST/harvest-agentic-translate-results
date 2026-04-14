use libc::FILE;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr;

pub const ALERTS_DAILY: &str = "alerts.log";

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

const OS_MAXSTR: usize = 1024;
const ALERT_BEGIN: &str = "** Alert";
const ALERT_BEGIN_SZ: usize = 8;
const RULE_BEGIN: &str = "Rule: ";
const RULE_BEGIN_SZ: usize = 6;
const SRCIP_BEGIN: &str = "Src IP: ";
const SRCIP_BEGIN_SZ: usize = 8;
const SRCPORT_BEGIN: &str = "Src Port: ";
const SRCPORT_BEGIN_SZ: usize = 10;
const DSTIP_BEGIN: &str = "Dst IP: ";
const DSTIP_BEGIN_SZ: usize = 8;
const DSTPORT_BEGIN: &str = "Dst Port: ";
const DSTPORT_BEGIN_SZ: usize = 10;
const USER_BEGIN: &str = "User: ";
const USER_BEGIN_SZ: usize = 6;
const ALERT_MAIL: &str = "mail";
const ALERT_MAIL_SZ: usize = 4;
const LOG_LIMIT: usize = 100;

#[repr(C)]
pub struct alert_data {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: *mut c_char,
    pub date: *mut c_char,
    pub location: *mut c_char,
    pub comment: *mut c_char,
    pub group: *mut c_char,
    pub srcip: *mut c_char,
    pub srcport: c_int,
    pub dstip: *mut c_char,
    pub dstport: c_int,
    pub user: *mut c_char,
    pub filename: *mut c_char,
}

fn cstring_dup(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_else(|_| CString::new("").unwrap()).into_raw()
}

unsafe fn free_c_string(p: *mut c_char) {
    if !p.is_null() {
        let _ = unsafe { CString::from_raw(p) };
    }
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
    }
}

fn parse_c_string(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

fn set_field(dst: &mut *mut c_char, value: &str) {
    unsafe {
        free_c_string(*dst);
    }
    *dst = cstring_dup(value);
}

fn alloc_alert_data() -> *mut alert_data {
    Box::into_raw(Box::new(alert_data {
        rule: 0,
        level: 0,
        alertid: ptr::null_mut(),
        date: ptr::null_mut(),
        location: ptr::null_mut(),
        comment: ptr::null_mut(),
        group: ptr::null_mut(),
        srcip: ptr::null_mut(),
        srcport: 0,
        dstip: ptr::null_mut(),
        dstport: 0,
        user: ptr::null_mut(),
        filename: ptr::null_mut(),
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    if al_data.is_null() {
        return;
    }
    unsafe {
        free_c_string((*al_data).alertid);
        free_c_string((*al_data).date);
        free_c_string((*al_data).location);
        free_c_string((*al_data).comment);
        free_c_string((*al_data).group);
        free_c_string((*al_data).srcip);
        free_c_string((*al_data).dstip);
        free_c_string((*al_data).user);
        free_c_string((*al_data).filename);
        drop(Box::from_raw(al_data));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    if fp.is_null() {
        return ptr::null_mut();
    }

    let al_data = alloc_alert_data();
    let mut state = 0;
    let mut issyscheck = false;
    let mut log_size = 0usize;
    let mut buf = vec![0i8; OS_MAXSTR + 1];

    loop {
        let line_ptr = unsafe { libc::fgets(buf.as_mut_ptr(), OS_MAXSTR as c_int, fp) };
        if line_ptr.is_null() {
            break;
        }

        let mut line = parse_c_string(buf.as_ptr());

        if line.len() >= ALERT_BEGIN_SZ && line.starts_with(ALERT_BEGIN) {
            if state == 2 {
                let back = -(line.len() as libc::c_long);
                let seek_res = unsafe { libc::fseek(fp, back, libc::SEEK_CUR) };
                if seek_res != -1 {
                    return al_data;
                } else {
                    break;
                }
            }

            let mut p = if line.len() > ALERT_BEGIN_SZ + 1 {
                line[ALERT_BEGIN_SZ + 1..].to_string()
            } else {
                String::new()
            };

            if let Some(midx) = p.find(':') {
                let z = midx;
                unsafe {
                    set_field(&mut (*al_data).alertid, &p[..z]);
                }
            } else {
                continue;
            }

            if let Some(space_idx) = p.find(' ') {
                p = p[space_idx + 1..].to_string();
            } else {
                continue;
            }

            if (flag & CRALERT_MAIL_SET) != 0 && !p.starts_with(&ALERT_MAIL[..ALERT_MAIL_SZ]) {
                continue;
            }

            if let Some(dash_idx) = p.find('-') {
                let mut group = p[dash_idx + 1..].to_string();
                while group.starts_with(' ') {
                    group.remove(0);
                }
                trim_newline(&mut group);
                unsafe {
                    set_field(&mut (*al_data).group, &group);
                }
                if group.contains("syscheck") {
                    issyscheck = true;
                }
            }

            state = 1;
            continue;
        }

        if state < 1 {
            continue;
        }

        if state == 1 {
            trim_newline(&mut line);
            let mut date = line.clone();
            let mut location: Option<String> = None;

            if let Some(first_colon) = date.find(':') {
                if let Some(space_after) = date[first_colon..].find(' ') {
                    let split_idx = first_colon + space_after;
                    let loc = date[split_idx + 1..].to_string();
                    date.truncate(split_idx);
                    location = Some(loc);
                } else {
                    FreeAlertData(al_data);
                    unsafe { libc::clearerr(fp) };
                    return ptr::null_mut();
                }
            }

            unsafe {
                if !(*al_data).date.is_null() || !(*al_data).location.is_null() || location.is_none() {
                    FreeAlertData(al_data);
                    libc::clearerr(fp);
                    return ptr::null_mut();
                }
                (*al_data).date = cstring_dup(&date);
                (*al_data).location = cstring_dup(location.as_deref().unwrap_or(""));
            }
            state = 2;
            log_size = 0;
            continue;
        }

        if state == 2 {
            if line.len() >= RULE_BEGIN_SZ && line.starts_with(RULE_BEGIN) {
                trim_newline(&mut line);
                let p = &line[RULE_BEGIN_SZ..];
                let mut parts = p.split_whitespace();
                let rule = parts.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
                let level = p
                    .split_whitespace()
                    .nth(2)
                    .and_then(|x| x.parse::<u32>().ok())
                    .unwrap_or(0);
                let start_quote = p.find('\'');
                let end_quote = p.rfind('\'');
                if start_quote.is_none() || end_quote.is_none() || start_quote == end_quote {
                    FreeAlertData(al_data);
                    unsafe { libc::clearerr(fp) };
                    return ptr::null_mut();
                }
                let comment = &p[start_quote.unwrap() + 1..end_quote.unwrap()];
                unsafe {
                    (*al_data).rule = rule;
                    (*al_data).level = level;
                    set_field(&mut (*al_data).comment, comment);
                }
            } else if line.len() >= SRCIP_BEGIN_SZ && line.starts_with(SRCIP_BEGIN) {
                trim_newline(&mut line);
                unsafe {
                    set_field(&mut (*al_data).srcip, &line[SRCIP_BEGIN_SZ..]);
                }
            } else if line.len() >= SRCPORT_BEGIN_SZ && line.starts_with(SRCPORT_BEGIN) {
                trim_newline(&mut line);
                unsafe {
                    (*al_data).srcport = line[SRCPORT_BEGIN_SZ..].trim().parse::<i32>().unwrap_or(0);
                }
            } else if line.len() >= DSTIP_BEGIN_SZ && line.starts_with(DSTIP_BEGIN) {
                trim_newline(&mut line);
                unsafe {
                    set_field(&mut (*al_data).dstip, &line[DSTIP_BEGIN_SZ..]);
                }
            } else if line.len() >= DSTPORT_BEGIN_SZ && line.starts_with(DSTPORT_BEGIN) {
                trim_newline(&mut line);
                unsafe {
                    (*al_data).dstport = line[DSTPORT_BEGIN_SZ..].trim().parse::<i32>().unwrap_or(0);
                }
            } else if line.len() >= USER_BEGIN_SZ && line.starts_with(USER_BEGIN) {
                trim_newline(&mut line);
                unsafe {
                    set_field(&mut (*al_data).user, &line[USER_BEGIN_SZ..]);
                }
            } else if log_size < LOG_LIMIT {
                trim_newline(&mut line);
                if issyscheck && line.starts_with("Integrity checksum changed for: '") {
                    let mut filename = line[33..].to_string();
                    if filename.ends_with('\'') {
                        filename.pop();
                    }
                    unsafe {
                        set_field(&mut (*al_data).filename, &filename);
                    }
                    issyscheck = false;
                }
            }
        }
    }

    if unsafe { libc::feof(fp) } != 0 && state == 2 {
        return al_data;
    }

    FreeAlertData(al_data);
    unsafe { libc::clearerr(fp) };
    ptr::null_mut()
}
