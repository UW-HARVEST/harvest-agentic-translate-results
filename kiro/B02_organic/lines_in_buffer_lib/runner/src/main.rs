#![cfg_attr(fuzzing, no_main)]

use cando2::*;

/// Processes the input buffer to replace all instances of \\0 to \0
/// FIXME: Look into if RON can handle \0 better than this
fn process_buffer(buffer: CString) -> Vec<c_char> {
    let mut vec = buffer.into_bytes();
    let mut i = 0;

    while i < vec.len() {
        if vec[i] == b'\\' && i + 1 < vec.len() && vec[i + 1] == b'0' {
            vec[i] = b'\0';
            vec.remove(i + 1);
        }
        i += 1;
    }

    vec.into_iter().map(|b| b as c_char).collect()
}

harness! {
    state: {
        buffer: CString,
        num_lines: usize,
        buffer_size: usize,
        returns: Vec<CString>
    },

    library: "driver",
    symbol: "UTIL_createLinePointers",

    signature: unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char,

    fn run(&mut self) {
        let buffer = process_buffer(self.buffer.clone());

        let ret = unsafe {
            (*SYMBOL)(
                buffer.as_ptr() as *mut c_char,
                self.num_lines,
                self.buffer_size
            )
        };

        if ret.is_null() {
            self.returns = Vec::new();
            return;
        }

        let mut vec_lines = Vec::new();

        for i in 0..self.num_lines {
            unsafe {
                let c_str = *ret.add(i);
                if c_str.is_null() {
                    break;
                }

                vec_lines.push(CString::from(CStr::from_ptr(c_str).to_owned()))
            }
        }
        self.returns = vec_lines;
    }
}
