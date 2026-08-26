use std::ffi::{c_char, c_int, c_long, c_uint, c_void};
use std::mem::size_of;
use std::ptr;

const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: c_long = 5;
const OS_MAXSTR: usize = 1024;

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_FP_SET: c_int = 0x010;

const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;

#[repr(C)]
pub struct CFile {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

#[repr(C)]
pub struct Stat {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    __pad0: c_int,
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
    __reserved: [i64; 3],
}

#[repr(C)]
pub struct FileQueue {
    last_change: i64,
    year: c_int,
    day: c_int,
    flags: c_int,
    mon: [c_char; 4],
    file_name: [c_char; MAX_FQUEUE + 1],
    fp: *mut CFile,
    f_status: Stat,
}

#[repr(C)]
pub struct AlertData {
    rule: c_uint,
    level: c_uint,
    alertid: *mut c_char,
    date: *mut c_char,
    location: *mut c_char,
    comment: *mut c_char,
    group: *mut c_char,
    srcip: *mut c_char,
    srcport: c_int,
    dstip: *mut c_char,
    dstport: c_int,
    user: *mut c_char,
    filename: *mut c_char,
}

const _: () = {
    assert!(size_of::<Tm>() == 56);
    assert!(size_of::<Stat>() == 144);
    assert!(std::mem::offset_of!(Stat, st_mtime) == 88);
    assert!(size_of::<FileQueue>() == 440);
    assert!(std::mem::offset_of!(FileQueue, fp) == 288);
    assert!(std::mem::offset_of!(FileQueue, f_status) == 296);
    assert!(size_of::<AlertData>() == 96);
    assert!(std::mem::offset_of!(AlertData, srcport) == 56);
    assert!(std::mem::offset_of!(AlertData, filename) == 88);
};

#[repr(C)]
struct Timeval {
    tv_sec: c_long,
    tv_usec: c_long,
}

unsafe extern "C" {
    static mut stderr: *mut CFile;

    fn calloc(num: usize, size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(value: *const c_char) -> *mut c_char;
    fn exit(status: c_int) -> !;

    fn strlen(value: *const c_char) -> usize;
    fn strchr(value: *const c_char, ch: c_int) -> *mut c_char;
    fn strrchr(value: *const c_char, ch: c_int) -> *mut c_char;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn strncmp(left: *const c_char, right: *const c_char, count: usize) -> c_int;
    fn strncpy(dest: *mut c_char, src: *const c_char, count: usize) -> *mut c_char;
    fn memset(dest: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn atoi(value: *const c_char) -> c_int;
    fn strerror(error: c_int) -> *mut c_char;

    fn fgets(buffer: *mut c_char, count: c_int, stream: *mut CFile) -> *mut c_char;
    fn fseek(stream: *mut CFile, offset: c_long, whence: c_int) -> c_int;
    fn feof(stream: *mut CFile) -> c_int;
    fn clearerr(stream: *mut CFile);
    fn fclose(stream: *mut CFile) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut CFile;
    fn fileno(stream: *mut CFile) -> c_int;
    fn fstat(fd: c_int, status: *mut Stat) -> c_int;
    fn perror(message: *const c_char);
    fn fprintf(stream: *mut CFile, format: *const c_char, ...) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn select(
        nfds: c_int,
        readfds: *mut c_void,
        writefds: *mut c_void,
        exceptfds: *mut c_void,
        timeout: *mut Timeval,
    ) -> c_int;
    fn __errno_location() -> *mut c_int;
}

const ALERT_BEGIN: &[u8] = b"** Alert\0";
const RULE_BEGIN: &[u8] = b"Rule: \0";
const SRCIP_BEGIN: &[u8] = b"Src IP: \0";
const SRCPORT_BEGIN: &[u8] = b"Src Port: \0";
const DSTIP_BEGIN: &[u8] = b"Dst IP: \0";
const DSTPORT_BEGIN: &[u8] = b"Dst Port: \0";
const USER_BEGIN: &[u8] = b"User: \0";
const ALERT_MAIL: &[u8] = b"mail\0";
const SYSCHECK: &[u8] = b"syscheck\0";
const INTEGRITY_CHANGED: &[u8] = b"Integrity checksum changed for: '\0";

const ALERTS_DAILY: &[u8] = b"alerts.log\0";
const STDIN_NAME: &[u8] = b"<stdin>\0";
const READ_MODE: &[u8] = b"r\0";
const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] = b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

const MONTHS: [[u8; 4]; 12] = [
    *b"Jan\0", *b"Feb\0", *b"Mar\0", *b"Apr\0", *b"May\0", *b"Jun\0", *b"Jul\0", *b"Aug\0",
    *b"Sep\0", *b"Oct\0", *b"Nov\0", *b"Dec\0",
];

#[inline]
unsafe fn clean_newline(value: *mut c_char) {
    let newline = unsafe { strrchr(value, b'\n' as c_int) };
    if !newline.is_null() {
        unsafe { *newline = 0 };
    }
}

#[inline]
unsafe fn free_field(field: &mut *mut c_char) {
    if !field.is_null() {
        unsafe { free((*field).cast()) };
        *field = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let output = unsafe { calloc(num, size) };
    if output.is_null() {
        unsafe {
            fprintf(stderr, c"Memory allocation failed in os_calloc".as_ptr());
            exit(1);
        }
    }
    output
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(input: *mut c_void, new_size: usize) -> *mut c_void {
    let output = unsafe { realloc(input, new_size) };
    if output.is_null() {
        unsafe {
            fprintf(stderr, c"Memory allocation failed in os_realloc".as_ptr());
            exit(1);
        }
    }
    output
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(value: *const c_char) -> *mut c_char {
    if value.is_null() {
        unsafe {
            fprintf(stderr, c"NULL string passed to os_strdup".as_ptr());
            exit(1);
        }
    }

    let duplicate = unsafe { strdup(value) };
    if duplicate.is_null() {
        unsafe {
            fprintf(stderr, c"Memory allocation failed in os_strdup".as_ptr());
            exit(1);
        }
    }
    duplicate
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    error_template: *const c_char,
    file_name: *const c_char,
    error: c_int,
    error_message: *const c_char,
) {
    let mut buffer = [0 as c_char; 256];
    unsafe {
        snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            error_template,
            file_name,
            error,
            error_message,
        );
        fprintf(stderr, c"%s\n".as_ptr(), buffer.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(data: *mut AlertData) {
    unsafe {
        free_field(&mut (*data).alertid);
        free_field(&mut (*data).date);
        free_field(&mut (*data).location);
        free_field(&mut (*data).comment);
        free_field(&mut (*data).group);
        free_field(&mut (*data).srcip);
        free_field(&mut (*data).dstip);
        free_field(&mut (*data).user);
        free_field(&mut (*data).filename);
        free(data.cast());
    }
}

#[inline]
unsafe fn alert_error(data: *mut AlertData, fp: *mut CFile) -> *mut AlertData {
    unsafe {
        FreeAlertData(data);
        clearerr(fp);
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut CFile) -> *mut AlertData {
    let data = unsafe { os_calloc(1, size_of::<AlertData>()).cast::<AlertData>() };
    let mut state = 0;
    let mut is_syscheck = 0;
    let log_size = 0usize;
    let mut string = [0 as c_char; OS_MAXSTR + 1];
    string[OS_MAXSTR] = 0;

    while !unsafe { fgets(string.as_mut_ptr(), OS_MAXSTR as c_int, fp) }.is_null() {
        if unsafe {
            strncmp(
                ALERT_BEGIN.as_ptr().cast(),
                string.as_ptr(),
                ALERT_BEGIN.len() - 1,
            )
        } == 0
        {
            if state == 2 {
                let offset = -(unsafe { strlen(string.as_ptr()) } as c_long);
                if unsafe { fseek(fp, offset, SEEK_CUR) } != -1 {
                    return data;
                }
                return unsafe { alert_error(data, fp) };
            }

            let mut p = unsafe { string.as_mut_ptr().add(ALERT_BEGIN.len()) };
            let marker = unsafe { strstr(p, c":".as_ptr()) };
            if marker.is_null() {
                continue;
            }

            let length = unsafe { strlen(p) - strlen(marker) };
            unsafe {
                (*data).alertid = os_realloc((*data).alertid.cast(), length + 1).cast::<c_char>();
                strncpy((*data).alertid, p, length);
                *(*data).alertid.add(length) = 0;
            }

            p = unsafe { strchr(p, b' ' as c_int) };
            if p.is_null() {
                continue;
            }
            p = unsafe { p.add(1) };

            if flag & CRALERT_MAIL_SET != 0
                && unsafe { strncmp(ALERT_MAIL.as_ptr().cast(), p, ALERT_MAIL.len() - 1) } != 0
            {
                continue;
            }

            p = unsafe { strchr(p, b'-' as c_int) };
            if !p.is_null() {
                p = unsafe { p.add(1) };
                while unsafe { *p } == b' ' as c_char {
                    p = unsafe { p.add(1) };
                }
                unsafe {
                    free_field(&mut (*data).group);
                    (*data).group = os_strdup(p);
                    clean_newline((*data).group);
                    if !(*data).group.is_null()
                        && !strstr((*data).group, SYSCHECK.as_ptr().cast()).is_null()
                    {
                        is_syscheck = 1;
                    }
                }
            }

            state = 1;
            continue;
        }

        if state < 1 {
            continue;
        }

        if state == 1 {
            unsafe { clean_newline(string.as_mut_ptr()) };

            let mut p = unsafe { strchr(string.as_ptr(), b':' as c_int) };
            if !p.is_null() {
                p = unsafe { strchr(p, b' ' as c_int) };
                if !p.is_null() {
                    unsafe {
                        *p = 0;
                        p = p.add(1);
                    }
                } else {
                    unsafe { perror(c"date of location not NULL".as_ptr()) };
                    return unsafe { alert_error(data, fp) };
                }
            }

            if unsafe { !(*data).date.is_null() || !(*data).location.is_null() } || p.is_null() {
                unsafe { perror(c"date or location not NULL or p is NULL".as_ptr()) };
                return unsafe { alert_error(data, fp) };
            }

            unsafe {
                (*data).date = os_strdup(string.as_ptr());
                (*data).location = os_strdup(p);
            }
            state = 2;
            continue;
        }

        if unsafe {
            strncmp(
                RULE_BEGIN.as_ptr().cast(),
                string.as_ptr(),
                RULE_BEGIN.len() - 1,
            )
        } == 0
        {
            unsafe { clean_newline(string.as_mut_ptr()) };
            let mut p = unsafe { string.as_mut_ptr().add(RULE_BEGIN.len() - 1) };
            unsafe { (*data).rule = atoi(p) as c_uint };

            p = unsafe { strchr(p, b' ' as c_int) };
            if !p.is_null() {
                p = unsafe { p.add(1) };
                p = unsafe { strchr(p, b' ' as c_int) };
                if !p.is_null() {
                    p = unsafe { p.add(1) };
                }
            }
            if p.is_null() {
                return unsafe { alert_error(data, fp) };
            }

            unsafe { (*data).level = atoi(p) as c_uint };
            p = unsafe { strchr(p, b'\'' as c_int) };
            if p.is_null() {
                return unsafe { alert_error(data, fp) };
            }

            p = unsafe { p.add(1) };
            unsafe {
                free_field(&mut (*data).comment);
                (*data).comment = os_strdup(p);
            }
            p = unsafe { strrchr((*data).comment, b'\'' as c_int) };
            if p.is_null() {
                return unsafe { alert_error(data, fp) };
            }
            unsafe { *p = 0 };
        } else if unsafe {
            strncmp(
                SRCIP_BEGIN.as_ptr().cast(),
                string.as_ptr(),
                SRCIP_BEGIN.len() - 1,
            )
        } == 0
        {
            unsafe {
                clean_newline(string.as_mut_ptr());
                free_field(&mut (*data).srcip);
                (*data).srcip = os_strdup(string.as_ptr().add(SRCIP_BEGIN.len() - 1));
            }
        } else if unsafe {
            strncmp(
                SRCPORT_BEGIN.as_ptr().cast(),
                string.as_ptr(),
                SRCPORT_BEGIN.len() - 1,
            )
        } == 0
        {
            unsafe {
                clean_newline(string.as_mut_ptr());
                (*data).srcport = atoi(string.as_ptr().add(SRCPORT_BEGIN.len() - 1));
            }
        } else if unsafe {
            strncmp(
                DSTIP_BEGIN.as_ptr().cast(),
                string.as_ptr(),
                DSTIP_BEGIN.len() - 1,
            )
        } == 0
        {
            unsafe {
                clean_newline(string.as_mut_ptr());
                free_field(&mut (*data).dstip);
                (*data).dstip = os_strdup(string.as_ptr().add(DSTIP_BEGIN.len() - 1));
            }
        } else if unsafe {
            strncmp(
                DSTPORT_BEGIN.as_ptr().cast(),
                string.as_ptr(),
                DSTPORT_BEGIN.len() - 1,
            )
        } == 0
        {
            unsafe {
                clean_newline(string.as_mut_ptr());
                (*data).dstport = atoi(string.as_ptr().add(DSTPORT_BEGIN.len() - 1));
            }
        } else if unsafe {
            strncmp(
                USER_BEGIN.as_ptr().cast(),
                string.as_ptr(),
                USER_BEGIN.len() - 1,
            )
        } == 0
        {
            unsafe {
                clean_newline(string.as_mut_ptr());
                free_field(&mut (*data).user);
                (*data).user = os_strdup(string.as_ptr().add(USER_BEGIN.len() - 1));
            }
        } else if log_size < 100 {
            unsafe { clean_newline(string.as_mut_ptr()) };
            if is_syscheck == 1 {
                if unsafe {
                    strncmp(
                        string.as_ptr(),
                        INTEGRITY_CHANGED.as_ptr().cast(),
                        INTEGRITY_CHANGED.len() - 1,
                    )
                } == 0
                {
                    unsafe {
                        (*data).filename = strdup(string.as_ptr().add(INTEGRITY_CHANGED.len() - 1));
                        if !(*data).filename.is_null() {
                            let length = strlen((*data).filename);
                            *(*data).filename.add(length.wrapping_sub(1)) = 0;
                        }
                    }
                }
                is_syscheck = 0;
            }
        }
    }

    if unsafe { feof(fp) } != 0 && state == 2 {
        return data;
    }
    unsafe { alert_error(data, fp) }
}

unsafe fn file_sleep() {
    let mut timeout = Timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    unsafe {
        select(
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut timeout,
        );
    }
}

unsafe fn get_file_queue(file_queue: *mut FileQueue) {
    unsafe {
        (*file_queue).file_name[0] = 0;
        (*file_queue).file_name[MAX_FQUEUE] = 0;
        let name = if (*file_queue).flags & CRALERT_FP_SET != 0 {
            STDIN_NAME.as_ptr()
        } else {
            ALERTS_DAILY.as_ptr()
        };
        snprintf(
            (*file_queue).file_name.as_mut_ptr(),
            MAX_FQUEUE,
            c"%s".as_ptr(),
            name,
        );
    }
}

unsafe fn handle_queue(file_queue: *mut FileQueue, flags: c_int) -> c_int {
    unsafe {
        if flags & CRALERT_FP_SET == 0 {
            if !(*file_queue).fp.is_null() {
                fclose((*file_queue).fp);
                (*file_queue).fp = ptr::null_mut();
            }

            (*file_queue).fp = fopen((*file_queue).file_name.as_ptr(), READ_MODE.as_ptr().cast());
            if (*file_queue).fp.is_null() {
                return 0;
            }
        }

        if flags & CRALERT_READ_ALL == 0 {
            if (*file_queue).fp.is_null() {
                return 0;
            }
            if fseek((*file_queue).fp, 0, SEEK_END) < 0 {
                let error = *__errno_location();
                merror(
                    FSEEK_ERROR.as_ptr().cast(),
                    (*file_queue).file_name.as_ptr(),
                    error,
                    strerror(error),
                );
                fclose((*file_queue).fp);
                (*file_queue).fp = ptr::null_mut();
                return -1;
            }
        }

        if !(*file_queue).fp.is_null()
            && fstat(fileno((*file_queue).fp), &mut (*file_queue).f_status) < 0
        {
            let error = *__errno_location();
            merror(
                FSTAT_ERROR.as_ptr().cast(),
                (*file_queue).file_name.as_ptr(),
                error,
                strerror(error),
            );
            fclose((*file_queue).fp);
            (*file_queue).fp = ptr::null_mut();
            return -1;
        }

        (*file_queue).last_change = (*file_queue).f_status.st_mtime;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    file_queue: *mut FileQueue,
    time: *const Tm,
    flags: c_int,
) -> c_int {
    unsafe {
        if flags & CRALERT_FP_SET == 0 {
            (*file_queue).fp = ptr::null_mut();
        }
        (*file_queue).last_change = 0;
        (*file_queue).flags = 0;
        (*file_queue).day = (*time).tm_mday;
        (*file_queue).year = (*time).tm_year + 1900;

        let month = MONTHS.as_ptr().offset((*time).tm_mon as isize);
        strncpy((*file_queue).mon.as_mut_ptr(), (*month).as_ptr().cast(), 3);
        memset(
            (*file_queue).file_name.as_mut_ptr().cast(),
            0,
            MAX_FQUEUE + 1,
        );
        (*file_queue).flags = flags;
        get_file_queue(file_queue);

        if handle_queue(file_queue, (*file_queue).flags) < 0 {
            return -1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    file_queue: *mut FileQueue,
    time: *const Tm,
    timeout: c_uint,
) -> *mut AlertData {
    unsafe {
        if (*file_queue).fp.is_null() {
            if handle_queue(file_queue, 0) != 1 {
                file_sleep();
                return ptr::null_mut();
            }
        }
        if (*file_queue).fp.is_null() {
            return ptr::null_mut();
        }

        let mut data = GetAlertData((*file_queue).flags, (*file_queue).fp);
        if !data.is_null() {
            return data;
        }

        (*file_queue).day = (*time).tm_mday;
        (*file_queue).year = (*time).tm_year + 1900;
        let month = MONTHS.as_ptr().offset((*time).tm_mon as isize);
        strncpy((*file_queue).mon.as_mut_ptr(), (*month).as_ptr().cast(), 3);
        get_file_queue(file_queue);

        if handle_queue(file_queue, 0) != 1 {
            file_sleep();
            return ptr::null_mut();
        }

        let mut attempt = 0;
        while attempt < timeout {
            data = GetAlertData((*file_queue).flags, (*file_queue).fp);
            if !data.is_null() {
                return data;
            }
            attempt += 1;
            file_sleep();
        }
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
) -> *mut AlertData {
    let mut time: Tm = unsafe { std::mem::zeroed() };
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut file_queue: FileQueue = unsafe { std::mem::zeroed() };
    if unsafe { Init_FileQueue(&mut file_queue, &time, flags) } < 0 {
        unsafe { fprintf(stderr, c"File queue initialization failed".as_ptr()) };
        return ptr::null_mut();
    }

    let data = unsafe { Read_FileMon(&mut file_queue, &time, timeout) };
    if !file_queue.fp.is_null() {
        unsafe { fclose(file_queue.fp) };
    }
    data
}
