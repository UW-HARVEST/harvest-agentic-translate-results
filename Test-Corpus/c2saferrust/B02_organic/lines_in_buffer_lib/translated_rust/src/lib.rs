
extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn UTIL_createLinePointers(
    buffer: &mut [u8],
    numLines: usize,
    bufferSize: usize,
) -> Option<Vec<&[u8]>> {
    let effective_size = bufferSize.min(buffer.len());
    let mut line_pointers = Vec::with_capacity(numLines);
    let mut pos = 0usize;

    while line_pointers.len() < numLines && pos < effective_size {
        let start = pos;
        let mut len = 0usize;

        while pos + len < effective_size && buffer[pos + len] != 0 {
            len += 1;
        }

        line_pointers.push(&buffer[start..start + len]);
        pos += len;

        if pos < effective_size {
            pos += 1;
        }
    }

    if line_pointers.len() != numLines {
        return None;
    }

    Some(line_pointers)
}

