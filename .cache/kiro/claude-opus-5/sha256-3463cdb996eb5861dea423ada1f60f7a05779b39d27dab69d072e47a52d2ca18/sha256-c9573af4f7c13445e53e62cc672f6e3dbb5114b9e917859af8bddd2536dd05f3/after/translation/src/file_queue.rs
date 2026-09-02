//! Translation of `c_src/src/file-queue.c` + `c_src/include/file-queue.h`.
//!
//! File monitoring functions.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

use crate::cbind::*;
use crate::read_alert::{ALERTS_DAILY, CRALERT_FP_SET, CRALERT_READ_ALL, GetAlertData, alert_data};

/// `#define MAX_FQUEUE 256`
pub const MAX_FQUEUE: usize = 256;
/// `#define FQ_TIMEOUT 5`
pub const FQ_TIMEOUT: i64 = 5;

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] = b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

/// `typedef struct file_queue { ... } file_queue;`
///
/// 440 bytes on x86_64 linux-gnu.
#[repr(C)]
pub struct file_queue {
    pub last_change: i64, // time_t
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,

    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],

    pub fp: *mut FILE,
    pub f_status: stat,
}

/// `void merror(const char *err_template, const char *file_name, int err, const char *err_msg)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    unsafe {
        let mut buffer = [0u8; 256];
        snprintf(
            buffer.as_mut_ptr() as *mut c_char,
            buffer.len(),
            err_template,
            file_name,
            err,
            err_msg,
        );
        fprintf(
            stderr,
            b"%s\n\0".as_ptr() as *const c_char,
            buffer.as_ptr() as *const c_char,
        );
    }
}

/* To translate between month (int) to month (char) */
struct MonthTable([*const c_char; 12]);
unsafe impl Sync for MonthTable {}

static S_MONTH: MonthTable = MonthTable([
    b"Jan\0".as_ptr() as *const c_char,
    b"Feb\0".as_ptr() as *const c_char,
    b"Mar\0".as_ptr() as *const c_char,
    b"Apr\0".as_ptr() as *const c_char,
    b"May\0".as_ptr() as *const c_char,
    b"Jun\0".as_ptr() as *const c_char,
    b"Jul\0".as_ptr() as *const c_char,
    b"Aug\0".as_ptr() as *const c_char,
    b"Sep\0".as_ptr() as *const c_char,
    b"Oct\0".as_ptr() as *const c_char,
    b"Nov\0".as_ptr() as *const c_char,
    b"Dec\0".as_ptr() as *const c_char,
]);

/// `s_month[idx]` — unchecked, exactly like the C array subscript (the original
/// performs no range validation on `tm_mon`).
#[inline]
unsafe fn s_month(idx: c_int) -> *const c_char {
    unsafe {
        ptr::read(
            (&raw const S_MONTH.0[0])
                .cast::<*const c_char>()
                .wrapping_offset(idx as isize),
        )
    }
}

/// `static void file_sleep(void)`
fn file_sleep() {
    let mut fp_timeout = timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };

    /* Wait for the select timeout */
    unsafe {
        select(
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut fp_timeout,
        );
    }
}

/// `static void GetFile_Queue(file_queue *fileq)`
///
/// Get the file queue for that specific hour.
unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    unsafe {
        /* Create the logfile name */
        (*fileq).file_name[0] = 0;
        (*fileq).file_name[MAX_FQUEUE] = 0;

        let name = if (*fileq).flags & CRALERT_FP_SET != 0 {
            b"<stdin>\0".as_ptr() as *const c_char
        } else {
            ALERTS_DAILY.as_ptr() as *const c_char
        };

        snprintf(
            (*fileq).file_name.as_mut_ptr(),
            MAX_FQUEUE,
            b"%s\0".as_ptr() as *const c_char,
            name,
        );
    }
}

/// `static int Handle_Queue(file_queue *fileq, int flags)`
///
/// Re Handle the file queue.
unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    unsafe {
        /* Close if it is open */
        if flags & CRALERT_FP_SET == 0 {
            if !(*fileq).fp.is_null() {
                fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
            }

            /* We must be able to open the file, fseek and get the
             * time of change from it.
             */
            (*fileq).fp = fopen(
                (*fileq).file_name.as_ptr(),
                b"r\0".as_ptr() as *const c_char,
            );
            if (*fileq).fp.is_null() {
                /* Queue not available */
                return 0;
            }
        }

        /* Seek to the end of the file */
        if flags & CRALERT_READ_ALL == 0 {
            if (*fileq).fp.is_null() {
                return 0;
            }

            if fseek((*fileq).fp, 0, SEEK_END) < 0 {
                merror(
                    FSEEK_ERROR.as_ptr() as *const c_char,
                    (*fileq).file_name.as_ptr(),
                    errno(),
                    strerror(errno()),
                );
                fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
                return -1;
            }
        }

        /* File change time */
        if !(*fileq).fp.is_null() {
            if fstat(fileno((*fileq).fp), &raw mut (*fileq).f_status) < 0 {
                merror(
                    FSTAT_ERROR.as_ptr() as *const c_char,
                    (*fileq).file_name.as_ptr(),
                    errno(),
                    strerror(errno()),
                );
                fclose((*fileq).fp);
                (*fileq).fp = ptr::null_mut();
                return -1;
            }
        }

        (*fileq).last_change = (*fileq).f_status.st_mtim.tv_sec;

        1
    }
}

/// `int Init_FileQueue(file_queue *fileq, const struct tm *p, int flags)`
///
/// Initiates the file monitoring.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    unsafe {
        /* Initialize file_queue fields */
        if flags & CRALERT_FP_SET == 0 {
            (*fileq).fp = ptr::null_mut();
        }
        (*fileq).last_change = 0;
        (*fileq).flags = 0;

        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year.wrapping_add(1900);

        strncpy((*fileq).mon.as_mut_ptr(), s_month((*p).tm_mon), 3);
        memset(
            (*fileq).file_name.as_mut_ptr() as *mut c_void,
            b'\0' as c_int,
            MAX_FQUEUE + 1,
        );

        /* Set the supplied flags */
        (*fileq).flags = flags;

        /* Get latest file */
        GetFile_Queue(fileq);

        /* Always seek to the end when starting the queue */
        if Handle_Queue(fileq, (*fileq).flags) < 0 {
            return -1;
        }

        0
    }
}

/// `alert_data *Read_FileMon(file_queue *fileq, const struct tm *p, unsigned int timeout)`
///
/// Reads from the monitored file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const tm,
    timeout: c_uint,
) -> *mut alert_data {
    unsafe {
        let mut i: c_uint = 0;
        let mut al_data: *mut alert_data;

        /* If the file queue is not available, try to access it */
        if (*fileq).fp.is_null() {
            if Handle_Queue(fileq, 0) != 1 {
                file_sleep();
                return ptr::null_mut();
            }
        }

        if (*fileq).fp.is_null() {
            return ptr::null_mut();
        }

        al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }

        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year.wrapping_add(1900);
        strncpy((*fileq).mon.as_mut_ptr(), s_month((*p).tm_mon), 3);

        /* Get latest file */
        GetFile_Queue(fileq);

        if Handle_Queue(fileq, 0) != 1 {
            file_sleep();
            return ptr::null_mut();
        }

        /* Try up to timeout times to get an event */
        while i < timeout {
            al_data = GetAlertData((*fileq).flags, (*fileq).fp);
            if !al_data.is_null() {
                return al_data;
            }

            i += 1;
            file_sleep();
        }

        /* Return NULL if timeout expires */
        ptr::null_mut()
    }
}
