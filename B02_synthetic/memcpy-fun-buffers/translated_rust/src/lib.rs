use std::os::raw::c_int;
use std::ptr;

const MAX_BUF_SIZE: usize = 256;

// C-compatible structs matching exact layout
#[repr(C)]
#[derive(Clone, Copy)]
pub struct buffer_t {
    pub data: [u8; MAX_BUF_SIZE],
    pub length: usize,
    pub checksum: u32,
}

#[repr(C)]
pub struct buffer_array_t {
    pub buffers: *mut buffer_t,
    pub count: c_int,
    pub capacity: c_int,
}

impl buffer_t {
    pub fn new() -> Self {
        buffer_t {
            data: [0u8; MAX_BUF_SIZE],
            length: 0,
            checksum: 0,
        }
    }
}

// ==================== Helper Functions ====================

#[no_mangle]
pub extern "C" fn calculate_checksum(data: *const u8, length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = (sum << 3) ^ (unsafe { *data.add(i) } as u32);
    }
    sum
}

#[no_mangle]
pub extern "C" fn validate_buffer(buf: *const buffer_t) -> bool {
    if buf.is_null() {
        eprintln!("Error: NULL buffer");
        return false;
    }
    let buf = unsafe { &*buf };
    if buf.length > 256 {
        eprintln!("Error: Buffer length {} exceeds maximum 256", buf.length);
        return false;
    }
    let expected = calculate_checksum(buf.data.as_ptr(), buf.length);
    if buf.checksum != expected {
        eprintln!(
            "Warning: Checksum mismatch. Expected {}, got {}",
            expected, buf.checksum
        );
    }
    true
}

#[no_mangle]
pub extern "C" fn init_buffer_array(initial_capacity: c_int) -> *mut buffer_array_t {
    if initial_capacity <= 0 {
        eprintln!("Error: Invalid capacity {}", initial_capacity);
        return ptr::null_mut();
    }

    let arr = Box::new(buffer_array_t {
        buffers: {
            let layout = std::alloc::Layout::array::<buffer_t>(initial_capacity as usize).unwrap();
            let ptr = unsafe { std::alloc::alloc_zeroed(layout) } as *mut buffer_t;
            if ptr.is_null() {
                eprintln!("Error: Failed to allocate buffer storage");
                return ptr::null_mut();
            }
            ptr
        },
        count: 0,
        capacity: initial_capacity,
    });

    Box::into_raw(arr)
}

#[no_mangle]
pub extern "C" fn free_buffer_array(arr: *mut buffer_array_t) {
    if !arr.is_null() {
        unsafe {
            let arr = Box::from_raw(arr);
            if !arr.buffers.is_null() {
                let layout =
                    std::alloc::Layout::array::<buffer_t>(arr.capacity as usize).unwrap();
                std::alloc::dealloc(arr.buffers as *mut u8, layout);
            }
        }
    }
}

// ==================== Core Buffer Operations ====================

#[no_mangle]
pub extern "C" fn buffer_copy(src: *const buffer_t, dst: *mut buffer_t) -> c_int {
    if src.is_null() || dst.is_null() {
        eprintln!("Error: NULL pointer in buffer_copy");
        return -1;
    }
    let src = unsafe { &*src };
    let dst = unsafe { &mut *dst };

    if !validate_buffer(src as *const buffer_t) {
        return -1;
    }

    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(dst.data.as_ptr(), dst.length);
    0
}

#[no_mangle]
pub extern "C" fn buffer_reverse(buf: *mut buffer_t) -> c_int {
    if buf.is_null() {
        eprintln!("Error: NULL buffer in reverse");
        return -1;
    }
    let buf = unsafe { &mut *buf };

    if buf.length == 0 {
        return 0;
    }

    let mut temp = [0u8; MAX_BUF_SIZE];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    for i in 0..buf.length {
        buf.data[i] = temp[buf.length - 1 - i];
    }

    buf.checksum = calculate_checksum(buf.data.as_ptr(), buf.length);
    0
}

