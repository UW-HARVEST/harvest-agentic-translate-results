//! Rust translation of the driver C library.
//!
//! Provides byte-identical behavior to the original C code under `c_src/`.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

// ----------------------------------------------------------------------------
// Constants from headers
// ----------------------------------------------------------------------------

const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: i64 = 5;

const ALERTS_DAILY: &[u8] = b"alerts.log\0";
const STDIN_NAME: &[u8] = b"<stdin>\0";

const CRALERT_MAIL_SET: c_int = 0x001;
#[allow(dead_code)]
const CRALERT_EXEC_SET: c_int = 0x002;
const CRALERT_READ_ALL: c_int = 0x004;
#[allow(dead_code)]
const CRALERT_READ_FAILED: c_int = 0x008;
const CRALERT_FP_SET: c_int = 0x010;

const OS_MAXSTR: usize = 1024;

// ----------------------------------------------------------------------------
// C struct layouts (must match the headers exactly)
// ----------------------------------------------------------------------------

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

// `struct tm` from <time.h>. Glibc layout (Linux x86_64):
//   int tm_sec;
//   int tm_min;
//   int tm_hour;
//   int tm_mday;
//   int tm_mon;
//   int tm_year;
//   int tm_wday;
//   int tm_yday;
//   int tm_isdst;
//   long tm_gmtoff;
//   const char *tm_zone;
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

    pub fp: *mut libc::FILE,
    pub f_status: libc::stat,
}

// ----------------------------------------------------------------------------
// Helpers: os_calloc / os_realloc / os_strdup mirror the C versions in shared.h
// ----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: libc::size_t, size: libc::size_t) -> *mut c_void {
    let out = unsafe { libc::calloc(num, size) };
    if out.is_null() {
        eprint_no_newline("Memory allocation failed in os_calloc");
        unsafe { libc::exit(libc::EXIT_FAILURE) };
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: libc::size_t) -> *mut c_void {
    let out = unsafe { libc::realloc(ptr, new_size) };
    if out.is_null() {
        eprint_no_newline("Memory allocation failed in os_realloc");
        unsafe { libc::exit(libc::EXIT_FAILURE) };
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(str_ptr: *const c_char) -> *mut c_char {
    if str_ptr.is_null() {
        eprint_no_newline("NULL string passed to os_strdup");
        unsafe { libc::exit(libc::EXIT_FAILURE) };
    }
    let dup = unsafe { libc::strdup(str_ptr) };
    if dup.is_null() {
        eprint_no_newline("Memory allocation failed in os_strdup");
        unsafe { libc::exit(libc::EXIT_FAILURE) };
    }
    dup
}

/// Mirrors `fprintf(stderr, "...")` (no trailing newline) used by os_calloc et al.
fn eprint_no_newline(s: &str) {
    use std::io::Write;
    let stderr = std::io::stderr();
    let mut h = stderr.lock();
    let _ = h.write_all(s.as_bytes());
    let _ = h.flush();
}

/// Equivalent of the `os_free(x)` macro: free if non-null and clear the slot.
unsafe fn os_free(p: &mut *mut c_char) {
    if !p.is_null() {
        unsafe { libc::free(*p as *mut c_void) };
        *p = ptr::null_mut();
    }
}

/// Equivalent of `os_clearnl(x, p)` which sets the last '\n' in `x` to '\0'.
unsafe fn os_clearnl(x: *mut c_char) {
    if x.is_null() {
        return;
    }
    let p = unsafe { libc::strrchr(x, b'\n' as c_int) };
    if !p.is_null() {
        unsafe { *p = 0 };
    }
}

// ----------------------------------------------------------------------------
// merror — formatted error to stderr (with trailing newline)
// ----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer: [c_char; 256] = [0; 256];
    unsafe {
        libc::snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            err_template,
            file_name,
            err,
            err_msg,
        );
        libc::fprintf(
            stderr_handle(),
            b"%s\n\0".as_ptr() as *const c_char,
            buffer.as_ptr(),
        );
    }
}

