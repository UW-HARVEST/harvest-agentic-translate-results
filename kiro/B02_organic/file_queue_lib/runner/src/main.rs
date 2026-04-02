#![cfg_attr(fuzzing, no_main)]

use cando2::*;
use std::{
    fs::{self, File},
    io::Write,
};

#[repr(C)]
#[derive(Arbitrary, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct alert_data {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: Option<CString>,
    pub date: Option<CString>,
    pub location: Option<CString>,
    pub comment: Option<CString>,
    pub group: Option<CString>,
    pub srcip: Option<CString>,
    pub srcport: c_int,
    pub dstip: Option<CString>,
    pub dstport: c_int,
    pub user: Option<CString>,
    pub filename: Option<CString>,
}

#[repr(C)]
#[derive(Clone)]
pub struct ffi_alert_data {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: *const c_char,
    pub date: *const c_char,
    pub location: *const c_char,
    pub comment: *const c_char,
    pub group: *const c_char,
    pub srcip: *const c_char,
    pub srcport: c_int,
    pub dstip: *const c_char,
    pub dstport: c_int,
    pub user: *const c_char,
    pub filename: *const c_char,
}

/// Helper function that panics if there's an improper CString
unsafe fn create_cstring(str: *const c_char) -> Option<CString> {
    if str.is_null() {
        return None;
    }

    unsafe {
        let cstr = CStr::from_ptr(str);
        Some(CString::new(cstr.to_bytes()).expect("Rust runner error: Failed to create CString"))
    }
}

harness! {
    state: {
        file_content: CString,
        day: c_int,
        month: c_int,
        year: c_int,
        timeout: c_uint,
        flags: c_int,
        returns: Option<alert_data>,
    },

    library: "driver",
    symbol: "driver",

    signature: unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut ffi_alert_data,

    fn run(&mut self) {
        let fname = "alerts.log";
        let mut file = File::create(fname).expect(&format!("Rust Runner: Error creating file {}", fname));
        file.write_all(self.file_content.as_bytes()).expect(&format!("Rust Runner: Error writing to file {}", fname));

        let ret = unsafe {
            (*SYMBOL)(
                self.day,
                self.month,
                self.year,
                self.timeout,
                self.flags
            )
        };

        self.returns = if ret.is_null() {
            None
        } else {
            unsafe {
                let alert = (*ret).clone();
                Some(alert_data {
                    rule: alert.rule,
                    level: alert.level,
                    alertid: create_cstring(alert.alertid),
                    date: create_cstring(alert.date),
                    location: create_cstring(alert.location),
                    comment: create_cstring(alert.comment),
                    group: create_cstring(alert.group),
                    srcip: create_cstring(alert.srcip),
                    srcport: alert.srcport,
                    dstip: create_cstring(alert.dstip),
                    dstport: alert.dstport,
                    user: create_cstring(alert.user),
                    filename: create_cstring(alert.filename),
                })
            }
        };

        let _ = fs::remove_file(fname);
    }
}
