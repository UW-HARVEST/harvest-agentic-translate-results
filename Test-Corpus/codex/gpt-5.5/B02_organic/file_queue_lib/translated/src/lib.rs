use libc::{FILE, SEEK_CUR, SEEK_END, size_t, stat, time_t, timeval, tm};
use std::ffi::{c_char, c_int, c_long, c_uint, c_void};
use std::mem;
use std::ptr;

const OS_MAXSTR: usize = 1024;
const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: libc::time_t = 5;

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_FP_SET: c_int = 0x010;

const ALERTS_DAILY: &[u8] = b"alerts.log\0";
const STDIN_NAME: &[u8] = b"<stdin>\0";

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

const INTEGRITY_PREFIX: &[u8] = b"Integrity checksum changed for: '\0";
const INTEGRITY_PREFIX_SZ: usize = 33;

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] =
    b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

const S_MONTH: [&[u8; 4]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0",
    b"Sep\0", b"Oct\0", b"Nov\0", b"Dec\0",
];

unsafe extern "C" {
    static mut stderr: *mut FILE;
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

unsafe fn c_lit(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

unsafe fn os_free_slot(slot: &mut *mut c_char) {
    if !slot.is_null() {
        unsafe {
            libc::free((*slot).cast::<c_void>());
        }
        *slot = ptr::null_mut();
    }
}

unsafe fn clear_newline(s: *mut c_char) -> *mut c_char {
    let p = unsafe { libc::strrchr(s, '\n' as c_int) };
    if !p.is_null() {
        unsafe {
            *p = 0;
        }
    }
    p
}

unsafe fn file_sleep() {
    let mut fp_timeout = timeval {
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

unsafe fn get_file_queue(fileq: *mut file_queue) {
    unsafe {
        (*fileq).file_name[0] = 0;
        (*fileq).file_name[MAX_FQUEUE] = 0;
        let name = if ((*fileq).flags & CRALERT_FP_SET) != 0 {
            STDIN_NAME
        } else {
            ALERTS_DAILY
        };
        libc::snprintf(
            (*fileq).file_name.as_mut_ptr(),
            MAX_FQUEUE,
            c_lit(b"%s\0"),
            c_lit(name),
        );
    }
}

unsafe fn handle_queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    unsafe {
        if (flags & CRALERT_FP_SET) == 0 {
            if !(*fileq).fp.is_null() {
                libc::fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
            }

            (*fileq).fp = libc::fopen((*fileq).file_name.as_ptr(), c_lit(b"r\0"));
            if (*fileq).fp.is_null() {
                return 0;
            }
        }

        if (flags & CRALERT_READ_ALL) == 0 {
            if (*fileq).fp.is_null() {
                return 0;
            }

            if libc::fseek((*fileq).fp, 0, SEEK_END) < 0 {
                let err = *libc::__errno_location();
                merror(
                    c_lit(FSEEK_ERROR),
                    (*fileq).file_name.as_ptr(),
                    err,
                    libc::strerror(err),
                );
                libc::fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
                return -1;
            }
        }

        if !(*fileq).fp.is_null() {
            if libc::fstat(libc::fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
                let err = *libc::__errno_location();
                merror(
                    c_lit(FSTAT_ERROR),
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
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0 as c_char; 256];
    unsafe {
        libc::snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            err_template,
            file_name,
            err,
            err_msg,
        );
        libc::fprintf(stderr, c_lit(b"%s\n\0"), buffer.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: size_t, size: size_t) -> *mut c_void {
    let out = unsafe { libc::calloc(num, size) };
    if out.is_null() {
        unsafe {
            libc::fprintf(
                stderr,
                c_lit(b"Memory allocation failed in os_calloc\0"),
            );
            libc::exit(libc::EXIT_FAILURE);
        }
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: size_t) -> *mut c_void {
    let out = unsafe { libc::realloc(ptr, new_size) };
    if out.is_null() {
        unsafe {
            libc::fprintf(
                stderr,
                c_lit(b"Memory allocation failed in os_realloc\0"),
            );
            libc::exit(libc::EXIT_FAILURE);
        }
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        unsafe {
            libc::fprintf(stderr, c_lit(b"NULL string passed to os_strdup\0"));
            libc::exit(libc::EXIT_FAILURE);
        }
    }
    let dup = unsafe { libc::strdup(str_) };
    if dup.is_null() {
        unsafe {
            libc::fprintf(
                stderr,
                c_lit(b"Memory allocation failed in os_strdup\0"),
            );
            libc::exit(libc::EXIT_FAILURE);
        }
    }
    dup
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    unsafe {
        os_free_slot(&mut (*al_data).alertid);
        os_free_slot(&mut (*al_data).date);
        os_free_slot(&mut (*al_data).location);
        os_free_slot(&mut (*al_data).comment);
        os_free_slot(&mut (*al_data).group);
        os_free_slot(&mut (*al_data).srcip);
        os_free_slot(&mut (*al_data).dstip);
        os_free_slot(&mut (*al_data).user);
        os_free_slot(&mut (*al_data).filename);
        libc::free(al_data.cast::<c_void>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    unsafe {
        let al_data = os_calloc(1, mem::size_of::<alert_data>()).cast::<alert_data>();
        let mut r = 0;
        let mut issyscheck = 0;
        let mut log_size: size_t = 0;
        let mut p: *mut c_char;
        let mut str_ = [0 as c_char; OS_MAXSTR + 1];
        str_[OS_MAXSTR] = 0;

        while !libc::fgets(str_.as_mut_ptr(), OS_MAXSTR as c_int, fp).is_null() {
            if libc::strncmp(c_lit(ALERT_BEGIN), str_.as_ptr(), ALERT_BEGIN_SZ) == 0 {
                if r == 2 {
                    if libc::fseek(
                        fp,
                        -(libc::strlen(str_.as_ptr()) as c_long),
                        SEEK_CUR,
                    ) != -1
                    {
                        return al_data;
                    } else {
                        goto_error(al_data, fp);
                        return ptr::null_mut();
                    }
                }

                p = str_.as_mut_ptr().add(ALERT_BEGIN_SZ + 1);

                let m = libc::strstr(p, c_lit(b":\0"));
                if m.is_null() {
                    continue;
                }

                let z = libc::strlen(p) - libc::strlen(m);
                (*al_data).alertid =
                    os_realloc((*al_data).alertid.cast::<c_void>(), z + 1).cast::<c_char>();
                libc::strncpy((*al_data).alertid, p, z);
                *(*al_data).alertid.add(z) = 0;

                p = libc::strchr(p, ' ' as c_int);
                if p.is_null() {
                    continue;
                }

                p = p.add(1);

                if (flag & CRALERT_MAIL_SET) != 0
                    && libc::strncmp(c_lit(ALERT_MAIL), p, ALERT_MAIL_SZ) != 0
                {
                    continue;
                }

                p = libc::strchr(p, '-' as c_int);
                if !p.is_null() {
                    p = p.add(1);
                    while *p == ' ' as c_char {
                        p = p.add(1);
                    }
                    os_free_slot(&mut (*al_data).group);
                    (*al_data).group = os_strdup(p);

                    let _ = clear_newline((*al_data).group);
                    if !(*al_data).group.is_null()
                        && !libc::strstr((*al_data).group, c_lit(b"syscheck\0")).is_null()
                    {
                        issyscheck = 1;
                    }
                }

                r = 1;
                continue;
            }

            if r < 1 {
                continue;
            }

            if r == 1 {
                let _ = clear_newline(str_.as_mut_ptr());

                p = libc::strchr(str_.as_mut_ptr(), ':' as c_int);
                if !p.is_null() {
                    p = libc::strchr(p, ' ' as c_int);
                    if !p.is_null() {
                        *p = 0;
                        p = p.add(1);
                    } else {
                        libc::perror(c_lit(b"date of location not NULL\0"));
                        goto_error(al_data, fp);
                        return ptr::null_mut();
                    }
                }

                if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                    libc::perror(c_lit(b"date or location not NULL or p is NULL\0"));
                    goto_error(al_data, fp);
                    return ptr::null_mut();
                }

                (*al_data).date = os_strdup(str_.as_ptr());
                (*al_data).location = os_strdup(p);
                r = 2;
                log_size = 0;
                continue;
            } else if r == 2 {
                if libc::strncmp(c_lit(RULE_BEGIN), str_.as_ptr(), RULE_BEGIN_SZ) == 0 {
                    let _ = clear_newline(str_.as_mut_ptr());

                    p = str_.as_mut_ptr().add(RULE_BEGIN_SZ);
                    (*al_data).rule = libc::atoi(p) as c_uint;

                    p = libc::strchr(p, ' ' as c_int);
                    if !p.is_null() {
                        p = p.add(1);
                        p = libc::strchr(p, ' ' as c_int);
                        if !p.is_null() {
                            p = p.add(1);
                        }
                    }

                    if p.is_null() {
                        goto_error(al_data, fp);
                        return ptr::null_mut();
                    }

                    (*al_data).level = libc::atoi(p) as c_uint;

                    p = libc::strchr(p, '\'' as c_int);
                    if p.is_null() {
                        goto_error(al_data, fp);
                        return ptr::null_mut();
                    }

                    p = p.add(1);
                    os_free_slot(&mut (*al_data).comment);
                    (*al_data).comment = os_strdup(p);

                    p = libc::strrchr((*al_data).comment, '\'' as c_int);
                    if !p.is_null() {
                        *p = 0;
                    } else {
                        goto_error(al_data, fp);
                        return ptr::null_mut();
                    }
                } else if libc::strncmp(c_lit(SRCIP_BEGIN), str_.as_ptr(), SRCIP_BEGIN_SZ) == 0 {
                    let _ = clear_newline(str_.as_mut_ptr());

                    p = str_.as_mut_ptr().add(SRCIP_BEGIN_SZ);
                    os_free_slot(&mut (*al_data).srcip);
                    (*al_data).srcip = os_strdup(p);
                } else if libc::strncmp(c_lit(SRCPORT_BEGIN), str_.as_ptr(), SRCPORT_BEGIN_SZ) == 0
                {
                    let _ = clear_newline(str_.as_mut_ptr());

                    p = str_.as_mut_ptr().add(SRCPORT_BEGIN_SZ);
                    (*al_data).srcport = libc::atoi(p);
                } else if libc::strncmp(c_lit(DSTIP_BEGIN), str_.as_ptr(), DSTIP_BEGIN_SZ) == 0 {
                    let _ = clear_newline(str_.as_mut_ptr());

                    p = str_.as_mut_ptr().add(DSTIP_BEGIN_SZ);
                    os_free_slot(&mut (*al_data).dstip);
                    (*al_data).dstip = os_strdup(p);
                } else if libc::strncmp(c_lit(DSTPORT_BEGIN), str_.as_ptr(), DSTPORT_BEGIN_SZ) == 0
                {
                    let _ = clear_newline(str_.as_mut_ptr());

                    p = str_.as_mut_ptr().add(DSTPORT_BEGIN_SZ);
                    (*al_data).dstport = libc::atoi(p);
                } else if libc::strncmp(c_lit(USER_BEGIN), str_.as_ptr(), USER_BEGIN_SZ) == 0 {
                    let _ = clear_newline(str_.as_mut_ptr());

                    p = str_.as_mut_ptr().add(USER_BEGIN_SZ);
                    os_free_slot(&mut (*al_data).user);
                    (*al_data).user = os_strdup(p);
                } else if log_size < LOG_LIMIT {
                    let _ = clear_newline(str_.as_mut_ptr());
                    if issyscheck == 1 {
                        if libc::strncmp(
                            str_.as_ptr(),
                            c_lit(INTEGRITY_PREFIX),
                            INTEGRITY_PREFIX_SZ,
                        ) == 0
                        {
                            (*al_data).filename =
                                libc::strdup(str_.as_ptr().add(INTEGRITY_PREFIX_SZ));
                            if !(*al_data).filename.is_null() {
                                let len = libc::strlen((*al_data).filename);
                                *(*al_data).filename.add(len - 1) = 0;
                            }
                        }
                        issyscheck = 0;
                    }
                }
            }
        }

        if libc::feof(fp) != 0 && r == 2 {
            return al_data;
        }

        goto_error(al_data, fp);
        ptr::null_mut()
    }
}

unsafe fn goto_error(al_data: *mut alert_data, fp: *mut FILE) {
    unsafe {
        FreeAlertData(al_data);
        libc::clearerr(fp);
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

        libc::strncpy(
            (*fileq).mon.as_mut_ptr(),
            c_lit(S_MONTH[(*p).tm_mon as usize]),
            3,
        );
        libc::memset(
            (*fileq).file_name.as_mut_ptr().cast::<c_void>(),
            0,
            MAX_FQUEUE + 1,
        );

        (*fileq).flags = flags;
        get_file_queue(fileq);

        if handle_queue(fileq, (*fileq).flags) < 0 {
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

        if (*fileq).fp.is_null() {
            if handle_queue(fileq, 0) != 1 {
                file_sleep();
                return ptr::null_mut();
            }
        }

        if (*fileq).fp.is_null() {
            return ptr::null_mut();
        }

        let mut al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }

        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year + 1900;
        libc::strncpy(
            (*fileq).mon.as_mut_ptr(),
            c_lit(S_MONTH[(*p).tm_mon as usize]),
            3,
        );

        get_file_queue(fileq);

        if handle_queue(fileq, 0) != 1 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    unsafe {
        let mut time: tm = mem::zeroed();
        time.tm_mday = day;
        time.tm_mon = month;
        time.tm_year = year;

        let mut fq: file_queue = mem::zeroed();

        if Init_FileQueue(&mut fq, &time, flags) < 0 {
            libc::fprintf(stderr, c_lit(b"File queue initialization failed\0"));
            return ptr::null_mut();
        }

        let al_data = Read_FileMon(&mut fq, &time, timeout);

        if !fq.fp.is_null() {
            libc::fclose(fq.fp);
        }
        al_data
    }
}
