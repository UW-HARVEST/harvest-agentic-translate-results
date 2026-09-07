//! Translation of `c_src/src/read-alert.c` + `c_src/include/read-alert.h`.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

use crate::cbind::*;
use crate::shared::{os_calloc, os_realloc, os_strdup};

/// `#define OS_MAXSTR 1024` (`shared.h`)
const OS_MAXSTR: usize = 1024;

/* ** Alert xyz: email active-response ** */

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

const LOG_LIMIT: usize = 100;

/// `#define CRALERT_MAIL_SET 0x001`
pub const CRALERT_MAIL_SET: c_int = 0x001;
/// `#define CRALERT_EXEC_SET 0x002`
pub const CRALERT_EXEC_SET: c_int = 0x002;
/// `#define CRALERT_READ_ALL 0x004`
pub const CRALERT_READ_ALL: c_int = 0x004;
/// `#define CRALERT_READ_FAILED 0x008`
pub const CRALERT_READ_FAILED: c_int = 0x008;
/// `#define CRALERT_FP_SET 0x010`
pub const CRALERT_FP_SET: c_int = 0x010;

/// `#define ALERTS_DAILY "alerts.log"`
pub const ALERTS_DAILY: &[u8] = b"alerts.log\0";

/// `typedef struct alert_data { ... } alert_data;`
///
/// 96 bytes on x86_64 linux-gnu.
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

/// `void FreeAlertData(alert_data *al_data)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    unsafe {
        os_free(&raw mut (*al_data).alertid);
        os_free(&raw mut (*al_data).date);
        os_free(&raw mut (*al_data).location);
        os_free(&raw mut (*al_data).comment);
        os_free(&raw mut (*al_data).group);
        os_free(&raw mut (*al_data).srcip);
        os_free(&raw mut (*al_data).dstip);
        os_free(&raw mut (*al_data).user);
        os_free(&raw mut (*al_data).filename);

        /* "9/19/2016 - Sivakumar Nellurandi - parsing additions" */
        // The al_data->log handling is commented out in the original C.

        // al_data can't be NULL
        free(al_data as *mut c_void);
    }
}

