// Rust translation of C buffer manipulation library.
// Preserves exact behavior and byte-identical I/O of original C program.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_char;
use std::os::raw::{c_int, c_uint, c_void};

// ==================== Data Structures ====================

#[repr(C)]
#[derive(Copy, Clone)]
pub struct buffer_t {
    pub data: [u8; 256],
    pub length: usize,
    pub checksum: u32,
}

#[repr(C)]
pub struct buffer_array_t {
    pub buffers: *mut buffer_t,
    pub count: c_int,
    pub capacity: c_int,
}

// operation_t is a C enum (int-sized)
pub const OP_COPY: c_int = 0;
pub const OP_REVERSE: c_int = 1;
pub const OP_MERGE: c_int = 2;
pub const OP_SPLIT: c_int = 3;
pub const OP_INTERLEAVE: c_int = 4;
pub const OP_ROTATE: c_int = 5;
pub const OP_CHECKSUM: c_int = 6;

// ==================== libc bindings ====================

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, fmt: *const c_char, ...) -> c_int;
    fn scanf(fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    static stderr: *mut c_void;
}

// ==================== Helper Functions ====================

/// Calculate simple checksum
#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_checksum(data: *const u8, length: usize) -> u32 {
    let mut sum: u32 = 0;
    let mut i: usize = 0;
    while i < length {
        let b = unsafe { *data.add(i) };
        sum = (sum.wrapping_shl(3)) ^ (b as u32);
        i += 1;
    }
    sum
}

/// Validate buffer integrity
#[unsafe(no_mangle)]
pub unsafe extern "C" fn validate_buffer(buf: *const buffer_t) -> bool {
    if buf.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL buffer\n".as_ptr());
        }
        return false;
    }
    let length = unsafe { (*buf).length };
    if length > 256 {
        unsafe {
            fprintf(
                stderr,
                c"Error: Buffer length %zu exceeds maximum 256\n".as_ptr(),
                length,
            );
        }
        return false;
    }
    let expected = unsafe { calculate_checksum((*buf).data.as_ptr(), length) };
    let checksum = unsafe { (*buf).checksum };
    if checksum != expected {
        unsafe {
            fprintf(
                stderr,
                c"Warning: Checksum mismatch. Expected %u, got %u\n".as_ptr(),
                expected as c_uint,
                checksum as c_uint,
            );
        }
    }
    true
}

/// Initialize buffer array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_buffer_array(initial_capacity: c_int) -> *mut buffer_array_t {
    if initial_capacity <= 0 {
        unsafe {
            fprintf(
                stderr,
                c"Error: Invalid capacity %d\n".as_ptr(),
                initial_capacity,
            );
        }
        return std::ptr::null_mut();
    }

    let arr = unsafe { malloc(std::mem::size_of::<buffer_array_t>()) as *mut buffer_array_t };
    if arr.is_null() {
        unsafe {
            fprintf(stderr, c"Error: Failed to allocate buffer array\n".as_ptr());
        }
        return std::ptr::null_mut();
    }

    let buffers_size = std::mem::size_of::<buffer_t>() * (initial_capacity as usize);
    let buffers = unsafe { malloc(buffers_size) as *mut buffer_t };
    if buffers.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Error: Failed to allocate buffer storage\n".as_ptr(),
            );
            free(arr as *mut c_void);
        }
        return std::ptr::null_mut();
    }

    unsafe {
        (*arr).buffers = buffers;
        (*arr).count = 0;
        (*arr).capacity = initial_capacity;
    }
    arr
}

/// Free buffer array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_buffer_array(arr: *mut buffer_array_t) {
    if !arr.is_null() {
        unsafe {
            free((*arr).buffers as *mut c_void);
            free(arr as *mut c_void);
        }
    }
}

// ==================== Core Buffer Operations ====================

/// Simple copy operation with memcpy
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_copy(src: *const buffer_t, dst: *mut buffer_t) -> c_int {
    if src.is_null() || dst.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL pointer in buffer_copy\n".as_ptr());
        }
        return -1;
    }

    if !unsafe { validate_buffer(src) } {
        return -1;
    }

    unsafe {
        memcpy(
            (*dst).data.as_mut_ptr() as *mut c_void,
            (*src).data.as_ptr() as *const c_void,
            (*src).length,
        );
        (*dst).length = (*src).length;
        (*dst).checksum = calculate_checksum((*dst).data.as_ptr(), (*dst).length);
    }

    0
}

