extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> libc::c_int;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    
    
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
}
pub use crate::src::file_queue::Init_FileQueue;
pub use crate::src::file_queue::Read_FileMon;
pub type size_t = usize;
pub type __dev_t = libc::c_ulong;
pub type __uid_t = libc::c_uint;
pub type __gid_t = libc::c_uint;
pub type __ino_t = libc::c_ulong;
pub type __mode_t = libc::c_uint;
pub type __nlink_t = libc::c_ulong;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub type __time_t = libc::c_long;
pub type __blksize_t = libc::c_long;
pub type __blkcnt_t = libc::c_long;
pub type __syscall_slong_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub type time_t = __time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: libc::c_int,
    pub tm_min: libc::c_int,
    pub tm_hour: libc::c_int,
    pub tm_mday: libc::c_int,
    pub tm_mon: libc::c_int,
    pub tm_year: libc::c_int,
    pub tm_wday: libc::c_int,
    pub tm_yday: libc::c_int,
    pub tm_isdst: libc::c_int,
    pub tm_gmtoff: libc::c_long,
    pub tm_zone: *const libc::c_char,
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
    pub __pad0: libc::c_int,
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
    pub year: libc::c_int,
    pub day: libc::c_int,
    pub flags: libc::c_int,
    pub mon: [libc::c_char; 4],
    pub file_name: [libc::c_char; 257],
    pub fp: *mut FILE,
    pub f_status: stat,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct alert_data {
    pub rule: libc::c_uint,
    pub level: libc::c_uint,
    pub alertid: *mut libc::c_char,
    pub date: *mut libc::c_char,
    pub location: *mut libc::c_char,
    pub comment: *mut libc::c_char,
    pub group: *mut libc::c_char,
    pub srcip: *mut libc::c_char,
    pub srcport: libc::c_int,
    pub dstip: *mut libc::c_char,
    pub dstport: libc::c_int,
    pub user: *mut libc::c_char,
    pub filename: *mut libc::c_char,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut day: libc::c_int,
    mut month: libc::c_int,
    mut year: libc::c_int,
    mut timeout: libc::c_uint,
    mut flags: libc::c_int,
) -> *mut alert_data {
    let mut time: tm = tm {
        tm_sec: 0 as libc::c_int,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null::<libc::c_char>(),
    };
    time.tm_mday = day;
    time.tm_mon = month;
    time.tm_year = year;
    let mut fq: file_queue = file_queue {
        last_change: 0,
        year: 0,
        day: 0,
        flags: 0,
        mon: [0; 4],
        file_name: [0; 257],
        fp: std::ptr::null_mut::<FILE>(),
        f_status: stat {
            st_dev: 0,
            st_ino: 0,
            st_nlink: 0,
            st_mode: 0,
            st_uid: 0,
            st_gid: 0,
            __pad0: 0,
            st_rdev: 0,
            st_size: 0,
            st_blksize: 0,
            st_blocks: 0,
            st_atim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_mtim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            st_ctim: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            __glibc_reserved: [0; 3],
        },
    };
    memset(
        &raw mut fq as *mut libc::c_void,
        0 as libc::c_int,
        std::mem::size_of::<file_queue>() as size_t,
    );
    if Init_FileQueue(&raw mut fq, &raw mut time, flags) < 0 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"File queue initialization failed\0" as *const u8 as *const libc::c_char,
        );
        return std::ptr::null_mut::<alert_data>();
    }
    let mut al_data: *mut alert_data = Read_FileMon(&raw mut fq, &raw mut time, timeout);
    if !fq.fp.is_null() {
        fclose(fq.fp);
    }
    return al_data;
}
