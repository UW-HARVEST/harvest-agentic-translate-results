//! Translation of `c_src/src/read-alert.c` and `c_src/include/read-alert.h`.

use core::ffi::{c_char, c_int, c_long, c_uint, c_void, CStr};

use crate::shared::{os_calloc, os_clearnl, os_free, os_realloc, os_strdup};

pub const _ALERTS_DAILY: &CStr = c"alerts.log";

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const _CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const _CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

/// `typedef struct alert_data { ... } alert_data;`
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

/* ** Alert xyz: email active-response ** */

// Layout of `alert_data` must match the C struct exactly (x86_64 Linux values).
const _: () = {
    assert!(core::mem::size_of::<alert_data>() == 96);
    assert!(core::mem::offset_of!(alert_data, alertid) == 8);
    assert!(core::mem::offset_of!(alert_data, srcport) == 56);
    assert!(core::mem::offset_of!(alert_data, dstip) == 64);
    assert!(core::mem::offset_of!(alert_data, filename) == 88);
};

const ALERT_BEGIN: &CStr = c"** Alert";
const ALERT_BEGIN_SZ: usize = 8;
const RULE_BEGIN: &CStr = c"Rule: ";
const RULE_BEGIN_SZ: usize = 6;
const SRCIP_BEGIN: &CStr = c"Src IP: ";
const SRCIP_BEGIN_SZ: usize = 8;
const SRCPORT_BEGIN: &CStr = c"Src Port: ";
const SRCPORT_BEGIN_SZ: usize = 10;
const DSTIP_BEGIN: &CStr = c"Dst IP: ";
const DSTIP_BEGIN_SZ: usize = 8;
const DSTPORT_BEGIN: &CStr = c"Dst Port: ";
const DSTPORT_BEGIN_SZ: usize = 10;
const USER_BEGIN: &CStr = c"User: ";
const USER_BEGIN_SZ: usize = 6;
const ALERT_MAIL: &CStr = c"mail";
const ALERT_MAIL_SZ: usize = 4;

const LOG_LIMIT: usize = 100;

const OS_MAXSTR: usize = 1024;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    // char **p; (unused in the original)
    os_free(&mut (*al_data).alertid);
    os_free(&mut (*al_data).date);
    os_free(&mut (*al_data).location);
    os_free(&mut (*al_data).comment);
    os_free(&mut (*al_data).group);
    os_free(&mut (*al_data).srcip);
    os_free(&mut (*al_data).dstip);
    os_free(&mut (*al_data).user);
    os_free(&mut (*al_data).filename);

    /* "9/19/2016 - Sivakumar Nellurandi - parsing additions" */
    // the al_data->log handling is commented out in the original

    // al_data can't be NULL
    libc::free(al_data as *mut c_void);
    // al_data = NULL; (no effect, by-value parameter)
}

/// Return alert data for the file specified
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut libc::FILE) -> *mut alert_data {
    let al_data = os_calloc(1, core::mem::size_of::<alert_data>()) as *mut alert_data;

    if parse_alert(flag, fp, al_data) {
        return al_data;
    }

    // l_error:
    /* Free the memory */
    FreeAlertData(al_data);
    /* We need to clean end of file before returning */
    libc::clearerr(fp);
    core::ptr::null_mut()
}

