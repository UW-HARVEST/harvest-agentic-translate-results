use libc::{c_char, c_int, c_uint, c_void, FILE};
use std::mem;
use std::ptr;

unsafe extern "C" {
    static mut stderr: *mut FILE;
}

const OS_MAXSTR: usize = 1024;
const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: i64 = 5;

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_FP_SET: c_int = 0x010;

const ALERT_BEGIN_SZ: usize = 8;
const RULE_BEGIN_SZ: usize = 6;
const SRCIP_BEGIN_SZ: usize = 8;
const SRCPORT_BEGIN_SZ: usize = 10;
const DSTIP_BEGIN_SZ: usize = 8;
const DSTPORT_BEGIN_SZ: usize = 10;
const USER_BEGIN_SZ: usize = 6;
const ALERT_MAIL_SZ: usize = 4;
const LOG_LIMIT: usize = 100;

static ALERTS_DAILY: &[u8] = b"alerts.log\0";
static ALERT_BEGIN: &[u8] = b"** Alert\0";
static RULE_BEGIN: &[u8] = b"Rule: \0";
static SRCIP_BEGIN: &[u8] = b"Src IP: \0";
static SRCPORT_BEGIN: &[u8] = b"Src Port: \0";
static DSTIP_BEGIN: &[u8] = b"Dst IP: \0";
static DSTPORT_BEGIN: &[u8] = b"Dst Port: \0";
static USER_BEGIN: &[u8] = b"User: \0";
static ALERT_MAIL: &[u8] = b"mail\0";
static SYSCHECK: &[u8] = b"syscheck\0";
static INTEGRITY_PREFIX: &[u8] = b"Integrity checksum changed for: '\0";
static COLON: &[u8] = b":\0";
static FOPEN_READ: &[u8] = b"r\0";
static STDIN_NAME: &[u8] = b"<stdin>\0";
static SNPRINTF_STR: &[u8] = b"%s\0";
static STDERR_LINE_FMT: &[u8] = b"%s\n\0";
static FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
static FSEEK_ERROR: &[u8] =
    b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";
static OS_CALLOC_ERR: &[u8] = b"Memory allocation failed in os_calloc\0";
static OS_REALLOC_ERR: &[u8] = b"Memory allocation failed in os_realloc\0";
static OS_STRDUP_NULL_ERR: &[u8] = b"NULL string passed to os_strdup\0";
static OS_STRDUP_ALLOC_ERR: &[u8] = b"Memory allocation failed in os_strdup\0";
static DATE_LOCATION_ERR: &[u8] = b"date of location not NULL\0";
static DATE_OR_LOCATION_ERR: &[u8] = b"date or location not NULL or p is NULL\0";
static FILE_QUEUE_INIT_ERR: &[u8] = b"File queue initialization failed\0";

static S_MONTH: [&[u8; 4]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
    b"Oct\0", b"Nov\0", b"Dec\0",
];

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
    pub last_change: libc::time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: libc::stat,
}

unsafe fn print_and_exit(msg: &[u8]) -> ! {
    libc::fputs(msg.as_ptr().cast(), stderr);
    libc::exit(libc::EXIT_FAILURE);
}

unsafe fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = libc::calloc(num, size);
    if out.is_null() {
        print_and_exit(OS_CALLOC_ERR);
    }
    out
}

unsafe fn os_realloc(ptr_in: *mut c_void, new_size: usize) -> *mut c_void {
    let out = libc::realloc(ptr_in, new_size);
    if out.is_null() {
        print_and_exit(OS_REALLOC_ERR);
    }
    out
}

unsafe fn os_strdup(str_in: *const c_char) -> *mut c_char {
    if str_in.is_null() {
        print_and_exit(OS_STRDUP_NULL_ERR);
    }
    let dup = libc::strdup(str_in);
    if dup.is_null() {
        print_and_exit(OS_STRDUP_ALLOC_ERR);
    }
    dup
}

unsafe fn os_free_char(field: &mut *mut c_char) {
    if !(*field).is_null() {
        libc::free((*field).cast());
        *field = ptr::null_mut();
    }
}

unsafe fn os_clearnl(buf: *mut c_char) -> *mut c_char {
    let p = libc::strrchr(buf, '\n' as c_int);
    if !p.is_null() {
        *p = 0;
    }
    p
}

unsafe fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0 as c_char; 256];
    libc::snprintf(
        buffer.as_mut_ptr(),
        buffer.len(),
        err_template,
        file_name,
        err,
        err_msg,
    );
    libc::fprintf(stderr, STDERR_LINE_FMT.as_ptr().cast(), buffer.as_ptr());
}

unsafe fn file_sleep() {
    let mut fp_timeout = libc::timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    libc::select(0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut fp_timeout);
}

