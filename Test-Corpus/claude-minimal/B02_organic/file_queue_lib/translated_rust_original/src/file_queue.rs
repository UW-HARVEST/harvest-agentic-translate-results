//! Translation of `file-queue.c` and `file-queue.h`.
//!
//! Implements the [`file_queue`] structure plus the `Init_FileQueue` and
//! `Read_FileMon` C-callable functions.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use libc::{c_char, c_int, c_uint, FILE};
use std::ffi::CString;
use std::ptr;

use crate::read_alert::{alert_data, GetAlertData, ALERTS_DAILY, CRALERT_FP_SET, CRALERT_READ_ALL};
use crate::Tm;

pub const MAX_FQUEUE: usize = 256;
pub const FQ_TIMEOUT: i64 = 5;

/// `struct stat` storage that matches the original `f_status` field. We store
/// a buffer large enough for the platform's `stat`, accessed via raw libc
/// calls.
#[repr(C)]
pub struct file_queue {
    pub last_change: libc::time_t,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: libc::stat,
}

impl Default for file_queue {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const S_MONTH: [&[u8; 3]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

const FSTAT_ERROR: &str = "(1118): Could not retrieve information of file '{}' due to [({})-({})].";
const FSEEK_ERROR: &str = "(1116): Could not set position in file '{}' due to [({})-({})].";

fn merror(template: &str, file_name: &str, err: i32, err_msg: &str) {
    // Replace the first three `{}` placeholders left-to-right.
    let mut out = String::new();
    let mut parts = template.split("{}");
    if let Some(p) = parts.next() {
        out.push_str(p);
    }
    out.push_str(file_name);
    if let Some(p) = parts.next() {
        out.push_str(p);
    }
    out.push_str(&err.to_string());
    if let Some(p) = parts.next() {
        out.push_str(p);
    }
    out.push_str(err_msg);
    if let Some(p) = parts.next() {
        out.push_str(p);
    }
    eprintln!("{}", out);
}

fn file_sleep() {
    let mut tv = libc::timeval {
        tv_sec: FQ_TIMEOUT as libc::time_t,
        tv_usec: 0,
    };
    unsafe {
        libc::select(0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut tv);
    }
}

/// Build the alerts file name, taking the `CRALERT_FP_SET` flag into account.
fn GetFile_Queue(fileq: &mut file_queue) {
    // Zero out the buffer
    for slot in fileq.file_name.iter_mut() {
        *slot = 0;
    }

    let chosen: &[u8] = if (fileq.flags & CRALERT_FP_SET) != 0 {
        b"<stdin>"
    } else {
        ALERTS_DAILY.as_bytes()
    };

    let max = MAX_FQUEUE; // mirror snprintf with size MAX_FQUEUE
    let copy_len = chosen.len().min(max - 1);
    for i in 0..copy_len {
        fileq.file_name[i] = chosen[i] as c_char;
    }
    fileq.file_name[copy_len] = 0;
}

/// Returns a borrowed `&str` view of the file_name field, up to its NUL
/// terminator. Falls back to an empty string on invalid UTF-8.
fn file_name_str(fileq: &file_queue) -> String {
    let len = fileq
        .file_name
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(fileq.file_name.len());
    let bytes: Vec<u8> = fileq.file_name[..len]
        .iter()
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn errno_value() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

fn errno_message() -> String {
    let e = errno_value();
    let ptr = unsafe { libc::strerror(e) };
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let cstr = std::ffi::CStr::from_ptr(ptr);
        cstr.to_string_lossy().into_owned()
    }
}

/// Re-handle the file queue, re-opening the file as needed and seeking to the
/// end (unless `CRALERT_READ_ALL` is set).
fn Handle_Queue(fileq: &mut file_queue, flags: c_int) -> c_int {
    unsafe {
        if (flags & CRALERT_FP_SET) == 0 {
            if !fileq.fp.is_null() {
                libc::fclose(fileq.fp);
                fileq.fp = ptr::null_mut();
            }

            // Reopen for reading.
            let name = file_name_str(fileq);
            let cname = match CString::new(name.clone()) {
                Ok(c) => c,
                Err(_) => return 0,
            };
            let mode = CString::new("r").unwrap();
            fileq.fp = libc::fopen(cname.as_ptr(), mode.as_ptr());
            if fileq.fp.is_null() {
                return 0;
            }
        }

        if (flags & CRALERT_READ_ALL) == 0 {
            if fileq.fp.is_null() {
                return 0;
            }
            if libc::fseek(fileq.fp, 0, libc::SEEK_END) < 0 {
                merror(
                    FSEEK_ERROR,
                    &file_name_str(fileq),
                    errno_value(),
                    &errno_message(),
                );
                libc::fclose(fileq.fp);
                fileq.fp = ptr::null_mut();
                return -1;
            }
        }

        if !fileq.fp.is_null() {
            let fd = libc::fileno(fileq.fp);
            if libc::fstat(fd, &mut fileq.f_status) < 0 {
                merror(
                    FSTAT_ERROR,
                    &file_name_str(fileq),
                    errno_value(),
                    &errno_message(),
                );
                libc::fclose(fileq.fp);
                fileq.fp = ptr::null_mut();
                return -1;
            }
        }

        fileq.last_change = fileq.f_status.st_mtime;
        1
    }
}

/// Initialize the file monitoring queue.
///
/// # Safety
/// `fileq` and `p` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Init_FileQueue(
    fileq: *mut file_queue,
    p: *const Tm,
    flags: c_int,
) -> c_int {
    if fileq.is_null() || p.is_null() {
        return -1;
    }
    let q = unsafe { &mut *fileq };
    let tm = unsafe { &*p };

    if (flags & CRALERT_FP_SET) == 0 {
        q.fp = ptr::null_mut();
    }
    q.last_change = 0;
    q.flags = 0;

    q.day = tm.tm_mday;
    q.year = tm.tm_year + 1900;

    let mon_idx = tm.tm_mon as usize;
    if mon_idx < 12 {
        let m = S_MONTH[mon_idx];
        for i in 0..3 {
            q.mon[i] = m[i] as c_char;
        }
        q.mon[3] = 0;
    } else {
        q.mon[0] = 0;
    }

    for slot in q.file_name.iter_mut() {
        *slot = 0;
    }

    q.flags = flags;

    GetFile_Queue(q);

    if Handle_Queue(q, q.flags) < 0 {
        return -1;
    }
    0
}

/// Read alert data from the monitored file.
///
/// # Safety
/// `fileq` and `p` must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Read_FileMon(
    fileq: *mut file_queue,
    p: *const Tm,
    timeout: c_uint,
) -> *mut alert_data {
    if fileq.is_null() || p.is_null() {
        return ptr::null_mut();
    }
    let q = unsafe { &mut *fileq };
    let tm = unsafe { &*p };

    if q.fp.is_null() {
        if Handle_Queue(q, 0) != 1 {
            file_sleep();
            return ptr::null_mut();
        }
    }

    if q.fp.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let al = GetAlertData(q.flags, q.fp);
        if !al.is_null() {
            return al;
        }
    }

    q.day = tm.tm_mday;
    q.year = tm.tm_year + 1900;
    let mon_idx = tm.tm_mon as usize;
    if mon_idx < 12 {
        let m = S_MONTH[mon_idx];
        for i in 0..3 {
            q.mon[i] = m[i] as c_char;
        }
        q.mon[3] = 0;
    }

    GetFile_Queue(q);

    if Handle_Queue(q, 0) != 1 {
        file_sleep();
        return ptr::null_mut();
    }

    let mut i: c_uint = 0;
    while i < timeout {
        unsafe {
            let al = GetAlertData(q.flags, q.fp);
            if !al.is_null() {
                return al;
            }
        }
        i += 1;
        file_sleep();
    }

    ptr::null_mut()
}
