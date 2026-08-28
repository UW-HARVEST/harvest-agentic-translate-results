#![allow(non_snake_case)]

use libc::{
    FILE, SEEK_CUR, SEEK_END, c_char, c_int, c_long, c_uint, c_void, size_t, stat, time_t, timeval,
    tm,
};
use std::mem;
use std::ptr;

const MAX_FQUEUE: usize = 256;
const FQ_TIMEOUT: time_t = 5;
const OS_MAXSTR: usize = 1024;

const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_FP_SET: c_int = 0x010;

const ALERT_BEGIN: &[u8] = b"** Alert\0";
const RULE_BEGIN: &[u8] = b"Rule: \0";
const SRCIP_BEGIN: &[u8] = b"Src IP: \0";
const SRCPORT_BEGIN: &[u8] = b"Src Port: \0";
const DSTIP_BEGIN: &[u8] = b"Dst IP: \0";
const DSTPORT_BEGIN: &[u8] = b"Dst Port: \0";
const USER_BEGIN: &[u8] = b"User: \0";
const ALERT_MAIL: &[u8] = b"mail\0";

unsafe extern "C" {
    static mut stderr: *mut FILE;
}

#[repr(C)]
pub struct AlertData {
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
pub struct FileQueue {
    pub last_change: time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

const _: [(); 96] = [(); mem::size_of::<AlertData>()];
const _: [(); 440] = [(); mem::size_of::<FileQueue>()];
const _: [(); 288] = [(); mem::offset_of!(FileQueue, fp)];
const _: [(); 296] = [(); mem::offset_of!(FileQueue, f_status)];

#[inline]
const fn cptr(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

unsafe fn report_allocation_failure(message: &[u8]) -> ! {
    unsafe {
        libc::fprintf(stderr, cptr(b"%s\0"), cptr(message));
        libc::exit(libc::EXIT_FAILURE);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: size_t, size: size_t) -> *mut c_void {
    let out = unsafe { libc::calloc(num, size) };
    if out.is_null() {
        unsafe { report_allocation_failure(b"Memory allocation failed in os_calloc\0") };
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: size_t) -> *mut c_void {
    let out = unsafe { libc::realloc(ptr, new_size) };
    if out.is_null() {
        unsafe { report_allocation_failure(b"Memory allocation failed in os_realloc\0") };
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        unsafe { report_allocation_failure(b"NULL string passed to os_strdup\0") };
    }
    let dup = unsafe { libc::strdup(str_) };
    if dup.is_null() {
        unsafe { report_allocation_failure(b"Memory allocation failed in os_strdup\0") };
    }
    dup
}

unsafe fn free_field(field: &mut *mut c_char) {
    if !field.is_null() {
        unsafe { libc::free((*field).cast()) };
        *field = ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FreeAlertData(al_data: *mut AlertData) {
    let alert = unsafe { &mut *al_data };
    unsafe {
        free_field(&mut alert.alertid);
        free_field(&mut alert.date);
        free_field(&mut alert.location);
        free_field(&mut alert.comment);
        free_field(&mut alert.group);
        free_field(&mut alert.srcip);
        free_field(&mut alert.dstip);
        free_field(&mut alert.user);
        free_field(&mut alert.filename);
        libc::free(al_data.cast());
    }
}

unsafe fn clear_newline(value: *mut c_char) {
    let newline = unsafe { libc::strrchr(value, b'\n' as c_int) };
    if !newline.is_null() {
        unsafe { *newline = 0 };
    }
}

unsafe fn replace_strdup(field: &mut *mut c_char, value: *const c_char) {
    unsafe {
        free_field(field);
        *field = os_strdup(value);
    }
}

unsafe fn alert_error(al_data: *mut AlertData, fp: *mut FILE) -> *mut AlertData {
    unsafe {
        FreeAlertData(al_data);
        libc::clearerr(fp);
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn GetAlertData(flag: c_int, fp: *mut FILE) -> *mut AlertData {
    let al_data = unsafe { os_calloc(1, mem::size_of::<AlertData>()) }.cast::<AlertData>();
    let mut state = 0;
    let mut is_syscheck = 0;
    let log_size: size_t = 0;
    let mut str_ = [0 as c_char; OS_MAXSTR + 1];
    str_[OS_MAXSTR] = 0;

    while !unsafe { libc::fgets(str_.as_mut_ptr(), OS_MAXSTR as c_int, fp) }.is_null() {
        if unsafe { libc::strncmp(cptr(ALERT_BEGIN), str_.as_ptr(), 8) } == 0 {
            if state == 2 {
                let offset = -(unsafe { libc::strlen(str_.as_ptr()) } as c_long);
                if unsafe { libc::fseek(fp, offset, SEEK_CUR) } != -1 {
                    return al_data;
                }
                return unsafe { alert_error(al_data, fp) };
            }

            let mut p = unsafe { str_.as_mut_ptr().add(9) };
            let m = unsafe { libc::strstr(p, cptr(b":\0")) };
            if m.is_null() {
                continue;
            }

            let z = unsafe { libc::strlen(p) - libc::strlen(m) };
            let alert = unsafe { &mut *al_data };
            alert.alertid = unsafe { os_realloc(alert.alertid.cast(), z + 1).cast::<c_char>() };
            unsafe {
                libc::strncpy(alert.alertid, p, z);
                *alert.alertid.add(z) = 0;
            }

            p = unsafe { libc::strchr(p, b' ' as c_int) };
            if p.is_null() {
                continue;
            }
            p = unsafe { p.add(1) };

            if flag & CRALERT_MAIL_SET != 0 && unsafe { libc::strncmp(cptr(ALERT_MAIL), p, 4) } != 0
            {
                continue;
            }

            p = unsafe { libc::strchr(p, b'-' as c_int) };
            if !p.is_null() {
                p = unsafe { p.add(1) };
                while unsafe { *p } == b' ' as c_char {
                    p = unsafe { p.add(1) };
                }
                let alert = unsafe { &mut *al_data };
                unsafe {
                    replace_strdup(&mut alert.group, p);
                    clear_newline(alert.group);
                }
                if !alert.group.is_null()
                    && !unsafe { libc::strstr(alert.group, cptr(b"syscheck\0")) }.is_null()
                {
                    is_syscheck = 1;
                }
            }

            state = 1;
            continue;
        }

        if state < 1 {
            continue;
        }

        if state == 1 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            let mut p = unsafe { libc::strchr(str_.as_mut_ptr(), b':' as c_int) };
            if !p.is_null() {
                p = unsafe { libc::strchr(p, b' ' as c_int) };
                if !p.is_null() {
                    unsafe { *p = 0 };
                    p = unsafe { p.add(1) };
                } else {
                    unsafe { libc::perror(cptr(b"date of location not NULL\0")) };
                    return unsafe { alert_error(al_data, fp) };
                }
            }

            let alert = unsafe { &mut *al_data };
            if !alert.date.is_null() || !alert.location.is_null() || p.is_null() {
                unsafe {
                    libc::perror(cptr(b"date or location not NULL or p is NULL\0"));
                }
                return unsafe { alert_error(al_data, fp) };
            }

            alert.date = unsafe { os_strdup(str_.as_ptr()) };
            alert.location = unsafe { os_strdup(p) };
            state = 2;
            continue;
        }

        if unsafe { libc::strncmp(cptr(RULE_BEGIN), str_.as_ptr(), 6) } == 0 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            let mut p = unsafe { str_.as_mut_ptr().add(6) };
            let alert = unsafe { &mut *al_data };
            alert.rule = unsafe { libc::atoi(p) } as c_uint;

            p = unsafe { libc::strchr(p, b' ' as c_int) };
            if !p.is_null() {
                p = unsafe { p.add(1) };
                p = unsafe { libc::strchr(p, b' ' as c_int) };
                if !p.is_null() {
                    p = unsafe { p.add(1) };
                }
            }
            if p.is_null() {
                return unsafe { alert_error(al_data, fp) };
            }

            alert.level = unsafe { libc::atoi(p) } as c_uint;
            p = unsafe { libc::strchr(p, b'\'' as c_int) };
            if p.is_null() {
                return unsafe { alert_error(al_data, fp) };
            }

            p = unsafe { p.add(1) };
            unsafe { replace_strdup(&mut alert.comment, p) };
            p = unsafe { libc::strrchr(alert.comment, b'\'' as c_int) };
            if p.is_null() {
                return unsafe { alert_error(al_data, fp) };
            }
            unsafe { *p = 0 };
        } else if unsafe { libc::strncmp(cptr(SRCIP_BEGIN), str_.as_ptr(), 8) } == 0 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            let p = unsafe { str_.as_ptr().add(8) };
            unsafe { replace_strdup(&mut (*al_data).srcip, p) };
        } else if unsafe { libc::strncmp(cptr(SRCPORT_BEGIN), str_.as_ptr(), 10) } == 0 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            unsafe {
                (*al_data).srcport = libc::atoi(str_.as_ptr().add(10));
            }
        } else if unsafe { libc::strncmp(cptr(DSTIP_BEGIN), str_.as_ptr(), 8) } == 0 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            let p = unsafe { str_.as_ptr().add(8) };
            unsafe { replace_strdup(&mut (*al_data).dstip, p) };
        } else if unsafe { libc::strncmp(cptr(DSTPORT_BEGIN), str_.as_ptr(), 10) } == 0 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            unsafe {
                (*al_data).dstport = libc::atoi(str_.as_ptr().add(10));
            }
        } else if unsafe { libc::strncmp(cptr(USER_BEGIN), str_.as_ptr(), 6) } == 0 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            let p = unsafe { str_.as_ptr().add(6) };
            unsafe { replace_strdup(&mut (*al_data).user, p) };
        } else if log_size < 100 {
            unsafe { clear_newline(str_.as_mut_ptr()) };
            if is_syscheck == 1 {
                if unsafe {
                    libc::strncmp(
                        str_.as_ptr(),
                        cptr(b"Integrity checksum changed for: '\0"),
                        33,
                    )
                } == 0
                {
                    let filename = unsafe { libc::strdup(str_.as_ptr().add(33)) };
                    unsafe {
                        (*al_data).filename = filename;
                    }
                    if !filename.is_null() {
                        let last = unsafe { libc::strlen(filename) - 1 };
                        unsafe { *filename.add(last) = 0 };
                    }
                }
                is_syscheck = 0;
            }
        }
    }

    if unsafe { libc::feof(fp) } != 0 && state == 2 {
        return al_data;
    }

    unsafe {
        FreeAlertData(al_data);
        libc::clearerr(fp);
    }
    ptr::null_mut()
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
        libc::fprintf(stderr, cptr(b"%s\n\0"), buffer.as_ptr());
    }
}

unsafe fn file_sleep() {
    let mut timeout = timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    unsafe {
        libc::select(
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut timeout,
        );
    }
}

unsafe fn get_file_queue(fileq: *mut FileQueue) {
    let queue = unsafe { &mut *fileq };
    queue.file_name[0] = 0;
    queue.file_name[MAX_FQUEUE] = 0;
    let name = if queue.flags & CRALERT_FP_SET != 0 {
        cptr(b"<stdin>\0")
    } else {
        cptr(b"alerts.log\0")
    };
    unsafe {
        libc::snprintf(
            queue.file_name.as_mut_ptr(),
            MAX_FQUEUE,
            cptr(b"%s\0"),
            name,
        );
    }
}

unsafe fn handle_queue(fileq: *mut FileQueue, flags: c_int) -> c_int {
    let queue = unsafe { &mut *fileq };
    if flags & CRALERT_FP_SET == 0 {
        if !queue.fp.is_null() {
            unsafe { libc::fclose(queue.fp) };
            queue.fp = ptr::null_mut();
        }
        queue.fp = unsafe { libc::fopen(queue.file_name.as_ptr(), cptr(b"r\0")) };
        if queue.fp.is_null() {
            return 0;
        }
    }

    if flags & CRALERT_READ_ALL == 0 {
        if queue.fp.is_null() {
            return 0;
        }
        if unsafe { libc::fseek(queue.fp, 0, SEEK_END) } < 0 {
            let errno = unsafe { *libc::__errno_location() };
            unsafe {
                merror(
                    cptr(b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0"),
                    queue.file_name.as_ptr(),
                    errno,
                    libc::strerror(errno),
                );
                libc::fclose(queue.fp);
            }
            queue.fp = ptr::null_mut();
            return -1;
        }
    }

    if !queue.fp.is_null()
        && unsafe { libc::fstat(libc::fileno(queue.fp), &mut queue.f_status) } < 0
    {
        let errno = unsafe { *libc::__errno_location() };
        unsafe {
            merror(
                cptr(b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0"),
                queue.file_name.as_ptr(),
                errno,
                libc::strerror(errno),
            );
            libc::fclose(queue.fp);
        }
        queue.fp = ptr::null_mut();
        return -1;
    }

    queue.last_change = queue.f_status.st_mtime;
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut FileQueue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    let queue = unsafe { &mut *fileq };
    if flags & CRALERT_FP_SET == 0 {
        queue.fp = ptr::null_mut();
    }
    queue.last_change = 0;
    queue.flags = 0;
    queue.day = unsafe { (*p).tm_mday };
    queue.year = unsafe { (*p).tm_year } + 1900;

    const MONTHS: [&[u8; 4]; 12] = [
        b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
        b"Oct\0", b"Nov\0", b"Dec\0",
    ];
    let month = unsafe { MONTHS.get_unchecked((*p).tm_mon as usize) };
    unsafe {
        libc::strncpy(queue.mon.as_mut_ptr(), cptr(*month), 3);
        libc::memset(queue.file_name.as_mut_ptr().cast(), 0, MAX_FQUEUE + 1);
    }

    queue.flags = flags;
    unsafe { get_file_queue(fileq) };
    if unsafe { handle_queue(fileq, queue.flags) } < 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut FileQueue,
    p: *const tm,
    timeout: c_uint,
) -> *mut AlertData {
    let queue = unsafe { &mut *fileq };
    if queue.fp.is_null() {
        if unsafe { handle_queue(fileq, 0) } != 1 {
            unsafe { file_sleep() };
            return ptr::null_mut();
        }
    }
    if queue.fp.is_null() {
        return ptr::null_mut();
    }

    let mut al_data = unsafe { GetAlertData(queue.flags, queue.fp) };
    if !al_data.is_null() {
        return al_data;
    }

    queue.day = unsafe { (*p).tm_mday };
    queue.year = unsafe { (*p).tm_year } + 1900;
    const MONTHS: [&[u8; 4]; 12] = [
        b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
        b"Oct\0", b"Nov\0", b"Dec\0",
    ];
    let month = unsafe { MONTHS.get_unchecked((*p).tm_mon as usize) };
    unsafe { libc::strncpy(queue.mon.as_mut_ptr(), cptr(*month), 3) };

    unsafe { get_file_queue(fileq) };
    if unsafe { handle_queue(fileq, 0) } != 1 {
        unsafe { file_sleep() };
        return ptr::null_mut();
    }

    let mut i = 0;
    while i < timeout {
        al_data = unsafe { GetAlertData(queue.flags, queue.fp) };
        if !al_data.is_null() {
            return al_data;
        }
        i += 1;
        unsafe { file_sleep() };
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
    let mut time: tm = unsafe { mem::zeroed() };
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq: FileQueue = unsafe { mem::zeroed() };
    if unsafe { Init_FileQueue(&mut fq, &time, flags) } < 0 {
        unsafe {
            libc::fprintf(
                stderr,
                cptr(b"%s\0"),
                cptr(b"File queue initialization failed\0"),
            );
        }
        return ptr::null_mut();
    }

    let al_data = unsafe { Read_FileMon(&mut fq, &time, timeout) };
    if !fq.fp.is_null() {
        unsafe { libc::fclose(fq.fp) };
    }
    al_data
}
