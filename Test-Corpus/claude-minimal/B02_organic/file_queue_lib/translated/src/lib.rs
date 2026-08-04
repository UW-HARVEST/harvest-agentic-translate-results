//! Rust translation of the C driver library that monitors an alerts file
//! and parses alert records.
//!
//! The public C ABI entry point is [`driver`], matching the original
//! `driver.c` function signature.

use libc::{c_int, c_uint};

pub mod read_alert;
pub mod file_queue;

pub use read_alert::{alert_data, FreeAlertData, GetAlertData};
pub use file_queue::{Init_FileQueue, Read_FileMon};

use file_queue::file_queue as FileQueueT;

/// `struct tm`-like representation that we pass internally.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
}

/// Main entry point for this library, mirroring the C `driver` function.
///
/// Returns a heap-allocated `alert_data` (via `Box::into_raw`) or null on
/// failure.
#[unsafe(no_mangle)]
pub extern "C" fn driver(
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> *mut alert_data {
    let mut time = Tm::default();
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq = FileQueueT::default();

    unsafe {
        if Init_FileQueue(&mut fq, &time, flags) < 0 {
            eprintln!("File queue initialization failed");
            return std::ptr::null_mut();
        }

        let al_data = Read_FileMon(&mut fq, &time, timeout);

        if !fq.fp.is_null() {
            libc::fclose(fq.fp);
        }

        al_data
    }
}
