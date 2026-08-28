//! Translation of `c_src/src/file-queue.c` and `c_src/include/file-queue.h`.

use core::ffi::{c_char, c_int, c_uint, CStr};

use crate::read_alert::{
    alert_data, GetAlertData, CRALERT_FP_SET, CRALERT_READ_ALL, _ALERTS_DAILY,
};
use crate::shared::stderr_line;

pub const MAX_FQUEUE: usize = 256;
pub const FQ_TIMEOUT: libc::time_t = 5;

/// `typedef struct file_queue { ... } file_queue;`
#[repr(C)]
pub struct file_queue {
    pub last_change: libc::time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,

    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],

    pub fp: *mut libc::FILE,
    pub f_status: libc::stat,
}

const FSTAT_ERROR: &CStr =
    c"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].";
const FSEEK_ERROR: &CStr = c"(1116): Could not set position in file '%s' due to [(%d)-(%s)].";

// Layout of `file_queue` must match the C struct exactly (x86_64 Linux values).
const _: () = {
    assert!(core::mem::size_of::<file_queue>() == 440);
    assert!(core::mem::offset_of!(file_queue, year) == 8);
    assert!(core::mem::offset_of!(file_queue, mon) == 20);
    assert!(core::mem::offset_of!(file_queue, file_name) == 24);
    assert!(core::mem::offset_of!(file_queue, fp) == 288);
    assert!(core::mem::offset_of!(file_queue, f_status) == 296);
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merror(
    err_template: *const c_char,
    file_name: *const c_char,
    err: c_int,
    err_msg: *const c_char,
) {
    let mut buffer = [0 as c_char; 256];
    libc::snprintf(
        buffer.as_mut_ptr(),
        core::mem::size_of_val(&buffer),
        err_template,
        file_name,
        err,
        err_msg,
    );
    stderr_line(buffer.as_ptr());
}

/* To translate between month (int) to month (char) */
static S_MONTH: [&[u8; 4]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
    b"Oct\0", b"Nov\0", b"Dec\0",
];

/// `s_month[i]`, keeping the original (unchecked) indexing behaviour.
unsafe fn s_month(i: c_int) -> *const c_char {
    let base = S_MONTH.as_ptr() as *const *const c_char;
    *base.offset(i as isize)
}

unsafe fn errno() -> c_int {
    *libc::__errno_location()
}

fn file_sleep() {
    let mut fp_timeout = libc::timeval {
        tv_sec: FQ_TIMEOUT,
        tv_usec: 0,
    };

    /* Wait for the select timeout */
    unsafe {
        libc::select(
            0,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut fp_timeout,
        );
    }
}

/// Get the file queue for that specific hour
unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    /* Create the logfile name */
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;

    let name = if (*fileq).flags & CRALERT_FP_SET != 0 {
        c"<stdin>".as_ptr()
    } else {
        _ALERTS_DAILY.as_ptr()
    };

    libc::snprintf(
        (*fileq).file_name.as_mut_ptr(),
        MAX_FQUEUE,
        c"%s".as_ptr(),
        name,
    );
}

/// Re Handle the file queue
unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    /* Close if it is open */
    if flags & CRALERT_FP_SET == 0 {
        if !(*fileq).fp.is_null() {
            libc::fclose((*fileq).fp);
            (*fileq).fp = core::ptr::null_mut();
        }

        /* We must be able to open the file, fseek and get the
         * time of change from it.
         */
        (*fileq).fp = libc::fopen((*fileq).file_name.as_ptr(), c"r".as_ptr());
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

        if libc::fseek((*fileq).fp, 0, libc::SEEK_END) < 0 {
            merror(
                FSEEK_ERROR.as_ptr(),
                (*fileq).file_name.as_ptr(),
                errno(),
                libc::strerror(errno()),
            );
            libc::fclose((*fileq).fp);
            (*fileq).fp = core::ptr::null_mut();
            return -1;
        }
    }

    /* File change time */
    if !(*fileq).fp.is_null() {
        if libc::fstat(libc::fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            merror(
                FSTAT_ERROR.as_ptr(),
                (*fileq).file_name.as_ptr(),
                errno(),
                libc::strerror(errno()),
            );
            libc::fclose((*fileq).fp);
            (*fileq).fp = core::ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;

    1
}

/// Initiates the file monitoring
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const libc::tm,
    flags: c_int,
) -> c_int {
    /* Initialize file_queue fields */
    if flags & CRALERT_FP_SET == 0 {
        (*fileq).fp = core::ptr::null_mut();
    }
    (*fileq).last_change = 0;
    (*fileq).flags = 0;

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;

    libc::strncpy((*fileq).mon.as_mut_ptr(), s_month((*p).tm_mon), 3);
    libc::memset(
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

/// Reads from the monitored file
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const libc::tm,
    timeout: c_uint,
) -> *mut alert_data {
    let mut i: c_uint = 0;
    let mut al_data: *mut alert_data;

    /* If the file queue is not available, try to access it */
    if (*fileq).fp.is_null() {
        if Handle_Queue(fileq, 0) != 1 {
            file_sleep();
            return core::ptr::null_mut();
        }
    }

    if (*fileq).fp.is_null() {
        return core::ptr::null_mut();
    }

    al_data = GetAlertData((*fileq).flags, (*fileq).fp);
    if !al_data.is_null() {
        return al_data;
    }

    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900;
    libc::strncpy((*fileq).mon.as_mut_ptr(), s_month((*p).tm_mon), 3);

    /* Get latest file */
    GetFile_Queue(fileq);

    if Handle_Queue(fileq, 0) != 1 {
        file_sleep();
        return core::ptr::null_mut();
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
    core::ptr::null_mut()
}
