use std::os::raw::c_char;

/// Create an array of pointers to the lines in a buffer.
///
/// # Safety
/// - `buffer` must point to a readable memory region of at least `buffer_size` bytes.
/// - The returned pointer (when non-null) is allocated with `Box`/the global Rust
///   allocator and must be freed via `UTIL_freeLinePointers` (do NOT free with
///   `libc::free`).
/// - The returned array contains `num_lines` pointers into `buffer`. The caller
///   must ensure `buffer` remains valid as long as the returned pointers are used.
#[no_mangle]
pub unsafe extern "C" fn UTIL_createLinePointers(
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> *mut *const c_char {
    if buffer.is_null() && buffer_size != 0 {
        return std::ptr::null_mut();
    }

    // Allocate a Vec of `num_lines` raw pointers, initialized to null.
    let mut line_pointers: Vec<*const c_char> = vec![std::ptr::null(); num_lines];

    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    while line_index < num_lines && pos < buffer_size {
        let mut len: usize = 0;
        line_pointers[line_index] = buffer.add(pos) as *const c_char;
        line_index += 1;

        // Find the next null terminator, being careful not to go past the buffer.
        while (pos + len) < buffer_size && *buffer.add(pos + len) != 0 {
            len += 1;
        }

        // Move past this string and its null terminator.
        pos += len;
        if pos < buffer_size {
            pos += 1; // Skip the null terminator if we're not at buffer end.
        }
    }

    // Verify we processed the expected number of lines.
    if line_index != num_lines {
        // Something went wrong - we didn't find as many lines as expected.
        // The Vec will be dropped (freed) automatically.
        return std::ptr::null_mut();
    }

    // Convert the Vec into a raw pointer to a heap-allocated boxed slice
    // so we can return ownership of the buffer over FFI.
    let boxed_slice: Box<[*const c_char]> = line_pointers.into_boxed_slice();
    Box::into_raw(boxed_slice) as *mut *const c_char
}

/// Free a pointer array previously returned by `UTIL_createLinePointers`.
///
/// # Safety
/// - `ptr` must be a pointer previously returned by `UTIL_createLinePointers`
///   with the same `num_lines`, or null.
/// - After calling this, `ptr` must not be used again.
#[no_mangle]
pub unsafe extern "C" fn UTIL_freeLinePointers(ptr: *mut *const c_char, num_lines: usize) {
    if ptr.is_null() {
        return;
    }
    let slice_ptr = std::ptr::slice_from_raw_parts_mut(ptr, num_lines);
    drop(Box::from_raw(slice_ptr));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_create_line_pointers_basic() {
        // Buffer with three null-terminated strings: "foo\0bar\0baz\0"
        let mut buffer: Vec<u8> = b"foo\0bar\0baz\0".to_vec();
        let buffer_size = buffer.len();
        let num_lines = 3;

        unsafe {
            let lines = UTIL_createLinePointers(
                buffer.as_mut_ptr() as *mut c_char,
                num_lines,
                buffer_size,
            );
            assert!(!lines.is_null());

            let s0 = CStr::from_ptr(*lines.add(0)).to_str().unwrap();
            let s1 = CStr::from_ptr(*lines.add(1)).to_str().unwrap();
            let s2 = CStr::from_ptr(*lines.add(2)).to_str().unwrap();
            assert_eq!(s0, "foo");
            assert_eq!(s1, "bar");
            assert_eq!(s2, "baz");

            UTIL_freeLinePointers(lines, num_lines);
        }
    }

    #[test]
    fn test_create_line_pointers_too_few() {
        // Only two strings but caller asks for three.
        let mut buffer: Vec<u8> = b"foo\0bar\0".to_vec();
        let buffer_size = buffer.len();
        let num_lines = 3;

        unsafe {
            let lines = UTIL_createLinePointers(
                buffer.as_mut_ptr() as *mut c_char,
                num_lines,
                buffer_size,
            );
            assert!(lines.is_null());
        }
    }

    #[test]
    fn test_create_line_pointers_zero_lines() {
        let mut buffer: Vec<u8> = b"".to_vec();
        unsafe {
            let lines =
                UTIL_createLinePointers(buffer.as_mut_ptr() as *mut c_char, 0, 0);
            // With num_lines == 0, the loop never runs and line_index == num_lines (0),
            // so we should get a non-null (empty) allocation.
            assert!(!lines.is_null());
            UTIL_freeLinePointers(lines, 0);
        }
    }
}
