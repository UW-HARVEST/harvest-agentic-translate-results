// Rust translation of the C library.
// Preserves the original behavior (including bugs) and produces byte-identical output.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use libc::{c_char, c_int, c_uint, c_void, size_t, stat, time_t, timeval, tm, FILE};

extern "C" {
    static mut stderr: *mut FILE;
}

const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: c_int = 5;

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_EXEC_SET: c_int = 0x002;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_READ_FAILED: c_int = 0x008;
const CRALERT_FP_SET: c_int = 0x010;

const OS_MAXSTR: usize = 1024;
const LOG_LIMIT: usize = 100;

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

const ALERTS_DAILY: &[u8] = b"alerts.log\0";

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] =
    b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

/* ------------------------------- Structures ------------------------------- */

#[repr(C)]
pub struct file_queue {
    pub last_change: time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

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

/* --------------------------------- Helpers -------------------------------- */

#[inline]
unsafe fn errno_val() -> c_int {
    *libc::__errno_location()
}

unsafe fn os_calloc(num: size_t, size: size_t) -> *mut c_void {
    let out = libc::calloc(num, size);
    if out.is_null() {
        libc::fprintf(
            stderr,
            b"Memory allocation failed in os_calloc\0".as_ptr() as *const c_char,
        );
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

unsafe fn os_realloc(ptr: *mut c_void, new_size: size_t) -> *mut c_void {
    let out = libc::realloc(ptr, new_size);
    if out.is_null() {
        libc::fprintf(
            stderr,
            b"Memory allocation failed in os_realloc\0".as_ptr() as *const c_char,
        );
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

unsafe fn os_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        libc::fprintf(
            stderr,
            b"NULL string passed to os_strdup\0".as_ptr() as *const c_char,
        );
        libc::exit(libc::EXIT_FAILURE);
    }
    let dup = libc::strdup(s);
    if dup.is_null() {
        libc::fprintf(
            stderr,
            b"Memory allocation failed in os_strdup\0".as_ptr() as *const c_char,
        );
        libc::exit(libc::EXIT_FAILURE);
    }
    dup
}

#[inline]
unsafe fn os_free_ptr(p: &mut *mut c_char) {
    if !(*p).is_null() {
        libc::free(*p as *mut c_void);
        *p = core::ptr::null_mut();
    }
}

/// Equivalent to: `if((p = strrchr(x, '\n'))) *p = '\0';`
#[inline]
unsafe fn os_clearnl(x: *mut c_char) {
    let p = libc::strrchr(x, b'\n' as c_int);
    if !p.is_null() {
        *p = 0;
    }
}

unsafe fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0u8; 256];
    libc::snprintf(
        buffer.as_mut_ptr() as *mut c_char,
        256,
        err_template,
        file_name,
        err,
        err_msg,
    );
    libc::fprintf(
        stderr,
        b"%s\n\0".as_ptr() as *const c_char,
        buffer.as_ptr() as *const c_char,
    );
}

/* --------------------------- Months translation --------------------------- */

static S_MONTH: [&[u8]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
    b"Oct\0", b"Nov\0", b"Dec\0",
];

/* ------------------------------ file-queue.c ------------------------------ */

unsafe fn file_sleep() {
    let mut fp_timeout = timeval {
        tv_sec: FQ_TIMEOUT as _,
        tv_usec: 0,
    };
    libc::select(
        0,
        core::ptr::null_mut(),
        core::ptr::null_mut(),
        core::ptr::null_mut(),
        &mut fp_timeout as *mut timeval,
    );
}

unsafe fn GetFile_Queue_internal(fileq: *mut file_queue) {
    let fq = &mut *fileq;

    fq.file_name[0] = 0;
    fq.file_name[MAX_FQUEUE] = 0;

    let chosen: *const c_char = if (fq.flags & CRALERT_FP_SET) != 0 {
        b"<stdin>\0".as_ptr() as *const c_char
    } else {
        ALERTS_DAILY.as_ptr() as *const c_char
    };

    libc::snprintf(
        fq.file_name.as_mut_ptr(),
        MAX_FQUEUE,
        b"%s\0".as_ptr() as *const c_char,
        chosen,
    );
}

unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    let fq = &mut *fileq;

