#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::io::Write;

use libc::{c_long, fseek, size_t, stat, time_t, tm, FILE, SEEK_CUR, SEEK_END};

const OS_MAXSTR: usize = 1024;
const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: c_long = 5;

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

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] =
    b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

const SYSCHECK_PREFIX: &[u8] = b"Integrity checksum changed for: '";
const SYSCHECK_PREFIX_SZ: usize = 33;

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

static S_MONTH: [&[u8; 3]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

// ---------- Memory helpers (mirrors of shared.h) ----------

unsafe fn os_calloc(num: size_t, size: size_t) -> *mut c_void {
    let out = libc::calloc(num, size);
    if out.is_null() {
        let _ = std::io::stderr().write_all(b"Memory allocation failed in os_calloc");
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

unsafe fn os_realloc(ptr: *mut c_void, new_size: size_t) -> *mut c_void {
    let out = libc::realloc(ptr, new_size);
    if out.is_null() {
        let _ = std::io::stderr().write_all(b"Memory allocation failed in os_realloc");
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

unsafe fn os_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        let _ = std::io::stderr().write_all(b"NULL string passed to os_strdup");
        libc::exit(libc::EXIT_FAILURE);
    }
    let dup = libc::strdup(s);
    if dup.is_null() {
        let _ = std::io::stderr().write_all(b"Memory allocation failed in os_strdup");
        libc::exit(libc::EXIT_FAILURE);
    }
    dup
}

#[inline]
unsafe fn os_free_field(p: &mut *mut c_char) {
    if !(*p).is_null() {
        libc::free(*p as *mut c_void);
        *p = std::ptr::null_mut();
    }
}

#[inline]
unsafe fn os_clearnl(s: *mut c_char) -> *mut c_char {
    let p = libc::strrchr(s, b'\n' as c_int);
    if !p.is_null() {
        *p = 0;
    }
    p
}

// ---------- merror ----------

unsafe fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0i8; 256];
    libc::snprintf(
        buffer.as_mut_ptr() as *mut c_char,
        256,
        err_template,
        file_name,
        err,
        err_msg,
    );
    let len = libc::strlen(buffer.as_ptr() as *const c_char);
    let bytes = std::slice::from_raw_parts(buffer.as_ptr() as *const u8, len);
    let mut stderr = std::io::stderr();
    let _ = stderr.write_all(bytes);
    let _ = stderr.write_all(b"\n");
}

// ---------- file-queue ----------

unsafe fn file_sleep() {
    let mut fp_timeout = libc::timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    libc::select(
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        &mut fp_timeout,
    );
}

unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;

    // snprintf "%s" copies up to MAX_FQUEUE-1 chars + null terminator
    let src: *const c_char = if (*fileq).flags & CRALERT_FP_SET != 0 {
        b"<stdin>\0".as_ptr() as *const c_char
    } else {
        ALERTS_DAILY.as_ptr() as *const c_char
    };
    libc::snprintf(
        (*fileq).file_name.as_mut_ptr(),
        MAX_FQUEUE,
        b"%s\0".as_ptr() as *const c_char,
        src,
    );
}

unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
        if !(*fileq).fp.is_null() {
            libc::fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut();
        }

        (*fileq).fp = libc::fopen(
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
            let errno = *libc::__errno_location();
            merror(
                FSEEK_ERROR.as_ptr() as *const c_char,
                (*fileq).file_name.as_ptr(),
                errno,
                libc::strerror(errno),
            );
            libc::fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut();
            return -1;
        }
    }

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
            (*fileq).fp = std::ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
        (*fileq).fp = std::ptr::null_mut();
    }
    (*fileq).last_change = 0;
    (*fileq).flags = 0;

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;

    // strncpy(fileq->mon, s_month[p->tm_mon], 3)
    let mon_idx = (*p).tm_mon as usize;
    let src = S_MONTH[mon_idx];
    for i in 0..3 {
        (*fileq).mon[i] = src[i] as c_char;
    }

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
            return std::ptr::null_mut();
        }
    }

    if (*fileq).fp.is_null() {
        return std::ptr::null_mut();
    }

    al_data = GetAlertData((*fileq).flags, (*fileq).fp);
    if !al_data.is_null() {
        return al_data;
    }

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;
    let mon_idx = (*p).tm_mon as usize;
    let src = S_MONTH[mon_idx];
    for k in 0..3 {
        (*fileq).mon[k] = src[k] as c_char;
    }

    GetFile_Queue(fileq);

    if Handle_Queue(fileq, 0) != 1 {
        file_sleep();
        return std::ptr::null_mut();
    }

    while i < timeout {
        al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }
        i += 1;
        file_sleep();
    }

    std::ptr::null_mut()
}

