//! Translation of `src/read-alert.c`.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

use crate::shared::{os_calloc, os_realloc, os_strdup, OS_MAXSTR};

// ---- Public C-visible types -----------------------------------------------

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

// ---- Flags (mirrored from include/read-alert.h) ---------------------------

pub const CRALERT_MAIL_SET: c_int = 0x001;
#[allow(dead_code)]
pub const CRALERT_EXEC_SET: c_int = 0x002;
#[allow(dead_code)]
pub const CRALERT_READ_ALL: c_int = 0x004;
#[allow(dead_code)]
pub const CRALERT_READ_FAILED: c_int = 0x008;
#[allow(dead_code)]
pub const CRALERT_FP_SET: c_int = 0x010;

// ---- libc imports we need -------------------------------------------------

extern "C" {
    fn fgets(s: *mut c_char, n: c_int, stream: *mut libc::FILE) -> *mut c_char;
    fn fseek(stream: *mut libc::FILE, offset: libc::c_long, whence: c_int) -> c_int;
    fn feof(stream: *mut libc::FILE) -> c_int;
    fn clearerr(stream: *mut libc::FILE);
    fn perror(s: *const c_char);
    fn strncmp(s1: *const c_char, s2: *const c_char, n: libc::size_t) -> c_int;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strlen(s: *const c_char) -> libc::size_t;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: libc::size_t) -> *mut c_char;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn free(ptr: *mut c_void);
}

const SEEK_CUR: c_int = 1;

// String prefix constants (must end with NUL terminator for strstr/strncmp).
const ALERT_BEGIN: &[u8] = b"** Alert\0";
const ALERT_BEGIN_SZ: usize = 8;
const RULE_BEGIN: &[u8] = b"Rule: \0";
const RULE_BEGIN_SZ: usize = 6;
const SRCIP_BEGIN: &[u8] = b"Src IP: \0";
const SRCIP_BEGIN_SZ: usize = 8;
const SRCPORT_BEGIN: &[u8] = b"Src Port: \0";
const SRCPORT_BEGIN_SZ: usize = 10;
const DSTIP_BEGIN: &[u8] = b"Dst IP: \0";
const DSTIP_BEGIN_SZ: usize = 8;
const DSTPORT_BEGIN: &[u8] = b"Dst Port: \0";
const DSTPORT_BEGIN_SZ: usize = 10;
const USER_BEGIN: &[u8] = b"User: \0";
const USER_BEGIN_SZ: usize = 6;
const ALERT_MAIL: &[u8] = b"mail\0";
const ALERT_MAIL_SZ: usize = 4;
const SYSCHECK_PREFIX: &[u8] = b"Integrity checksum changed for: '\0";
const SYSCHECK_PREFIX_SZ: usize = 33;
const SYSCHECK_NEEDLE: &[u8] = b"syscheck\0";

const LOG_LIMIT: usize = 100;

// Helper: clear newline by replacing the last '\n' (if any) with '\0'.
unsafe fn os_clearnl(s: *mut c_char) {
    let p = strrchr(s, b'\n' as c_int);
    if !p.is_null() {
        *p = 0;
    }
}

/// Free the alert data structure (mirror of `FreeAlertData`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    // C version dereferences without checking; preserve that behavior.
    let d = &mut *al_data;

    macro_rules! os_free_field {
        ($field:expr) => {{
            if !$field.is_null() {
                free($field as *mut c_void);
                $field = std::ptr::null_mut();
            }
        }};
    }

    os_free_field!(d.alertid);
    os_free_field!(d.date);
    os_free_field!(d.location);
    os_free_field!(d.comment);
    os_free_field!(d.group);
    os_free_field!(d.srcip);
    os_free_field!(d.dstip);
    os_free_field!(d.user);
    os_free_field!(d.filename);

    // The "log" code is commented out in the C source; we mirror that.

    // al_data can't be NULL (per C comment).
    free(al_data as *mut c_void);
    // The C does `al_data = NULL;` after free — that's a no-op since al_data
    // is a local copy of the parameter. We replicate by doing nothing.
}

// Convenience: free a pointer field (mirrors `os_free(x)`).
unsafe fn os_free_ptr(field: &mut *mut c_char) {
    if !field.is_null() {
        free(*field as *mut c_void);
        *field = std::ptr::null_mut();
    }
}

