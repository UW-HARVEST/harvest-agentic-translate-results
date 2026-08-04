#![allow(non_camel_case_types, non_snake_case, unused_assignments, dead_code, private_interfaces)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

// libc bindings
extern "C" {
    fn calloc(num: usize, size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc_FILE, fmt: *const c_char, ...) -> c_int;
    fn perror(s: *const c_char);
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut libc_FILE;
    fn fclose(fp: *mut libc_FILE) -> c_int;
    fn fgets(buf: *mut c_char, n: c_int, fp: *mut libc_FILE) -> *mut c_char;
    fn fseek(fp: *mut libc_FILE, offset: i64, whence: c_int) -> c_int;
    fn feof(fp: *mut libc_FILE) -> c_int;
    fn clearerr(fp: *mut libc_FILE);
    fn fstat(fd: c_int, buf: *mut stat) -> c_int;
    fn fileno(fp: *mut libc_FILE) -> c_int;
    fn select(
        nfds: c_int,
        readfds: *mut c_void,
        writefds: *mut c_void,
        exceptfds: *mut c_void,
        timeout: *mut timeval,
    ) -> c_int;
    fn exit(status: c_int) -> !;
    fn strerror(errnum: c_int) -> *mut c_char;
    static mut stderr: *mut libc_FILE;
}

extern "C" {
    #[link_name = "__errno_location"]
    fn errno_location() -> *mut c_int;
}

fn errno() -> c_int {
    unsafe { *errno_location() }
}

#[repr(C)]
pub struct libc_FILE {
    _opaque: [u8; 0],
}

const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;
const EXIT_FAILURE: c_int = 1;

#[repr(C)]
struct timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
struct stat {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    __pad0: u32,
    st_rdev: u64,
    st_size: i64,
    st_blksize: i64,
    st_blocks: i64,
    st_atime: i64,
    st_atime_nsec: i64,
    st_mtime: i64,
    st_mtime_nsec: i64,
    st_ctime: i64,
    st_ctime_nsec: i64,
    __unused: [i64; 3],
}

// ---- shared.h helpers ----

const OS_MAXSTR: usize = 1024;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = calloc(num, size);
    if out.is_null() {
        fprintf(
            stderr,
            b"Memory allocation failed in os_calloc\0".as_ptr() as *const c_char,
        );
        exit(EXIT_FAILURE);
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    let out = realloc(ptr, new_size);
    if out.is_null() {
        fprintf(
            stderr,
            b"Memory allocation failed in os_realloc\0".as_ptr() as *const c_char,
        );
        exit(EXIT_FAILURE);
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        fprintf(
            stderr,
            b"NULL string passed to os_strdup\0".as_ptr() as *const c_char,
        );
        exit(EXIT_FAILURE);
    }
    let dup = strdup(s);
    if dup.is_null() {
        fprintf(
            stderr,
            b"Memory allocation failed in os_strdup\0".as_ptr() as *const c_char,
        );
        exit(EXIT_FAILURE);
    }
    dup
}

/// os_free macro: free and null out
macro_rules! os_free {
    ($x:expr) => {
        if !$x.is_null() {
            free($x as *mut c_void);
            $x = ptr::null_mut();
        }
    };
}

/// os_clearnl macro: find last '\n' and replace with '\0'
macro_rules! os_clearnl {
    ($x:expr, $p:ident) => {
        $p = strrchr($x.as_mut_ptr() as *const c_char, b'\n' as c_int);
        if !$p.is_null() {
            *$p = 0;
        }
    };
    // variant for *mut c_char (use @ to disambiguate)
    (@ptr $x:expr, $p:ident) => {
        $p = strrchr($x as *const c_char, b'\n' as c_int);
        if !$p.is_null() {
            *$p = 0;
        }
    };
}

// ---- read-alert.h constants ----

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_EXEC_SET: c_int = 0x002;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_READ_FAILED: c_int = 0x008;
const CRALERT_FP_SET: c_int = 0x010;

const ALERTS_DAILY: &[u8] = b"alerts.log\0";

// ---- alert_data struct ----

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

// ---- file_queue struct ----

const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: i64 = 5;

#[repr(C)]
pub struct file_queue {
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut libc_FILE,
    pub f_status: stat,
}