// ---------- read-alert ----------

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

    libc::free(al_data as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    let al_data = os_calloc(1, std::mem::size_of::<alert_data>()) as *mut alert_data;

    let mut _r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: size_t = 0;

    let mut str_buf = [0i8; OS_MAXSTR + 1];
    str_buf[OS_MAXSTR] = 0;

    let success: bool = 'outer: loop {
        while !libc::fgets(str_buf.as_mut_ptr(), OS_MAXSTR as c_int, fp).is_null() {
            // ALERT_BEGIN check
            if libc::strncmp(
                ALERT_BEGIN.as_ptr() as *const c_char,
                str_buf.as_ptr(),
                ALERT_BEGIN_SZ,
            ) == 0
            {
                if _r == 2 {
                    let len = libc::strlen(str_buf.as_ptr());
                    if fseek(fp, -(len as c_long), SEEK_CUR) != -1 {
                        break 'outer true;
                    } else {
                        break 'outer false;
                    }
                }

                let mut p = str_buf.as_mut_ptr().add(ALERT_BEGIN_SZ + 1);

                let m = libc::strstr(p, b":\0".as_ptr() as *const c_char);
                if m.is_null() {
                    continue;
                }

                let z = libc::strlen(p) - libc::strlen(m);
                (*al_data).alertid = os_realloc(
                    (*al_data).alertid as *mut c_void,
                    (z + 1) * std::mem::size_of::<c_char>(),
                ) as *mut c_char;
                libc::strncpy((*al_data).alertid, p, z);
                *((*al_data).alertid.add(z)) = 0;

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
                    while *p == b' ' as c_char {
                        p = p.add(1);
                    }
                    os_free_field(&mut (*al_data).group);
                    (*al_data).group = os_strdup(p);

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
                continue;
            }

            if _r < 1 {
                continue;
            }

            if _r == 1 {
                os_clearnl(str_buf.as_mut_ptr());

                let mut p = libc::strchr(str_buf.as_ptr(), b':' as c_int);
                if !p.is_null() {
                    p = libc::strchr(p, b' ' as c_int);
                    if !p.is_null() {
                        *(p as *mut c_char) = 0;
                        p = p.add(1);
                    } else {
                        libc::perror(b"date of location not NULL\0".as_ptr() as *const c_char);
                        break 'outer false;
                    }
                }

                if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                    libc::perror(
                        b"date or location not NULL or p is NULL\0".as_ptr() as *const c_char,
                    );
                    break 'outer false;
                }

                (*al_data).date = os_strdup(str_buf.as_ptr());
                (*al_data).location = os_strdup(p);
                _r = 2;
                log_size = 0;
                continue;
            } else if _r == 2 {
                // RULE_BEGIN
                if libc::strncmp(
                    RULE_BEGIN.as_ptr() as *const c_char,
                    str_buf.as_ptr(),
                    RULE_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_buf.as_mut_ptr());

                    let mut p = str_buf.as_mut_ptr().add(RULE_BEGIN_SZ);
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
                        break 'outer false;
                    }

                    (*al_data).level = libc::atoi(p) as c_uint;

                    p = libc::strchr(p, b'\'' as c_int);
                    if p.is_null() {
                        break 'outer false;
                    }

                    p = p.add(1);
                    os_free_field(&mut (*al_data).comment);
                    (*al_data).comment = os_strdup(p);

                    let p2 = libc::strrchr((*al_data).comment, b'\'' as c_int);
                    if !p2.is_null() {
                        *p2 = 0;
                    } else {
                        break 'outer false;
                    }
                }
                // SRCIP
                else if libc::strncmp(
                    SRCIP_BEGIN.as_ptr() as *const c_char,
                    str_buf.as_ptr(),
                    SRCIP_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_buf.as_mut_ptr());

                    let p = str_buf.as_mut_ptr().add(SRCIP_BEGIN_SZ);
                    os_free_field(&mut (*al_data).srcip);
                    (*al_data).srcip = os_strdup(p);
                }
                // SRCPORT
                else if libc::strncmp(
                    SRCPORT_BEGIN.as_ptr() as *const c_char,
                    str_buf.as_ptr(),
                    SRCPORT_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_buf.as_mut_ptr());

                    let p = str_buf.as_mut_ptr().add(SRCPORT_BEGIN_SZ);
                    (*al_data).srcport = libc::atoi(p);
                }
                // DSTIP
                else if libc::strncmp(
                    DSTIP_BEGIN.as_ptr() as *const c_char,
                    str_buf.as_ptr(),
                    DSTIP_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_buf.as_mut_ptr());

                    let p = str_buf.as_mut_ptr().add(DSTIP_BEGIN_SZ);
                    os_free_field(&mut (*al_data).dstip);
                    (*al_data).dstip = os_strdup(p);
                }
                // DSTPORT
                else if libc::strncmp(
                    DSTPORT_BEGIN.as_ptr() as *const c_char,
                    str_buf.as_ptr(),
                    DSTPORT_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_buf.as_mut_ptr());

                    let p = str_buf.as_mut_ptr().add(DSTPORT_BEGIN_SZ);
                    (*al_data).dstport = libc::atoi(p);
                }
                // USER
                else if libc::strncmp(
                    USER_BEGIN.as_ptr() as *const c_char,
                    str_buf.as_ptr(),
                    USER_BEGIN_SZ,
                ) == 0
                {
                    os_clearnl(str_buf.as_mut_ptr());

                    let p = str_buf.as_mut_ptr().add(USER_BEGIN_SZ);
                    os_free_field(&mut (*al_data).user);
                    (*al_data).user = os_strdup(p);
                }
                // log message
                else if log_size < LOG_LIMIT {
                    os_clearnl(str_buf.as_mut_ptr());
                    if issyscheck == 1 {
                        if libc::strncmp(
                            str_buf.as_ptr(),
                            SYSCHECK_PREFIX.as_ptr() as *const c_char,
                            SYSCHECK_PREFIX_SZ,
                        ) == 0
                        {
                            (*al_data).filename =
                                libc::strdup(str_buf.as_ptr().add(SYSCHECK_PREFIX_SZ));
                            if !(*al_data).filename.is_null() {
                                let len = libc::strlen((*al_data).filename);
                                if len > 0 {
                                    *((*al_data).filename.add(len - 1)) = 0;
                                }
                            }
                        }
                        issyscheck = 0;
                    }
                    // log array commented out in C
                    let _ = log_size;
                }
            }
        }

        // After fgets returned NULL: feof check
        if libc::feof(fp) != 0 && _r == 2 {
            break 'outer true;
        }

        break 'outer false;
    };

    if success {
        return al_data;
    }

    // l_error:
    FreeAlertData(al_data);
    libc::clearerr(fp);
    std::ptr::null_mut()
}

// ---------- driver ----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    let mut time: tm = std::mem::zeroed();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq: file_queue = std::mem::zeroed();

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        let _ = std::io::stderr().write_all(b"File queue initialization failed");
        return std::ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq, &time, timeout);

    if !fq.fp.is_null() {
        libc::fclose(fq.fp);
    }

    al_data
}