fn stderr_handle() -> *mut libc::FILE {
    // libc on glibc/musl exposes `stderr` indirectly through a TLS / extern.
    // Use `fdopen`-free approach via the libc crate's extern "C" stderr.
    unsafe extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

// ----------------------------------------------------------------------------
// read-alert.c
// ----------------------------------------------------------------------------

// String constants (with their lengths)
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    if al_data.is_null() {
        // The C version is __attribute__((nonnull)), so passing NULL is UB.
        // Match C behavior and just dereference; but we choose to early-return
        // safely here since we only use `os_free` semantics.
        return;
    }
    unsafe {
        os_free(&mut (*al_data).alertid);
        os_free(&mut (*al_data).date);
        os_free(&mut (*al_data).location);
        os_free(&mut (*al_data).comment);
        os_free(&mut (*al_data).group);
        os_free(&mut (*al_data).srcip);
        os_free(&mut (*al_data).dstip);
        os_free(&mut (*al_data).user);
        os_free(&mut (*al_data).filename);
        libc::free(al_data as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut libc::FILE) -> *mut alert_data {
    unsafe {
        let al_data = os_calloc(1, std::mem::size_of::<alert_data>() as libc::size_t)
            as *mut alert_data;

        let mut _r: c_int = 0;
        let mut issyscheck: c_int = 0;
        let mut log_size: usize = 0;
        let mut p: *mut c_char;
        let mut str_buf: [c_char; OS_MAXSTR + 1] = [0; OS_MAXSTR + 1];
        str_buf[OS_MAXSTR] = 0;

        loop {
            let res = libc::fgets(str_buf.as_mut_ptr(), OS_MAXSTR as c_int, fp);
            if res.is_null() {
                break;
            }
            let str_ptr = str_buf.as_mut_ptr();

            // Check for "** Alert" prefix
            if libc::strncmp(
                ALERT_BEGIN.as_ptr() as *const c_char,
                str_ptr,
                ALERT_BEGIN_SZ,
            ) == 0
            {
                let z: libc::size_t;

                // End of the alert
                if _r == 2 {
                    let len = libc::strlen(str_ptr);
                    if libc::fseek(fp, -(len as libc::c_long), libc::SEEK_CUR) != -1 {
                        return al_data;
                    } else {
                        // l_error
                        FreeAlertData(al_data);
                        libc::clearerr(fp);
                        return ptr::null_mut();
                    }
                }

                p = str_ptr.add(ALERT_BEGIN_SZ + 1);

                let m = libc::strstr(p, b":\0".as_ptr() as *const c_char);
                if m.is_null() {
                    continue;
                }

                z = libc::strlen(p) - libc::strlen(m);
                (*al_data).alertid = os_realloc(
                    (*al_data).alertid as *mut c_void,
                    (z + 1) * std::mem::size_of::<c_char>(),
                ) as *mut c_char;
                libc::strncpy((*al_data).alertid, p, z);
                *((*al_data).alertid.add(z)) = 0;

                // Search for email flag
                p = libc::strchr(p, b' ' as c_int);
                if p.is_null() {
                    continue;
                }
                p = p.add(1);

                if (flag & CRALERT_MAIL_SET) != 0
                    && libc::strncmp(
                        ALERT_MAIL.as_ptr() as *const c_char,
                        p,
                        ALERT_MAIL_SZ,
                    ) != 0
                {
                    continue;
                }

                p = libc::strchr(p, b'-' as c_int);
                if !p.is_null() {
                    p = p.add(1);
                    // Skip leading spaces
                    while *p == (b' ' as c_char) {
                        p = p.add(1);
                    }
                    os_free(&mut (*al_data).group);
                    (*al_data).group = os_strdup(p);

                    // Clean newline from group
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
                let _ = z; // silence unused warning if reached
                continue;
            }

            if _r < 1 {
                continue;
            }

            // r == 1: date / location line
            if _r == 1 {
                os_clearnl(str_ptr);

                p = libc::strchr(str_ptr, b':' as c_int);
                if !p.is_null() {
                    p = libc::strchr(p, b' ' as c_int);
                    if !p.is_null() {
                        *p = 0;
                        p = p.add(1);
                    } else {
                        libc::perror(b"date of location not NULL\0".as_ptr() as *const c_char);
                        // l_error
                        FreeAlertData(al_data);
                        libc::clearerr(fp);
                        return ptr::null_mut();
                    }
                }

                if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                    libc::perror(
                        b"date or location not NULL or p is NULL\0".as_ptr() as *const c_char,
                    );
                    FreeAlertData(al_data);
                    libc::clearerr(fp);
                    return ptr::null_mut();
                }

                (*al_data).date = os_strdup(str_ptr);
                (*al_data).location = os_strdup(p);
                _r = 2;
                log_size = 0;
                continue;
            } else if _r == 2 {
                // Rule begin
                if libc::strncmp(RULE_BEGIN.as_ptr() as *const c_char, str_ptr, RULE_BEGIN_SZ)
                    == 0
                {
                    os_clearnl(str_ptr);

                    p = str_ptr.add(RULE_BEGIN_SZ);
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
                        FreeAlertData(al_data);
                        libc::clearerr(fp);
                        return ptr::null_mut();
                    }

                    (*al_data).level = libc::atoi(p) as c_uint;

                    // Get the comment
                    p = libc::strchr(p, b'\'' as c_int);
                    if p.is_null() {
                        FreeAlertData(al_data);
                        libc::clearerr(fp);
                        return ptr::null_mut();
                    }

                    p = p.add(1);
                    os_free(&mut (*al_data).comment);
                    (*al_data).comment = os_strdup(p);

                    // Closing '
                    p = libc::strrchr((*al_data).comment, b'\'' as c_int);
                    if !p.is_null() {
                        *p = 0;
                    } else {
                        FreeAlertData(al_data);
                        libc::clearerr(fp);
                        return ptr::null_mut();
                    }
                } else if libc::strncmp(
                    SRCIP_BEGIN.as_ptr() as *const c_char,
                    str_ptr,
                    SRCIP_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_ptr);

                    p = str_ptr.add(SRCIP_BEGIN_SZ);
                    os_free(&mut (*al_data).srcip);
                    (*al_data).srcip = os_strdup(p);
                } else if libc::strncmp(
                    SRCPORT_BEGIN.as_ptr() as *const c_char,
                    str_ptr,
                    SRCPORT_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_ptr);
                    p = str_ptr.add(SRCPORT_BEGIN_SZ);
                    (*al_data).srcport = libc::atoi(p);
                } else if libc::strncmp(
                    DSTIP_BEGIN.as_ptr() as *const c_char,
                    str_ptr,
                    DSTIP_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_ptr);
                    p = str_ptr.add(DSTIP_BEGIN_SZ);
                    os_free(&mut (*al_data).dstip);
                    (*al_data).dstip = os_strdup(p);
                } else if libc::strncmp(
                    DSTPORT_BEGIN.as_ptr() as *const c_char,
                    str_ptr,
                    DSTPORT_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_ptr);
                    p = str_ptr.add(DSTPORT_BEGIN_SZ);
                    (*al_data).dstport = libc::atoi(p);
                } else if libc::strncmp(
                    USER_BEGIN.as_ptr() as *const c_char,
                    str_ptr,
                    USER_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_ptr);
                    p = str_ptr.add(USER_BEGIN_SZ);
                    os_free(&mut (*al_data).user);
                    (*al_data).user = os_strdup(p);
                } else if log_size < LOG_LIMIT {
                    os_clearnl(str_ptr);
                    if issyscheck == 1 {
                        if libc::strncmp(
                            str_ptr,
                            b"Integrity checksum changed for: '\0".as_ptr() as *const c_char,
                            33,
                        ) == 0
                        {
                            (*al_data).filename = libc::strdup(str_ptr.add(33));
                            if !(*al_data).filename.is_null() {
                                let len = libc::strlen((*al_data).filename);
                                if len > 0 {
                                    *((*al_data).filename.add(len - 1)) = 0;
                                }
                            }
                        }
                        issyscheck = 0;
                    }
                    let _ = log_size; // not incremented (matches C — log_size only tracked in dead code)
                }
            }
        }

        if libc::feof(fp) != 0 && _r == 2 {
            return al_data;
        }

        // l_error
        FreeAlertData(al_data);
        libc::clearerr(fp);
        ptr::null_mut()
    }
}

