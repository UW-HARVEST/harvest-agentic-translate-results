//! Translation of `c_src/src/driver.c`.

use core::ffi::{c_int, c_uint};
use core::ptr;

use crate::cbind::*;
use crate::file_queue::{file_queue, Init_FileQueue, Read_FileMon};
use crate::read_alert::alert_data;

/// ```c
/// // Main entrypoint for this library
/// alert_data* driver(int day, int month, int year, unsigned int timeout, int flags)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    // `struct tm time = {0};`
    let mut time: tm = core::mem::zeroed();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    // `file_queue fq; memset(&fq, 0, sizeof(file_queue));`
    let mut fq: file_queue = core::mem::zeroed();
    memset(
        &mut fq as *mut file_queue as *mut core::ffi::c_void,
        0,
        core::mem::size_of::<file_queue>(),
    );

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        fprintf(stderr, cs(b"File queue initialization failed\0"));
        return ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq, &time, timeout);

    if !fq.fp.is_null() {
        fclose(fq.fp);
    }
    al_data
}
