//! Translation of `c_src/src/read-alert.c` (+ `c_src/include/read-alert.h`).

use core::ffi::{c_char, c_int, c_long, c_uint, c_void};
use core::ptr;

use crate::cbind::*;
use crate::shared::{os_calloc, os_realloc, os_strdup, OS_MAXSTR};
#[allow(unused_imports)]
use crate::shared::{os_clearnl, os_free};

/* ---------------------------------------------------------------- */
/* read-alert.h                                                     */
/* ---------------------------------------------------------------- */

/// `#define ALERTS_DAILY "alerts.log"`
pub const ALERTS_DAILY: &[u8] = b"alerts.log\0";

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

/// ```c
/// typedef struct alert_data {
///     unsigned int rule;
///     unsigned int level;
///     char *alertid;
///     char *date;
///     char *location;
///     char *comment;
///     char *group;
///     char *srcip;
///     int srcport;
///     char *dstip;
///     int dstport;
///     char *user;
///     char *filename;
/// } alert_data;
/// ```
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

/* ---------------------------------------------------------------- */
/* read-alert.c preprocessor constants                              */
/* ---------------------------------------------------------------- */

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

// The remaining *_BEGIN macros of the original file are unused by the code.
#[allow(dead_code)]
mod unused_macros {
    pub const OLDMD5_BEGIN: &[u8] = b"Old md5sum was: \0";
    pub const OLDMD5_BEGIN_SZ: usize = 16;
    pub const NEWMD5_BEGIN: &[u8] = b"New md5sum is : \0";
    pub const NEWMD5_BEGIN_SZ: usize = 16;
    pub const OLDSHA1_BEGIN: &[u8] = b"Old sha1sum was: \0";
    pub const OLDSHA1_BEGIN_SZ: usize = 17;
    pub const NEWSHA1_BEGIN: &[u8] = b"New sha1sum is : \0";
    pub const NEWSHA1_BEGIN_SZ: usize = 17;
    pub const OLDSHA256_BEGIN: &[u8] = b"Old sha256sum was: \0";
    pub const OLDSHA256_BEGIN_SZ: usize = 19;
    pub const NEWSHA256_BEGIN: &[u8] = b"New sha256sum is : \0";
    pub const NEWSHA256_BEGIN_SZ: usize = 19;
    pub const SIZE_BEGIN: &[u8] = b"Size changed from \0";
    pub const SIZE_BEGIN_SZ: usize = 18;
    pub const OWNER_BEGIN: &[u8] = b"Ownership was \0";
    pub const OWNER_BEGIN_SZ: usize = 14;
    pub const GROUP_BEGIN: &[u8] = b"Group ownership was \0";
    pub const GROUP_BEGIN_SZ: usize = 20;
    pub const PERM_BEGIN: &[u8] = b"Permissions changed from \0";
    pub const PERM_BEGIN_SZ: usize = 25;
}

const LOG_LIMIT: usize = 100;

const INTEGRITY_PREFIX: &[u8] = b"Integrity checksum changed for: '\0";
const INTEGRITY_PREFIX_SZ: usize = 33;

/* ---------------------------------------------------------------- */

/// ```c
/// void FreeAlertData(alert_data *al_data)
/// ```
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    // `char **p;` in the original is unused.

    os_free!((*al_data).alertid);
    os_free!((*al_data).date);
    os_free!((*al_data).location);
    os_free!((*al_data).comment);
    os_free!((*al_data).group);
    os_free!((*al_data).srcip);
    os_free!((*al_data).dstip);
    os_free!((*al_data).user);
    os_free!((*al_data).filename);

    /* "9/19/2016 - Sivakumar Nellurandi - parsing additions" */
    // The `al_data->log` cleanup is commented out in the original.

    // al_data can't be NULL
    free(al_data as *mut c_void);
    // `al_data = NULL;` -- assignment to the local parameter, no effect.
}