/// Reverse buffer contents
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_reverse(buf: *mut buffer_t) -> c_int {
    if buf.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL buffer in reverse\n".as_ptr());
        }
        return -1;
    }

    let length = unsafe { (*buf).length };
    if length == 0 {
        return 0; // Nothing to reverse
    }

    let mut temp: [u8; 256] = [0u8; 256];
    unsafe {
        memcpy(
            temp.as_mut_ptr() as *mut c_void,
            (*buf).data.as_ptr() as *const c_void,
            length,
        );
    }

    let mut i: usize = 0;
    while i < length {
        unsafe {
            (*buf).data[i] = temp[length - 1 - i];
        }
        i += 1;
    }

    unsafe {
        (*buf).checksum = calculate_checksum((*buf).data.as_ptr(), length);
    }
    0
}

/// Merge two buffers into destination
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_merge(
    src1: *const buffer_t,
    src2: *const buffer_t,
    dst: *mut buffer_t,
) -> c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL pointer in buffer_merge\n".as_ptr());
        }
        return -1;
    }

    let len1 = unsafe { (*src1).length };
    let len2 = unsafe { (*src2).length };
    if len1 + len2 > 256 {
        unsafe {
            fprintf(
                stderr,
                c"Error: Merged length %zu exceeds maximum\n".as_ptr(),
                len1 + len2,
            );
        }
        return -1;
    }

    unsafe {
        memcpy(
            (*dst).data.as_mut_ptr() as *mut c_void,
            (*src1).data.as_ptr() as *const c_void,
            len1,
        );
        memcpy(
            (*dst).data.as_mut_ptr().add(len1) as *mut c_void,
            (*src2).data.as_ptr() as *const c_void,
            len2,
        );

        (*dst).length = len1 + len2;
        (*dst).checksum = calculate_checksum((*dst).data.as_ptr(), (*dst).length);
    }

    0
}

/// Split buffer at position into two buffers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_split(
    src: *const buffer_t,
    split_pos: usize,
    dst1: *mut buffer_t,
    dst2: *mut buffer_t,
) -> c_int {
    if src.is_null() || dst1.is_null() || dst2.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL pointer in buffer_split\n".as_ptr());
        }
        return -1;
    }

    let src_length = unsafe { (*src).length };
    if split_pos > src_length {
        unsafe {
            fprintf(
                stderr,
                c"Error: Split position %zu exceeds length %zu\n".as_ptr(),
                split_pos,
                src_length,
            );
        }
        return -1;
    }

    unsafe {
        if split_pos > 0 {
            memcpy(
                (*dst1).data.as_mut_ptr() as *mut c_void,
                (*src).data.as_ptr() as *const c_void,
                split_pos,
            );
        }
        (*dst1).length = split_pos;
        (*dst1).checksum = calculate_checksum((*dst1).data.as_ptr(), (*dst1).length);

        let remaining = src_length - split_pos;
        if remaining > 0 {
            memcpy(
                (*dst2).data.as_mut_ptr() as *mut c_void,
                (*src).data.as_ptr().add(split_pos) as *const c_void,
                remaining,
            );
        }
        (*dst2).length = remaining;
        (*dst2).checksum = calculate_checksum((*dst2).data.as_ptr(), (*dst2).length);
    }

    0
}

/// Interleave two buffers (alternating bytes)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_interleave(
    src1: *const buffer_t,
    src2: *const buffer_t,
    dst: *mut buffer_t,
) -> c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Error: NULL pointer in buffer_interleave\n".as_ptr(),
            );
        }
        return -1;
    }

    let len1 = unsafe { (*src1).length };
    let len2 = unsafe { (*src2).length };
    let max_len = if len1 > len2 { len1 } else { len2 };
    if len1 + len2 > 256 {
        unsafe {
            fprintf(
                stderr,
                c"Error: Interleaved length exceeds maximum\n".as_ptr(),
            );
        }
        return -1;
    }

    let mut dst_pos: usize = 0;
    let mut i: usize = 0;
    while i < max_len {
        unsafe {
            if i < len1 {
                memcpy(
                    (*dst).data.as_mut_ptr().add(dst_pos) as *mut c_void,
                    (*src1).data.as_ptr().add(i) as *const c_void,
                    1,
                );
                dst_pos += 1;
            }
            if i < len2 {
                memcpy(
                    (*dst).data.as_mut_ptr().add(dst_pos) as *mut c_void,
                    (*src2).data.as_ptr().add(i) as *const c_void,
                    1,
                );
                dst_pos += 1;
            }
        }
        i += 1;
    }

    unsafe {
        (*dst).length = dst_pos;
        (*dst).checksum = calculate_checksum((*dst).data.as_ptr(), (*dst).length);
    }

    0
}

