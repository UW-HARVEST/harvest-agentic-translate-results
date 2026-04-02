#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        orig: Option<Vec<c_char>>,
        search: Option<Vec<c_char>>,
        value: Option<Vec<c_char>>,
        returns: Option<Vec<c_char>>
    },

    library: "driver",
    symbol: "searchAndReplace",

    signature: unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char,

    fn run(&mut self) {
        let orig = match &self.orig {
            Some(o) => o.as_ptr(),
            None => std::ptr::null(),
        };
        let search = match &self.search {
            Some(s) => s.as_ptr(),
            None => std::ptr::null(),
        };
        let value = match &self.value {
            Some(v) => v.as_ptr(),
            None => std::ptr::null(),
        };

        let ret = unsafe {
            (*SYMBOL)(
                orig,
                search,
                value
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
