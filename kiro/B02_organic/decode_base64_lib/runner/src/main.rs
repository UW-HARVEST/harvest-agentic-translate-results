#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        src: Option<Vec<c_char>>,
        returns: Option<Vec<c_char>>
    },

    library: "driver",
    symbol: "decode_base64",

    signature: unsafe extern "C" fn(*const c_char) -> *mut c_char,

    fn run(&mut self) {
        let src = match &self.src {
            Some(s) => s.as_ptr(),
            None => std::ptr::null()
        };

        let ret = unsafe {
            (*SYMBOL)(src)
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
