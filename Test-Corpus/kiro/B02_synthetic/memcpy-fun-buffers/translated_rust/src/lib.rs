use std::os::raw::c_int;
use std::ptr;

// ==================== Data Structures ====================

#[repr(C)]
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

// ==================== Helper Functions ====================

#[no_mangle]
pub extern "C" fn calculate_checksum(data: *const u8, length: usize) -> u32 {
    let mut sum: u32 = 0;
    for i in 0..length {
        sum = (sum << 3) ^ unsafe { *data.add(i) } as u32;
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
    let layout_arr = std::alloc::Layout::new::<buffer_array_t>();
    let arr = unsafe { std::alloc::alloc_zeroed(layout_arr) as *mut buffer_array_t };
    if arr.is_null() {
        eprintln!("Error: Failed to allocate buffer array");
        return ptr::null_mut();
    }
    let layout_bufs = std::alloc::Layout::from_size_align(
        std::mem::size_of::<buffer_t>() * initial_capacity as usize,
        std::mem::align_of::<buffer_t>(),
    )
    .unwrap();
    let bufs = unsafe { std::alloc::alloc_zeroed(layout_bufs) as *mut buffer_t };
    if bufs.is_null() {
        eprintln!("Error: Failed to allocate buffer storage");
        unsafe { std::alloc::dealloc(arr as *mut u8, layout_arr) };
        return ptr::null_mut();
    }
    unsafe {
        (*arr).buffers = bufs;
        (*arr).count = 0;
        (*arr).capacity = initial_capacity;
    }
    arr
}

#[no_mangle]
pub extern "C" fn free_buffer_array(arr: *mut buffer_array_t) {
    if !arr.is_null() {
        unsafe {
            let cap = (*arr).capacity;
            if !(*arr).buffers.is_null() && cap > 0 {
                let layout_bufs = std::alloc::Layout::from_size_align(
                    std::mem::size_of::<buffer_t>() * cap as usize,
                    std::mem::align_of::<buffer_t>(),
                )
                .unwrap();
                std::alloc::dealloc((*arr).buffers as *mut u8, layout_bufs);
            }
            let layout_arr = std::alloc::Layout::new::<buffer_array_t>();
            std::alloc::dealloc(arr as *mut u8, layout_arr);
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
    if !validate_buffer(src) {
        return -1;
    }
    unsafe {
        let s = &*src;
        let d = &mut *dst;
        d.data[..s.length].copy_from_slice(&s.data[..s.length]);
        d.length = s.length;
        d.checksum = calculate_checksum(d.data.as_ptr(), d.length);
    }
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
    let mut temp = [0u8; 256];
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
    unsafe {
        let s1 = &*src1;
        let s2 = &*src2;
        let d = &mut *dst;
        if s1.length + s2.length > 256 {
            eprintln!("Error: Merged length {} exceeds maximum", s1.length + s2.length);
            return -1;
        }
        d.data[..s1.length].copy_from_slice(&s1.data[..s1.length]);
        d.data[s1.length..s1.length + s2.length].copy_from_slice(&s2.data[..s2.length]);
        d.length = s1.length + s2.length;
        d.checksum = calculate_checksum(d.data.as_ptr(), d.length);
    }
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
    unsafe {
        let s = &*src;
        let d1 = &mut *dst1;
        let d2 = &mut *dst2;
        if split_pos > s.length {
            eprintln!("Error: Split position {} exceeds length {}", split_pos, s.length);
            return -1;
        }
        if split_pos > 0 {
            d1.data[..split_pos].copy_from_slice(&s.data[..split_pos]);
        }
        d1.length = split_pos;
        d1.checksum = calculate_checksum(d1.data.as_ptr(), d1.length);
        let remaining = s.length - split_pos;
        if remaining > 0 {
            d2.data[..remaining].copy_from_slice(&s.data[split_pos..split_pos + remaining]);
        }
        d2.length = remaining;
        d2.checksum = calculate_checksum(d2.data.as_ptr(), d2.length);
    }
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
    unsafe {
        let s1 = &*src1;
        let s2 = &*src2;
        let d = &mut *dst;
        let max_len = if s1.length > s2.length { s1.length } else { s2.length };
        if s1.length + s2.length > 256 {
            eprintln!("Error: Interleaved length exceeds maximum");
            return -1;
        }
        let mut dst_pos = 0usize;
        for i in 0..max_len {
            if i < s1.length {
                d.data[dst_pos] = s1.data[i];
                dst_pos += 1;
            }
            if i < s2.length {
                d.data[dst_pos] = s2.data[i];
                dst_pos += 1;
            }
        }
        d.length = dst_pos;
        d.checksum = calculate_checksum(d.data.as_ptr(), d.length);
    }
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
    let mut pos = positions % buf.length as i32;
    if pos < 0 {
        pos += buf.length as i32;
    }
    let pos = pos as usize;
    let mut temp = [0u8; 256];
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
    unsafe {
        let s = &*src;
        let d = &mut *dst;
        let mut dst_pos = 0usize;
        for i in 0..s.length {
            let matches = s.data[i] == pattern;
            if matches == copy_matching {
                d.data[dst_pos] = s.data[i];
                dst_pos += 1;
            }
        }
        d.length = dst_pos;
        d.checksum = calculate_checksum(d.data.as_ptr(), d.length);
    }
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
    if stride <= 0 {
        eprintln!("Error: Invalid stride {}", stride);
        return -1;
    }
    unsafe {
        let s = &*src;
        let d = &mut *dst;
        let mut dst_pos = 0usize;
        let mut i = 0usize;
        while i < s.length {
            d.data[dst_pos] = s.data[i];
            dst_pos += 1;
            i += stride as usize;
        }
        d.length = dst_pos;
        d.checksum = calculate_checksum(d.data.as_ptr(), d.length);
    }
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
    let bufs = arr.buffers;
    let count = arr.count as usize;

    match op {
        0 => {
            // OP_COPY: copy first buffer to all others
            for i in 1..count {
                unsafe {
                    if buffer_copy(bufs.add(0), bufs.add(i)) != 0 {
                        return -1;
                    }
                }
            }
        }
        1 => {
            // OP_REVERSE
            for i in 0..count {
                unsafe {
                    if buffer_reverse(bufs.add(i)) != 0 {
                        return -1;
                    }
                }
            }
        }
        2 => {
            // OP_MERGE
            if count < 2 {
                eprintln!("Error: Need at least 2 buffers for merge");
                return -1;
            }
            let mut i = 0;
            while i < count - 1 {
                unsafe {
                    let mut merged: buffer_t = std::mem::zeroed();
                    if buffer_merge(bufs.add(i), bufs.add(i + 1), &mut merged) != 0 {
                        return -1;
                    }
                    ptr::copy_nonoverlapping(&merged, bufs.add(i), 1);
                }
                i += 2;
            }
        }
        5 => {
            // OP_ROTATE
            for i in 0..count {
                unsafe {
                    if buffer_rotate(bufs.add(i), param) != 0 {
                        return -1;
                    }
                }
            }
        }
        6 => {
            // OP_CHECKSUM
            for i in 0..count {
                unsafe {
                    if !validate_buffer(bufs.add(i)) {
                        return -1;
                    }
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

// ==================== I/O Functions ====================

#[no_mangle]
pub extern "C" fn read_buffer(buf: *mut buffer_t) -> c_int {
    if buf.is_null() {
        eprintln!("Error: NULL buffer in read_buffer");
        return -1;
    }
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut tokens: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        for tok in line.split_whitespace() {
            tokens.push(tok.to_string());
        }
        if !tokens.is_empty() {
            break;
        }
    }
    let buf = unsafe { &mut *buf };
    let mut idx = 0;
    let length: i32 = match tokens.get(idx).and_then(|t| t.parse().ok()) {
        Some(v) => v,
        None => {
            eprintln!("Error: Failed to read buffer length");
            return -1;
        }
    };
    idx += 1;
    if length < 0 || length > 256 {
        eprintln!("Error: Invalid buffer length {}", length);
        return -1;
    }
    buf.length = length as usize;
    for j in 0..buf.length {
        let byte_val: i32 = match tokens.get(idx).and_then(|t| t.parse().ok()) {
            Some(v) => v,
            None => {
                eprintln!("Error: Failed to read byte {}", j);
                return -1;
            }
        };
        idx += 1;
        buf.data[j] = byte_val as u8;
    }
    buf.checksum = calculate_checksum(buf.data.as_ptr(), buf.length);
    0
}

#[no_mangle]
pub extern "C" fn write_buffer(buf: *const buffer_t) {
    if buf.is_null() {
        eprintln!("Error: NULL buffer in write_buffer");
        return;
    }
    use std::io::{self, Write};
    let buf = unsafe { &*buf };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}", buf.length);
    for i in 0..buf.length {
        let _ = write!(out, " {}", buf.data[i]);
    }
    let _ = writeln!(out);
}

// main is exported via a linker alias to avoid conflicting with test harness
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    0
}
