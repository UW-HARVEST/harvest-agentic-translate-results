use crate::file_queue::{file_queue, Init_FileQueue, Read_FileMon};
use crate::read_alert::alert_data;
use libc::tm;
use std::mem;
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn driver(day: i32, month: i32, year: i32, timeout: u32, flags: i32) -> *mut alert_data {
    let mut time: tm = unsafe { mem::zeroed() };
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;

    let mut fq: file_queue = unsafe { mem::zeroed() };

    let init = Init_FileQueue(&mut fq as *mut file_queue, &time as *const tm, flags);
    if init < 0 {
        eprint!("File queue initialization failed");
        return ptr::null_mut();
    }

    let al_data = Read_FileMon(&mut fq as *mut file_queue, &time as *const tm, timeout);

    if !fq.fp.is_null() {
        unsafe {
            libc::fclose(fq.fp);
        }
    }

    al_data
}
