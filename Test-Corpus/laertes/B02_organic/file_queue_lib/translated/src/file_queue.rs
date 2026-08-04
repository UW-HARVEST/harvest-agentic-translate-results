extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> libc::c_int;
    fn fopen(
        __filename: *const libc::c_char,
        __modes: *const libc::c_char,
    ) -> *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn snprintf(
        __s: *mut libc::c_char,
        __maxlen: size_t,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn fseek(
        __stream: *mut FILE,
        __off: libc::c_long,
        __whence: libc::c_int,
    ) -> libc::c_int;
    fn fileno(__stream: *mut FILE) -> libc::c_int;
    fn fstat(__fd: libc::c_int, __buf: *mut stat) -> libc::c_int;
    
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strerror(__errnum: libc::c_int) -> *mut libc::c_char;
    fn select(
        __nfds: libc::c_int,
        __readfds: *mut fd_set,
        __writefds: *mut fd_set,
        __exceptfds: *mut fd_set,
        __timeout: *mut timeval,
    ) -> libc::c_int;
    fn __errno_location() -> *mut libc::c_int;
}
pub use crate::src::read_alert::GetAlertData;
pub use crate::src::driver::size_t;
pub use crate::src::driver::__dev_t;
pub use crate::src::driver::__uid_t;
pub use crate::src::driver::__gid_t;
pub use crate::src::driver::__ino_t;
pub use crate::src::driver::__mode_t;
pub use crate::src::driver::__nlink_t;
pub use crate::src::driver::__off_t;
pub use crate::src::driver::__off64_t;
pub use crate::src::driver::__time_t;
pub type __suseconds_t = libc::c_long;
pub use crate::src::driver::__blksize_t;
pub use crate::src::driver::__blkcnt_t;
pub use crate::src::driver::__syscall_slong_t;
// #[derive(Copy, Clone)]

pub use crate::src::driver::_IO_FILE;
pub use crate::src::driver::_IO_lock_t;
// #[derive(Copy, Clone)]

pub use crate::src::driver::_IO_marker;
pub use crate::src::driver::FILE;
pub use crate::src::driver::time_t;
// #[derive(Copy, Clone)]

pub use crate::src::driver::tm;
// #[derive(Copy, Clone)]

pub use crate::src::driver::timespec;
// #[derive(Copy, Clone)]

pub use crate::src::driver::stat;
// #[derive(Copy, Clone)]

pub use crate::src::driver::file_queue;
// #[derive(Copy, Clone)]

