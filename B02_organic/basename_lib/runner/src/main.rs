#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        path: CString,
        returns: CString,
    },

    library: "driver",
    symbol: "tool_basename",

    signature: unsafe extern "C" fn(*mut c_char) -> *mut c_char,

    fn run(&mut self) {
        let ret = unsafe {
            CStr::from_ptr(
                (*SYMBOL)(
                    self.path.as_ptr() as *mut c_char
                )
            )
        };

        self.returns = ret.to_owned();
    }
}
