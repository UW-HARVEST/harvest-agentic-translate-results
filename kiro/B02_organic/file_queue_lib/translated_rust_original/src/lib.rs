#![allow(non_camel_case_types, non_snake_case, unused_assignments)]

use libc::{
    atoi, c_char, c_int, c_uint, c_void, calloc, clearerr, fclose, feof, fgets, fileno, fopen,
    fseek, fstat, free, memset, perror, realloc, select, snprintf, stat, strchr, strdup,
    strlen, strncmp, strncpy, strrchr, strstr, strerror, timeval, FILE, SEEK_CUR, SEEK_END,
    EXIT_FAILURE,
};
use std::ptr;

extern "C" {
    static stderr: *mut FILE;
}

// ── constants from shared.h ──
const OS_MAXSTR: usize = 1024;

// ── constants from file-queue.h ──
const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: libc::c_long = 5;

// ── constants from read-alert.h ──
const ALERTS_DAILY: &[u8] = b"alerts.log\0";

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_EXEC_SET: c_int = 0x002;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_READ_FAILED: c_int = 0x008;
const CRALERT_FP_SET: c_int = 0x010;

// ── string constants for read-alert parsing ──
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

// ── month table from file-queue.c ──
static S_MONTH: [&[u8; 4]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0",
    b"Jul\0", b"Aug\0", b"Sep\0", b"Oct\0", b"Nov\0", b"Dec\0",
];

// ── structs ──

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

#[repr(C)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: libc::c_long,
    pub tm_zone: *const c_char,
}

#[repr(C)]
pub struct file_queue {
    pub last_change: libc::time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

// ── shared.h helpers ──

unsafe fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = calloc(num, size);
    if out.is_null() {
        libc::fprintf(
            stderr,
            b"Memory allocation failed in os_calloc\0".as_ptr() as *const c_char,
        );
        libc::exit(EXIT_FAILURE);
    }
    out
}

unsafe fn os_realloc_fn(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    let out = realloc(ptr, new_size);
    if out.is_null() {
        libc::fprintf(
            stderr,
            b"Memory allocation failed in os_realloc\0".as_ptr() as *const c_char,
        );
        libc::exit(EXIT_FAILURE);
    }
    out
}

unsafe fn os_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        libc::fprintf(
            stderr,
            b"NULL string passed to os_strdup\0".as_ptr() as *const c_char,
        );
        libc::exit(EXIT_FAILURE);
    }
    let dup = strdup(s);
    if dup.is_null() {
        libc::fprintf(
            stderr,
            b"Memory allocation failed in os_strdup\0".as_ptr() as *const c_char,
        );
        libc::exit(EXIT_FAILURE);
    }
    dup
}

/// os_free macro: free and null
unsafe fn os_free(p: &mut *mut c_char) {
    if !(*p).is_null() {
        free(*p as *mut c_void);
        *p = ptr::null_mut();
    }
}

/// os_clearnl macro: strip trailing newline
unsafe fn os_clearnl(s: *mut c_char) {
    let p = strrchr(s, b'\n' as c_int);
    if !p.is_null() {
        *p = 0;
    }
}

// ── file-queue.c: merror ──

unsafe fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer: [c_char; 256] = [0; 256];
    snprintf(
        buffer.as_mut_ptr(),
        256,
        err_template,
        file_name,
        err,
        err_msg,
    );
    libc::fprintf(
        stderr,
        b"%s\n\0".as_ptr() as *const c_char,
        buffer.as_ptr(),
    );
}

// ── file-queue.c: file_sleep ──

unsafe fn file_sleep() {
    let mut fp_timeout = timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    select(0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut fp_timeout);
}

// ── file-queue.c: GetFile_Queue ──

unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;

    if (*fileq).flags & CRALERT_FP_SET != 0 {
        snprintf(
            (*fileq).file_name.as_mut_ptr(),
            MAX_FQUEUE,
            b"%s\0".as_ptr() as *const c_char,
            b"<stdin>\0".as_ptr() as *const c_char,
        );
    } else {
        snprintf(
            (*fileq).file_name.as_mut_ptr(),
            MAX_FQUEUE,
            b"%s\0".as_ptr() as *const c_char,
            ALERTS_DAILY.as_ptr() as *const c_char,
        );
    }
}

// ── file-queue.c: Handle_Queue ──

unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    let errno_ptr = libc::__errno_location();

    if flags & CRALERT_FP_SET == 0 {
        if !(*fileq).fp.is_null() {
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
        }

        (*fileq).fp = fopen(
            (*fileq).file_name.as_ptr(),
            b"r\0".as_ptr() as *const c_char,
        );
        if (*fileq).fp.is_null() {
            return 0;
        }
    }

    if flags & CRALERT_READ_ALL == 0 {
        if (*fileq).fp.is_null() {
            return 0;
        }

        if fseek((*fileq).fp, 0, SEEK_END) < 0 {
            merror(
                b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0".as_ptr()
                    as *const c_char,
                (*fileq).file_name.as_ptr(),
                *errno_ptr,
                strerror(*errno_ptr),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    if !(*fileq).fp.is_null() {
        if fstat(fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            merror(
                b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0"
                    .as_ptr() as *const c_char,
                (*fileq).file_name.as_ptr(),
                *errno_ptr,
                strerror(*errno_ptr),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;

    1
}

// ── file-queue.c: Init_FileQueue ──

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    if flags & CRALERT_FP_SET == 0 {
        (*fileq).fp = ptr::null_mut();
    }
    (*fileq).last_change = 0;
    (*fileq).flags = 0;

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;

    strncpy(
        (*fileq).mon.as_mut_ptr(),
        S_MONTH[(*p).tm_mon as usize].as_ptr() as *const c_char,
        3,
    );
    memset(
        (*fileq).file_name.as_mut_ptr() as *mut c_void,
        0,
        MAX_FQUEUE + 1,
    );

    (*fileq).flags = flags;

    GetFile_Queue(fileq);

    if Handle_Queue(fileq, (*fileq).flags) < 0 {
        return -1;
    }

    0
}

// ── file-queue.c: Read_FileMon ──

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const tm,
    timeout: c_uint,
) -> *mut alert_data {
    let mut i: c_uint = 0;

    if (*fileq).fp.is_null() {
        if Handle_Queue(fileq, 0) != 1 {
            file_sleep();
            return ptr::null_mut();
        }
    }

    if (*fileq).fp.is_null() {
        return ptr::null_mut();
    }

    let al_data = GetAlertData((*fileq).flags, (*fileq).fp);
    if !al_data.is_null() {
        return al_data;
    }

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;
    strncpy(
        (*fileq).mon.as_mut_ptr(),
        S_MONTH[(*p).tm_mon as usize].as_ptr() as *const c_char,
        3,
    );

    GetFile_Queue(fileq);

    if Handle_Queue(fileq, 0) != 1 {
        file_sleep();
        return ptr::null_mut();
    }

    while i < timeout {
        let al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }
        i += 1;
        file_sleep();
    }

    ptr::null_mut()
}

// ── read-alert.c: FreeAlertData ──

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    os_free(&mut (*al_data).alertid);
    os_free(&mut (*al_data).date);
    os_free(&mut (*al_data).location);
    os_free(&mut (*al_data).comment);
    os_free(&mut (*al_data).group);
    os_free(&mut (*al_data).srcip);
    os_free(&mut (*al_data).dstip);
    os_free(&mut (*al_data).user);
    os_free(&mut (*al_data).filename);

    free(al_data as *mut c_void);
    // al_data = NULL; -- no-op in C (local param), omitted
}

// ── read-alert.c: GetAlertData ──

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    let al_data: *mut alert_data = os_calloc(1, std::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;
    let mut p: *mut c_char;
    let mut str_buf: [c_char; OS_MAXSTR + 1] = [0; OS_MAXSTR + 1];
    str_buf[OS_MAXSTR] = 0;

    while !fgets(
        str_buf.as_mut_ptr(),
        OS_MAXSTR as c_int,
        fp,
    )
    .is_null()
    {
        // Check for ALERT_BEGIN
        if strncmp(
            ALERT_BEGIN.as_ptr() as *const c_char,
            str_buf.as_ptr(),
            ALERT_BEGIN_SZ,
        ) == 0
        {
            let mut z: usize = 0;

            if _r == 2 {
                if fseek(fp, -(strlen(str_buf.as_ptr()) as libc::c_long), SEEK_CUR) != -1 {
                    return al_data;
                } else {
                    // goto l_error
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return ptr::null_mut();
                }
            }

            p = str_buf.as_mut_ptr().add(ALERT_BEGIN_SZ + 1);

            let m = strstr(p, b":\0".as_ptr() as *const c_char);
            if m.is_null() {
                continue;
            }

            z = strlen(p).wrapping_sub(strlen(m));
            (*al_data).alertid =
                os_realloc_fn((*al_data).alertid as *mut c_void, (z + 1) * std::mem::size_of::<c_char>())
                    as *mut c_char;
            strncpy((*al_data).alertid, p, z);
            *(*al_data).alertid.add(z) = 0;

            // Search for email flag
            p = strchr(p, b' ' as c_int);
            if p.is_null() {
                continue;
            }
            p = p.add(1);

            // Check for the flags
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
                os_free(&mut (*al_data).group);
                (*al_data).group = os_strdup(p);

                // Clean newline from group
                os_clearnl((*al_data).group);
                if !(*al_data).group.is_null()
                    && !strstr(
                        (*al_data).group,
                        b"syscheck\0".as_ptr() as *const c_char,
                    )
                    .is_null()
                {
                    issyscheck = 1;
                }
            }

            _r = 1;
            continue;
        }

        if _r < 1 {
            continue;
        }

        // _r == 1: date/location line
        if _r == 1 {
            os_clearnl(str_buf.as_mut_ptr());

            p = strchr(str_buf.as_ptr(), b':' as c_int);
            if !p.is_null() {
                p = strchr(p, b' ' as c_int);
                if !p.is_null() {
                    *p = 0;
                    p = p.add(1);
                } else {
                    perror(b"date of location not NULL\0".as_ptr() as *const c_char);
                    // goto l_error
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return ptr::null_mut();
                }
            }

            if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                perror(
                    b"date or location not NULL or p is NULL\0".as_ptr() as *const c_char,
                );
                FreeAlertData(al_data);
                clearerr(fp);
                return ptr::null_mut();
            }

            (*al_data).date = os_strdup(str_buf.as_ptr());
            (*al_data).location = os_strdup(p);
            _r = 2;
            log_size = 0;
            continue;
        } else if _r == 2 {
            // Rule begin
            if strncmp(
                RULE_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                RULE_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_buf.as_mut_ptr());

                p = str_buf.as_mut_ptr().add(RULE_BEGIN_SZ);
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
                    return ptr::null_mut();
                }

                (*al_data).level = atoi(p) as c_uint;

                // Get the comment
                p = strchr(p, b'\'' as c_int);
                if p.is_null() {
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return ptr::null_mut();
                }

                p = p.add(1);
                os_free(&mut (*al_data).comment);
                (*al_data).comment = os_strdup(p);

                // Must have the closing \'
                p = strrchr((*al_data).comment, b'\'' as c_int);
                if !p.is_null() {
                    *p = 0;
                } else {
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return ptr::null_mut();
                }
            }
            // srcip
            else if strncmp(
                SRCIP_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                SRCIP_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_buf.as_mut_ptr());
                p = str_buf.as_mut_ptr().add(SRCIP_BEGIN_SZ);
                os_free(&mut (*al_data).srcip);
                (*al_data).srcip = os_strdup(p);
            }
            // srcport
            else if strncmp(
                SRCPORT_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                SRCPORT_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_buf.as_mut_ptr());
                p = str_buf.as_mut_ptr().add(SRCPORT_BEGIN_SZ);
                (*al_data).srcport = atoi(p);
            }
            // dstip
            else if strncmp(
                DSTIP_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                DSTIP_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_buf.as_mut_ptr());
                p = str_buf.as_mut_ptr().add(DSTIP_BEGIN_SZ);
                os_free(&mut (*al_data).dstip);
                (*al_data).dstip = os_strdup(p);
            }
            // dstport
            else if strncmp(
                DSTPORT_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                DSTPORT_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_buf.as_mut_ptr());
                p = str_buf.as_mut_ptr().add(DSTPORT_BEGIN_SZ);
                (*al_data).dstport = atoi(p);
            }
            // username
            else if strncmp(
                USER_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                USER_BEGIN_SZ,
            ) == 0
            {
                os_clearnl(str_buf.as_mut_ptr());
                p = str_buf.as_mut_ptr().add(USER_BEGIN_SZ);
                os_free(&mut (*al_data).user);
                (*al_data).user = os_strdup(p);
            }
            // log message / syscheck
            else if log_size < LOG_LIMIT {
                os_clearnl(str_buf.as_mut_ptr());
                if issyscheck == 1 {
                    if strncmp(
                        str_buf.as_ptr(),
                        b"Integrity checksum changed for: '\0".as_ptr() as *const c_char,
                        33,
                    ) == 0
                    {
                        (*al_data).filename = strdup(str_buf.as_ptr().add(33));
                        if !(*al_data).filename.is_null() {
                            let flen = strlen((*al_data).filename);
                            *(*al_data).filename.add(flen - 1) = 0;
                        }
                    }
                    issyscheck = 0;
                }
            }
        }
    }

    // End of file with valid data
    if feof(fp) != 0 && _r == 2 {
        return al_data;
    }

    // l_error
    FreeAlertData(al_data);
    clearerr(fp);
    ptr::null_mut()
}

// ── driver.c: driver ──

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    let mut time = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: day,
        tm_mon: month,
        tm_year: year,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: ptr::null(),
    };

    let mut fq: file_queue = std::mem::zeroed();

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        libc::fprintf(
            stderr,
            b"File queue initialization failed\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq, &time, timeout);

    if !fq.fp.is_null() {
        fclose(fq.fp);
    }
    al_data
}
