//! Translation of `c_src/src/read-alert.c` + `c_src/include/read-alert.h`.

#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

use crate::cbits::*;
use crate::shared::{os_calloc, os_strdup, OS_MAXSTR};

// ---------------------------------------------------------------------------
// read-alert.h
// ---------------------------------------------------------------------------

/// `#define ALERTS_DAILY "alerts.log"`
pub const ALERTS_DAILY: &core::ffi::CStr = c"alerts.log";

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

const _: () = {
    assert!(core::mem::size_of::<alert_data>() == 96);
    assert!(core::mem::align_of::<alert_data>() == 8);
};

// ---------------------------------------------------------------------------
// read-alert.c token table
// ---------------------------------------------------------------------------

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

const LOG_LIMIT: usize = 100;

/// `"Integrity checksum changed for: '"` — 33 bytes.
const SYSCHECK_BEGIN: &[u8] = b"Integrity checksum changed for: '";
const SYSCHECK_BEGIN_SZ: usize = 33;

// ---------------------------------------------------------------------------

/// ```c
/// void FreeAlertData(alert_data *al_data) {
///     char **p;
///     os_free(al_data->alertid);
///     os_free(al_data->date);
///     os_free(al_data->location);
///     os_free(al_data->comment);
///     os_free(al_data->group);
///     os_free(al_data->srcip);
///     os_free(al_data->dstip);
///     os_free(al_data->user);
///     os_free(al_data->filename);
///     // al_data can't be NULL
///     free(al_data);
///     al_data = NULL;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    os_free_slot(&raw mut (*al_data).alertid);
    os_free_slot(&raw mut (*al_data).date);
    os_free_slot(&raw mut (*al_data).location);
    os_free_slot(&raw mut (*al_data).comment);
    os_free_slot(&raw mut (*al_data).group);
    os_free_slot(&raw mut (*al_data).srcip);
    os_free_slot(&raw mut (*al_data).dstip);
    os_free_slot(&raw mut (*al_data).user);
    os_free_slot(&raw mut (*al_data).filename);

    // The commented-out `al_data->log` cleanup from the original source has no
    // corresponding field and is intentionally omitted here as well.

    // al_data can't be NULL
    free(al_data as *mut c_void);
    // `al_data = NULL;` in the C is a dead store to the local parameter.
}