    /* Close if it is open */
    if (flags & CRALERT_FP_SET) == 0 {
        if !fq.fp.is_null() {
            libc::fclose(fq.fp);
            fq.fp = core::ptr::null_mut();
        }

        fq.fp = libc::fopen(
            fq.file_name.as_ptr(),
            b"r\0".as_ptr() as *const c_char,
        );
        if fq.fp.is_null() {
            return 0;
        }
    }

    /* Seek to the end of the file */
    if (flags & CRALERT_READ_ALL) == 0 {
        if fq.fp.is_null() {
            return 0;
        }

        if libc::fseek(fq.fp, 0, libc::SEEK_END) < 0 {
            let e = errno_val();
            merror(
                FSEEK_ERROR.as_ptr() as *const c_char,
                fq.file_name.as_ptr(),
                e,
                libc::strerror(e),
            );
            libc::fclose(fq.fp);
            fq.fp = core::ptr::null_mut();
            return -1;
        }
    }

    /* File change time */
    if !fq.fp.is_null() {
        if libc::fstat(libc::fileno(fq.fp), &mut fq.f_status as *mut stat) < 0 {
            let e = errno_val();
            merror(
                FSTAT_ERROR.as_ptr() as *const c_char,
                fq.file_name.as_ptr(),
                e,
                libc::strerror(e),
            );
            libc::fclose(fq.fp);
            fq.fp = core::ptr::null_mut();
            return -1;
        }
    }

    fq.last_change = fq.f_status.st_mtime;

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    let fq = &mut *fileq;
    let pt = &*p;

    if (flags & CRALERT_FP_SET) == 0 {
        fq.fp = core::ptr::null_mut();
    }
    fq.last_change = 0;
    fq.flags = 0;

    fq.day = pt.tm_mday;
    fq.year = pt.tm_year + 1900;

    libc::strncpy(
        fq.mon.as_mut_ptr(),
        S_MONTH[pt.tm_mon as usize].as_ptr() as *const c_char,
        3,
    );
    libc::memset(
        fq.file_name.as_mut_ptr() as *mut c_void,
        b'\0' as c_int,
        MAX_FQUEUE + 1,
    );

    fq.flags = flags;

    GetFile_Queue_internal(fileq);