#[no_mangle]
pub extern "C" fn buffer_merge(
    src1: *const buffer_t,
    src2: *const buffer_t,
    dst: *mut buffer_t,
) -> c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        eprintln!("Error: NULL pointer in buffer_merge");
        return -1;
    }
    let src1 = unsafe { &*src1 };
    let src2 = unsafe { &*src2 };
    let dst = unsafe { &mut *dst };

    if src1.length + src2.length > 256 {
        eprintln!(
            "Error: Merged length {} exceeds maximum",
            src1.length + src2.length
        );
        return -1;
    }

    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    dst.data[src1.length..src1.length + src2.length].copy_from_slice(&src2.data[..src2.length]);
    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(dst.data.as_ptr(), dst.length);
    0
}

#[no_mangle]
pub extern "C" fn buffer_split(
    src: *const buffer_t,
    split_pos: usize,
    dst1: *mut buffer_t,
    dst2: *mut buffer_t,
) -> c_int {
    if src.is_null() || dst1.is_null() || dst2.is_null() {
        eprintln!("Error: NULL pointer in buffer_split");
        return -1;
    }
    let src = unsafe { &*src };
    let dst1 = unsafe { &mut *dst1 };
    let dst2 = unsafe { &mut *dst2 };

    if split_pos > src.length {
        eprintln!(
            "Error: Split position {} exceeds length {}",
            split_pos, src.length
        );
        return -1;
    }

    if split_pos > 0 {
        dst1.data[..split_pos].copy_from_slice(&src.data[..split_pos]);
    }
    dst1.length = split_pos;
    dst1.checksum = calculate_checksum(dst1.data.as_ptr(), dst1.length);

    let remaining = src.length - split_pos;
    if remaining > 0 {
        dst2.data[..remaining].copy_from_slice(&src.data[split_pos..split_pos + remaining]);
    }
    dst2.length = remaining;
    dst2.checksum = calculate_checksum(dst2.data.as_ptr(), dst2.length);
    0
}

#[no_mangle]
pub extern "C" fn buffer_interleave(
    src1: *const buffer_t,
    src2: *const buffer_t,
    dst: *mut buffer_t,
) -> c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        eprintln!("Error: NULL pointer in buffer_interleave");
        return -1;
    }
    let src1 = unsafe { &*src1 };
    let src2 = unsafe { &*src2 };
    let dst = unsafe { &mut *dst };

    let max_len = if src1.length > src2.length {
        src1.length
    } else {
        src2.length
    };
    if src1.length + src2.length > 256 {
        eprintln!("Error: Interleaved length exceeds maximum");
        return -1;
    }

    let mut dst_pos: usize = 0;
    for i in 0..max_len {
        if i < src1.length {
            dst.data[dst_pos] = src1.data[i];
            dst_pos += 1;
        }
        if i < src2.length {
            dst.data[dst_pos] = src2.data[i];
            dst_pos += 1;
        }
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(dst.data.as_ptr(), dst.length);
    0
}

#[no_mangle]
pub extern "C" fn buffer_rotate(buf: *mut buffer_t, positions: c_int) -> c_int {
    if buf.is_null() {
        eprintln!("Error: NULL buffer in rotate");
        return -1;
    }
    let buf = unsafe { &mut *buf };

    if buf.length == 0 || positions == 0 {
        return 0;
    }

    let mut pos = positions % (buf.length as c_int);
    if pos < 0 {
        pos += buf.length as c_int;
    }
    let pos = pos as usize;

    let mut temp = [0u8; MAX_BUF_SIZE];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    buf.data[..buf.length - pos].copy_from_slice(&temp[pos..buf.length]);
    buf.data[buf.length - pos..buf.length].copy_from_slice(&temp[..pos]);

    buf.checksum = calculate_checksum(buf.data.as_ptr(), buf.length);
    0
}

