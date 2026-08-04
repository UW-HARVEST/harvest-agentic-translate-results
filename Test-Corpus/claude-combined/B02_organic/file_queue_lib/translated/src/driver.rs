//! Translation of `src/driver.c`.

use std::ffi::c_void;
use std::os::raw::{c_int, c_uint};

use crate::file_queue::{file_queue, Init_FileQueue, Read_FileMon};
use crate::read_alert::alert_data;

extern "C" {
    fn memset(dst: *mut c_void, c: c_int, n: libc::size_t) -> *mut c_void;
    fn fclose(stream: *mut libc::FILE) -> c_int;
    fn fdopen(fd: c_int, mode: *const std::os::raw::c_char) -> *mut libc::FILE;
    fn fputs(s: *const std::os::raw::c_char, stream: *mut libc::FILE) -> c_int;
}

/// Main entrypoint of the library: drive the file queue for one read.
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
    memset(
        &mut fq as *mut file_queue as *mut c_void,
        0,
        std::mem::size_of::<file_queue>(),
    );

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        // fprintf(stderr, "File queue initialization failed");
        let mode = b"w\0".as_ptr() as *const std::os::raw::c_char;
        let stderr_fp = fdopen(2, mode);
        if !stderr_fp.is_null() {
            let msg = b"File queue initialization failed\0";
            fputs(msg.as_ptr() as *const std::os::raw::c_char, stderr_fp);
        }
        return std::ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq, &time, timeout);

    if !fq.fp.is_null() {
        fclose(fq.fp);
    }
    al_data
}
