// Translation of c_src/ to Rust producing byte-identical output for the same inputs.
// This is a cdylib that exposes the same C ABI as the original library.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use libc::{c_char, c_int, c_uint, c_void, size_t, time_t, FILE};
use std::ptr;

const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: i64 = 5;
const OS_MAXSTR: usize = 1024;

const CRALERT_MAIL_SET: c_int = 0x001;
#[allow(dead_code)]
const CRALERT_EXEC_SET: c_int = 0x002;
const CRALERT_READ_ALL: c_int = 0x004;
#[allow(dead_code)]
const CRALERT_READ_FAILED: c_int = 0x008;
const CRALERT_FP_SET: c_int = 0x010;

// Constants used by GetAlertData
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

// File-queue error templates
const FSTAT_ERROR: &[u8] = b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] = b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

// Months
const S_MONTH: [&[u8]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
    b"Oct\0", b"Nov\0", b"Dec\0",
];

// ---------- Structures (must match the C ABI) ----------

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
pub struct file_queue {
    pub last_change: time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: libc::stat,
}

// ---------- shared.h helpers (calloc/realloc/strdup with abort on failure) ----------

unsafe fn os_calloc(num: size_t, size: size_t) -> *mut c_void {
    let out = libc::calloc(num, size);
    if out.is_null() {
        write_stderr_raw(b"Memory allocation failed in os_calloc");
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

unsafe fn os_realloc(p: *mut c_void, new_size: size_t) -> *mut c_void {
    let out = libc::realloc(p, new_size);
    if out.is_null() {
        write_stderr_raw(b"Memory allocation failed in os_realloc");
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

unsafe fn os_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        write_stderr_raw(b"NULL string passed to os_strdup");
        libc::exit(libc::EXIT_FAILURE);
    }
    let dup = libc::strdup(s);
    if dup.is_null() {
        write_stderr_raw(b"Memory allocation failed in os_strdup");
        libc::exit(libc::EXIT_FAILURE);
    }
    dup
}

/// Mimic the os_free macro: free if non-NULL and set the slot to NULL.
unsafe fn os_free_field(slot: *mut *mut c_char) {
    if !(*slot).is_null() {
        libc::free(*slot as *mut c_void);
        *slot = ptr::null_mut();
    }
}

unsafe fn write_stderr_raw(buf: &[u8]) {
    // The C code does fprintf(stderr, "...") with no newline.
    libc::write(2, buf.as_ptr() as *const c_void, buf.len());
}

// Replicate the merror() helper from file-queue.c:
//   char buffer[256];
//   snprintf(buffer, sizeof(buffer), err_template, file_name, err, err_msg);
//   fprintf(stderr, "%s\n", buffer);
unsafe fn merror(
    template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0u8; 256];
    libc::snprintf(
        buffer.as_mut_ptr() as *mut c_char,
        buffer.len(),
        template,
        file_name,
        err,
        err_msg,
    );
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len() - 1);
    libc::write(2, buffer.as_ptr() as *const c_void, len);
    libc::write(2, b"\n".as_ptr() as *const c_void, 1);
}

// Sleep for FQ_TIMEOUT seconds (using select with NULL fdsets, like the C version).
unsafe fn file_sleep() {
    let mut fp_timeout = libc::timeval {
        tv_sec: FQ_TIMEOUT as libc::time_t,
        tv_usec: 0,
    };
    libc::select(
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        &mut fp_timeout,
    );
}

// ---------- file-queue helpers (file-scope in C) ----------

unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    // fileq->file_name[0] = '\0';
    (*fileq).file_name[0] = 0;
    // fileq->file_name[MAX_FQUEUE] = '\0';
    (*fileq).file_name[MAX_FQUEUE] = 0;

    let label: *const c_char = if (*fileq).flags & CRALERT_FP_SET != 0 {
        b"<stdin>\0".as_ptr() as *const c_char
    } else {
        b"alerts.log\0".as_ptr() as *const c_char
    };

    libc::snprintf(
        (*fileq).file_name.as_mut_ptr(),
        MAX_FQUEUE,
        b"%s\0".as_ptr() as *const c_char,
        label,
    );
}

unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    // Close if it is open
    if flags & CRALERT_FP_SET == 0 {
        if !(*fileq).fp.is_null() {
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
        }

        // We must be able to open the file, fseek and get the time of change.
        (*fileq).fp = libc::fopen(
            (*fileq).file_name.as_ptr(),
            b"r\0".as_ptr() as *const c_char,
        );
        if (*fileq).fp.is_null() {
            // Queue not available
            return 0;
        }
    }

    // Seek to the end of the file
    if flags & CRALERT_READ_ALL == 0 {
        if (*fileq).fp.is_null() {
            return 0;
        }

        if libc::fseek((*fileq).fp, 0, libc::SEEK_END) < 0 {
            let errno = *libc::__errno_location();
            merror(
                FSEEK_ERROR.as_ptr() as *const c_char,
                (*fileq).file_name.as_ptr(),
                errno,
                libc::strerror(errno),
            );
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    // File change time
    if !(*fileq).fp.is_null() {
        if libc::fstat(libc::fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            let errno = *libc::__errno_location();
            merror(
                FSTAT_ERROR.as_ptr() as *const c_char,
                (*fileq).file_name.as_ptr(),
                errno,
                libc::strerror(errno),
            );
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;

    1
}

// ---------- Public exports ----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const libc::tm,
    flags: c_int,
) -> c_int {
    // Initialize file_queue fields
    if flags & CRALERT_FP_SET == 0 {
        (*fileq).fp = ptr::null_mut();
    }
    (*fileq).last_change = 0;
    (*fileq).flags = 0;

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;

    // strncpy(fileq->mon, s_month[p->tm_mon], 3)
    let mon_idx = (*p).tm_mon as usize;
    let src = S_MONTH[mon_idx];
    libc::strncpy(
        (*fileq).mon.as_mut_ptr(),
        src.as_ptr() as *const c_char,
        3,
    );

    // memset(fileq->file_name, '\0', MAX_FQUEUE + 1);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const libc::tm,
    timeout: c_uint,
) -> *mut alert_data {
    let mut i: c_uint = 0;

    // If the file queue is not available, try to access it
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
    let mon_idx = (*p).tm_mon as usize;
    libc::strncpy(
        (*fileq).mon.as_mut_ptr(),
        S_MONTH[mon_idx].as_ptr() as *const c_char,
        3,
    );

    // Get latest file
    GetFile_Queue(fileq);

    if Handle_Queue(fileq, 0) != 1 {
        file_sleep();
        return ptr::null_mut();
    }

    // Try up to timeout times to get an event
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    os_free_field(&mut (*al_data).alertid);
    os_free_field(&mut (*al_data).date);
    os_free_field(&mut (*al_data).location);
    os_free_field(&mut (*al_data).comment);
    os_free_field(&mut (*al_data).group);
    os_free_field(&mut (*al_data).srcip);
    os_free_field(&mut (*al_data).dstip);
    os_free_field(&mut (*al_data).user);
    os_free_field(&mut (*al_data).filename);

    // al_data can't be NULL
    libc::free(al_data as *mut c_void);
    // The C code sets the local variable to NULL after free; this has no
    // observable effect for the caller, so we mirror nothing here.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    let al_data = os_calloc(1, std::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;
    let mut p: *mut c_char;
    let mut str_buf: [c_char; OS_MAXSTR + 1] = [0; OS_MAXSTR + 1];
    str_buf[OS_MAXSTR] = 0;

    'outer: loop {
        if libc::fgets(str_buf.as_mut_ptr(), OS_MAXSTR as c_int, fp).is_null() {
            break 'outer;
        }
        let s = str_buf.as_mut_ptr();

        // Check for ALERT_BEGIN
        if libc::strncmp(
            ALERT_BEGIN.as_ptr() as *const c_char,
            s,
            ALERT_BEGIN_SZ,
        ) == 0
        {
            // End of the alert (we hit the next one).
            if _r == 2 {
                let cur_len = libc::strlen(s);
                if libc::fseek(fp, -(cur_len as libc::c_long), libc::SEEK_CUR) != -1 {
                    return al_data;
                } else {
                    // l_error
                    FreeAlertData(al_data);
                    libc::clearerr(fp);
                    return ptr::null_mut();
                }
            }

            // p = str + ALERT_BEGIN_SZ + 1;
            p = s.add(ALERT_BEGIN_SZ + 1);

            // m = strstr(p, ":");
            let m = libc::strstr(p, b":\0".as_ptr() as *const c_char);
            if m.is_null() {
                continue;
            }

            // z = strlen(p) - strlen(m);
            let z = libc::strlen(p) - libc::strlen(m);
            (*al_data).alertid =
                os_realloc((*al_data).alertid as *mut c_void, (z + 1) * std::mem::size_of::<c_char>())
                    as *mut c_char;
            libc::strncpy((*al_data).alertid, p, z);
            *(*al_data).alertid.add(z) = 0;

            // Search for email flag: p = strchr(p, ' ')
            p = libc::strchr(p, b' ' as c_int);
            if p.is_null() {
                continue;
            }
            p = p.add(1);

            // Check for the flags
            if (flag & CRALERT_MAIL_SET) != 0
                && libc::strncmp(
                    ALERT_MAIL.as_ptr() as *const c_char,
                    p,
                    ALERT_MAIL_SZ,
                ) != 0
            {
                continue;
            }

            // p = strchr(p, '-')
            p = libc::strchr(p, b'-' as c_int);
            if !p.is_null() {
                p = p.add(1);
                // Skip leading spaces
                while *p == b' ' as c_char {
                    p = p.add(1);
                }
                os_free_field(&mut (*al_data).group);
                (*al_data).group = os_strdup(p);

                // Clean newline from group: os_clearnl(al_data->group, p)
                // expands to: if((p = strrchr(al_data->group, '\n'))) *p = '\0';
                let nl = libc::strrchr((*al_data).group, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

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
            continue;
        }

        if _r < 1 {
            continue;
        }

        // r1: 2006 Apr 13 16:15:17 /var/log/auth.log
        if _r == 1 {
            // Clear newline: os_clearnl(str, p)
            let nl = libc::strrchr(s, b'\n' as c_int);
            if !nl.is_null() {
                *nl = 0;
            }

            // p = strchr(str, ':')
            p = libc::strchr(s, b':' as c_int);
            if !p.is_null() {
                // p = strchr(p, ' ')
                p = libc::strchr(p, b' ' as c_int);
                if !p.is_null() {
                    *p = 0;
                    p = p.add(1);
                } else {
                    libc::perror(b"date of location not NULL\0".as_ptr() as *const c_char);
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

            (*al_data).date = os_strdup(s);
            (*al_data).location = os_strdup(p);
            _r = 2;
            log_size = 0;
            continue;
        } else if _r == 2 {
            // Rule begin
            if libc::strncmp(
                RULE_BEGIN.as_ptr() as *const c_char,
                s,
                RULE_BEGIN_SZ,
            ) == 0
            {
                // os_clearnl(str, p)
                let nl = libc::strrchr(s, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

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
                os_free_field(&mut (*al_data).comment);
                (*al_data).comment = os_strdup(p);

                // Must have the closing '\''
                let q = libc::strrchr((*al_data).comment, b'\'' as c_int);
                if !q.is_null() {
                    *q = 0;
                } else {
                    FreeAlertData(al_data);
                    libc::clearerr(fp);
                    return ptr::null_mut();
                }
            }
            // srcip
            else if libc::strncmp(
                SRCIP_BEGIN.as_ptr() as *const c_char,
                s,
                SRCIP_BEGIN_SZ,
            ) == 0
            {
                let nl = libc::strrchr(s, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

                p = s.add(SRCIP_BEGIN_SZ);
                os_free_field(&mut (*al_data).srcip);
                (*al_data).srcip = os_strdup(p);
            }
            // srcport
            else if libc::strncmp(
                SRCPORT_BEGIN.as_ptr() as *const c_char,
                s,
                SRCPORT_BEGIN_SZ,
            ) == 0
            {
                let nl = libc::strrchr(s, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

                p = s.add(SRCPORT_BEGIN_SZ);
                (*al_data).srcport = libc::atoi(p);
            }
            // dstip
            else if libc::strncmp(
                DSTIP_BEGIN.as_ptr() as *const c_char,
                s,
                DSTIP_BEGIN_SZ,
            ) == 0
            {
                let nl = libc::strrchr(s, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

                p = s.add(DSTIP_BEGIN_SZ);
                os_free_field(&mut (*al_data).dstip);
                (*al_data).dstip = os_strdup(p);
            }
            // dstport
            else if libc::strncmp(
                DSTPORT_BEGIN.as_ptr() as *const c_char,
                s,
                DSTPORT_BEGIN_SZ,
            ) == 0
            {
                let nl = libc::strrchr(s, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

                p = s.add(DSTPORT_BEGIN_SZ);
                (*al_data).dstport = libc::atoi(p);
            }
            // username
            else if libc::strncmp(
                USER_BEGIN.as_ptr() as *const c_char,
                s,
                USER_BEGIN_SZ,
            ) == 0
            {
                let nl = libc::strrchr(s, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

                p = s.add(USER_BEGIN_SZ);
                os_free_field(&mut (*al_data).user);
                (*al_data).user = os_strdup(p);
            }
            // It is a log message
            else if log_size < LOG_LIMIT {
                let nl = libc::strrchr(s, b'\n' as c_int);
                if !nl.is_null() {
                    *nl = 0;
                }

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
                            }
                        }
                    }
                    issyscheck = 0;
                }
                // The original C code's log buffer code was commented out.
                // Variable is intentionally unused here to mirror C semantics.
                let _ = log_size;
            }
        }
    }

    // We reached the end of the alert and the information is saved.
    if libc::feof(fp) != 0 && _r == 2 {
        return al_data;
    }

    // l_error
    FreeAlertData(al_data);
    libc::clearerr(fp);
    ptr::null_mut()
}

// ---------- driver entrypoint ----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    // struct tm time = {0};
    let mut time: libc::tm = std::mem::zeroed();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    // file_queue fq; memset(&fq, 0, sizeof(file_queue));
    let mut fq: file_queue = std::mem::zeroed();

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        write_stderr_raw(b"File queue initialization failed");
        return ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq, &time, timeout);

    if !fq.fp.is_null() {
        libc::fclose(fq.fp);
    }
    al_data
}
