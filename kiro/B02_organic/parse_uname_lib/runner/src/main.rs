#![cfg_attr(fuzzing, no_main)]

use cando2::*;

state_member! {
    pub struct os_data {
        pub os_name: Option<CString>,
        pub os_version: Option<CString>,
        pub os_major: Option<CString>,
        pub os_minor: Option<CString>,
        pub os_codename: Option<CString>,
        pub os_platform: Option<CString>,
        pub os_build: Option<CString>,
        pub os_uname: Option<CString>,
        pub os_arch: Option<CString>,
    }
}

#[repr(C)]
pub struct os_data_p {
    pub os_name: *mut c_char,
    pub os_version: *mut c_char,
    pub os_major: *mut c_char,
    pub os_minor: *mut c_char,
    pub os_codename: *mut c_char,
    pub os_platform: *mut c_char,
    pub os_build: *mut c_char,
    pub os_uname: *mut c_char,
    pub os_arch: *mut c_char,
}

harness! {
    state: {
        uname: Option<CString>,
        osd: Option<os_data>,
    },

    library: "driver",
    symbol: "parse_uname_string",

    signature: unsafe extern "C" fn(*mut c_char, *mut os_data_p),

    fn run(&mut self) {
        let uname = match &self.uname {
            Some(s) => s.clone().into_raw(),
            None => std::ptr::null_mut(),
        };

        // Put osd from JSON into proper form for FFI
        let mut osd = match &mut self.osd {
            Some(d) => os_data_p {
                os_name: d.os_name.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_version: d.os_version.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_major: d.os_major.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_minor: d.os_minor.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_codename: d.os_codename.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_platform: d.os_platform.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_build: d.os_build.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_uname: d.os_uname.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
                os_arch: d.os_arch.as_ref().map_or(std::ptr::null_mut(), |s| s.clone().into_raw()),
            },
            None => os_data_p {
                os_name: std::ptr::null_mut(),
                os_version: std::ptr::null_mut(),
                os_major: std::ptr::null_mut(),
                os_minor: std::ptr::null_mut(),
                os_codename: std::ptr::null_mut(),
                os_platform: std::ptr::null_mut(),
                os_build: std::ptr::null_mut(),
                os_uname: std::ptr::null_mut(),
                os_arch: std::ptr::null_mut(),
            },
        };

        unsafe {
            (*SYMBOL)(
                uname,
                &raw mut osd,
            )
        }

        self.uname = if uname.is_null() {
            None
        } else {
            unsafe {Some(CString::from_raw(uname))}
        };

        // Get back into os_data for comparison with JSON
        self.osd = Some(os_data {
            os_name: if osd.os_name.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_name)) }
            },
            os_version: if osd.os_version.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_version)) }
            },
            os_major: if osd.os_major.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_major)) }
            },
            os_minor: if osd.os_minor.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_minor)) }
            },
            os_codename: if osd.os_codename.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_codename)) }
            },
            os_platform: if osd.os_platform.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_platform)) }
            },
            os_build: if osd.os_build.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_build)) }
            },
            os_uname: if osd.os_uname.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_uname)) }
            },
            os_arch: if osd.os_arch.is_null() {
                None
            } else {
                unsafe { Some(CString::from_raw(osd.os_arch)) }
            },
        });
    }
}