/// Rotate buffer left by n positions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_rotate(buf: *mut buffer_t, mut positions: c_int) -> c_int {
    if buf.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL buffer in rotate\n".as_ptr());
        }
        return -1;
    }

    let length = unsafe { (*buf).length };
    if length == 0 || positions == 0 {
        return 0; // Nothing to rotate
    }

    // Normalize positions to valid range
    positions %= length as c_int;
    if positions < 0 {
        positions += length as c_int;
    }

    let positions_usz = positions as usize;

    let mut temp: [u8; 256] = [0u8; 256];
    unsafe {
        memcpy(
            temp.as_mut_ptr() as *mut c_void,
            (*buf).data.as_ptr() as *const c_void,
            length,
        );

        // Copy rotated portions
        memcpy(
            (*buf).data.as_mut_ptr() as *mut c_void,
            temp.as_ptr().add(positions_usz) as *const c_void,
            length - positions_usz,
        );
        memcpy(
            (*buf).data.as_mut_ptr().add(length - positions_usz) as *mut c_void,
            temp.as_ptr() as *const c_void,
            positions_usz,
        );

        (*buf).checksum = calculate_checksum((*buf).data.as_ptr(), length);
    }

    0
}

/// Conditional copy based on pattern matching
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_conditional_copy(
    src: *const buffer_t,
    dst: *mut buffer_t,
    pattern: u8,
    copy_matching: bool,
) -> c_int {
    if src.is_null() || dst.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Error: NULL pointer in conditional_copy\n".as_ptr(),
            );
        }
        return -1;
    }

    let length = unsafe { (*src).length };
    let mut dst_pos: usize = 0;
    let mut i: usize = 0;
    while i < length {
        unsafe {
            let matches = (*src).data[i] == pattern;
            if matches == copy_matching {
                memcpy(
                    (*dst).data.as_mut_ptr().add(dst_pos) as *mut c_void,
                    (*src).data.as_ptr().add(i) as *const c_void,
                    1,
                );
                dst_pos += 1;
            }
        }
        i += 1;
    }

    unsafe {
        (*dst).length = dst_pos;
        (*dst).checksum = calculate_checksum((*dst).data.as_ptr(), (*dst).length);
    }

    0
}

/// Copy with stride (every nth byte)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_copy_strided(
    src: *const buffer_t,
    dst: *mut buffer_t,
    stride: c_int,
) -> c_int {
    if src.is_null() || dst.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL pointer in copy_strided\n".as_ptr());
        }
        return -1;
    }

    if stride <= 0 {
        unsafe {
            fprintf(stderr, c"Error: Invalid stride %d\n".as_ptr(), stride);
        }
        return -1;
    }

    let length = unsafe { (*src).length };
    let mut dst_pos: usize = 0;
    let mut i: usize = 0;
    while i < length {
        unsafe {
            memcpy(
                (*dst).data.as_mut_ptr().add(dst_pos) as *mut c_void,
                (*src).data.as_ptr().add(i) as *const c_void,
                1,
            );
            dst_pos += 1;
        }
        i += stride as usize;
    }

    unsafe {
        (*dst).length = dst_pos;
        (*dst).checksum = calculate_checksum((*dst).data.as_ptr(), (*dst).length);
    }

    0
}

// ==================== Complex Processing Functions ====================

