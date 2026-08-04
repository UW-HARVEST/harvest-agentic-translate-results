//! Translation of `read-alert.c` and `read-alert.h`.
//!
//! Provides the [`alert_data`] structure plus the `GetAlertData` and
//! `FreeAlertData` C-callable functions.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use libc::{c_char, c_int, c_uint, FILE};
use std::ffi::CString;
use std::ptr;

pub const ALERTS_DAILY: &str = "alerts.log";

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

pub const OS_MAXSTR: usize = 1024;

/* Alert format constants */
const ALERT_BEGIN: &[u8] = b"** Alert";
const ALERT_BEGIN_SZ: usize = 8;
const RULE_BEGIN: &[u8] = b"Rule: ";
const RULE_BEGIN_SZ: usize = 6;
const SRCIP_BEGIN: &[u8] = b"Src IP: ";
const SRCIP_BEGIN_SZ: usize = 8;
const SRCPORT_BEGIN: &[u8] = b"Src Port: ";
const SRCPORT_BEGIN_SZ: usize = 10;
const DSTIP_BEGIN: &[u8] = b"Dst IP: ";
const DSTIP_BEGIN_SZ: usize = 8;
const DSTPORT_BEGIN: &[u8] = b"Dst Port: ";
const DSTPORT_BEGIN_SZ: usize = 10;
const USER_BEGIN: &[u8] = b"User: ";
const USER_BEGIN_SZ: usize = 6;
const ALERT_MAIL: &[u8] = b"mail";
const ALERT_MAIL_SZ: usize = 4;

const LOG_LIMIT: usize = 100;

const SYSCHECK_PREFIX: &[u8] = b"Integrity checksum changed for: '";
const SYSCHECK_PREFIX_LEN: usize = 33;

/// Equivalent of the C `alert_data` struct.
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

impl alert_data {
    pub fn empty() -> Self {
        alert_data {
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
        }
    }
}

/// Free a single C string previously allocated with `CString::into_raw` or
/// `libc::strdup`.
unsafe fn os_free(p: &mut *mut c_char) {
    if !p.is_null() {
        unsafe { libc::free(*p as *mut libc::c_void) };
        *p = ptr::null_mut();
    }
}

/// Allocate a C string copy of `s` using libc `malloc` (so it can be freed with
/// `free`, matching the C semantics of `os_strdup` / `strdup`).
unsafe fn os_strdup(s: &[u8]) -> *mut c_char {
    let len = s.len();
    let dst = unsafe { libc::malloc(len + 1) } as *mut c_char;
    if dst.is_null() {
        eprintln!("Memory allocation failed in os_strdup");
        std::process::exit(1);
    }
    unsafe {
        ptr::copy_nonoverlapping(s.as_ptr() as *const c_char, dst, len);
        *dst.add(len) = 0;
    }
    dst
}

/// Replace `*p` (allocated via `malloc`) with a copy of `bytes` (also via
/// `malloc`).
unsafe fn replace_cstr(p: &mut *mut c_char, bytes: &[u8]) {
    unsafe {
        os_free(p);
        *p = os_strdup(bytes);
    }
}

/// Strip a trailing `\n` from a byte slice, returning the trimmed length.
fn clear_nl(bytes: &[u8]) -> &[u8] {
    if let Some(pos) = bytes.iter().rposition(|&b| b == b'\n') {
        &bytes[..pos]
    } else {
        bytes
    }
}

/// Read up to `max_size - 1` bytes (or up to and including a newline) from
/// `fp` into a `Vec<u8>`. Returns `None` on EOF (with no bytes read).
unsafe fn fgets_vec(fp: *mut FILE, max_size: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; max_size];
    let res = unsafe { libc::fgets(buf.as_mut_ptr() as *mut c_char, max_size as c_int, fp) };
    if res.is_null() {
        return None;
    }
    // Find the null terminator
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(len);
    Some(buf)
}

