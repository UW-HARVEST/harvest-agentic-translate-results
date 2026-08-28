//! Translation of `c_src/src/driver.c`.

use core::ffi::{c_int, c_uint};

use crate::file_queue::{file_queue, Init_FileQueue, Read_FileMon};
use crate::read_alert::alert_data;
use crate::shared::stderr_str;

// Main entrypoint for this library
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    let mut time: libc::tm = core::mem::zeroed();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq: file_queue = core::mem::zeroed();

    if Init_FileQueue(&mut fq, &time, flags) < 0 {
        stderr_str(c"File queue initialization failed");
        return core::ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq, &time, timeout);

    if !fq.fp.is_null() {
        libc::fclose(fq.fp);
    }
    al_data
}
