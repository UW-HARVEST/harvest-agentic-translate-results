#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        string: Option<Vec<c_char>>,
        replacement: bool,
        returns: Option<Vec<c_char>>
    },

    library: "driver",
    symbol: "w_utf8_filter",

    signature: unsafe extern "C" fn(*const c_char, bool) -> *mut c_char,

    fn run(&mut self) {
        let string = match &self.string {
            Some(s) => s.as_ptr(),
            None => std::ptr::null(),
        };

        let ret = unsafe {
            (*SYMBOL)(
                string,
                self.replacement
            )
        };

        self.returns = if ret.is_null() {
            None
        } else {
            unsafe {
                let s = CString::from_raw(ret);
                Some(s.to_bytes_with_nul().iter().map(|&b| b as c_char).collect())
            }
        }
    }
}
