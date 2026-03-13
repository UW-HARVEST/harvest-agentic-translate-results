#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        path: CString,
        out_dir_name: CString,
        suffix_len: usize,
        returns: CString
    },

    library: "driver",
    symbol: "FIO_createFilename_fromOutDir",

    signature: unsafe extern "C" fn(*const c_char, *const c_char, usize) -> *mut c_char,

    fn run(&mut self) {
        self.returns = unsafe {
            CString::from_raw(
                (*SYMBOL)(
                    self.path.as_ptr(),
                    self.out_dir_name.as_ptr(),
                    self.suffix_len
                )
            )
        };
    }
}