    if Handle_Queue(fileq, fq.flags) < 0 {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const tm,
    timeout: c_uint,
) -> *mut alert_data {
    let fq = &mut *fileq;
    let pt = &*p;
    let mut i: c_uint = 0;
    let mut al_data: *mut alert_data;

    if fq.fp.is_null() {
        if Handle_Queue(fileq, 0) != 1 {
            file_sleep();
            return core::ptr::null_mut();
        }
    }

    if fq.fp.is_null() {
        return core::ptr::null_mut();
    }

    al_data = GetAlertData(fq.flags, fq.fp);
    if !al_data.is_null() {
        return al_data;
    }

    fq.day = pt.tm_mday;
    fq.year = pt.tm_year + 1900;
    libc::strncpy(
        fq.mon.as_mut_ptr(),
        S_MONTH[pt.tm_mon as usize].as_ptr() as *const c_char,
        3,
    );

    GetFile_Queue_internal(fileq);

    if Handle_Queue(fileq, 0) != 1 {
        file_sleep();
        return core::ptr::null_mut();
    }

    while i < timeout {
        al_data = GetAlertData(fq.flags, fq.fp);
        if !al_data.is_null() {
            return al_data;
        }
        i += 1;
        file_sleep();
    }

    core::ptr::null_mut()
}

/* ------------------------------ read-alert.c ------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    let ad = &mut *al_data;
    os_free_ptr(&mut ad.alertid);
    os_free_ptr(&mut ad.date);
    os_free_ptr(&mut ad.location);
    os_free_ptr(&mut ad.comment);
    os_free_ptr(&mut ad.group);
    os_free_ptr(&mut ad.srcip);
    os_free_ptr(&mut ad.dstip);
    os_free_ptr(&mut ad.user);
    os_free_ptr(&mut ad.filename);

    libc::free(al_data as *mut c_void);
    // (Original sets local `al_data = NULL;` which has no observable effect.)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    let al_data = os_calloc(1, core::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;

    let mut p: *mut c_char;
    let mut str_buf = [0u8; OS_MAXSTR + 1];
    str_buf[OS_MAXSTR] = 0;
    let s = str_buf.as_mut_ptr() as *mut c_char;

    'outer: loop {
        if libc::fgets(s, OS_MAXSTR as c_int, fp).is_null() {
            break 'outer;
        }

        /* End of alert */
        if libc::strncmp(ALERT_BEGIN.as_ptr() as *const c_char, s, ALERT_BEGIN_SZ) == 0 {
            let m: *mut c_char;
            let z: usize;

            /* End of the alert */
            if _r == 2 {
                if libc::fseek(fp, -(libc::strlen(s) as libc::c_long), libc::SEEK_CUR) != -1 {
                    return al_data;
                } else {
                    return l_error(al_data, fp);
                }
            }

            p = s.add(ALERT_BEGIN_SZ + 1);

            m = libc::strstr(p, b":\0".as_ptr() as *const c_char);
            if m.is_null() {
                continue 'outer;
            }

            z = libc::strlen(p) - libc::strlen(m);
            (*al_data).alertid = os_realloc(
                (*al_data).alertid as *mut c_void,
                (z + 1) * core::mem::size_of::<c_char>(),
            ) as *mut c_char;
            libc::strncpy((*al_data).alertid, p, z);
            *(*al_data).alertid.add(z) = 0;

            /* Search for email flag */
            p = libc::strchr(p, b' ' as c_int);
            if p.is_null() {
                continue 'outer;
            }

            p = p.add(1);

            /* Check for the flags */
            if (flag & CRALERT_MAIL_SET) != 0
                && libc::strncmp(ALERT_MAIL.as_ptr() as *const c_char, p, ALERT_MAIL_SZ) != 0
            {
                continue 'outer;
            }

            p = libc::strchr(p, b'-' as c_int);
            if !p.is_null() {
                p = p.add(1);
                while *p == b' ' as c_char {
                    p = p.add(1);
                }
                os_free_ptr(&mut (*al_data).group);
                (*al_data).group = os_strdup(p);

                /* Clean newline from group */
                os_clearnl((*al_data).group);
                if !(*al_data).group.is_null()
                    && !libc::strstr(
                        (*al_data).group,
                        b"syscheck\0".as_ptr() as *const c_char,
                    )
                    .is_null()
                {
                    issyscheck = 1;
                }
            }

            _r = 1;
            continue 'outer;
        }

        if _r < 1 {
            continue 'outer;
        }

        /*** Extract information from the event ***/

        /* r1: 2006 Apr 13 16:15:17 /var/log/auth.log */
        if _r == 1 {
            os_clearnl(s);

            p = libc::strchr(s, b':' as c_int);
            if !p.is_null() {
                p = libc::strchr(p, b' ' as c_int);
                if !p.is_null() {
                    *p = 0;
                    p = p.add(1);
                } else {
                    libc::perror(b"date of location not NULL\0".as_ptr() as *const c_char);
                    return l_error(al_data, fp);
                }
            }

            if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                libc::perror(
                    b"date or location not NULL or p is NULL\0".as_ptr() as *const c_char,
                );
                return l_error(al_data, fp);
            }

