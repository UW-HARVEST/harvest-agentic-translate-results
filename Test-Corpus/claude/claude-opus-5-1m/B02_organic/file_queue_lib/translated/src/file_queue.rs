//! Translation of `c_src/src/file-queue.c` + `c_src/include/file-queue.h`.

#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

use crate::cbits::*;
use crate::read_alert::{alert_data, GetAlertData, ALERTS_DAILY, CRALERT_FP_SET, CRALERT_READ_ALL};

// ---------------------------------------------------------------------------
// file-queue.h
// ---------------------------------------------------------------------------

/// `#define MAX_FQUEUE 256`
pub const MAX_FQUEUE: usize = 256;
/// `#define FQ_TIMEOUT 5`
pub const FQ_TIMEOUT: i64 = 5;

/// ```c
/// typedef struct file_queue {
///     time_t last_change;
///     int year;
///     int day;
///     int flags;
///
///     char mon[4];
///     char file_name[MAX_FQUEUE + 1];
///
///     FILE *fp;
///     struct stat f_status;
/// } file_queue;
/// ```
#[repr(C)]
pub struct file_queue {
    pub last_change: time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,

    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],

    pub fp: *mut FILE,
    pub f_status: stat,
}

const _: () = {
    assert!(core::mem::size_of::<stat>() == 144);
    assert!(core::mem::size_of::<file_queue>() == 440);
    assert!(core::mem::align_of::<file_queue>() == 8);
    assert!(core::mem::size_of::<tm>() == 56);
};

// ---------------------------------------------------------------------------
// file-queue.c
// ---------------------------------------------------------------------------

const FSTAT_ERROR: &core::ffi::CStr =
    c"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].";
const FSEEK_ERROR: &core::ffi::CStr =
    c"(1116): Could not set position in file '%s' due to [(%d)-(%s)].";

/// ```c
/// void merror(const char *err_template, const char *file_name, int err, const char *err_msg) {
///     char buffer[256];
///     snprintf(buffer, sizeof(buffer), err_template, file_name, err, err_msg);
///     fprintf(stderr, "%s\n", buffer);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0u8; 256];
    snprintf(
        buffer.as_mut_ptr() as *mut c_char,
        buffer.len(),
        err_template,
        file_name,
        err,
        err_msg,
    );
    fprintf(stderr, c"%s\n".as_ptr(), buffer.as_ptr() as *const c_char);
}

/// To translate between month (int) to month (char).
///
/// ```c
/// static const char *(s_month[]) = {"Jan", "Feb", ... "Dec"};
/// ```
static S_MONTH: [&core::ffi::CStr; 12] = [
    c"Jan", c"Feb", c"Mar", c"Apr", c"May", c"Jun", c"Jul", c"Aug", c"Sep", c"Oct", c"Nov", c"Dec",
];

/// `strncpy(fileq->mon, s_month[p->tm_mon], 3)`
///
/// A `tm_mon` outside `0..=11` indexes `s_month` out of bounds in the original
/// C, which is undefined behaviour with no defined observable result.  The
/// `mon` field is never read back by any function in this library and is not
/// part of any returned value, so declining to write it for out-of-range
/// months preserves every observable output.
#[inline]
unsafe fn copy_mon(fileq: *mut file_queue, tm_mon: c_int) {
    if (0..12).contains(&tm_mon) {
        strncpy(
            (*fileq).mon.as_mut_ptr(),
            S_MONTH[tm_mon as usize].as_ptr(),
            3,
        );
    }
}

/// ```c
/// static void file_sleep() {
///     struct timeval fp_timeout;
///     fp_timeout.tv_sec = FQ_TIMEOUT;
///     fp_timeout.tv_usec = 0;
///     select(0, NULL, NULL, NULL, &fp_timeout);
///     return;
/// }
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

/// Get the file queue for that specific hour.
///
/// ```c
/// static void GetFile_Queue(file_queue *fileq) {
///     fileq->file_name[0] = '\0';
///     fileq->file_name[MAX_FQUEUE] = '\0';
///     snprintf(fileq->file_name, MAX_FQUEUE, "%s",
///              fileq->flags & CRALERT_FP_SET ? "<stdin>" : ALERTS_DAILY);
/// }
/// ```
unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    /* Create the logfile name */
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;

    let src = if ((*fileq).flags & CRALERT_FP_SET) != 0 {
        c"<stdin>"
    } else {
        ALERTS_DAILY
    };

    snprintf(
        (*fileq).file_name.as_mut_ptr(),
        MAX_FQUEUE,
        c"%s".as_ptr(),
        src.as_ptr(),
    );
}

/// Re Handle the file queue.
///
/// ```c
/// static int Handle_Queue(file_queue *fileq, int flags);
/// ```
unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    /* Close if it is open */
    if (flags & CRALERT_FP_SET) == 0 {
        if !(*fileq).fp.is_null() {
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
        }

        /* We must be able to open the file, fseek and get the
         * time of change from it.
         */
        (*fileq).fp = fopen((*fileq).file_name.as_ptr(), c"r".as_ptr());
        if (*fileq).fp.is_null() {
            /* Queue not available */
            return 0;
        }
    }

    /* Seek to the end of the file */
    if (flags & CRALERT_READ_ALL) == 0 {
        if (*fileq).fp.is_null() {
            return 0;
        }

        if fseek((*fileq).fp, 0, SEEK_END) < 0 {
            let e = errno();
            merror(
                FSEEK_ERROR.as_ptr(),
                (*fileq).file_name.as_ptr(),
                e,
                strerror(e),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    /* File change time */
    if !(*fileq).fp.is_null() {
        if fstat(fileno((*fileq).fp), &raw mut (*fileq).f_status) < 0 {
            let e = errno();
            merror(
                FSTAT_ERROR.as_ptr(),
                (*fileq).file_name.as_ptr(),
                e,
                strerror(e),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtim.tv_sec;

    1
}

/// Initiates the file monitoring.
///
/// ```c
/// int Init_FileQueue(file_queue *fileq, const struct tm *p, int flags);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const tm,
    flags: c_int,
) -> c_int {
    /* Initialize file_queue fields */
    if (flags & CRALERT_FP_SET) == 0 {
        (*fileq).fp = ptr::null_mut();
    }
    (*fileq).last_change = 0;
    (*fileq).flags = 0;

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year.wrapping_add(1900);

    copy_mon(fileq, (*p).tm_mon);
    ptr::write_bytes((*fileq).file_name.as_mut_ptr() as *mut u8, 0, MAX_FQUEUE + 1);

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

/// Reads from the monitored file.
///
/// ```c
/// alert_data *Read_FileMon(file_queue *fileq, const struct tm *p, unsigned int timeout);
/// ```
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
    (*fileq).year = (*p).tm_year.wrapping_add(1900);
    copy_mon(fileq, (*p).tm_mon);

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

// Keep the `c_void` import meaningful for the raw-pointer casts above on all
// compiler versions.
const _: Option<*mut c_void> = None;