// ----------------------------------------------------------------------------
// file-queue.c
// ----------------------------------------------------------------------------

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] =
    b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

static S_MONTH: [&[u8]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
    b"Oct\0", b"Nov\0", b"Dec\0",
];

unsafe fn file_sleep() {
    let mut fp_timeout = libc::timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    unsafe {
        libc::select(
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut fp_timeout,
        );
    }
}

unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    unsafe {
        (*fileq).file_name[0] = 0;
        (*fileq).file_name[MAX_FQUEUE] = 0;

        let name_ptr: *const c_char = if ((*fileq).flags & CRALERT_FP_SET) != 0 {
            STDIN_NAME.as_ptr() as *const c_char
        } else {
            ALERTS_DAILY.as_ptr() as *const c_char
        };

        libc::snprintf(
            (*fileq).file_name.as_mut_ptr(),
            MAX_FQUEUE,
            b"%s\0".as_ptr() as *const c_char,
            name_ptr,
        );
    }
}

unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    unsafe {
        // Close if open (when not FP_SET)
        if (flags & CRALERT_FP_SET) == 0 {
            if !(*fileq).fp.is_null() {
                libc::fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
            }

            (*fileq).fp = libc::fopen(
                (*fileq).file_name.as_ptr(),
                b"r\0".as_ptr() as *const c_char,
            );
            if (*fileq).fp.is_null() {
                return 0;
            }
        }

        // Seek to the end of the file
        if (flags & CRALERT_READ_ALL) == 0 {
            if (*fileq).fp.is_null() {
                return 0;
            }

            if libc::fseek((*fileq).fp, 0, libc::SEEK_END) < 0 {
                let err = *libc::__errno_location();
                merror(
                    FSEEK_ERROR.as_ptr() as *const c_char,
                    (*fileq).file_name.as_ptr(),
                    err,
                    libc::strerror(err),
                );
                libc::fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
                return -1;
            }
        }

        // File change time
        if !(*fileq).fp.is_null() {
            if libc::fstat(libc::fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
                let err = *libc::__errno_location();
                merror(
                    FSTAT_ERROR.as_ptr() as *const c_char,
                    (*fileq).file_name.as_ptr(),
                    err,
                    libc::strerror(err),
                );
                libc::fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
                return -1;
            }
        }

        (*fileq).last_change = (*fileq).f_status.st_mtime;
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    unsafe {
        if (flags & CRALERT_FP_SET) == 0 {
            (*fileq).fp = ptr::null_mut();
        }
        (*fileq).last_change = 0;
        (*fileq).flags = 0;

        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year + 1900;

        let month_idx = (*p).tm_mon as usize;
        let mon_str = S_MONTH[month_idx];
        // strncpy(fileq->mon, s_month[p->tm_mon], 3);
        // copies up to 3 bytes; does NOT null-terminate beyond what's copied.
        libc::strncpy(
            (*fileq).mon.as_mut_ptr(),
            mon_str.as_ptr() as *const c_char,
            3,
        );

        // memset(fileq->file_name, '\0', MAX_FQUEUE + 1)
        libc::memset(
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const tm,
    timeout: c_uint,
) -> *mut alert_data {
    unsafe {
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
        let month_idx = (*p).tm_mon as usize;
        let mon_str = S_MONTH[month_idx];
        libc::strncpy(
            (*fileq).mon.as_mut_ptr(),
            mon_str.as_ptr() as *const c_char,
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
}

// ----------------------------------------------------------------------------
// driver.c
// ----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    unsafe {
        // Match the C zero-init; only mday/mon/year set by the C version.
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

        if Init_FileQueue(&mut fq, &mut time, flags) < 0 {
            eprint_no_newline("File queue initialization failed");
            return ptr::null_mut();
        }

        let al_data = Read_FileMon(&mut fq, &mut time, timeout);

        if !fq.fp.is_null() {
            libc::fclose(fq.fp);
        }
        al_data
    }
}

// Quiet "unused" warnings for constants intentionally kept to mirror the C source.
#[allow(dead_code)]
const _UNUSED_FQ_TIMEOUT_REF: i64 = FQ_TIMEOUT;

// ----------------------------------------------------------------------------
// `_init` and `_fini` are linker-generated symbols that originate in crti.o /
// crtn.o. The C build includes these implicitly. They are present in the
// Rust .so as well — the linker emits them — but they're not exported in the
// dynamic symbol table by default. The C .so exports them via the dynamic
// symbol table; we leave the Rust ones alone since they are linker-generated
// .init/.fini section markers and cannot be overridden from Rust without
// fighting the linker. (Modern Rust shared libraries rely on
// `.init_array`/`.fini_array` instead, but the linker still synthesizes
// `_init`/`_fini` segment markers.)
// ----------------------------------------------------------------------------