pub use crate::src::driver::alert_data;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timeval {
    pub tv_sec: __time_t,
    pub tv_usec: __suseconds_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct fd_set {
    pub __fds_bits: [__fd_mask; 16],
}
pub type __fd_mask = libc::c_long;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const SEEK_END: libc::c_int = 2 as libc::c_int;
pub const MAX_FQUEUE: libc::c_int = 256 as libc::c_int;
pub const FQ_TIMEOUT: libc::c_int = 5 as libc::c_int;
pub const ALERTS_DAILY: [libc::c_char; 11] =
    unsafe { std::mem::transmute::<[u8; 11], [libc::c_char; 11]>(*b"alerts.log\0") };
pub const CRALERT_READ_ALL: libc::c_int = 0x4 as libc::c_int;
pub const CRALERT_FP_SET: libc::c_int = 0x10 as libc::c_int;
pub const FSTAT_ERROR: [libc::c_char; 72] = unsafe {
    std::mem::transmute::<[u8; 72], [libc::c_char; 72]>(
        *b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0",
    )
};
pub const FSEEK_ERROR: [libc::c_char; 64] = unsafe {
    std::mem::transmute::<[u8; 64], [libc::c_char; 64]>(
        *b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0",
    )
};
#[no_mangle]
pub unsafe extern "C" fn merror(
    mut err_template: *const libc::c_char,
    mut file_name: *const libc::c_char,
    mut err: libc::c_int,
    mut err_msg: *const libc::c_char,
) {
    let mut buffer: [libc::c_char; 256] = [0; 256];
    snprintf(
        &raw mut buffer as *mut libc::c_char,
        std::mem::size_of::<[libc::c_char; 256]>() as size_t,
        err_template,
        file_name,
        err,
        err_msg,
    );
    fprintf(
        stderr as *mut FILE,
        b"%s\n\0" as *const u8 as *const libc::c_char,
        &raw mut buffer as *mut libc::c_char,
    );
}
static mut s_month: [*const libc::c_char; 12] = [
    b"Jan\0" as *const u8 as *const libc::c_char,
    b"Feb\0" as *const u8 as *const libc::c_char,
    b"Mar\0" as *const u8 as *const libc::c_char,
    b"Apr\0" as *const u8 as *const libc::c_char,
    b"May\0" as *const u8 as *const libc::c_char,
    b"Jun\0" as *const u8 as *const libc::c_char,
    b"Jul\0" as *const u8 as *const libc::c_char,
    b"Aug\0" as *const u8 as *const libc::c_char,
    b"Sep\0" as *const u8 as *const libc::c_char,
    b"Oct\0" as *const u8 as *const libc::c_char,
    b"Nov\0" as *const u8 as *const libc::c_char,
    b"Dec\0" as *const u8 as *const libc::c_char,
];
unsafe extern "C" fn file_sleep() {
    let mut fp_timeout: timeval = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    fp_timeout.tv_sec = FQ_TIMEOUT as __time_t;
    fp_timeout.tv_usec = 0 as __suseconds_t;
    select(
        0 as libc::c_int,
        std::ptr::null_mut::<fd_set>(),
        std::ptr::null_mut::<fd_set>(),
        std::ptr::null_mut::<fd_set>(),
        &raw mut fp_timeout,
    );
}
unsafe extern "C" fn GetFile_Queue(mut fileq: *mut file_queue) {
    (*fileq).file_name[0 as libc::c_int as usize] = '\0' as i32 as libc::c_char;
    (*fileq).file_name[MAX_FQUEUE as usize] = '\0' as i32 as libc::c_char;
    snprintf(
        &raw mut (*fileq).file_name as *mut libc::c_char,
        MAX_FQUEUE as size_t,
        b"%s\0" as *const u8 as *const libc::c_char,
        if (*fileq).flags & CRALERT_FP_SET != 0 {
            b"<stdin>\0" as *const u8 as *const libc::c_char
        } else {
            ALERTS_DAILY.as_ptr()
        },
    );
}
unsafe extern "C" fn Handle_Queue(
    mut fileq: *mut file_queue,
    mut flags: libc::c_int,
) -> libc::c_int {
    if flags & CRALERT_FP_SET == 0 {
        if !(*fileq).fp.is_null() {
            fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut::<FILE>();
        }
        (*fileq).fp = fopen(
            &raw mut (*fileq).file_name as *mut libc::c_char,
            b"r\0" as *const u8 as *const libc::c_char,
        );
        if (*fileq).fp.is_null() {
            return 0 as libc::c_int;
        }
    }
    if flags & CRALERT_READ_ALL == 0 {
        if (*fileq).fp.is_null() {
            return 0 as libc::c_int;
        }
        if fseek((*fileq).fp, 0 as libc::c_long, SEEK_END) < 0 as libc::c_int {
            merror(
                FSEEK_ERROR.as_ptr(),
                &raw mut (*fileq).file_name as *mut libc::c_char,
                *__errno_location(),
                strerror(*__errno_location()),
            );
            fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut::<FILE>();
            return -(1 as libc::c_int);
        }
    }
    if !(*fileq).fp.is_null() {
        if fstat(fileno((*fileq).fp), &raw mut (*fileq).f_status) < 0 as libc::c_int {
            merror(
                FSTAT_ERROR.as_ptr(),
                &raw mut (*fileq).file_name as *mut libc::c_char,
                *__errno_location(),
                strerror(*__errno_location()),
            );
            fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut::<FILE>();
            return -(1 as libc::c_int);
        }
    }
    (*fileq).last_change = (*fileq).f_status.st_mtim.tv_sec as time_t;
    return 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn Init_FileQueue(
    mut fileq: *mut file_queue,
    mut p: *const tm,
    mut flags: libc::c_int,
) -> libc::c_int {
    if flags & CRALERT_FP_SET == 0 {
        (*fileq).fp = std::ptr::null_mut::<FILE>();
    }
    (*fileq).last_change = 0 as time_t;
    (*fileq).flags = 0 as libc::c_int;
    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900 as libc::c_int;
    strncpy(
        &raw mut (*fileq).mon as *mut libc::c_char,
        s_month[(*p).tm_mon as usize],
        3 as size_t,
    );
    memset(
        &raw mut (*fileq).file_name as *mut libc::c_char as *mut libc::c_void,
        '\0' as i32,
        (MAX_FQUEUE + 1 as libc::c_int) as size_t,
    );
    (*fileq).flags = flags;
    GetFile_Queue(fileq);
    if Handle_Queue(fileq, (*fileq).flags) < 0 as libc::c_int {
        return -(1 as libc::c_int);
    }
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn Read_FileMon(
    mut fileq: *mut file_queue,
    mut p: *const tm,
    mut timeout: libc::c_uint,
) -> *mut alert_data {
    let mut i: libc::c_uint = 0 as libc::c_uint;
    let mut al_data: *mut alert_data = std::ptr::null_mut::<alert_data>();
    if (*fileq).fp.is_null() {
        if Handle_Queue(fileq, 0 as libc::c_int) != 1 as libc::c_int {
            file_sleep();
            return std::ptr::null_mut::<alert_data>();
        }
    }
    if (*fileq).fp.is_null() {
        return std::ptr::null_mut::<alert_data>();
    }
    al_data = GetAlertData((*fileq).flags, (*fileq).fp);
    if !al_data.is_null() {
        return al_data;
    }
    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900 as libc::c_int;
    strncpy(
        &raw mut (*fileq).mon as *mut libc::c_char,
        s_month[(*p).tm_mon as usize],
        3 as size_t,
    );
    GetFile_Queue(fileq);
    if Handle_Queue(fileq, 0 as libc::c_int) != 1 as libc::c_int {
        file_sleep();
        return std::ptr::null_mut::<alert_data>();
    }
    while i < timeout {
        al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }
        i = i.wrapping_add(1);
        file_sleep();
    }
    return std::ptr::null_mut::<alert_data>();
}
