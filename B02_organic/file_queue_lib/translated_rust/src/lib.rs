use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};
use std::ptr;

// ============================================================
// Constants from shared.h, read-alert.h, file-queue.h
// ============================================================
const OS_MAXSTR: usize = 1024;
const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: i64 = 5;

const ALERTS_DAILY: &[u8] = b"alerts.log\0";

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_EXEC_SET: c_int = 0x002;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_READ_FAILED: c_int = 0x008;
const CRALERT_FP_SET: c_int = 0x010;

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

// ============================================================
// libc bindings
// ============================================================
extern "C" {
    fn calloc(num: usize, size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *const c_char;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc_FILE, fmt: *const c_char, ...) -> c_int;
    fn perror(s: *const c_char);
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut libc_FILE;
    fn fclose(fp: *mut libc_FILE) -> c_int;
    fn fgets(buf: *mut c_char, n: c_int, fp: *mut libc_FILE) -> *mut c_char;
    fn fseek(fp: *mut libc_FILE, offset: i64, whence: c_int) -> c_int;
    fn feof(fp: *mut libc_FILE) -> c_int;
    fn clearerr(fp: *mut libc_FILE);
    fn fileno(fp: *mut libc_FILE) -> c_int;
    fn fstat(fd: c_int, buf: *mut Stat) -> c_int;
    fn select(
        nfds: c_int,
        readfds: *mut c_void,
        writefds: *mut c_void,
        exceptfds: *mut c_void,
        timeout: *mut Timeval,
    ) -> c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn exit(status: c_int) -> !;

    static mut stderr: *mut libc_FILE;
    static mut errno: c_int;
}

fn get_errno() -> c_int {
    unsafe { *libc_errno_location() }
}

extern "C" {
    #[cfg(target_os = "linux")]
    fn __errno_location() -> *mut c_int;
}

#[cfg(target_os = "linux")]
fn libc_errno_location() -> *mut c_int {
    unsafe { __errno_location() }
}

// Opaque FILE type
#[repr(C)]
struct libc_FILE {
    _opaque: [u8; 0],
}

const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;
const EXIT_FAILURE: c_int = 1;

// struct timeval
#[repr(C)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

// struct stat - use a large enough buffer
#[repr(C)]
struct Stat {
    _buf: [u8; 256], // oversized to cover all platforms
}

// Offset of st_mtime in struct stat on Linux x86_64 is 88 bytes
// (st_mtime is a time_t = i64)
impl Stat {
    fn st_mtime(&self) -> i64 {
        // On Linux x86_64, st_mtim.tv_sec is at offset 88
        let bytes: [u8; 8] = self._buf[88..96].try_into().unwrap();
        i64::from_ne_bytes(bytes)
    }
}

// ============================================================
// alert_data struct (matches C layout)
// ============================================================
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

// ============================================================
// file_queue struct (matches C layout)
// ============================================================
#[repr(C)]
pub struct file_queue {
    pub last_change: i64, // time_t
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut libc_FILE,
    pub f_status: Stat,
}

// ============================================================
// shared.h helpers
// ============================================================
unsafe fn os_free(x: &mut *mut c_char) {
    if !(*x).is_null() {
        free(*x as *mut c_void);
        *x = ptr::null_mut();
    }
}

unsafe fn os_calloc(num: usize, size: usize) -> *mut c_void {
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

unsafe fn os_realloc_raw(ptr: *mut c_void, new_size: usize) -> *mut c_void {
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

unsafe fn os_strdup(s: *const c_char) -> *mut c_char {
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

/// os_clearnl: if (p = strrchr(x, '\n')) *p = '\0';
unsafe fn os_clearnl(x: *mut c_char) -> *mut c_char {
    let p = strrchr(x, b'\n' as c_int);
    if !p.is_null() {
        *p = 0;
    }
    p
}

// ============================================================
// file-queue.c: merror
// ============================================================
static FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
static FSEEK_ERROR: &[u8] =
    b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

unsafe fn merror(err_template: *const c_char, file_name: *const c_char, err: c_int, err_msg: *const c_char) {
    let mut buffer: [c_char; 256] = [0; 256];
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

// ============================================================
// s_month table
// ============================================================
static S_MONTH: [&[u8]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0",
    b"Jul\0", b"Aug\0", b"Sep\0", b"Oct\0", b"Nov\0", b"Dec\0",
];

// ============================================================
// file-queue.c: file_sleep
// ============================================================
unsafe fn file_sleep() {
    let mut fp_timeout = Timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    select(0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut fp_timeout);
}

// ============================================================
// file-queue.c: GetFile_Queue
// ============================================================
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

// ============================================================
// file-queue.c: Handle_Queue
// ============================================================
unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
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
                FSEEK_ERROR.as_ptr() as *const c_char,
                (*fileq).file_name.as_ptr(),
                get_errno(),
                strerror(get_errno()),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    if !(*fileq).fp.is_null() {
        if fstat(fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            merror(
                FSTAT_ERROR.as_ptr() as *const c_char,
                (*fileq).file_name.as_ptr(),
                get_errno(),
                strerror(get_errno()),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime();

    1
}

// ============================================================
// struct tm (C)
// ============================================================
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

// ============================================================
// file-queue.c: Init_FileQueue
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(fileq: *mut file_queue, p: *const tm, flags: c_int) -> c_int {
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

// ============================================================
// file-queue.c: Read_FileMon
// ============================================================
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

// ============================================================
// read-alert.c: FreeAlertData
// ============================================================
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
    // al_data = NULL; -- C sets local param, no-op
}

// ============================================================
// read-alert.c: GetAlertData
// ============================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut libc_FILE) -> *mut alert_data {
    let al_data: *mut alert_data = os_calloc(1, std::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;
    let mut p: *mut c_char;
    let mut str_buf: [c_char; OS_MAXSTR + 1] = [0; OS_MAXSTR + 1];
    str_buf[OS_MAXSTR] = 0;

    while !fgets(str_buf.as_mut_ptr(), OS_MAXSTR as c_int, fp).is_null() {
        // End of alert
        if strncmp(
            ALERT_BEGIN.as_ptr() as *const c_char,
            str_buf.as_ptr(),
            ALERT_BEGIN_SZ,
        ) == 0
        {
            let mut z: usize;

            if _r == 2 {
                let neg_len = -(strlen(str_buf.as_ptr()) as i64);
                if fseek(fp, neg_len, SEEK_CUR) != -1 {
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

            z = strlen(p) - strlen(m);
            (*al_data).alertid =
                os_realloc_raw((*al_data).alertid as *mut c_void, (z + 1) * std::mem::size_of::<c_char>())
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

        // r1: extract date and location
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
            // log message
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

    // End of file with _r == 2
    if feof(fp) != 0 && _r == 2 {
        return al_data;
    }

    // l_error:
    FreeAlertData(al_data);
    clearerr(fp);
    ptr::null_mut()
}

// ============================================================
// driver.c: driver
// ============================================================
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