/// Read one alert record from a file. Mirror of `GetAlertData`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut libc::FILE) -> *mut alert_data {
    // Allocate zero-initialized alert_data via os_calloc, exactly as C does.
    let al_data = os_calloc(1, std::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: libc::size_t = 0;
    let mut p: *mut c_char;

    // str[OS_MAXSTR + 1]; str[OS_MAXSTR] = '\0';
    let mut buf: [c_char; OS_MAXSTR + 1] = [0; OS_MAXSTR + 1];
    buf[OS_MAXSTR] = 0;
    let str_ptr = buf.as_mut_ptr();

    'outer: loop {
        // Read up to OS_MAXSTR bytes (matching C: fgets(str, OS_MAXSTR, fp)).
        let r = fgets(str_ptr, OS_MAXSTR as c_int, fp);
        if r.is_null() {
            break;
        }

        // End of alert / new alert begins
        if strncmp(ALERT_BEGIN.as_ptr() as *const c_char, str_ptr, ALERT_BEGIN_SZ) == 0 {
            // End of the alert — we already collected one above.
            if _r == 2 {
                let len = strlen(str_ptr) as libc::c_long;
                if fseek(fp, -len, SEEK_CUR) != -1 {
                    return al_data;
                } else {
                    // l_error
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return std::ptr::null_mut();
                }
            }

            // p = str + ALERT_BEGIN_SZ + 1;
            p = str_ptr.add(ALERT_BEGIN_SZ + 1);

            // m = strstr(p, ":");
            let colon = b":\0";
            let m = strstr(p, colon.as_ptr() as *const c_char);
            if m.is_null() {
                continue;
            }

            // z = strlen(p) - strlen(m);
            let z = strlen(p) - strlen(m);
            (*al_data).alertid = os_realloc(
                (*al_data).alertid as *mut c_void,
                (z + 1) * std::mem::size_of::<c_char>(),
            ) as *mut c_char;
            strncpy((*al_data).alertid, p, z);
            *(*al_data).alertid.add(z) = 0;

            // Search for email flag
            p = strchr(p, b' ' as c_int);
            if p.is_null() {
                continue;
            }
            p = p.add(1);

            // Check for the mail flag
            if (flag & CRALERT_MAIL_SET) != 0
                && strncmp(ALERT_MAIL.as_ptr() as *const c_char, p, ALERT_MAIL_SZ) != 0
            {
                continue;
            }

            p = strchr(p, b'-' as c_int);
            if !p.is_null() {
                p = p.add(1);
                // Skip leading spaces
                while *p == b' ' as c_char {
                    p = p.add(1);
                }
                os_free_ptr(&mut (*al_data).group);
                (*al_data).group = os_strdup(p);

                // Clean newline from group
                os_clearnl((*al_data).group);
                if !(*al_data).group.is_null()
                    && !strstr((*al_data).group, SYSCHECK_NEEDLE.as_ptr() as *const c_char)
                        .is_null()
                {
                    issyscheck = 1;
                }
            }

            // Search for active-response flag
            _r = 1;
            continue;
        }

        if _r < 1 {
            continue;
        }

        /* Extract information from the event */

        if _r == 1 {
            os_clearnl(str_ptr);

            p = strchr(str_ptr, b':' as c_int);
            if !p.is_null() {
                p = strchr(p, b' ' as c_int);
                if !p.is_null() {
                    *p = 0;
                    p = p.add(1);
                } else {
                    perror(b"date of location not NULL\0".as_ptr() as *const c_char);
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return std::ptr::null_mut();
                }
            }

            // If not, str is date and p is the location
            if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                perror(b"date or location not NULL or p is NULL\0".as_ptr() as *const c_char);
                FreeAlertData(al_data);
                clearerr(fp);
                return std::ptr::null_mut();
            }

            (*al_data).date = os_strdup(str_ptr);
            (*al_data).location = os_strdup(p);
            _r = 2;
            log_size = 0;
            continue;
        } else if _r == 2 {
            // Rule begin
            if strncmp(RULE_BEGIN.as_ptr() as *const c_char, str_ptr, RULE_BEGIN_SZ) == 0 {
                os_clearnl(str_ptr);

                p = str_ptr.add(RULE_BEGIN_SZ);
                (*al_data).rule = atoi(p) as c_uint;

                p = strchr(p, b' ' as c_int);
                if !p.is_null() {
                    p = p.add(1);
                    p = strchr(p, b' ' as c_int);
                    if !p.is_null() {
                        p = p.add(1);
                    }
                }

                if p.is_null() {
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return std::ptr::null_mut();
                }

                (*al_data).level = atoi(p) as c_uint;

                // Get the comment
                p = strchr(p, b'\'' as c_int);
                if p.is_null() {
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return std::ptr::null_mut();
                }

                p = p.add(1);
                os_free_ptr(&mut (*al_data).comment);
                (*al_data).comment = os_strdup(p);

                // Must have closing single-quote
                p = strrchr((*al_data).comment, b'\'' as c_int);
                if !p.is_null() {
                    *p = 0;
                } else {
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return std::ptr::null_mut();
                }
            }
            // srcip
            else if strncmp(SRCIP_BEGIN.as_ptr() as *const c_char, str_ptr, SRCIP_BEGIN_SZ) == 0 {
                os_clearnl(str_ptr);

                p = str_ptr.add(SRCIP_BEGIN_SZ);
                os_free_ptr(&mut (*al_data).srcip);
                (*al_data).srcip = os_strdup(p);
            }
            // srcport
            else if strncmp(
                SRCPORT_BEGIN.as_ptr() as *const c_char,
                str_ptr,
                SRCPORT_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_ptr);

                p = str_ptr.add(SRCPORT_BEGIN_SZ);
                (*al_data).srcport = atoi(p);
            }
            // dstip
            else if strncmp(DSTIP_BEGIN.as_ptr() as *const c_char, str_ptr, DSTIP_BEGIN_SZ) == 0
            {
                os_clearnl(str_ptr);

                p = str_ptr.add(DSTIP_BEGIN_SZ);
                os_free_ptr(&mut (*al_data).dstip);
                (*al_data).dstip = os_strdup(p);
            }
            // dstport
            else if strncmp(
                DSTPORT_BEGIN.as_ptr() as *const c_char,
                str_ptr,
                DSTPORT_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_ptr);

                p = str_ptr.add(DSTPORT_BEGIN_SZ);
                (*al_data).dstport = atoi(p);
            }
            // username
            else if strncmp(USER_BEGIN.as_ptr() as *const c_char, str_ptr, USER_BEGIN_SZ) == 0 {
                os_clearnl(str_ptr);

                p = str_ptr.add(USER_BEGIN_SZ);
                os_free_ptr(&mut (*al_data).user);
                (*al_data).user = os_strdup(p);
            }
            // It is a log message (the body of `else if (log_size < LOG_LIMIT)`)
            else if log_size < LOG_LIMIT {
                os_clearnl(str_ptr);
                if issyscheck == 1 {
                    if strncmp(
                        str_ptr,
                        SYSCHECK_PREFIX.as_ptr() as *const c_char,
                        SYSCHECK_PREFIX_SZ,
                    ) == 0
                    {
                        (*al_data).filename = strdup(str_ptr.add(SYSCHECK_PREFIX_SZ));
                        if !(*al_data).filename.is_null() {
                            let l = strlen((*al_data).filename);
                            if l > 0 {
                                *(*al_data).filename.add(l - 1) = 0;
                            } else {
                                // Mirror the C bug exactly: the C version does
                                // `al_data->filename[strlen - 1] = '\0';`
                                // unconditionally. To match byte-identical
                                // output we need to do the same. Skipping
                                // this branch keeps memory safety; in
                                // practice the filename is always non-empty.
                            }
                        }
                    }
                    issyscheck = 0;
                }

                // log array storage is commented out in C; mirror that.
                // log_size is never incremented in the C source as written.
                let _ = &mut log_size; // silence unused-mut warning
            }

            // Continue outer loop for next line
            continue 'outer;
        }
    }

    // We reached the end of the alert and the information is saved.
    if feof(fp) != 0 && _r == 2 {
        return al_data;
    }

    // l_error
    FreeAlertData(al_data);
    clearerr(fp);
    std::ptr::null_mut()
}