// ---- read-alert.c ----

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    if al_data.is_null() {
        return;
    }
    os_free!((*al_data).alertid);
    os_free!((*al_data).date);
    os_free!((*al_data).location);
    os_free!((*al_data).comment);
    os_free!((*al_data).group);
    os_free!((*al_data).srcip);
    os_free!((*al_data).dstip);
    os_free!((*al_data).user);
    os_free!((*al_data).filename);
    free(al_data as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut libc_FILE) -> *mut alert_data {
    let al_data: *mut alert_data = os_calloc(1, std::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;
    let mut p: *mut c_char;
    let mut str_buf = [0i8; OS_MAXSTR + 1];
    str_buf[OS_MAXSTR] = 0;

    while !fgets(str_buf.as_mut_ptr(), OS_MAXSTR as c_int, fp).is_null() {
        // Check for "** Alert"
        if strncmp(
            ALERT_BEGIN.as_ptr() as *const c_char,
            str_buf.as_ptr(),
            ALERT_BEGIN_SZ,
        ) == 0
        {
            let m: *mut c_char;
            let mut z: usize = 0;

            if _r == 2 {
                if fseek(fp, -(strlen(str_buf.as_ptr()) as i64), SEEK_CUR) != -1 {
                    return al_data;
                } else {
                    // goto l_error
                    FreeAlertData(al_data);
                    clearerr(fp);
                    return ptr::null_mut();
                }
            }

            p = str_buf.as_mut_ptr().add(ALERT_BEGIN_SZ + 1);

            m = strstr(p, b":\0".as_ptr() as *const c_char);
            if m.is_null() {
                continue;
            }

            z = strlen(p) - strlen(m);
            (*al_data).alertid =
                os_realloc((*al_data).alertid as *mut c_void, (z + 1) * std::mem::size_of::<c_char>())
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
                os_free!((*al_data).group);
                (*al_data).group = os_strdup(p);

                // Clean newline from group
                os_clearnl!(@ptr (*al_data).group, p);
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

        // _r == 1: extract date and location
        if _r == 1 {
            os_clearnl!(str_buf, p);

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
                os_clearnl!(str_buf, p);

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
                os_free!((*al_data).comment);
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
                os_clearnl!(str_buf, p);
                p = str_buf.as_mut_ptr().add(SRCIP_BEGIN_SZ);
                os_free!((*al_data).srcip);
                (*al_data).srcip = os_strdup(p);
            }
            // srcport
            else if strncmp(
                SRCPORT_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                SRCPORT_BEGIN_SZ,
            ) == 0
            {
                os_clearnl!(str_buf, p);
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
                os_clearnl!(str_buf, p);
                p = str_buf.as_mut_ptr().add(DSTIP_BEGIN_SZ);
                os_free!((*al_data).dstip);
                (*al_data).dstip = os_strdup(p);
            }
            // dstport
            else if strncmp(
                DSTPORT_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                DSTPORT_BEGIN_SZ,
            ) == 0
            {
                os_clearnl!(str_buf, p);
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
                os_clearnl!(str_buf, p);
                p = str_buf.as_mut_ptr().add(USER_BEGIN_SZ);
                os_free!((*al_data).user);
                (*al_data).user = os_strdup(p);
            }
            // log message
            else if log_size < LOG_LIMIT {
                os_clearnl!(str_buf, p);
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

    // Reached end of file with valid data
    if feof(fp) != 0 && _r == 2 {
        return al_data;
    }

    // l_error
    FreeAlertData(al_data);
    clearerr(fp);
    ptr::null_mut()
}

// ---- file-queue.c ----

static S_MONTH: [&[u8]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0",
    b"Jul\0", b"Aug\0", b"Sep\0", b"Oct\0", b"Nov\0", b"Dec\0",
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0i8; 256];
    snprintf(
        buffer.as_mut_ptr(),
        256,
        err_template,
        file_name,
        err,
        err_msg,
    );
    fprintf(stderr, b"%s\n\0".as_ptr() as *const c_char, buffer.as_ptr());
}

unsafe fn file_sleep() {
    let mut fp_timeout = timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    select(0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut fp_timeout);
}

unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;

    if ((*fileq).flags & CRALERT_FP_SET) != 0 {
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

unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
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

    if (flags & CRALERT_READ_ALL) == 0 {
        if (*fileq).fp.is_null() {
            return 0;
        }

        if fseek((*fileq).fp, 0, SEEK_END) < 0 {
            merror(
                b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0".as_ptr()
                    as *const c_char,
                (*fileq).file_name.as_ptr(),
                errno(),
                strerror(errno()),
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
                errno(),
                strerror(errno()),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;

    1
}

// ---- tm struct for C interop ----

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
    pub tm_gmtoff: i64,
    pub tm_zone: *const c_char,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const tm,
    timeout: c_uint,
) -> *mut alert_data {
    let mut i: c_uint = 0;
    let mut al_data: *mut alert_data;

    if (*fileq).fp.is_null() {
        if Handle_Queue(fileq, 0) != 1 {
            file_sleep();
            return ptr::null_mut();
        }
    }

    if (*fileq).fp.is_null() {
        return ptr::null_mut();
    }

    al_data = GetAlertData((*fileq).flags, (*fileq).fp);
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
        al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }
        i += 1;
        file_sleep();
    }

    ptr::null_mut()
}

// ---- driver.c ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    let mut time = std::mem::MaybeUninit::<tm>::zeroed().assume_init();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq = std::mem::MaybeUninit::<file_queue>::zeroed().assume_init();

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        fprintf(
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