/// `alert_data *GetAlertData(int flag, FILE *fp)`
///
/// Returns alert data for the file specified.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    unsafe {
        let al_data = os_calloc(1, size_of::<alert_data>()) as *mut alert_data;

        let mut _r: c_int = 0;
        let mut issyscheck: c_int = 0;
        let mut log_size: usize = 0;
        let mut p: *mut c_char;
        let mut buf = [0u8; OS_MAXSTR + 1];
        let str_: *mut c_char = buf.as_mut_ptr() as *mut c_char;
        *str_.add(OS_MAXSTR) = 0;

        'l_error: {
            while !fgets(str_, OS_MAXSTR as c_int, fp).is_null() {
                /* End of alert */
                if strncmp(ALERT_BEGIN.as_ptr() as *const c_char, str_, ALERT_BEGIN_SZ) == 0 {
                    let m: *mut c_char;
                    let z: usize;

                    /* End of the alert. */
                    if _r == 2 {
                        if fseek(fp, (strlen(str_) as i64).wrapping_neg(), SEEK_CUR) != -1 {
                            return al_data;
                        } else {
                            break 'l_error;
                        }
                    }

                    p = str_.add(ALERT_BEGIN_SZ + 1);

                    m = strstr(p, b":\0".as_ptr() as *const c_char);
                    if m.is_null() {
                        continue;
                    }

                    z = strlen(p) - strlen(m);
                    (*al_data).alertid = os_realloc(
                        (*al_data).alertid as *mut c_void,
                        (z + 1) * size_of::<c_char>(),
                    ) as *mut c_char;
                    strncpy((*al_data).alertid, p, z);
                    *(*al_data).alertid.add(z) = 0;

                    /* Search for email flag */
                    p = strchr(p, b' ' as c_int);
                    if p.is_null() {
                        continue;
                    }

                    p = p.add(1);

                    /* Check for the flags */
                    if (flag & CRALERT_MAIL_SET) != 0
                        && strncmp(ALERT_MAIL.as_ptr() as *const c_char, p, ALERT_MAIL_SZ) != 0
                    {
                        continue;
                    }

                    p = strchr(p, b'-' as c_int);
                    if !p.is_null() {
                        p = p.add(1);
                        /* Skip leading spaces */
                        while *p == b' ' as c_char {
                            p = p.add(1);
                        }
                        os_free(&raw mut (*al_data).group);
                        (*al_data).group = os_strdup(p);

                        /* Clean newline from group */
                        p = os_clearnl((*al_data).group);
                        let _ = p;
                        if !(*al_data).group.is_null()
                            && !strstr((*al_data).group, b"syscheck\0".as_ptr() as *const c_char)
                                .is_null()
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

                    p = strchr(str_, b':' as c_int);
                    if !p.is_null() {
                        p = strchr(p, b' ' as c_int);
                        if !p.is_null() {
                            *p = 0;
                            p = p.add(1);
                        } else {
                            /* If p is null it is because strchr failed */
                            perror(b"date of location not NULL\0".as_ptr() as *const c_char);
                            break 'l_error;
                        }
                    }

                    /* If not, str is date and p is the location */
                    if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                        perror(
                            b"date or location not NULL or p is NULL\0".as_ptr() as *const c_char
                        );
                        break 'l_error;
                    }

                    (*al_data).date = os_strdup(str_);
                    (*al_data).location = os_strdup(p);
                    _r = 2;
                    log_size = 0;
                    continue;
                } else if _r == 2 {
                    /* Rule begin */
                    if strncmp(RULE_BEGIN.as_ptr() as *const c_char, str_, RULE_BEGIN_SZ) == 0 {
                        os_clearnl(str_);

                        p = str_.add(RULE_BEGIN_SZ);
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
                            break 'l_error;
                        }

                        (*al_data).level = atoi(p) as c_uint;

                        /* Get the comment */
                        p = strchr(p, b'\'' as c_int);
                        if p.is_null() {
                            break 'l_error;
                        }

                        p = p.add(1);
                        os_free(&raw mut (*al_data).comment);
                        (*al_data).comment = os_strdup(p);

                        /* Must have the closing \' */
                        p = strrchr((*al_data).comment, b'\'' as c_int);
                        if !p.is_null() {
                            *p = 0;
                        } else {
                            break 'l_error;
                        }
                    }
                    /* srcip */
                    else if strncmp(SRCIP_BEGIN.as_ptr() as *const c_char, str_, SRCIP_BEGIN_SZ)
                        == 0
                    {
                        os_clearnl(str_);

                        p = str_.add(SRCIP_BEGIN_SZ);
                        os_free(&raw mut (*al_data).srcip);
                        (*al_data).srcip = os_strdup(p);
                    }
                    /* srcport */
                    else if strncmp(
                        SRCPORT_BEGIN.as_ptr() as *const c_char,
                        str_,
                        SRCPORT_BEGIN_SZ,
                    ) == 0
                    {
                        os_clearnl(str_);

                        p = str_.add(SRCPORT_BEGIN_SZ);
                        (*al_data).srcport = atoi(p);
                    }
                    /* dstip */
                    else if strncmp(DSTIP_BEGIN.as_ptr() as *const c_char, str_, DSTIP_BEGIN_SZ)
                        == 0
                    {
                        os_clearnl(str_);

                        p = str_.add(DSTIP_BEGIN_SZ);
                        os_free(&raw mut (*al_data).dstip);
                        (*al_data).dstip = os_strdup(p);
                    }
                    /* dstport */
                    else if strncmp(
                        DSTPORT_BEGIN.as_ptr() as *const c_char,
                        str_,
                        DSTPORT_BEGIN_SZ,
                    ) == 0
                    {
                        os_clearnl(str_);

                        p = str_.add(DSTPORT_BEGIN_SZ);
                        (*al_data).dstport = atoi(p);
                    }
                    /* username */
                    else if strncmp(USER_BEGIN.as_ptr() as *const c_char, str_, USER_BEGIN_SZ) == 0
                    {
                        os_clearnl(str_);

                        p = str_.add(USER_BEGIN_SZ);
                        os_free(&raw mut (*al_data).user);
                        (*al_data).user = os_strdup(p);
                    }
                    /* "9/19/2016 - Sivakumar Nellurandi - parsing additions" */
                    /* It is a log message */
                    else if log_size < LOG_LIMIT {
                        os_clearnl(str_);
                        if issyscheck == 1 {
                            if strncmp(
                                str_,
                                b"Integrity checksum changed for: '\0".as_ptr() as *const c_char,
                                33,
                            ) == 0
                            {
                                (*al_data).filename = strdup(str_.add(33));
                                if !(*al_data).filename.is_null() {
                                    let f = (*al_data).filename;
                                    *f.wrapping_offset(strlen(f) as isize - 1) = 0;
                                }
                            }
                            issyscheck = 0;
                        }

                        // The al_data->log accumulation is commented out in the
                        // original C, so log_size is never incremented.
                        let _ = &mut log_size;
                    }
                }
            }

            // We reached the end of the alert and the information is saved.
            if feof(fp) != 0 && _r == 2 {
                return al_data;
            }
        }

        // l_error:
        /* Free the memory */
        FreeAlertData(al_data);
        /* We need to clean end of file before returning */
        clearerr(fp);
        ptr::null_mut()
    }
}