            (*al_data).date = os_strdup(s);
            (*al_data).location = os_strdup(p);
            _r = 2;
            log_size = 0;
            continue 'outer;
        } else if _r == 2 {
            /* Rule begin */
            if libc::strncmp(RULE_BEGIN.as_ptr() as *const c_char, s, RULE_BEGIN_SZ) == 0 {
                os_clearnl(s);

                p = s.add(RULE_BEGIN_SZ);
                (*al_data).rule = libc::atoi(p) as c_uint;

                p = libc::strchr(p, b' ' as c_int);
                if !p.is_null() {
                    p = p.add(1);
                    p = libc::strchr(p, b' ' as c_int);
                    if !p.is_null() {
                        p = p.add(1);
                    }
                }

                if p.is_null() {
                    return l_error(al_data, fp);
                }

                (*al_data).level = libc::atoi(p) as c_uint;

                /* Get the comment */
                p = libc::strchr(p, b'\'' as c_int);
                if p.is_null() {
                    return l_error(al_data, fp);
                }

                p = p.add(1);
                os_free_ptr(&mut (*al_data).comment);
                (*al_data).comment = os_strdup(p);

                /* Must have the closing \' */
                p = libc::strrchr((*al_data).comment, b'\'' as c_int);
                if !p.is_null() {
                    *p = 0;
                } else {
                    return l_error(al_data, fp);
                }
            }
            /* srcip */
            else if libc::strncmp(SRCIP_BEGIN.as_ptr() as *const c_char, s, SRCIP_BEGIN_SZ) == 0 {
                os_clearnl(s);

                p = s.add(SRCIP_BEGIN_SZ);
                os_free_ptr(&mut (*al_data).srcip);
                (*al_data).srcip = os_strdup(p);
            }
            /* srcport */
            else if libc::strncmp(SRCPORT_BEGIN.as_ptr() as *const c_char, s, SRCPORT_BEGIN_SZ)
                == 0
            {
                os_clearnl(s);

                p = s.add(SRCPORT_BEGIN_SZ);
                (*al_data).srcport = libc::atoi(p);
            }
            /* dstip */
            else if libc::strncmp(DSTIP_BEGIN.as_ptr() as *const c_char, s, DSTIP_BEGIN_SZ) == 0 {
                os_clearnl(s);

                p = s.add(DSTIP_BEGIN_SZ);
                os_free_ptr(&mut (*al_data).dstip);
                (*al_data).dstip = os_strdup(p);
            }
            /* dstport */
            else if libc::strncmp(DSTPORT_BEGIN.as_ptr() as *const c_char, s, DSTPORT_BEGIN_SZ)
                == 0
            {
                os_clearnl(s);

                p = s.add(DSTPORT_BEGIN_SZ);
                (*al_data).dstport = libc::atoi(p);
            }
            /* user */
            else if libc::strncmp(USER_BEGIN.as_ptr() as *const c_char, s, USER_BEGIN_SZ) == 0 {
                os_clearnl(s);

                p = s.add(USER_BEGIN_SZ);
                os_free_ptr(&mut (*al_data).user);
                (*al_data).user = os_strdup(p);
            }
            /* log message */
            else if log_size < LOG_LIMIT {
                os_clearnl(s);
                if issyscheck == 1 {
                    if libc::strncmp(
                        s,
                        b"Integrity checksum changed for: '\0".as_ptr() as *const c_char,
                        33,
                    ) == 0
                    {
                        (*al_data).filename = libc::strdup(s.add(33));
                        if !(*al_data).filename.is_null() {
                            let len = libc::strlen((*al_data).filename);
                            if len > 0 {
                                *(*al_data).filename.add(len - 1) = 0;
                            } else {
                                // Reproduce buggy behavior: write to filename[-1]
                                *(*al_data).filename.offset(-1) = 0;
                            }
                        }
                    }
                    issyscheck = 0;
                }
                // log_size++ disabled in original (commented-out log array)
            }
        }
    }

    if libc::feof(fp) != 0 && _r == 2 {
        return al_data;
    }

    l_error(al_data, fp)
}

unsafe fn l_error(al_data: *mut alert_data, fp: *mut FILE) -> *mut alert_data {
    FreeAlertData(al_data);
    libc::clearerr(fp);
    core::ptr::null_mut()
}

/* --------------------------------- driver --------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    let mut time: tm = core::mem::zeroed();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq: file_queue = core::mem::zeroed();

    if Init_FileQueue(&mut fq as *mut file_queue, &time as *const tm, flags) < 0 {
        libc::fprintf(
            stderr,
            b"File queue initialization failed\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq as *mut file_queue, &time as *const tm, timeout);

    if !fq.fp.is_null() {
        libc::fclose(fq.fp);
    }

    al_data
}
