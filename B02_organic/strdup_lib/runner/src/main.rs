#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        str: Option<CString>,
        returns: Option<CString>,
    },

    library: "driver",
    symbol: "custom_strdup",

    signature: unsafe extern "C" fn(*const c_char) -> *mut c_char,

    fn run(&mut self) {
        let s = if let Some(ref s) = self.str {
            s.as_ptr()
        } else {
            std::ptr::null()
        };

        let ret = unsafe {
            (*SYMBOL)(
                s
            )
        };

        if ret.is_null() {
            self.returns = None;
        } else {
            self.returns = unsafe { Some(CString::from_raw(ret)) };
        }
    }
}