/// Return alert data for the file specified.
///
/// ```c
/// alert_data *GetAlertData(int flag, FILE *fp);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    let al_data = os_calloc(1, core::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    // `log_size` is reset to 0 whenever a header is parsed and is never
    // incremented (the increment lives in the commented-out log-collection
    // block), so the `log_size < LOG_LIMIT` test below is always true.  Keep
    // the variable to mirror the source.
    let log_size: usize = 0;

    // `char str[OS_MAXSTR + 1]; str[OS_MAXSTR] = '\0';`
    //
    // The buffer lives across loop iterations exactly as in C, so bytes past
    // the NUL written by a shorter `fgets` still hold the previous line's
    // contents (`p = str + ALERT_BEGIN_SZ + 1` can read them).
    let mut str_buf = [0u8; OS_MAXSTR + 1];
    str_buf[OS_MAXSTR] = 0;

    // Set when the C code would `goto l_error` from inside the loop, so that
    // the trailing `feof()` check is skipped just like the `goto` does.
    let mut goto_error = false;

    'lines: loop {
        if fgets(
            str_buf.as_mut_ptr() as *mut c_char,
            OS_MAXSTR as c_int,
            fp,
        )
        .is_null()
        {
            break 'lines;
        }

        /* End of alert */
        if c_ncmp_eq(&str_buf, 0, ALERT_BEGIN) {
            /* End of the alert. */
            if _r == 2 {
                // fseek(fp, -strlen(str), SEEK_CUR)
                //
                // `-strlen(str)` negates a size_t and is then converted to
                // `long`, which on LP64 yields exactly `-(long)strlen(str)`.
                let off = (c_len(&str_buf, 0) as u64).wrapping_neg() as i64;
                if fseek(fp, off, SEEK_CUR) != -1 {
                    return al_data;
                } else {
                    goto_error = true;
                    break 'lines;
                }
            }

            let mut pi = ALERT_BEGIN_SZ + 1;

            let mi = match c_str_find(&str_buf, pi, b":") {
                Some(m) => m,
                None => continue 'lines,
            };

            // z = strlen(p) - strlen(m)
            let z = c_len(&str_buf, pi) - c_len(&str_buf, mi);
            (*al_data).alertid = crate::shared::os_realloc(
                (*al_data).alertid as *mut c_void,
                (z + 1) * core::mem::size_of::<c_char>(),
            ) as *mut c_char;
            strncpy(
                (*al_data).alertid,
                str_buf.as_ptr().add(pi) as *const c_char,
                z,
            );
            *(*al_data).alertid.add(z) = 0;

            /* Search for email flag */
            pi = match c_chr(&str_buf, pi, b' ') {
                Some(s) => s + 1,
                None => continue 'lines,
            };

            /* Check for the flags */
            if (flag & CRALERT_MAIL_SET) != 0 && !c_ncmp_eq(&str_buf, pi, ALERT_MAIL) {
                continue 'lines;
            }

            if let Some(dash) = c_chr(&str_buf, pi, b'-') {
                pi = dash + 1;
                /* Skip leading spaces */
                while *str_buf.get(pi).unwrap_or(&0) == b' ' {
                    pi += 1;
                }
                os_free_slot(&raw mut (*al_data).group);
                (*al_data).group = os_strdup(str_buf.as_ptr().add(pi) as *const c_char);

                /* Clean newline from group */
                let nl = raw_rchr((*al_data).group, b'\n');
                if !nl.is_null() {
                    *nl = 0;
                }
                if !(*al_data).group.is_null() && raw_contains((*al_data).group, b"syscheck") {
                    issyscheck = 1;
                }
            }

            /* Search for active-response flag */
            _r = 1;
            continue 'lines;
        }

        if _r < 1 {
            continue 'lines;
        }

        /*** Extract information from the event ***/

        /* r1 means: 2006 Apr 13 16:15:17 /var/log/auth.log */
        if _r == 1 {
            /* Clear newline */
            if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                str_buf[nl] = 0;
            }

            let mut popt = c_chr(&str_buf, 0, b':');
            if let Some(ci) = popt {
                match c_chr(&str_buf, ci, b' ') {
                    Some(si) => {
                        str_buf[si] = 0;
                        popt = Some(si + 1);
                    }
                    None => {
                        /* If p is null it is because strchr failed */
                        perror(c"date of location not NULL".as_ptr());
                        goto_error = true;
                        break 'lines;
                    }
                }
            }

            /* If not, str is date and p is the location */
            if !(*al_data).date.is_null() || !(*al_data).location.is_null() || popt.is_none() {
                perror(c"date or location not NULL or p is NULL".as_ptr());
                goto_error = true;
                break 'lines;
            }

            let pi = popt.unwrap();
            (*al_data).date = os_strdup(str_buf.as_ptr() as *const c_char);
            (*al_data).location = os_strdup(str_buf.as_ptr().add(pi) as *const c_char);
            _r = 2;
            // log_size = 0;
            continue 'lines;
        } else if _r == 2 {
            /* Rule begin */
            if c_ncmp_eq(&str_buf, 0, RULE_BEGIN) {
                if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                    str_buf[nl] = 0;
                }

                let mut pi = RULE_BEGIN_SZ;
                (*al_data).rule = c_atoi(&str_buf, pi) as c_uint;

                let mut popt = c_chr(&str_buf, pi, b' ');
                if let Some(s1) = popt {
                    popt = c_chr(&str_buf, s1 + 1, b' ').map(|s2| s2 + 1);
                }

                let Some(p2) = popt else {
                    goto_error = true;
                    break 'lines;
                };
                pi = p2;

                (*al_data).level = c_atoi(&str_buf, pi) as c_uint;

                /* Get the comment */
                let Some(q) = c_chr(&str_buf, pi, b'\'') else {
                    goto_error = true;
                    break 'lines;
                };
                pi = q + 1;

                os_free_slot(&raw mut (*al_data).comment);
                (*al_data).comment = os_strdup(str_buf.as_ptr().add(pi) as *const c_char);

                /* Must have the closing \' */
                let close = raw_rchr((*al_data).comment, b'\'');
                if !close.is_null() {
                    *close = 0;
                } else {
                    goto_error = true;
                    break 'lines;
                }
            }
            /* srcip */
            else if c_ncmp_eq(&str_buf, 0, SRCIP_BEGIN) {
                if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                    str_buf[nl] = 0;
                }
                let pi = SRCIP_BEGIN_SZ;
                os_free_slot(&raw mut (*al_data).srcip);
                (*al_data).srcip = os_strdup(str_buf.as_ptr().add(pi) as *const c_char);
            }
            /* srcport */
            else if c_ncmp_eq(&str_buf, 0, SRCPORT_BEGIN) {
                if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                    str_buf[nl] = 0;
                }
                let pi = SRCPORT_BEGIN_SZ;
                (*al_data).srcport = c_atoi(&str_buf, pi);
            }
            /* dstip */
            else if c_ncmp_eq(&str_buf, 0, DSTIP_BEGIN) {
                if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                    str_buf[nl] = 0;
                }
                let pi = DSTIP_BEGIN_SZ;
                os_free_slot(&raw mut (*al_data).dstip);
                (*al_data).dstip = os_strdup(str_buf.as_ptr().add(pi) as *const c_char);
            }
            /* dstport */
            else if c_ncmp_eq(&str_buf, 0, DSTPORT_BEGIN) {
                if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                    str_buf[nl] = 0;
                }
                let pi = DSTPORT_BEGIN_SZ;
                (*al_data).dstport = c_atoi(&str_buf, pi);
            }
            /* username */
            else if c_ncmp_eq(&str_buf, 0, USER_BEGIN) {
                if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                    str_buf[nl] = 0;
                }
                let pi = USER_BEGIN_SZ;
                os_free_slot(&raw mut (*al_data).user);
                (*al_data).user = os_strdup(str_buf.as_ptr().add(pi) as *const c_char);
            }
            /* It is a log message */
            else if log_size < LOG_LIMIT {
                if let Some(nl) = c_rchr(&str_buf, 0, b'\n') {
                    str_buf[nl] = 0;
                }
                if issyscheck == 1 {
                    if c_ncmp_eq(&str_buf, 0, SYSCHECK_BEGIN) {
                        // NOTE: plain `strdup`, not `os_strdup`.
                        (*al_data).filename =
                            strdup(str_buf.as_ptr().add(SYSCHECK_BEGIN_SZ) as *const c_char);
                        if !(*al_data).filename.is_null() {
                            // Faithful reproduction of the original
                            // out-of-bounds write when the remainder is the
                            // empty string (`strlen(...) - 1` underflows).
                            let l = raw_len((*al_data).filename);
                            let target = (*al_data).filename.wrapping_offset(l as isize - 1);
                            *target = 0;
                        }
                    }
                    issyscheck = 0;
                }

                // The `al_data->log` accumulation is commented out upstream.
            }
        }
    }

    if !goto_error {
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
