extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn fseek(
        __stream: *mut FILE,
        __off: ::core::ffi::c_long,
        __whence: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn fileno(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fstat(__fd: ::core::ffi::c_int, __buf: *mut stat) -> ::core::ffi::c_int;
    fn GetAlertData(flag: ::core::ffi::c_int, fp: *mut FILE) -> *mut alert_data;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    fn select(
        __nfds: ::core::ffi::c_int,
        __readfds: *mut fd_set,
        __writefds: *mut fd_set,
        __exceptfds: *mut fd_set,
        __timeout: *mut timeval,
    ) -> ::core::ffi::c_int;
    fn __errno_location() -> *mut ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __dev_t = ::core::ffi::c_ulong;
pub type __uid_t = ::core::ffi::c_uint;
pub type __gid_t = ::core::ffi::c_uint;
pub type __ino_t = ::core::ffi::c_ulong;
pub type __mode_t = ::core::ffi::c_uint;
pub type __nlink_t = ::core::ffi::c_ulong;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __time_t = ::core::ffi::c_long;
pub type __suseconds_t = ::core::ffi::c_long;
pub type __blksize_t = ::core::ffi::c_long;
pub type __blkcnt_t = ::core::ffi::c_long;
pub type __syscall_slong_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub type time_t = __time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: ::core::ffi::c_int,
    pub tm_min: ::core::ffi::c_int,
    pub tm_hour: ::core::ffi::c_int,
    pub tm_mday: ::core::ffi::c_int,
    pub tm_mon: ::core::ffi::c_int,
    pub tm_year: ::core::ffi::c_int,
    pub tm_wday: ::core::ffi::c_int,
    pub tm_yday: ::core::ffi::c_int,
    pub tm_isdst: ::core::ffi::c_int,
    pub tm_gmtoff: ::core::ffi::c_long,
    pub tm_zone: *const ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __time_t,
    pub tv_nsec: __syscall_slong_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: __dev_t,
    pub st_ino: __ino_t,
    pub st_nlink: __nlink_t,
    pub st_mode: __mode_t,
    pub st_uid: __uid_t,
    pub st_gid: __gid_t,
    pub __pad0: ::core::ffi::c_int,
    pub st_rdev: __dev_t,
    pub st_size: __off_t,
    pub st_blksize: __blksize_t,
    pub st_blocks: __blkcnt_t,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [__syscall_slong_t; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct file_queue {
    pub last_change: time_t,
    pub year: ::core::ffi::c_int,
    pub day: ::core::ffi::c_int,
    pub flags: ::core::ffi::c_int,
    pub mon: [::core::ffi::c_char; 4],
    pub file_name: [::core::ffi::c_char; 257],
    pub fp: *mut FILE,
    pub f_status: stat,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct alert_data {
    pub rule: ::core::ffi::c_uint,
    pub level: ::core::ffi::c_uint,
    pub alertid: *mut ::core::ffi::c_char,
    pub date: *mut ::core::ffi::c_char,
    pub location: *mut ::core::ffi::c_char,
    pub comment: *mut ::core::ffi::c_char,
    pub group: *mut ::core::ffi::c_char,
    pub srcip: *mut ::core::ffi::c_char,
    pub srcport: ::core::ffi::c_int,
    pub dstip: *mut ::core::ffi::c_char,
    pub dstport: ::core::ffi::c_int,
    pub user: *mut ::core::ffi::c_char,
    pub filename: *mut ::core::ffi::c_char,
}
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
pub type __fd_mask = ::core::ffi::c_long;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const SEEK_END: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const MAX_FQUEUE: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const FQ_TIMEOUT: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const ALERTS_DAILY: [::core::ffi::c_char; 11] =
    unsafe { ::core::mem::transmute::<[u8; 11], [::core::ffi::c_char; 11]>(*b"alerts.log\0") };
pub const CRALERT_READ_ALL: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const CRALERT_FP_SET: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const FSTAT_ERROR: [::core::ffi::c_char; 72] = unsafe {
    ::core::mem::transmute::<[u8; 72], [::core::ffi::c_char; 72]>(
        *b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0",
    )
};
pub const FSEEK_ERROR: [::core::ffi::c_char; 64] = unsafe {
    ::core::mem::transmute::<[u8; 64], [::core::ffi::c_char; 64]>(
        *b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0",
    )
};
#[no_mangle]
pub unsafe extern "C" fn merror(
    mut err_template: *const ::core::ffi::c_char,
    mut file_name: *const ::core::ffi::c_char,
    mut err: ::core::ffi::c_int,
    mut err_msg: *const ::core::ffi::c_char,
) {
    let mut buffer: [::core::ffi::c_char; 256] = [0; 256];
    snprintf(
        &raw mut buffer as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 256]>() as size_t,
        err_template,
        file_name,
        err,
        err_msg,
    );
    fprintf(
        stderr as *mut FILE,
        b"%s\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut buffer as *mut ::core::ffi::c_char,
    );
}
static mut s_month: [*const ::core::ffi::c_char; 12] = [
    b"Jan\0" as *const u8 as *const ::core::ffi::c_char,
    b"Feb\0" as *const u8 as *const ::core::ffi::c_char,
    b"Mar\0" as *const u8 as *const ::core::ffi::c_char,
    b"Apr\0" as *const u8 as *const ::core::ffi::c_char,
    b"May\0" as *const u8 as *const ::core::ffi::c_char,
    b"Jun\0" as *const u8 as *const ::core::ffi::c_char,
    b"Jul\0" as *const u8 as *const ::core::ffi::c_char,
    b"Aug\0" as *const u8 as *const ::core::ffi::c_char,
    b"Sep\0" as *const u8 as *const ::core::ffi::c_char,
    b"Oct\0" as *const u8 as *const ::core::ffi::c_char,
    b"Nov\0" as *const u8 as *const ::core::ffi::c_char,
    b"Dec\0" as *const u8 as *const ::core::ffi::c_char,
];
unsafe extern "C" fn file_sleep() {
    let mut fp_timeout: timeval = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    fp_timeout.tv_sec = FQ_TIMEOUT as __time_t;
    fp_timeout.tv_usec = 0 as __suseconds_t;
    select(
        0 as ::core::ffi::c_int,
        ::core::ptr::null_mut::<fd_set>(),
        ::core::ptr::null_mut::<fd_set>(),
        ::core::ptr::null_mut::<fd_set>(),
        &raw mut fp_timeout,
    );
}
unsafe extern "C" fn GetFile_Queue(mut fileq: *mut file_queue) {
    (*fileq).file_name[0 as ::core::ffi::c_int as usize] = '\0' as i32 as ::core::ffi::c_char;
    (*fileq).file_name[MAX_FQUEUE as usize] = '\0' as i32 as ::core::ffi::c_char;
    snprintf(
        &raw mut (*fileq).file_name as *mut ::core::ffi::c_char,
        MAX_FQUEUE as size_t,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        if (*fileq).flags & CRALERT_FP_SET != 0 {
            b"<stdin>\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            ALERTS_DAILY.as_ptr()
        },
    );
}
unsafe extern "C" fn Handle_Queue(
    mut fileq: *mut file_queue,
    mut flags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if flags & CRALERT_FP_SET == 0 {
        if !(*fileq).fp.is_null() {
            fclose((*fileq).fp);
            (*fileq).fp = ::core::ptr::null_mut::<FILE>();
        }
        (*fileq).fp = fopen(
            &raw mut (*fileq).file_name as *mut ::core::ffi::c_char,
            b"r\0" as *const u8 as *const ::core::ffi::c_char,
        );
        if (*fileq).fp.is_null() {
            return 0 as ::core::ffi::c_int;
        }
    }
    if flags & CRALERT_READ_ALL == 0 {
        if (*fileq).fp.is_null() {
            return 0 as ::core::ffi::c_int;
        }
        if fseek((*fileq).fp, 0 as ::core::ffi::c_long, SEEK_END) < 0 as ::core::ffi::c_int {
            merror(
                FSEEK_ERROR.as_ptr(),
                &raw mut (*fileq).file_name as *mut ::core::ffi::c_char,
                *__errno_location(),
                strerror(*__errno_location()),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ::core::ptr::null_mut::<FILE>();
            return -(1 as ::core::ffi::c_int);
        }
    }
    if !(*fileq).fp.is_null() {
        if fstat(fileno((*fileq).fp), &raw mut (*fileq).f_status) < 0 as ::core::ffi::c_int {
            merror(
                FSTAT_ERROR.as_ptr(),
                &raw mut (*fileq).file_name as *mut ::core::ffi::c_char,
                *__errno_location(),
                strerror(*__errno_location()),
            );
            fclose((*fileq).fp);
            (*fileq).fp = ::core::ptr::null_mut::<FILE>();
            return -(1 as ::core::ffi::c_int);
        }
    }
    (*fileq).last_change = (*fileq).f_status.st_mtim.tv_sec as time_t;
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn Init_FileQueue(
    mut fileq: *mut file_queue,
    mut p: *const tm,
    mut flags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if flags & CRALERT_FP_SET == 0 {
        (*fileq).fp = ::core::ptr::null_mut::<FILE>();
    }
    (*fileq).last_change = 0 as time_t;
    (*fileq).flags = 0 as ::core::ffi::c_int;
    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900 as ::core::ffi::c_int;
    strncpy(
        &raw mut (*fileq).mon as *mut ::core::ffi::c_char,
        s_month[(*p).tm_mon as usize],
        3 as size_t,
    );
    memset(
        &raw mut (*fileq).file_name as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        '\0' as i32,
        (MAX_FQUEUE + 1 as ::core::ffi::c_int) as size_t,
    );
    (*fileq).flags = flags;
    GetFile_Queue(fileq);
    if Handle_Queue(fileq, (*fileq).flags) < 0 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn Read_FileMon(
    mut fileq: *mut file_queue,
    mut p: *const tm,
    mut timeout: ::core::ffi::c_uint,
) -> *mut alert_data {
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut al_data: *mut alert_data = ::core::ptr::null_mut::<alert_data>();
    if (*fileq).fp.is_null() {
        if Handle_Queue(fileq, 0 as ::core::ffi::c_int) != 1 as ::core::ffi::c_int {
            file_sleep();
            return ::core::ptr::null_mut::<alert_data>();
        }
    }
    if (*fileq).fp.is_null() {
        return ::core::ptr::null_mut::<alert_data>();
    }
    al_data = GetAlertData((*fileq).flags, (*fileq).fp);
    if !al_data.is_null() {
        return al_data;
    }
    (*fileq).day = (*p).tm_mday;
    (*fileq).year = (*p).tm_year + 1900 as ::core::ffi::c_int;
    strncpy(
        &raw mut (*fileq).mon as *mut ::core::ffi::c_char,
        s_month[(*p).tm_mon as usize],
        3 as size_t,
    );
    GetFile_Queue(fileq);
    if Handle_Queue(fileq, 0 as ::core::ffi::c_int) != 1 as ::core::ffi::c_int {
        file_sleep();
        return ::core::ptr::null_mut::<alert_data>();
    }
    while i < timeout {
        al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }
        i = i.wrapping_add(1);
        file_sleep();
    }
    return ::core::ptr::null_mut::<alert_data>();
}