/// Body of `GetAlertData`.
///
/// Returns `true` where the original returns `al_data`, and `false` where the
/// original jumps to (or falls through to) the `l_error` label.
unsafe fn parse_alert(flag: c_int, fp: *mut libc::FILE, al_data: *mut alert_data) -> bool {
    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;
    let mut p: *mut c_char;

    let mut buf = [0 as c_char; OS_MAXSTR + 1];
    let str_: *mut c_char = buf.as_mut_ptr();
    *str_.add(OS_MAXSTR) = 0;

    while !libc::fgets(str_, OS_MAXSTR as c_int, fp).is_null() {
        /* End of alert */
        if libc::strncmp(ALERT_BEGIN.as_ptr(), str_, ALERT_BEGIN_SZ) == 0 {
            let m: *mut c_char;
            let z: usize;
            /* End of the alert. */
            if _r == 2 {
                if libc::fseek(fp, -(libc::strlen(str_) as c_long), libc::SEEK_CUR) != -1 {
                    return true;
                } else {
                    return false;
                }
            }

            p = str_.add(ALERT_BEGIN_SZ + 1);

            m = libc::strstr(p, c":".as_ptr());
            if m.is_null() {
                continue;
            }

            z = libc::strlen(p) - libc::strlen(m);
            (*al_data).alertid = os_realloc(
                (*al_data).alertid as *mut c_void,
                (z + 1) * core::mem::size_of::<c_char>(),
            ) as *mut c_char;
            libc::strncpy((*al_data).alertid, p, z);
            *(*al_data).alertid.add(z) = 0;

            /* Search for email flag */
            p = libc::strchr(p, ' ' as c_int);
            if p.is_null() {
                continue;
            }

            p = p.add(1);

            /* Check for the flags */
            if (flag & CRALERT_MAIL_SET) != 0
                && libc::strncmp(ALERT_MAIL.as_ptr(), p, ALERT_MAIL_SZ) != 0
            {
                continue;
            }

            p = libc::strchr(p, '-' as c_int);
            if !p.is_null() {
                p = p.add(1);
                /* Skip leading spaces */
                while *p == b' ' as c_char {
                    p = p.add(1);
                }
                os_free(&mut (*al_data).group);
                (*al_data).group = os_strdup(p);

                /* Clean newline from group */
                os_clearnl((*al_data).group);
                if !(*al_data).group.is_null()
                    && !libc::strstr((*al_data).group, c"syscheck".as_ptr()).is_null()
                {
                    issyscheck = 1;
                }
            }

            /* Search for active-response flag */
            _r = 1;
            continue;
        }

        if _r < 1 {
            continue;
        }

        /*** Extract information from the event ***/

        /* r1 means: 2006 Apr 13 16:15:17 /var/log/auth.log */
        if _r == 1 {
            /* Clear newline */
            os_clearnl(str_);

            p = libc::strchr(str_, ':' as c_int);
            if !p.is_null() {
                p = libc::strchr(p, ' ' as c_int);
                if !p.is_null() {
                    *p = 0;
                    p = p.add(1);
                } else {
                    /* If p is null it is because strchr failed */
                    libc::perror(c"date of location not NULL".as_ptr());
                    return false;
                }
            }

            /* If not, str is date and p is the location */
            if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                libc::perror(c"date or location not NULL or p is NULL".as_ptr());
                return false;
            }

            (*al_data).date = os_strdup(str_);
            (*al_data).location = os_strdup(p);
            _r = 2;
            log_size = 0;
            continue;
        } else if _r == 2 {
            /* Rule begin */
            if libc::strncmp(RULE_BEGIN.as_ptr(), str_, RULE_BEGIN_SZ) == 0 {
                os_clearnl(str_);

                p = str_.add(RULE_BEGIN_SZ);
                (*al_data).rule = libc::atoi(p) as c_uint;

                p = libc::strchr(p, ' ' as c_int);
                if !p.is_null() {
                    p = p.add(1);
                    p = libc::strchr(p, ' ' as c_int);
                    if !p.is_null() {
                        p = p.add(1);
                    }
                }

                if p.is_null() {
                    return false;
                }

                (*al_data).level = libc::atoi(p) as c_uint;

                /* Get the comment */
                p = libc::strchr(p, '\'' as c_int);
                if p.is_null() {
                    return false;
                }

                p = p.add(1);
                os_free(&mut (*al_data).comment);
                (*al_data).comment = os_strdup(p);

                /* Must have the closing \' */
                p = libc::strrchr((*al_data).comment, '\'' as c_int);
                if !p.is_null() {
                    *p = 0;
                } else {
                    return false;
                }
            }
            /* srcip */
            else if libc::strncmp(SRCIP_BEGIN.as_ptr(), str_, SRCIP_BEGIN_SZ) == 0 {
                os_clearnl(str_);

                p = str_.add(SRCIP_BEGIN_SZ);
                os_free(&mut (*al_data).srcip);
                (*al_data).srcip = os_strdup(p);
            }
            /* srcport */
            else if libc::strncmp(SRCPORT_BEGIN.as_ptr(), str_, SRCPORT_BEGIN_SZ) == 0 {
                os_clearnl(str_);

                p = str_.add(SRCPORT_BEGIN_SZ);
                (*al_data).srcport = libc::atoi(p);
            }
            /* dstip */
            else if libc::strncmp(DSTIP_BEGIN.as_ptr(), str_, DSTIP_BEGIN_SZ) == 0 {
                os_clearnl(str_);

                p = str_.add(DSTIP_BEGIN_SZ);
                os_free(&mut (*al_data).dstip);
                (*al_data).dstip = os_strdup(p);
            }
            /* dstport */
            else if libc::strncmp(DSTPORT_BEGIN.as_ptr(), str_, DSTPORT_BEGIN_SZ) == 0 {
                os_clearnl(str_);

                p = str_.add(DSTPORT_BEGIN_SZ);
                (*al_data).dstport = libc::atoi(p);
            }
            /* username */
            else if libc::strncmp(USER_BEGIN.as_ptr(), str_, USER_BEGIN_SZ) == 0 {
                os_clearnl(str_);

                p = str_.add(USER_BEGIN_SZ);
                os_free(&mut (*al_data).user);
                (*al_data).user = os_strdup(p);
            }
            /* "9/19/2016 - Sivakumar Nellurandi - parsing additions" */
            /* It is a log message */
            else if log_size < LOG_LIMIT {
                os_clearnl(str_);
                if issyscheck == 1 {
                    if libc::strncmp(
                        str_,
                        c"Integrity checksum changed for: '".as_ptr(),
                        33,
                    ) == 0
                    {
                        (*al_data).filename = libc::strdup(str_.add(33));
                        if !(*al_data).filename.is_null() {
                            // Reproduced verbatim, including the underflow when
                            // the duplicated string is empty.
                            let len = libc::strlen((*al_data).filename);
                            *(*al_data)
                                .filename
                                .wrapping_offset(len.wrapping_sub(1) as isize) = 0;
                        }
                    }
                    issyscheck = 0;
                }

                // the al_data->log bookkeeping is commented out in the original
            }
        }
    }

    // We reached the end of the alert and the information is saved.
    libc::feof(fp) != 0 && _r == 2
}