/// Free an `alert_data` allocated via [`alloc_alert_data`].
///
/// # Safety
/// `al_data` must be a valid pointer previously returned by
/// [`alloc_alert_data`] (or `GetAlertData`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    if al_data.is_null() {
        return;
    }
    unsafe {
        let mut data = Box::from_raw(al_data);
        os_free(&mut data.alertid);
        os_free(&mut data.date);
        os_free(&mut data.location);
        os_free(&mut data.comment);
        os_free(&mut data.group);
        os_free(&mut data.srcip);
        os_free(&mut data.dstip);
        os_free(&mut data.user);
        os_free(&mut data.filename);
        // Box drops the struct itself
    }
}

/// Allocate a fresh `alert_data` on the heap, returning a raw pointer.
fn alloc_alert_data() -> *mut alert_data {
    Box::into_raw(Box::new(alert_data::empty()))
}

/// Locate the first occurrence of `needle` (a single byte) in `hay`,
/// returning its index.
fn memchr_byte(hay: &[u8], needle: u8) -> Option<usize> {
    hay.iter().position(|&b| b == needle)
}

/// Locate the last occurrence of `needle` (a single byte) in `hay`.
fn memrchr_byte(hay: &[u8], needle: u8) -> Option<usize> {
    hay.iter().rposition(|&b| b == needle)
}

/// Parse a leading signed decimal integer (mirrors C `atoi` semantics).
fn atoi_bytes(bytes: &[u8]) -> c_int {
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let mut sign: i64 = 1;
    if let Some(&c) = chars.peek() {
        if c == '-' {
            sign = -1;
            chars.next();
        } else if c == '+' {
            chars.next();
        }
    }
    let mut acc: i64 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            acc = acc.saturating_mul(10).saturating_add(d as i64);
        } else {
            break;
        }
    }
    (sign * acc) as c_int
}

