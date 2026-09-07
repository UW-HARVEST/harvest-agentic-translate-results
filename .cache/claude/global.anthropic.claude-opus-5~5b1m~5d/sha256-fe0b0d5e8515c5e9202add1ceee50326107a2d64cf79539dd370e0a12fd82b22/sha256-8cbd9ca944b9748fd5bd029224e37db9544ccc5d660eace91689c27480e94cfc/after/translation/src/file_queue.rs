//! Translation of `c_src/src/file-queue.c` (+ `c_src/include/file-queue.h`).

use core::ffi::{c_char, c_int, c_long, c_uint};
use core::ptr;

use crate::cbind::*;
use crate::read_alert::{alert_data, GetAlertData, ALERTS_DAILY, CRALERT_FP_SET, CRALERT_READ_ALL};

/* ---------------------------------------------------------------- */
/* file-queue.h                                                     */
/* ---------------------------------------------------------------- */

/// `#define MAX_FQUEUE 256`
pub const MAX_FQUEUE: usize = 256;
/// `#define FQ_TIMEOUT 5`
pub const FQ_TIMEOUT: c_long = 5;

/// ```c
/// typedef struct file_queue {
///     time_t last_change;
///     int year;
///     int day;
///     int flags;
///     char mon[4];
///     char file_name[MAX_FQUEUE + 1];
///     FILE *fp;
///     struct stat f_status;
/// } file_queue;
/// ```
#[repr(C)]
pub struct file_queue {
    pub last_change: c_long,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

/* ---------------------------------------------------------------- */
/* file-queue.c                                                     */
/* ---------------------------------------------------------------- */

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0";
const FSEEK_ERROR: &[u8] = b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0";

/// ```c
/// void merror(const char *err_template, const char *file_name, int err, const char *err_msg)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0 as c_char; 256];
    snprintf(
        buffer.as_mut_ptr(),
        core::mem::size_of_val(&buffer),
        err_template,
        file_name,
        err,
        err_msg,
    );
    fprintf(stderr, cs(b"%s\n\0"), buffer.as_ptr());
}

/// `static const char *(s_month[]) = {"Jan", ... "Dec"};`
///
/// Held as a raw `[*const c_char; 12]` so that indexing can be reproduced
/// exactly as the C does it -- see `copy_month`.
#[repr(transparent)]
struct MonthTable([*const c_char; 12]);

// The table is immutable, read-only data; the raw pointers only exist because
// the C declares `const char *[]`.
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

/// `strncpy(fileq->mon, s_month[p->tm_mon], 3);`
///
/// The C indexes `s_month` without validating `tm_mon`, so an out-of-range
/// month reads past the end of the 12-entry table and then `strncpy`s from
/// whatever pointer that yields. That is undefined behaviour in C, but it is
/// NOT a rejection the C performs, and adding a bounds check here would make
/// the Rust behave *differently* from the ground truth for in-range-looking
/// callers of `Init_FileQueue`/`Read_FileMon`. The unchecked read is therefore
/// reproduced verbatim rather than "fixed".
#[inline]
unsafe fn copy_month(fileq: *mut file_queue, tm_mon: c_int) {
    let entry = *S_MONTH.0.as_ptr().offset(tm_mon as isize);
    strncpy((*fileq).mon.as_mut_ptr(), entry, 3);
}

/// ```c
/// static void file_sleep(void)
/// ```
unsafe fn file_sleep() {
    let mut fp_timeout = timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };

    /* Wait for the select timeout */
    select(
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        &mut fp_timeout,
    );
}

/// ```c
/// static void GetFile_Queue(file_queue *fileq)
/// ```
#[allow(non_snake_case)]
unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    /* Create the logfile name */
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;

    let src = if (*fileq).flags & CRALERT_FP_SET != 0 {
        cs(b"<stdin>\0")
    } else {
        cs(ALERTS_DAILY)
    };

    snprintf(
        (*fileq).file_name.as_mut_ptr(),
        MAX_FQUEUE,
        cs(b"%s\0"),
        src,
    );
}

/// ```c
/// static int Handle_Queue(file_queue *fileq, int flags)
/// ```
#[allow(non_snake_case)]
unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    /* Close if it is open */
    if flags & CRALERT_FP_SET == 0 {
        if !(*fileq).fp.is_null() {
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
        }

        /* We must be able to open the file, fseek and get the
         * time of change from it.
         */
        (*fileq).fp = fopen((*fileq).file_name.as_ptr(), cs(b"r\0"));
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
                cs(FSEEK_ERROR),
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
        if fstat(fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            merror(
                cs(FSTAT_ERROR),
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

/// ```c
/// int Init_FileQueue(file_queue *fileq, const struct tm *p, int flags)
/// ```
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    /* Initialize file_queue fields */
    if flags & CRALERT_FP_SET == 0 {
        (*fileq).fp = ptr::null_mut();
    }
    (*fileq).last_change = 0;
    (*fileq).flags = 0;

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;

    copy_month(fileq, (*p).tm_mon);
    memset(
        (*fileq).file_name.as_mut_ptr() as *mut core::ffi::c_void,
        0,
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

/// ```c
/// alert_data *Read_FileMon(file_queue *fileq, const struct tm *p, unsigned int timeout)
/// ```
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const tm,
    timeout: c_uint,
) -> *mut alert_data {
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
    (*fileq).year = (*p).tm_year + 1900;
    copy_month(fileq, (*p).tm_mon);

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
