//! Translation of `src/file-queue.c`.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

use crate::read_alert::{alert_data, GetAlertData, CRALERT_FP_SET, CRALERT_READ_ALL};

// ---- Types mirroring the C struct ----------------------------------------

pub const MAX_FQUEUE: usize = 256;
pub const FQ_TIMEOUT: libc::c_long = 5;

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

// ---- libc imports ---------------------------------------------------------

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut libc::FILE;
    fn fclose(stream: *mut libc::FILE) -> c_int;
    fn fseek(stream: *mut libc::FILE, offset: libc::c_long, whence: c_int) -> c_int;
    fn fileno(stream: *mut libc::FILE) -> c_int;
    fn fstat(fd: c_int, buf: *mut libc::stat) -> c_int;
    fn select(
        nfds: c_int,
        readfds: *mut libc::fd_set,
        writefds: *mut libc::fd_set,
        exceptfds: *mut libc::fd_set,
        timeout: *mut libc::timeval,
    ) -> c_int;
    fn snprintf(s: *mut c_char, n: libc::size_t, fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, fmt: *const c_char, ...) -> c_int;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: libc::size_t) -> *mut c_char;
    fn memset(dst: *mut c_void, c: c_int, n: libc::size_t) -> *mut c_void;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn fdopen(fd: c_int, mode: *const c_char) -> *mut libc::FILE;
    fn __errno_location() -> *mut c_int;
}

const SEEK_END: c_int = 2;

const ALERTS_DAILY: &[u8] = b"alerts.log\0";
const STDIN_NAME: &[u8] = b"<stdin>\0";

// Error templates (with NUL terminators for printf-family).
const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] =
    b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

// Translation table month -> 3-char abbreviation.
static S_MONTH: [&[u8; 4]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
    b"Oct\0", b"Nov\0", b"Dec\0",
];

/// `merror` from file-queue.c — emit a formatted message to stderr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer: [c_char; 256] = [0; 256];
    snprintf(buffer.as_mut_ptr(), 256, err_template, file_name, err, err_msg);
    // fprintf(stderr, "%s\n", buffer);
    let mode = b"w\0".as_ptr() as *const c_char;
    let stderr_fp = fdopen(2, mode);
    if !stderr_fp.is_null() {
        let fmt = b"%s\n\0";
        fprintf(stderr_fp, fmt.as_ptr() as *const c_char, buffer.as_ptr());
    }
}

/// Sleep for `FQ_TIMEOUT` seconds via select(0, NULL, NULL, NULL, &tv).
unsafe fn file_sleep() {
    let mut fp_timeout = libc::timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };
    select(
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        &mut fp_timeout,
    );
}

/// Get the file queue file name for that specific hour.
unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    let f = &mut *fileq;
    f.file_name[0] = 0;
    f.file_name[MAX_FQUEUE] = 0;

    let src: *const c_char = if (f.flags & CRALERT_FP_SET) != 0 {
        STDIN_NAME.as_ptr() as *const c_char
    } else {
        ALERTS_DAILY.as_ptr() as *const c_char
    };

    let fmt = b"%s\0";
    snprintf(
        f.file_name.as_mut_ptr(),
        MAX_FQUEUE,
        fmt.as_ptr() as *const c_char,
        src,
    );
}

/// Re-handle the file queue.
unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    let f = &mut *fileq;

    // Close if open
    if (flags & CRALERT_FP_SET) == 0 {
        if !f.fp.is_null() {
            fclose(f.fp);
            f.fp = std::ptr::null_mut();
        }

        // Open the file
        let mode = b"r\0";
        f.fp = fopen(f.file_name.as_ptr(), mode.as_ptr() as *const c_char);
        if f.fp.is_null() {
            return 0;
        }
    }

    // Seek to end
    if (flags & CRALERT_READ_ALL) == 0 {
        if f.fp.is_null() {
            return 0;
        }

        if fseek(f.fp, 0, SEEK_END) < 0 {
            let errno = *__errno_location();
            merror(
                FSEEK_ERROR.as_ptr() as *const c_char,
                f.file_name.as_ptr(),
                errno,
                strerror(errno),
            );
            fclose(f.fp);
            f.fp = std::ptr::null_mut();
            return -1;
        }
    }

    // File change time
    if !f.fp.is_null() {
        if fstat(fileno(f.fp), &mut f.f_status) < 0 {
            let errno = *__errno_location();
            merror(
                FSTAT_ERROR.as_ptr() as *const c_char,
                f.file_name.as_ptr(),
                errno,
                strerror(errno),
            );
            fclose(f.fp);
            f.fp = std::ptr::null_mut();
            return -1;
        }
    }

    f.last_change = f.f_status.st_mtime;
    1
}

/// Initiate the file monitoring queue.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const libc::tm,
    flags: c_int,
) -> c_int {
    let f = &mut *fileq;

    if (flags & CRALERT_FP_SET) == 0 {
        f.fp = std::ptr::null_mut();
    }
    f.last_change = 0;
    f.flags = 0;

    f.day = (*p).tm_mday;
    f.year = (*p).tm_year + 1900;

    let mon_idx = (*p).tm_mon as usize;
    // Note: out-of-range tm_mon would index OOB in C too — replicate that
    // (panic in Rust would be a difference in observable behavior, but we
    // keep this safe via bounds-check for sanity; real callers always pass
    // 0..=11).
    let month_str: &[u8; 4] = S_MONTH[mon_idx];
    strncpy(
        f.mon.as_mut_ptr(),
        month_str.as_ptr() as *const c_char,
        3,
    );

    memset(
        f.file_name.as_mut_ptr() as *mut c_void,
        0,
        MAX_FQUEUE + 1,
    );

    f.flags = flags;

    GetFile_Queue(fileq);

    if Handle_Queue(fileq, f.flags) < 0 {
        return -1;
    }

    0
}

/// Read from the monitored file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const libc::tm,
    timeout: c_uint,
) -> *mut alert_data {
    let f = &mut *fileq;
    let mut i: c_uint = 0;

    // If the file queue is not available, try to access it
    if f.fp.is_null() {
        if Handle_Queue(fileq, 0) != 1 {
            file_sleep();
            return std::ptr::null_mut();
        }
    }

    if f.fp.is_null() {
        return std::ptr::null_mut();
    }

    let al_data = GetAlertData(f.flags, f.fp);
    if !al_data.is_null() {
        return al_data;
    }

    f.day = (*p).tm_mday;
    f.year = (*p).tm_year + 1900;
    let mon_idx = (*p).tm_mon as usize;
    let month_str: &[u8; 4] = S_MONTH[mon_idx];
    strncpy(
        f.mon.as_mut_ptr(),
        month_str.as_ptr() as *const c_char,
        3,
    );

    GetFile_Queue(fileq);

    if Handle_Queue(fileq, 0) != 1 {
        file_sleep();
        return std::ptr::null_mut();
    }

    // Try up to `timeout` times
    while i < timeout {
        let al_data = GetAlertData(f.flags, f.fp);
        if !al_data.is_null() {
            return al_data;
        }
        i += 1;
        file_sleep();
    }

    std::ptr::null_mut()
}
