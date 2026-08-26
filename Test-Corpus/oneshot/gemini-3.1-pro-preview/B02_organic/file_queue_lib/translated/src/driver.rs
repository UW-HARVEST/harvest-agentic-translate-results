use libc::{c_int, c_uint, c_char};
use crate::read_alert::alert_data;
use crate::file_queue::{file_queue, Init_FileQueue, Read_FileMon};

#[unsafe(no_mangle)]
pub extern "C" fn driver(day: c_int, month: c_int, year: c_int, timeout: c_uint, flags: c_int) -> *mut alert_data {
    unsafe {
        let mut time: libc::tm = std::mem::zeroed();
        time.tm_mday = day;
        time.tm_mon = month;
        time.tm_year = year;

        let mut fq: file_queue = std::mem::zeroed();

        if Init_FileQueue(&mut fq, &time, flags) < 0 {
            libc::fprintf(libc::stderr, b"File queue initialization failed\0".as_ptr() as *const c_char);
            return std::ptr::null_mut();
        }

        let al_data = Read_FileMon(&mut fq, &time, timeout);

        if !fq.fp.is_null() {
            libc::fclose(fq.fp);
        }
        al_data
    }
}