unsafe fn get_file_queue(fileq: *mut file_queue) {
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;

    let file_name = if ((*fileq).flags & CRALERT_FP_SET) != 0 {
        STDIN_NAME.as_ptr()
    } else {
        ALERTS_DAILY.as_ptr()
    };

    libc::snprintf(
        (*fileq).file_name.as_mut_ptr(),
        MAX_FQUEUE,
        SNPRINTF_STR.as_ptr().cast(),
        file_name,
    );
}

unsafe fn handle_queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
        if !(*fileq).fp.is_null() {
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
        }

        (*fileq).fp = libc::fopen((*fileq).file_name.as_ptr(), FOPEN_READ.as_ptr().cast());
        if (*fileq).fp.is_null() {
            return 0;
        }
    }

    if (flags & CRALERT_READ_ALL) == 0 {
        if (*fileq).fp.is_null() {
            return 0;
        }

        if libc::fseek((*fileq).fp, 0, libc::SEEK_END) < 0 {
            merror(
                FSEEK_ERROR.as_ptr().cast(),
                (*fileq).file_name.as_ptr(),
                *libc::__errno_location(),
                libc::strerror(*libc::__errno_location()),
            );
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    if !(*fileq).fp.is_null() {
        if libc::fstat(libc::fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            merror(
                FSTAT_ERROR.as_ptr().cast(),
                (*fileq).file_name.as_ptr(),
                *libc::__errno_location(),
                libc::strerror(*libc::__errno_location()),
            );
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut alert_data) {
    let al_data = &mut *al_data;
    os_free_char(&mut al_data.alertid);
    os_free_char(&mut al_data.date);
    os_free_char(&mut al_data.location);
    os_free_char(&mut al_data.comment);
    os_free_char(&mut al_data.group);
    os_free_char(&mut al_data.srcip);
    os_free_char(&mut al_data.dstip);
    os_free_char(&mut al_data.user);
    os_free_char(&mut al_data.filename);
    libc::free(al_data as *mut alert_data as *mut c_void);
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut alert_data {
    let al_data = os_calloc(1, mem::size_of::<alert_data>()) as *mut alert_data;

    let mut r: c_int = 0;
    let mut issyscheck: c_int = 0;
    let mut log_size: usize = 0;
    let mut p: *mut c_char = ptr::null_mut();
    let mut error = false;
    let mut str = [0 as c_char; OS_MAXSTR + 1];
    str[OS_MAXSTR] = 0;

    while !libc::fgets(str.as_mut_ptr(), OS_MAXSTR as c_int, fp).is_null() {
        if libc::strncmp(ALERT_BEGIN.as_ptr().cast(), str.as_ptr(), ALERT_BEGIN_SZ) == 0 {
            if r == 2 {
                if libc::fseek(fp, -(libc::strlen(str.as_ptr()) as libc::c_long), libc::SEEK_CUR) != -1 {
                    return al_data;
                } else {
                    error = true;
                    break;
                }
            }

            p = str.as_mut_ptr().add(ALERT_BEGIN_SZ + 1);

            let m = libc::strstr(p, COLON.as_ptr().cast());
            if m.is_null() {
                continue;
            }

            let z = libc::strlen(p) - libc::strlen(m);
            (*al_data).alertid =
                os_realloc((*al_data).alertid.cast(), (z + 1) * mem::size_of::<c_char>())
                    as *mut c_char;
            libc::strncpy((*al_data).alertid, p, z);
            *(*al_data).alertid.add(z) = 0;

            p = libc::strchr(p, ' ' as c_int);
            if p.is_null() {
                continue;
            }

            p = p.add(1);

            if (flag & CRALERT_MAIL_SET) != 0
                && libc::strncmp(ALERT_MAIL.as_ptr().cast(), p, ALERT_MAIL_SZ) != 0
            {
                continue;
            }

            p = libc::strchr(p, '-' as c_int);
            if !p.is_null() {
                p = p.add(1);
                while *p == ' ' as c_char {
                    p = p.add(1);
                }

                os_free_char(&mut (*al_data).group);
                (*al_data).group = os_strdup(p);

                p = os_clearnl((*al_data).group);
                if !(*al_data).group.is_null()
                    && !libc::strstr((*al_data).group, SYSCHECK.as_ptr().cast()).is_null()
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
            p = os_clearnl(str.as_mut_ptr());

            p = libc::strchr(str.as_mut_ptr(), ':' as c_int);
            if !p.is_null() {
                p = libc::strchr(p, ' ' as c_int);
                if !p.is_null() {
                    *p = 0;
                    p = p.add(1);
                } else {
                    libc::perror(DATE_LOCATION_ERR.as_ptr().cast());
                    error = true;
                    break;
                }
            }

            if !(*al_data).date.is_null() || !(*al_data).location.is_null() || p.is_null() {
                libc::perror(DATE_OR_LOCATION_ERR.as_ptr().cast());
                error = true;
                break;
            }

            (*al_data).date = os_strdup(str.as_ptr());
            (*al_data).location = os_strdup(p);
            r = 2;
            log_size = 0;
            continue;
        } else if r == 2 {
            if libc::strncmp(RULE_BEGIN.as_ptr().cast(), str.as_ptr(), RULE_BEGIN_SZ) == 0 {
                p = os_clearnl(str.as_mut_ptr());

                p = str.as_mut_ptr().add(RULE_BEGIN_SZ);
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
                    error = true;
                    break;
                }

                (*al_data).level = libc::atoi(p) as c_uint;

                p = libc::strchr(p, '\'' as c_int);
                if p.is_null() {
                    error = true;
                    break;
                }

                p = p.add(1);
                os_free_char(&mut (*al_data).comment);
                (*al_data).comment = os_strdup(p);

                p = libc::strrchr((*al_data).comment, '\'' as c_int);
                if !p.is_null() {
                    *p = 0;
                } else {
                    error = true;
                    break;
                }
            } else if libc::strncmp(SRCIP_BEGIN.as_ptr().cast(), str.as_ptr(), SRCIP_BEGIN_SZ) == 0 {
                p = os_clearnl(str.as_mut_ptr());
                p = str.as_mut_ptr().add(SRCIP_BEGIN_SZ);
                os_free_char(&mut (*al_data).srcip);
                (*al_data).srcip = os_strdup(p);
            } else if libc::strncmp(SRCPORT_BEGIN.as_ptr().cast(), str.as_ptr(), SRCPORT_BEGIN_SZ)
                == 0
            {
                p = os_clearnl(str.as_mut_ptr());
                p = str.as_mut_ptr().add(SRCPORT_BEGIN_SZ);
                (*al_data).srcport = libc::atoi(p);
            } else if libc::strncmp(DSTIP_BEGIN.as_ptr().cast(), str.as_ptr(), DSTIP_BEGIN_SZ) == 0 {
                p = os_clearnl(str.as_mut_ptr());
                p = str.as_mut_ptr().add(DSTIP_BEGIN_SZ);
                os_free_char(&mut (*al_data).dstip);
                (*al_data).dstip = os_strdup(p);
            } else if libc::strncmp(
                DSTPORT_BEGIN.as_ptr().cast(),
                str.as_ptr(),
                DSTPORT_BEGIN_SZ,
            ) == 0
            {
                p = os_clearnl(str.as_mut_ptr());
                p = str.as_mut_ptr().add(DSTPORT_BEGIN_SZ);
                (*al_data).dstport = libc::atoi(p);
            } else if libc::strncmp(USER_BEGIN.as_ptr().cast(), str.as_ptr(), USER_BEGIN_SZ) == 0 {
                p = os_clearnl(str.as_mut_ptr());
                p = str.as_mut_ptr().add(USER_BEGIN_SZ);
                os_free_char(&mut (*al_data).user);
                (*al_data).user = os_strdup(p);
            } else if log_size < LOG_LIMIT {
                p = os_clearnl(str.as_mut_ptr());
                if issyscheck == 1 {
                    if libc::strncmp(
                        str.as_ptr(),
                        INTEGRITY_PREFIX.as_ptr().cast(),
                        INTEGRITY_PREFIX.len() - 1,
                    ) == 0
                    {
                        (*al_data).filename = libc::strdup(str.as_ptr().add(33));
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

    if !error && libc::feof(fp) != 0 && r == 2 {
        return al_data;
    }

    FreeAlertData(al_data);
    libc::clearerr(fp);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const libc::tm,
    flags: c_int,
) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
        (*fileq).fp = ptr::null_mut();
    }
    (*fileq).last_change = 0;
    (*fileq).flags = 0;

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;

    ptr::copy_nonoverlapping(
        S_MONTH[(*p).tm_mon as usize].as_ptr().cast::<c_char>(),
        (*fileq).mon.as_mut_ptr(),
        3,
    );
    libc::memset(
        (*fileq).file_name.as_mut_ptr().cast(),
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const libc::tm,
    timeout: c_uint,
) -> *mut alert_data {
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
    ptr::copy_nonoverlapping(
        S_MONTH[(*p).tm_mon as usize].as_ptr().cast::<c_char>(),
        (*fileq).mon.as_mut_ptr(),
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    let mut time: libc::tm = mem::zeroed();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq: file_queue = mem::zeroed();

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        libc::fprintf(stderr, FILE_QUEUE_INIT_ERR.as_ptr().cast());
        return ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq, &time, timeout);

    if !fq.fp.is_null() {
        libc::fclose(fq.fp);
    }

    al_data
}
