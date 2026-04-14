use crate::read_alert::{alert_data, GetAlertData, ALERTS_DAILY, CRALERT_FP_SET, CRALERT_READ_ALL};
use libc::{FILE, stat, time_t, tm};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr;
use std::thread;
use std::time::Duration;

pub const MAX_FQUEUE: usize = 256;
pub const FQ_TIMEOUT: u64 = 5;

const FSTAT_ERROR: &str = "(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].";
const FSEEK_ERROR: &str = "(1116): Could not set position in file '%s' due to [(%d)-(%s)].";

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

fn merror(err_template: &str, file_name: &str, err: c_int, err_msg: &str) {
    let formatted = err_template
        .replace("%s", "{}")
        .replacen("%d", "{}", 1)
        .replacen("%s", "{}", 1);
    eprintln!("{}", formatted.replacen("{}", file_name, 1).replacen("{}", &err.to_string(), 1).replacen("{}", err_msg, 1));
}

fn file_sleep() {
    thread::sleep(Duration::from_secs(FQ_TIMEOUT));
}

static S_MONTH: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

unsafe fn write_c_string_fixed(dst: *mut c_char, cap: usize, s: &str) {
    if cap == 0 {
        return;
    }
    ptr::write_bytes(dst, 0, cap);
    let bytes = s.as_bytes();
    let len = bytes.len().min(cap - 1);
    ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, dst, len);
}

unsafe fn read_c_string_fixed(src: *const c_char) -> String {
    if src.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(src).to_string_lossy().into_owned()
}

unsafe fn get_file_queue(fileq: *mut file_queue) {
    (*fileq).file_name[0] = 0;
    (*fileq).file_name[MAX_FQUEUE] = 0;
    let name = if ((*fileq).flags & CRALERT_FP_SET) != 0 { "<stdin>" } else { ALERTS_DAILY };
    write_c_string_fixed((*fileq).file_name.as_mut_ptr(), MAX_FQUEUE + 1, name);
}

unsafe fn handle_queue(fileq: *mut file_queue, flags: c_int) -> c_int {
    if (flags & CRALERT_FP_SET) == 0 {
        if !(*fileq).fp.is_null() {
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
        }

        let filename = read_c_string_fixed((*fileq).file_name.as_ptr());
        let c_filename = CString::new(filename).unwrap_or_else(|_| CString::new("").unwrap());
        let mode = CString::new("r").unwrap();
        (*fileq).fp = libc::fopen(c_filename.as_ptr(), mode.as_ptr());
        if (*fileq).fp.is_null() {
            return 0;
        }
    }

    if (flags & CRALERT_READ_ALL) == 0 {
        if (*fileq).fp.is_null() {
            return 0;
        }
        if libc::fseek((*fileq).fp, 0, libc::SEEK_END) < 0 {
            let filename = read_c_string_fixed((*fileq).file_name.as_ptr());
            let err = *libc::__errno_location();
            let err_msg = std::ffi::CStr::from_ptr(libc::strerror(err)).to_string_lossy().into_owned();
            merror(FSEEK_ERROR, &filename, err, &err_msg);
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    if !(*fileq).fp.is_null() {
        if libc::fstat(libc::fileno((*fileq).fp), &mut (*fileq).f_status) < 0 {
            let filename = read_c_string_fixed((*fileq).file_name.as_ptr());
            let err = *libc::__errno_location();
            let err_msg = std::ffi::CStr::from_ptr(libc::strerror(err)).to_string_lossy().into_owned();
            merror(FSTAT_ERROR, &filename, err, &err_msg);
            libc::fclose((*fileq).fp);
            (*fileq).fp = ptr::null_mut();
            return -1;
        }
    }

    (*fileq).last_change = (*fileq).f_status.st_mtime;
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn Init_FileQueue(fileq: *mut file_queue, p: *const tm, flags: c_int) -> c_int {
    if fileq.is_null() || p.is_null() {
        return -1;
    }
    unsafe {
        if (flags & CRALERT_FP_SET) == 0 {
            (*fileq).fp = ptr::null_mut();
        }
        (*fileq).last_change = 0;
        (*fileq).flags = 0;
        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year + 1900;
        let mon = if (*p).tm_mon >= 0 && (*p).tm_mon < 12 { S_MONTH[(*p).tm_mon as usize] } else { "" };
        write_c_string_fixed((*fileq).mon.as_mut_ptr(), 4, mon);
        ptr::write_bytes((*fileq).file_name.as_mut_ptr(), 0, MAX_FQUEUE + 1);
        (*fileq).flags = flags;
        get_file_queue(fileq);
        if handle_queue(fileq, (*fileq).flags) < 0 {
            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Read_FileMon(fileq: *mut file_queue, p: *const tm, timeout: c_uint) -> *mut alert_data {
    if fileq.is_null() || p.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        let mut i = 0u32;

        if (*fileq).fp.is_null() {
            if handle_queue(fileq, 0) != 1 {
                file_sleep();
                return ptr::null_mut();
            }
        }

        if (*fileq).fp.is_null() {
            return ptr::null_mut();
        }

        let first = GetAlertData((*fileq).flags, (*fileq).fp);
        if !first.is_null() {
            return first;
        }

        (*fileq).day = (*p).tm_mday;
        (*fileq).year = (*p).tm_year + 1900;
        let mon = if (*p).tm_mon >= 0 && (*p).tm_mon < 12 { S_MONTH[(*p).tm_mon as usize] } else { "" };
        write_c_string_fixed((*fileq).mon.as_mut_ptr(), 4, mon);

        get_file_queue(fileq);

        if handle_queue(fileq, 0) != 1 {
            file_sleep();
            return ptr::null_mut();
        }

        while i < timeout {
            let al_data = GetAlertData((*fileq).flags, (*fileq).fp);
            if !al_data.is_null() {
                return al_data;
            }
            i += 1;
            file_sleep();
        }

        ptr::null_mut()
    }
}