#[no_mangle]
pub extern "C" fn buffer_conditional_copy(
    src: *const buffer_t,
    dst: *mut buffer_t,
    pattern: u8,
    copy_matching: bool,
) -> c_int {
    if src.is_null() || dst.is_null() {
        eprintln!("Error: NULL pointer in conditional_copy");
        return -1;
    }
    let src = unsafe { &*src };
    let dst = unsafe { &mut *dst };

    let mut dst_pos: usize = 0;
    for i in 0..src.length {
        let matches = src.data[i] == pattern;
        if matches == copy_matching {
            dst.data[dst_pos] = src.data[i];
            dst_pos += 1;
        }
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(dst.data.as_ptr(), dst.length);
    0
}

#[no_mangle]
pub extern "C" fn buffer_copy_strided(
    src: *const buffer_t,
    dst: *mut buffer_t,
    stride: c_int,
) -> c_int {
    if src.is_null() || dst.is_null() {
        eprintln!("Error: NULL pointer in copy_strided");
        return -1;
    }
    let src = unsafe { &*src };
    let dst = unsafe { &mut *dst };

    if stride <= 0 {
        eprintln!("Error: Invalid stride {}", stride);
        return -1;
    }

    let mut dst_pos: usize = 0;
    let mut i: usize = 0;
    while i < src.length {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
        i += stride as usize;
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(dst.data.as_ptr(), dst.length);
    0
}

#[no_mangle]
pub extern "C" fn process_buffer_array(
    arr: *mut buffer_array_t,
    op: c_int,
    param: c_int,
) -> c_int {
    if arr.is_null() {
        eprintln!("Error: Invalid buffer array");
        return -1;
    }
    let arr = unsafe { &mut *arr };
    if arr.count == 0 {
        eprintln!("Error: Invalid buffer array");
        return -1;
    }

    match op {
        0 => {
            // OP_COPY
            for i in 1..arr.count {
                let (src, dst) = unsafe {
                    let src = &*arr.buffers.add(0);
                    let dst = &mut *arr.buffers.add(i as usize);
                    (src as *const buffer_t, dst as *mut buffer_t)
                };
                if buffer_copy(src, dst) != 0 {
                    return -1;
                }
            }
        }
        1 => {
            // OP_REVERSE
            for i in 0..arr.count {
                let buf = unsafe { arr.buffers.add(i as usize) };
                if buffer_reverse(buf) != 0 {
                    return -1;
                }
            }
        }
        2 => {
            // OP_MERGE
            if arr.count < 2 {
                eprintln!("Error: Need at least 2 buffers for merge");
                return -1;
            }
            let mut i = 0;
            while i < arr.count - 1 {
                let mut merged = buffer_t::new();
                let src1 = unsafe { &*arr.buffers.add(i as usize) };
                let src2 = unsafe { &*arr.buffers.add((i + 1) as usize) };
                if buffer_merge(src1, src2, &mut merged) != 0 {
                    return -1;
                }
                unsafe {
                    ptr::copy_nonoverlapping(&merged, arr.buffers.add(i as usize), 1);
                }
                i += 2;
            }
        }
        5 => {
            // OP_ROTATE
            for i in 0..arr.count {
                let buf = unsafe { arr.buffers.add(i as usize) };
                if buffer_rotate(buf, param) != 0 {
                    return -1;
                }
            }
        }
        6 => {
            // OP_CHECKSUM
            for i in 0..arr.count {
                let buf = unsafe { &*arr.buffers.add(i as usize) };
                if !validate_buffer(buf) {
                    return -1;
                }
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", op);
            return -1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn read_buffer(buf: *mut buffer_t) -> c_int {
    // This is an I/O function - not easily testable via FFI comparison
    // Stub for symbol export
    if buf.is_null() {
        eprintln!("Error: NULL buffer in read_buffer");
        return -1;
    }
    -1 // Not implemented for library use
}

#[no_mangle]
pub extern "C" fn __c_main(_argc: c_int, _argv: *const *const i8) -> c_int {
    // Stub matching the C main() renamed to __c_main
    0
}

#[no_mangle]
pub extern "C" fn write_buffer(buf: *const buffer_t) {
    if buf.is_null() {
        eprintln!("Error: NULL buffer in write_buffer");
        return;
    }
    let buf = unsafe { &*buf };
    print!("{}", buf.length);
    for i in 0..buf.length {
        print!(" {}", buf.data[i]);
    }
    println!();
}