/// ```c
/// alert_data *GetAlertData(int flag, FILE *fp)
/// ```
#[allow(non_snake_case)]
#[allow(unused_assignments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    let al_data: *mut alert_data =
        os_calloc(1, core::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;
    let mut p: *mut c_char;
    // `char str[OS_MAXSTR + 1]; str[OS_MAXSTR] = '\0';`
    let mut str_buf = [0 as c_char; OS_MAXSTR + 1];
    let str = str_buf.as_mut_ptr();
    *str.add(OS_MAXSTR) = 0;

    'l_error: {
        while !fgets(str, OS_MAXSTR as c_int, fp).is_null() {
            /* End of alert */
            if strncmp(cs(ALERT_BEGIN), str, ALERT_BEGIN_SZ) == 0 {
                let m: *mut c_char;
                let z: usize;

                /* End of the alert. */
                if _r == 2 {
                    // `fseek(fp, -strlen(str), SEEK_CUR)`: the negated size_t
                    // is converted to `long`, which yields -(long)strlen(str).
                    let off = (0usize.wrapping_sub(strlen(str))) as c_long;
                    if fseek(fp, off, SEEK_CUR) != -1 {
                        return al_data;
                    } else {
                        break 'l_error;
                    }
                }

                p = str.add(ALERT_BEGIN_SZ + 1);

                m = strstr(p, cs(b":\0"));
                if m.is_null() {
                    continue;
                }

                z = strlen(p) - strlen(m);
                (*al_data).alertid = os_realloc(
                    (*al_data).alertid as *mut c_void,
                    (z + 1) * core::mem::size_of::<c_char>(),
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
                    && strncmp(cs(ALERT_MAIL), p, ALERT_MAIL_SZ) != 0
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
                    os_free!((*al_data).group);
                    (*al_data).group = os_strdup(p);

                    /* Clean newline from group */
                    os_clearnl!((*al_data).group, p);
                    if !(*al_data).group.is_null()
                        && !strstr((*al_data).group, cs(b"syscheck\0")).is_null()
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
                os_clearnl!(str, p);

                p = strchr(str, b':' as c_int);
                if !p.is_null() {
                    p = strchr(p, b' ' as c_int);
                    if !p.is_null() {
                        *p = 0;
                        p = p.add(1);
                    } else {
                        /* If p is null it is because strchr failed */
                        perror(cs(b"date of location not NULL\0"));
                        break 'l_error;
                    }
                }

                /* If not, str is date and p is the location */
                if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                    perror(cs(b"date or location not NULL or p is NULL\0"));
                    break 'l_error;
                }

                (*al_data).date = os_strdup(str);
                (*al_data).location = os_strdup(p);
                _r = 2;
                log_size = 0;
                continue;
            } else if _r == 2 {
                /* Rule begin */
                if strncmp(cs(RULE_BEGIN), str, RULE_BEGIN_SZ) == 0 {
                    os_clearnl!(str, p);

                    p = str.add(RULE_BEGIN_SZ);
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
                    os_free!((*al_data).comment);
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
                else if strncmp(cs(SRCIP_BEGIN), str, SRCIP_BEGIN_SZ) == 0 {
                    os_clearnl!(str, p);

                    p = str.add(SRCIP_BEGIN_SZ);
                    os_free!((*al_data).srcip);
                    (*al_data).srcip = os_strdup(p);
                }
                /* srcport */
                else if strncmp(cs(SRCPORT_BEGIN), str, SRCPORT_BEGIN_SZ) == 0 {
                    os_clearnl!(str, p);

                    p = str.add(SRCPORT_BEGIN_SZ);
                    (*al_data).srcport = atoi(p);
                }
                /* dstip */
                else if strncmp(cs(DSTIP_BEGIN), str, DSTIP_BEGIN_SZ) == 0 {
                    os_clearnl!(str, p);

                    p = str.add(DSTIP_BEGIN_SZ);
                    os_free!((*al_data).dstip);
                    (*al_data).dstip = os_strdup(p);
                }
                /* dstport */
                else if strncmp(cs(DSTPORT_BEGIN), str, DSTPORT_BEGIN_SZ) == 0 {
                    os_clearnl!(str, p);

                    p = str.add(DSTPORT_BEGIN_SZ);
                    (*al_data).dstport = atoi(p);
                }
                /* username */
                else if strncmp(cs(USER_BEGIN), str, USER_BEGIN_SZ) == 0 {
                    os_clearnl!(str, p);

                    p = str.add(USER_BEGIN_SZ);
                    os_free!((*al_data).user);
                    (*al_data).user = os_strdup(p);
                }
                /* "9/19/2016 - Sivakumar Nellurandi - parsing additions" */
                /* It is a log message */
                else if log_size < LOG_LIMIT {
                    os_clearnl!(str, p);
                    if issyscheck == 1 {
                        if strncmp(str, cs(INTEGRITY_PREFIX), INTEGRITY_PREFIX_SZ) == 0 {
                            (*al_data).filename = strdup(str.add(INTEGRITY_PREFIX_SZ));
                            if !(*al_data).filename.is_null() {
                                let f = (*al_data).filename;
                                // Faithful reproduction of the original
                                // `filename[strlen(filename) - 1] = '\0';`
                                // (underflows by one byte for an empty name).
                                *f.wrapping_add(strlen(f).wrapping_sub(1)) = 0;
                            }
                        }
                        issyscheck = 0;
                    }

                    // al_data->log bookkeeping is commented out upstream.
                }
            }
        }

        // We reached the end of the alert and the information is saved.
        if feof(fp) != 0 && _r == 2 {
            return al_data;
        }
    }

    /* l_error: */
    /* Free the memory */
    FreeAlertData(al_data);
    /* We need to clean end of file before returning */
    clearerr(fp);
    ptr::null_mut()
}
