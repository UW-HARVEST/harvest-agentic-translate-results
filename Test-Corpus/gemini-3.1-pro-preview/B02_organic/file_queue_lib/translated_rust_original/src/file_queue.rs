use libc::{c_int, c_uint, c_char};
use crate::read_alert::{alert_data, GetAlertData, ALERTS_DAILY, CRALERT_FP_SET, CRALERT_READ_ALL};

pub const MAX_FQUEUE: usize = 256;
pub const FQ_TIMEOUT: u32 = 5;

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

unsafe fn merror(err_template: *const c_char, file_name: *const c_char, err: c_int, err_msg: *const c_char) {
    let mut buffer: [c_char; 256] = [0; 256];
    libc::snprintf(buffer.as_mut_ptr(), 256, err_template, file_name, err, err_msg);
    libc::fprintf(libc::stderr, b"%s\n\0".as_ptr() as *const c_char, buffer.as_ptr());
}

unsafe fn file_sleep() {
    let mut fp_timeout = libc::timeval { tv_sec: FQ_TIMEOUT as _, tv_usec: 0 };
    libc::select(0, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), &mut fp_timeout);
}

unsafe fn GetFile_Queue(fileq: *mut file_queue) {
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;
    let name = if ((*fileq).flags & CRALERT_FP_SET) != 0 {
        b"<stdin>\0".as_ptr() as *const c_char
    } else {
        ALERTS_DAILY.as_ptr() as *const c_char
    };
    libc::snprintf((*fileq).file_name.as_mut_ptr(), MAX_FQUEUE, b"%s\0".as_ptr() as *const c_char, name);
}

unsafe fn Handle_Queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
        if !(*fileq).fp.is_null() {
            libc::fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut();
        }
        (*fileq).fp = libc::fopen((*fileq).file_name.as_ptr(), b"r\0".as_ptr() as *const c_char);
        if (*fileq).fp.is_null() {
            return 0;
        }
    }

    if (flags & CRALERT_READ_ALL) == 0 {
        if (*fileq).fp.is_null() {
            return 0;
        }
        if libc::fseek((*fileq).fp, 0, libc::SEEK_END) < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            merror(b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].\0".as_ptr() as *const c_char, (*fileq).file_name.as_ptr(), errno, libc::strerror(errno));
            libc::fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut();
            return -1;
        }
    }

    if !(*fileq).fp.is_null() {
        if libc::fstat(libc::fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            merror(b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].\0".as_ptr() as *const c_char, (*fileq).file_name.as_ptr(), errno, libc::strerror(errno));
            libc::fclose((*fileq).fp);
            (*fileq).fp = std::ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;
    1
}

static S_MONTH: [*const c_char; 12] = [
    b"Jan\0".as_ptr() as *const c_char, b"Feb\0".as_ptr() as *const c_char, b"Mar\0".as_ptr() as *const c_char,
    b"Apr\0".as_ptr() as *const c_char, b"May\0".as_ptr() as *const c_char, b"Jun\0".as_ptr() as *const c_char,
    b"Jul\0".as_ptr() as *const c_char, b"Aug\0".as_ptr() as *const c_char, b"Sep\0".as_ptr() as *const c_char,
    b"Oct\0".as_ptr() as *const c_char, b"Nov\0".as_ptr() as *const c_char, b"Dec\0".as_ptr() as *const c_char,
];

#[unsafe(no_mangle)]
pub extern "C" fn Init_FileQueue(fileq: *mut file_queue, p: *const libc::tm, flags: c_int) -> c_int {
    unsafe {
        if (flags & CRALERT_FP_SET) == 0 {
            (*fileq).fp = std::ptr::null_mut();
        }
        (*fileq).last_change = 0;
        (*fileq).flags = 0;

        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year + 1900;

        libc::strncpy((*fileq).mon.as_mut_ptr(), S_MONTH[(*p).tm_mon as usize], 3);
        libc::memset((*fileq).file_name.as_mut_ptr() as *mut libc::c_void, 0, MAX_FQUEUE + 1);

        (*fileq).flags = flags;

        GetFile_Queue(fileq);

        if Handle_Queue(fileq, (*fileq).flags) < 0 {
            return -1;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Read_FileMon(fileq: *mut file_queue, p: *const libc::tm, timeout: c_uint) -> *mut alert_data {
    unsafe {
        let mut i = 0;

        if (*fileq).fp.is_null() {
            if Handle_Queue(fileq, 0) != 1 {
                file_sleep();
                return std::ptr::null_mut();
            }
        }

        if (*fileq).fp.is_null() {
            return std::ptr::null_mut();
        }

        let mut al_data = GetAlertData((*fileq).flags, (*fileq).fp);
        if !al_data.is_null() {
            return al_data;
        }

        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year + 1900;
        libc::strncpy((*fileq).mon.as_mut_ptr(), S_MONTH[(*p).tm_mon as usize], 3);

        GetFile_Queue(fileq);

        if Handle_Queue(fileq, 0) != 1 {
            file_sleep();
            return std::ptr::null_mut();
        }

        while i < timeout {
            al_data = GetAlertData((*fileq).flags, (*fileq).fp);
            if !al_data.is_null() {
                return al_data;
            }
            i += 1;
            file_sleep();
        }

        std::ptr::null_mut()
    }
}
