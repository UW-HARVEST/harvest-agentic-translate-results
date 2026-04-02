#![cfg_attr(fuzzing, no_main)]

use cando2::*;

state_member! {
    pub struct cJSON {
        type_t: c_int,
        valueint: c_int,
        valuedouble: c_double,
    }
}

state_member! {
    /// This is the struct that is being parsed from the file to be
    /// able to treat content as a String
    pub struct parse_buffer_from_file {
        content: CString,
        length: usize,
        offset: usize,
        depth: usize,
    }
}

#[repr(C)]
/// This is the struct used to actually pass the parse_buffer struct over
/// the FFI to the C side of things with the appropriate content value
pub struct parse_buffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
}

#[allow(dead_code)]
const CJSON_NUMBER: u32 = 1 << 3;

harness! {
    state: {
        item: Option<cJSON>,
        input_buffer: Option<parse_buffer_from_file>,
        returns: bool
    },

    library: "driver",
    symbol: "parse_number",

    signature: unsafe extern "C" fn(*const cJSON, *const parse_buffer) -> bool,

    fn run(&mut self) {
        let item = match self.item {
            Some(ref i) => i,
            None => std::ptr::null(),
        };

        // Convert the CString to a *const c_uchar to pass to C
        let input_buffer = match self.input_buffer {
            Some(ref b) => &parse_buffer {
                content: b.content.as_ptr() as *const c_uchar,
                length: b.length,
                offset: b.offset,
                depth: b.depth
            },
            None => std::ptr::null(),
        };


        self.returns = unsafe {
            (*SYMBOL)(
                item,
                input_buffer
            )
        }
    }
}