/// Process buffer array with operation
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer_array(
    arr: *mut buffer_array_t,
    op: c_int,
    param: c_int,
) -> c_int {
    if arr.is_null() || unsafe { (*arr).count == 0 } {
        unsafe {
            fprintf(stderr, c"Error: Invalid buffer array\n".as_ptr());
        }
        return -1;
    }

    let count = unsafe { (*arr).count };
    let buffers = unsafe { (*arr).buffers };

    match op {
        x if x == OP_COPY => {
            // Copy first buffer to all others
            let mut i = 1;
            while i < count {
                unsafe {
                    if buffer_copy(buffers.offset(0), buffers.offset(i as isize)) != 0 {
                        return -1;
                    }
                }
                i += 1;
            }
        }
        x if x == OP_REVERSE => {
            // Reverse all buffers
            let mut i = 0;
            while i < count {
                unsafe {
                    if buffer_reverse(buffers.offset(i as isize)) != 0 {
                        return -1;
                    }
                }
                i += 1;
            }
        }
        x if x == OP_MERGE => {
            // Merge consecutive pairs
            if count < 2 {
                unsafe {
                    fprintf(
                        stderr,
                        c"Error: Need at least 2 buffers for merge\n".as_ptr(),
                    );
                }
                return -1;
            }
            let mut i = 0;
            while i < count - 1 {
                let mut merged: buffer_t = buffer_t {
                    data: [0u8; 256],
                    length: 0,
                    checksum: 0,
                };
                unsafe {
                    if buffer_merge(
                        buffers.offset(i as isize),
                        buffers.offset((i + 1) as isize),
                        &mut merged,
                    ) != 0
                    {
                        return -1;
                    }
                    memcpy(
                        buffers.offset(i as isize) as *mut c_void,
                        &merged as *const buffer_t as *const c_void,
                        std::mem::size_of::<buffer_t>(),
                    );
                }
                i += 2;
            }
        }
        x if x == OP_ROTATE => {
            // Rotate all buffers by param positions
            let mut i = 0;
            while i < count {
                unsafe {
                    if buffer_rotate(buffers.offset(i as isize), param) != 0 {
                        return -1;
                    }
                }
                i += 1;
            }
        }
        x if x == OP_CHECKSUM => {
            // Verify all checksums
            let mut i = 0;
            while i < count {
                unsafe {
                    if !validate_buffer(buffers.offset(i as isize)) {
                        return -1;
                    }
                }
                i += 1;
            }
        }
        _ => {
            unsafe {
                fprintf(stderr, c"Error: Unknown operation %d\n".as_ptr(), op);
            }
            return -1;
        }
    }

    0
}

// ==================== Input/Output Functions ====================

/// Read buffer from stdin
#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_buffer(buf: *mut buffer_t) -> c_int {
    if buf.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL buffer in read_buffer\n".as_ptr());
        }
        return -1;
    }

    let mut length: c_int = 0;
    unsafe {
        if scanf(c"%d".as_ptr(), &mut length as *mut c_int) != 1 {
            fprintf(stderr, c"Error: Failed to read buffer length\n".as_ptr());
            return -1;
        }
    }

    if length < 0 || length > 256 {
        unsafe {
            fprintf(
                stderr,
                c"Error: Invalid buffer length %d\n".as_ptr(),
                length,
            );
        }
        return -1;
    }

    unsafe {
        (*buf).length = length as usize;
    }
    let mut i: usize = 0;
    let buf_length = unsafe { (*buf).length };
    while i < buf_length {
        let mut byte: c_int = 0;
        unsafe {
            if scanf(c"%d".as_ptr(), &mut byte as *mut c_int) != 1 {
                fprintf(stderr, c"Error: Failed to read byte %zu\n".as_ptr(), i);
                return -1;
            }
            (*buf).data[i] = byte as u8;
        }
        i += 1;
    }

    unsafe {
        (*buf).checksum = calculate_checksum((*buf).data.as_ptr(), (*buf).length);
    }
    0
}

/// Write buffer to stdout
#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_buffer(buf: *const buffer_t) {
    if buf.is_null() {
        unsafe {
            fprintf(stderr, c"Error: NULL buffer in write_buffer\n".as_ptr());
        }
        return;
    }

    let length = unsafe { (*buf).length };
    unsafe {
        printf(c"%zu".as_ptr(), length);
    }
    let mut i: usize = 0;
    while i < length {
        unsafe {
            printf(c" %u".as_ptr(), (*buf).data[i] as c_uint);
        }
        i += 1;
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}

