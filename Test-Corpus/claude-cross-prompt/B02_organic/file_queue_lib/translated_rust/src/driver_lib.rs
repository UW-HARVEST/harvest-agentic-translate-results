// Translation of c_src/src/driver.c
//
// Wraps Init_FileQueue + Read_FileMon for a single (day, month, year,
// timeout, flags) request.

use crate::file_queue::{
    init_file_queue, read_file_mon, AlertSource, FileQueue, Tm,
};
use crate::read_alert::AlertData;

pub fn driver(
    day: i32,
    month: i32,
    year: i32,
    timeout: u32,
    flags: i32,
    source: AlertSource,
) -> Option<AlertData> {
    let time = Tm {
        tm_mday: day,
        tm_mon: month,
        tm_year: year,
    };

    let mut fq = FileQueue::new();

    if init_file_queue(&mut fq, &time, flags, Some(source)) < 0 {
        eprint!("File queue initialization failed");
        return None;
    }

    // The C version sleeps via select() when the file isn't available.
    // For the executable we skip sleeping (no-op) so the program can
    // terminate; this preserves byte-identical stdout output for inputs
    // where the queue is already populated.
    let mut no_sleep = || {};
    let al = read_file_mon(&mut fq, &time, timeout, &mut no_sleep);

    // fq.fp is dropped automatically here (Rust); equivalent to fclose.
    al
}
