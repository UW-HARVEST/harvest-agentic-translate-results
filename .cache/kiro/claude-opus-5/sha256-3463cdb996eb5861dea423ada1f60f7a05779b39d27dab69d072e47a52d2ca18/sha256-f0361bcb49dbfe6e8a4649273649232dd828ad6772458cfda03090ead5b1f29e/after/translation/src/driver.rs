//! Translation of `c_src/src/driver.c`.

#![allow(non_snake_case)]

use core::ffi::{c_int, c_uint, c_void};
use core::mem::MaybeUninit;
use core::ptr;

use crate::cbind::*;
use crate::file_queue::{Init_FileQueue, Read_FileMon, file_queue};
use crate::read_alert::alert_data;

/// `alert_data *driver(int day, int month, int year, unsigned int timeout, int flags)`
///
/// Main entrypoint for this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    unsafe {
        let mut time: tm = core::mem::zeroed();
        time.tm_mday = day;
        time.tm_mon = month;
        time.tm_year = year;

        let mut fq: MaybeUninit<file_queue> = MaybeUninit::uninit();
        memset(
            fq.as_mut_ptr() as *mut c_void,
            0,
            size_of::<file_queue>(),
        );
        let fq = fq.assume_init_mut();

        if Init_FileQueue(fq, &time, flags) < 0 {
            fputs_stderr(b"File queue initialization failed\0");
            return ptr::null_mut();
        }

        let al_data = Read_FileMon(fq, &time, timeout);

        if !fq.fp.is_null() {
            fclose(fq.fp);
        }
        al_data
    }
}