// ==================== Main Function ====================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    let mut operation: c_int = 0;
    let mut buffer_count: c_int = 0;

    // Read operation type
    unsafe {
        if scanf(c"%d".as_ptr(), &mut operation as *mut c_int) != 1 {
            fprintf(stderr, c"Error: Failed to read operation\n".as_ptr());
            return 1;
        }
    }

    // Read buffer count
    unsafe {
        if scanf(c"%d".as_ptr(), &mut buffer_count as *mut c_int) != 1 {
            fprintf(stderr, c"Error: Failed to read buffer count\n".as_ptr());
            return 1;
        }
    }

    if buffer_count <= 0 || buffer_count > 100 {
        unsafe {
            fprintf(
                stderr,
                c"Error: Invalid buffer count %d\n".as_ptr(),
                buffer_count,
            );
        }
        return 1;
    }

    // Allocate buffer array
    let buffers = unsafe { init_buffer_array(buffer_count) };
    if buffers.is_null() {
        return 1;
    }

    // Read all buffers
    let mut i: c_int = 0;
    while i < buffer_count {
        unsafe {
            if read_buffer((*buffers).buffers.offset(i as isize)) != 0 {
                free_buffer_array(buffers);
                return 1;
            }
            (*buffers).count += 1;
        }
        i += 1;
    }

    // Execute operation based on type
    let mut result: c_int = 0;
    match operation {
        x if x == OP_COPY => {
            if buffer_count >= 2 {
                let mut temp: buffer_t = buffer_t {
                    data: [0u8; 256],
                    length: 0,
                    checksum: 0,
                };
                unsafe {
                    result = buffer_copy((*buffers).buffers.offset(0), &mut temp);
                    if result == 0 {
                        write_buffer(&temp);
                    }
                }
            } else {
                unsafe {
                    fprintf(stderr, c"Error: Copy needs at least 2 buffers\n".as_ptr());
                }
                result = -1;
            }
        }
        x if x == OP_REVERSE => {
            let mut i: c_int = 0;
            while i < buffer_count {
                unsafe {
                    result = buffer_reverse((*buffers).buffers.offset(i as isize));
                    if result != 0 {
                        break;
                    }
                    write_buffer((*buffers).buffers.offset(i as isize));
                }
                i += 1;
            }
        }
        x if x == OP_MERGE => {
            if buffer_count >= 2 {
                let mut merged: buffer_t = buffer_t {
                    data: [0u8; 256],
                    length: 0,
                    checksum: 0,
                };
                unsafe {
                    result = buffer_merge(
                        (*buffers).buffers.offset(0),
                        (*buffers).buffers.offset(1),
                        &mut merged,
                    );
                    if result == 0 {
                        write_buffer(&merged);
                    }
                }
            } else {
                unsafe {
                    fprintf(
                        stderr,
                        c"Error: Merge needs at least 2 buffers\n".as_ptr(),
                    );
                }
                result = -1;
            }
        }
        x if x == OP_SPLIT => {
            if buffer_count >= 1 {
                let mut split_pos: c_int = 0;
                unsafe {
                    if scanf(c"%d".as_ptr(), &mut split_pos as *mut c_int) != 1 {
                        fprintf(
                            stderr,
                            c"Error: Failed to read split position\n".as_ptr(),
                        );
                        result = -1;
                    } else {
                        let mut part1: buffer_t = buffer_t {
                            data: [0u8; 256],
                            length: 0,
                            checksum: 0,
                        };
                        let mut part2: buffer_t = buffer_t {
                            data: [0u8; 256],
                            length: 0,
                            checksum: 0,
                        };
                        result = buffer_split(
                            (*buffers).buffers.offset(0),
                            split_pos as usize,
                            &mut part1,
                            &mut part2,
                        );
                        if result == 0 {
                            write_buffer(&part1);
                            write_buffer(&part2);
                        }
                    }
                }
            }
        }
        x if x == OP_INTERLEAVE => {
            if buffer_count >= 2 {
                let mut interleaved: buffer_t = buffer_t {
                    data: [0u8; 256],
                    length: 0,
                    checksum: 0,
                };
                unsafe {
                    result = buffer_interleave(
                        (*buffers).buffers.offset(0),
                        (*buffers).buffers.offset(1),
                        &mut interleaved,
                    );
                    if result == 0 {
                        write_buffer(&interleaved);
                    }
                }
            } else {
                unsafe {
                    fprintf(
                        stderr,
                        c"Error: Interleave needs at least 2 buffers\n".as_ptr(),
                    );
                }
                result = -1;
            }
        }
        x if x == OP_ROTATE => {
            let mut positions: c_int = 0;
            unsafe {
                if scanf(c"%d".as_ptr(), &mut positions as *mut c_int) != 1 {
                    fprintf(
                        stderr,
                        c"Error: Failed to read rotation amount\n".as_ptr(),
                    );
                    result = -1;
                } else {
                    let mut i: c_int = 0;
                    while i < buffer_count {
                        result = buffer_rotate((*buffers).buffers.offset(i as isize), positions);
                        if result != 0 {
                            break;
                        }
                        write_buffer((*buffers).buffers.offset(i as isize));
                        i += 1;
                    }
                }
            }
        }
        x if x == OP_CHECKSUM => {
            let mut i: c_int = 0;
            while i < buffer_count {
                unsafe {
                    let cs = (*(*buffers).buffers.offset(i as isize)).checksum;
                    printf(c"%u\n".as_ptr(), cs as c_uint);
                }
                i += 1;
            }
        }
        _ => {
            unsafe {
                fprintf(
                    stderr,
                    c"Error: Unknown operation %d\n".as_ptr(),
                    operation,
                );
            }
            result = -1;
        }
    }

    unsafe {
        free_buffer_array(buffers);
    }
    if result != 0 {
        1
    } else {
        0
    }
}