/// Read an `alert_data` from the FILE pointer `fp` and return it.
///
/// # Safety
/// `fp` must be a valid `FILE *` opened for reading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    if fp.is_null() {
        return ptr::null_mut();
    }

    let al_data = alloc_alert_data();

    let mut _r: i32 = 0;
    let mut issyscheck: i32 = 0;
    let mut log_size: usize = 0;

    loop {
        let line = match unsafe { fgets_vec(fp, OS_MAXSTR + 1) } {
            Some(l) => l,
            None => break,
        };

        // End of alert / start of new alert
        if line.len() >= ALERT_BEGIN_SZ && &line[..ALERT_BEGIN_SZ] == ALERT_BEGIN {
            // End of the alert.
            if _r == 2 {
                let back = -(line.len() as libc::c_long);
                if unsafe { libc::fseek(fp, back, libc::SEEK_CUR) } != -1 {
                    return al_data;
                } else {
                    return error_cleanup(al_data, fp);
                }
            }

            // p = str + ALERT_BEGIN_SZ + 1  (skip "** Alert ")
            if line.len() <= ALERT_BEGIN_SZ + 1 {
                continue;
            }
            let p = &line[ALERT_BEGIN_SZ + 1..];

            // m = strstr(p, ":")
            let colon_idx = match memchr_byte(p, b':') {
                Some(i) => i,
                None => continue,
            };

            // alertid = p[0..colon_idx]
            unsafe {
                let aldata = &mut *al_data;
                os_free(&mut aldata.alertid);
                aldata.alertid = os_strdup(&p[..colon_idx]);
            }

            // Search for next space, then advance past it
            let space_idx = match memchr_byte(p, b' ') {
                Some(i) => i,
                None => continue,
            };
            let after_space = &p[space_idx + 1..];

            // Check mail flag
            if (flag & CRALERT_MAIL_SET) != 0
                && (after_space.len() < ALERT_MAIL_SZ
                    || &after_space[..ALERT_MAIL_SZ] != ALERT_MAIL)
            {
                continue;
            }

            // p = strchr(p, '-') (search after_space)
            if let Some(dash_idx) = memchr_byte(after_space, b'-') {
                let mut q = &after_space[dash_idx + 1..];
                // Skip leading spaces
                while let Some(&c) = q.first() {
                    if c == b' ' {
                        q = &q[1..];
                    } else {
                        break;
                    }
                }

                // group = strdup(q); then strip trailing newline
                let group_bytes = clear_nl(q);
                unsafe {
                    let aldata = &mut *al_data;
                    replace_cstr(&mut aldata.group, group_bytes);
                    if !aldata.group.is_null() {
                        // Check for "syscheck" substring
                        if libc::strstr(
                            aldata.group as *const c_char,
                            b"syscheck\0".as_ptr() as *const c_char,
                        )
                        .is_null()
                            == false
                        {
                            issyscheck = 1;
                        }
                    }
                }
            }

            _r = 1;
            continue;
        }

        if _r < 1 {
            continue;
        }

        if _r == 1 {
            // Strip newline
            let trimmed = clear_nl(&line);
            // Find first ':' in str
            let mut date_bytes: Option<&[u8]> = None;
            let mut loc_bytes: Option<&[u8]> = None;

            if let Some(colon_idx) = memchr_byte(trimmed, b':') {
                // From colon onward, find next space
                let after_colon = &trimmed[colon_idx..];
                if let Some(space_off) = memchr_byte(after_colon, b' ') {
                    let split_pos = colon_idx + space_off;
                    // date is trimmed[..split_pos]
                    date_bytes = Some(&trimmed[..split_pos]);
                    loc_bytes = Some(&trimmed[split_pos + 1..]);
                } else {
                    eprintln!("date of location not NULL");
                    return error_cleanup(al_data, fp);
                }
            }

            // The C code does: if (al_data->date || al_data->location || !p) goto l_error.
            // p is null when no ':' was found OR when strchr(p, ' ') returned null.
            // The `!p` branch only triggers when colon was found but space wasn't (above
            // we already exit). When there's no colon at all, p stays NULL from the prior
            // value: actually in C, after the inner if(p) block, p would still be NULL
            // (since strchr returned NULL initially). So we error if no colon.
            unsafe {
                let aldata = &mut *al_data;
                if !aldata.date.is_null() || !aldata.location.is_null() {
                    eprintln!("date or location not NULL or p is NULL");
                    return error_cleanup(al_data, fp);
                }
            }

            let (db, lb) = match (date_bytes, loc_bytes) {
                (Some(d), Some(l)) => (d, l),
                _ => {
                    eprintln!("date or location not NULL or p is NULL");
                    return error_cleanup(al_data, fp);
                }
            };

            unsafe {
                let aldata = &mut *al_data;
                aldata.date = os_strdup(db);
                aldata.location = os_strdup(lb);
            }
            _r = 2;
            log_size = 0;
            continue;
        } else if _r == 2 {
            // Rule begin
            if line.len() >= RULE_BEGIN_SZ && &line[..RULE_BEGIN_SZ] == RULE_BEGIN {
                let trimmed = clear_nl(&line);
                if trimmed.len() < RULE_BEGIN_SZ {
                    return error_cleanup(al_data, fp);
                }
                let p = &trimmed[RULE_BEGIN_SZ..];
                let rule_val = atoi_bytes(p);

                // Skip two spaces
                let after_first_space = match memchr_byte(p, b' ') {
                    Some(i) => &p[i + 1..],
                    None => return error_cleanup(al_data, fp),
                };
                let after_second_space = match memchr_byte(after_first_space, b' ') {
                    Some(i) => &after_first_space[i + 1..],
                    None => return error_cleanup(al_data, fp),
                };

                let level_val = atoi_bytes(after_second_space);

                // Find first quote
                let q_idx = match memchr_byte(after_second_space, b'\'') {
                    Some(i) => i,
                    None => return error_cleanup(al_data, fp),
                };
                let after_quote = &after_second_space[q_idx + 1..];

                // Make a comment string and strip trailing single quote
                let close_idx = match memrchr_byte(after_quote, b'\'') {
                    Some(i) => i,
                    None => return error_cleanup(al_data, fp),
                };
                let comment_bytes = &after_quote[..close_idx];

                unsafe {
                    let aldata = &mut *al_data;
                    aldata.rule = rule_val as c_uint;
                    aldata.level = level_val as c_uint;
                    replace_cstr(&mut aldata.comment, comment_bytes);
                }
            }
            // srcip
            else if line.len() >= SRCIP_BEGIN_SZ && &line[..SRCIP_BEGIN_SZ] == SRCIP_BEGIN {
                let trimmed = clear_nl(&line);
                let p = &trimmed[SRCIP_BEGIN_SZ..];
                unsafe {
                    let aldata = &mut *al_data;
                    replace_cstr(&mut aldata.srcip, p);
                }
            }
            // srcport
            else if line.len() >= SRCPORT_BEGIN_SZ && &line[..SRCPORT_BEGIN_SZ] == SRCPORT_BEGIN
            {
                let trimmed = clear_nl(&line);
                let p = &trimmed[SRCPORT_BEGIN_SZ..];
                unsafe {
                    (*al_data).srcport = atoi_bytes(p);
                }
            }
            // dstip
            else if line.len() >= DSTIP_BEGIN_SZ && &line[..DSTIP_BEGIN_SZ] == DSTIP_BEGIN {
                let trimmed = clear_nl(&line);
                let p = &trimmed[DSTIP_BEGIN_SZ..];
                unsafe {
                    let aldata = &mut *al_data;
                    replace_cstr(&mut aldata.dstip, p);
                }
            }
            // dstport
            else if line.len() >= DSTPORT_BEGIN_SZ && &line[..DSTPORT_BEGIN_SZ] == DSTPORT_BEGIN
            {
                let trimmed = clear_nl(&line);
                let p = &trimmed[DSTPORT_BEGIN_SZ..];
                unsafe {
                    (*al_data).dstport = atoi_bytes(p);
                }
            }
            // user
            else if line.len() >= USER_BEGIN_SZ && &line[..USER_BEGIN_SZ] == USER_BEGIN {
                let trimmed = clear_nl(&line);
                let p = &trimmed[USER_BEGIN_SZ..];
                unsafe {
                    let aldata = &mut *al_data;
                    replace_cstr(&mut aldata.user, p);
                }
            }
            // log message
            else if log_size < LOG_LIMIT {
                let trimmed = clear_nl(&line);
                if issyscheck == 1 {
                    if trimmed.len() >= SYSCHECK_PREFIX_LEN
                        && &trimmed[..SYSCHECK_PREFIX_LEN] == SYSCHECK_PREFIX
                    {
                        let after = &trimmed[SYSCHECK_PREFIX_LEN..];
                        // Drop the trailing single-quote (mirroring `[strlen-1]='\0'`).
                        let body = if !after.is_empty() {
                            &after[..after.len() - 1]
                        } else {
                            after
                        };
                        unsafe {
                            let aldata = &mut *al_data;
                            os_free(&mut aldata.filename);
                            aldata.filename = os_strdup(body);
                        }
                    }
                    issyscheck = 0;
                }
                // Note: log accumulation removed in original C (commented out).
                let _ = log_size; // placeholder, kept for parity
            }
        }
    }

    // We reached end of file
    let at_eof = unsafe { libc::feof(fp) } != 0;
    if at_eof && _r == 2 {
        return al_data;
    }

    error_cleanup(al_data, fp)
}

/// Free `al_data`, clear EOF on `fp`, and return a null pointer.
fn error_cleanup(al_data: *mut alert_data, fp: *mut FILE) -> *mut alert_data {
    unsafe {
        FreeAlertData(al_data);
        libc::clearerr(fp);
    }
    ptr::null_mut()
}

// Suppress unused-import warning for CString in some toolchains.
#[allow(dead_code)]
fn _unused() -> CString {
    CString::new("").unwrap()
}
