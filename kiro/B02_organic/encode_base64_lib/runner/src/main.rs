#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        size: c_int,
        src: Option<Vec<c_char>>,
        returns: Option<CString>,
    },

    library: "driver",
    symbol: "encode_base64",

    signature: unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char,

    fn run(&mut self) {
        let src = match &self.src {
            Some(s) => s.as_ptr(),
            None => std::ptr::null()
        };

        let ret = unsafe {
            (*SYMBOL)(
                self.size,
                src
            )
        };

        self.returns = if ret.is_null() {
            None
        } else {
            Some(unsafe {CString::from_raw(ret)})
        };
    }
}
