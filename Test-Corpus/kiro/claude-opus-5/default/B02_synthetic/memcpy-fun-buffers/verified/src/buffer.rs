//! Buffer data structures and operations, translated from `c_src/src/main.c`.
//!
//! Pointer-null checks from the C original are dropped where Rust references
//! make them unreachable; every other check, message and ordering is preserved.
//! C's uninitialized `buffer_t` locals are modelled as zeroed buffers: only the
//! prefix that the operations actually write is ever read back or printed, so
//! this is observationally identical.

pub const MAX_LEN: usize = 256;

#[derive(Clone, Copy)]
pub struct Buffer {
    pub data: [u8; MAX_LEN],
    pub length: usize,
    pub checksum: u32,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            data: [0u8; MAX_LEN],
            length: 0,
            checksum: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Operation {
    Copy = 0,
    Reverse = 1,
    Merge = 2,
    Split = 3,
    Interleave = 4,
    Rotate = 5,
    Checksum = 6,
}

pub struct BufferArray {
    pub buffers: Vec<Buffer>,
    pub count: i32,
    #[allow(dead_code)]
    pub capacity: i32,
}

// ==================== Helper Functions ====================

/// Calculate simple checksum. `sum = (sum << 3) ^ data[i]` on `uint32_t`.
pub fn calculate_checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &byte in data {
        sum = (sum << 3) ^ u32::from(byte);
    }
    sum
}

/// Validate buffer integrity.
pub fn validate_buffer(buf: &Buffer) -> bool {
    if buf.length > MAX_LEN {
        eprint!(
            "Error: Buffer length {} exceeds maximum 256\n",
            buf.length
        );
        return false;
    }
    let expected = calculate_checksum(&buf.data[..buf.length]);
    if buf.checksum != expected {
        eprint!(
            "Warning: Checksum mismatch. Expected {}, got {}\n",
            expected, buf.checksum
        );
    }
    true
}

/// Initialize buffer array.
pub fn init_buffer_array(initial_capacity: i32) -> Option<BufferArray> {
    if initial_capacity <= 0 {
        eprint!("Error: Invalid capacity {}\n", initial_capacity);
        return None;
    }

    Some(BufferArray {
        buffers: vec![Buffer::new(); initial_capacity as usize],
        count: 0,
        capacity: initial_capacity,
    })
}

// ==================== Core Buffer Operations ====================

/// Simple copy operation.
pub fn buffer_copy(src: &Buffer, dst: &mut Buffer) -> i32 {
    if !validate_buffer(src) {
        return -1;
    }

    dst.data[..src.length].copy_from_slice(&src.data[..src.length]);
    dst.length = src.length;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);

    0
}

/// Reverse buffer contents.
pub fn buffer_reverse(buf: &mut Buffer) -> i32 {
    if buf.length == 0 {
        return 0; // Nothing to reverse
    }

    let mut temp = [0u8; MAX_LEN];
    temp[..buf.length].copy_from_slice(&buf.data[..buf.length]);

    for i in 0..buf.length {
        buf.data[i] = temp[buf.length - 1 - i];
    }

    buf.checksum = calculate_checksum(&buf.data[..buf.length]);
    0
}

/// Merge two buffers into destination.
pub fn buffer_merge(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    if src1.length + src2.length > MAX_LEN {
        eprint!(
            "Error: Merged length {} exceeds maximum\n",
            src1.length + src2.length
        );
        return -1;
    }

    dst.data[..src1.length].copy_from_slice(&src1.data[..src1.length]);
    dst.data[src1.length..src1.length + src2.length]
        .copy_from_slice(&src2.data[..src2.length]);

    dst.length = src1.length + src2.length;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);

    0
}

/// Split buffer at position into two buffers.
///
/// `split_pos` is a `size_t` in C; a negative `int` argument therefore arrives
/// here sign-extended into a huge value, which the caller reproduces.
pub fn buffer_split(
    src: &Buffer,
    split_pos: usize,
    dst1: &mut Buffer,
    dst2: &mut Buffer,
) -> i32 {
    if split_pos > src.length {
        eprint!(
            "Error: Split position {} exceeds length {}\n",
            split_pos, src.length
        );
        return -1;
    }

    // Copy first part
    if split_pos > 0 {
        dst1.data[..split_pos].copy_from_slice(&src.data[..split_pos]);
    }
    dst1.length = split_pos;
    dst1.checksum = calculate_checksum(&dst1.data[..dst1.length]);

    // Copy second part
    let remaining = src.length - split_pos;
    if remaining > 0 {
        dst2.data[..remaining].copy_from_slice(&src.data[split_pos..split_pos + remaining]);
    }
    dst2.length = remaining;
    dst2.checksum = calculate_checksum(&dst2.data[..dst2.length]);

    0
}

/// Interleave two buffers (alternating bytes).
pub fn buffer_interleave(src1: &Buffer, src2: &Buffer, dst: &mut Buffer) -> i32 {
    let max_len = if src1.length > src2.length {
        src1.length
    } else {
        src2.length
    };
    if src1.length + src2.length > MAX_LEN {
        eprint!("Error: Interleaved length exceeds maximum\n");
        return -1;
    }

    let mut dst_pos = 0usize;
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
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);

    0
}

/// Rotate buffer left by n positions.
pub fn buffer_rotate(buf: &mut Buffer, positions: i32) -> i32 {
    if buf.length == 0 || positions == 0 {
        return 0; // Nothing to rotate
    }

    // Normalize positions to valid range
    let mut positions = positions % (buf.length as i32);
    if positions < 0 {
        positions += buf.length as i32;
    }

    let n = buf.length;
    let p = positions as usize;

    let mut temp = [0u8; MAX_LEN];
    temp[..n].copy_from_slice(&buf.data[..n]);

    // Copy rotated portions
    buf.data[..n - p].copy_from_slice(&temp[p..n]);
    buf.data[n - p..n].copy_from_slice(&temp[..p]);

    buf.checksum = calculate_checksum(&buf.data[..n]);

    0
}

/// Conditional copy based on pattern matching. (Unused by `main`, as in the C.)
#[allow(dead_code)]
pub fn buffer_conditional_copy(
    src: &Buffer,
    dst: &mut Buffer,
    pattern: u8,
    copy_matching: bool,
) -> i32 {
    let mut dst_pos = 0usize;
    for i in 0..src.length {
        let matches = src.data[i] == pattern;
        if matches == copy_matching {
            dst.data[dst_pos] = src.data[i];
            dst_pos += 1;
        }
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);

    0
}

/// Copy with stride (every nth byte). (Unused by `main`, as in the C.)
#[allow(dead_code)]
pub fn buffer_copy_strided(src: &Buffer, dst: &mut Buffer, stride: i32) -> i32 {
    if stride <= 0 {
        eprint!("Error: Invalid stride {}\n", stride);
        return -1;
    }

    let mut dst_pos = 0usize;
    let mut i = 0usize;
    while i < src.length {
        dst.data[dst_pos] = src.data[i];
        dst_pos += 1;
        i += stride as usize;
    }

    dst.length = dst_pos;
    dst.checksum = calculate_checksum(&dst.data[..dst.length]);

    0
}

// ==================== Complex Processing Functions ====================

/// Process buffer array with operation. (Unused by `main`, as in the C.)
#[allow(dead_code)]
pub fn process_buffer_array(arr: &mut BufferArray, op: Operation, param: i32) -> i32 {
    if arr.count == 0 {
        eprint!("Error: Invalid buffer array\n");
        return -1;
    }

    match op {
        Operation::Copy => {
            // Copy first buffer to all others
            let src = arr.buffers[0];
            for i in 1..arr.count as usize {
                if buffer_copy(&src, &mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }
        Operation::Reverse => {
            // Reverse all buffers
            for i in 0..arr.count as usize {
                if buffer_reverse(&mut arr.buffers[i]) != 0 {
                    return -1;
                }
            }
        }
        Operation::Merge => {
            // Merge consecutive pairs
            if arr.count < 2 {
                eprint!("Error: Need at least 2 buffers for merge\n");
                return -1;
            }
            let mut i = 0usize;
            while i < (arr.count - 1) as usize {
                let mut merged = Buffer::new();
                let (a, b) = (arr.buffers[i], arr.buffers[i + 1]);
                if buffer_merge(&a, &b, &mut merged) != 0 {
                    return -1;
                }
                arr.buffers[i] = merged;
                i += 2;
            }
        }
        Operation::Rotate => {
            // Rotate all buffers by param positions
            for i in 0..arr.count as usize {
                if buffer_rotate(&mut arr.buffers[i], param) != 0 {
                    return -1;
                }
            }
        }
        Operation::Checksum => {
            // Verify all checksums
            for i in 0..arr.count as usize {
                if !validate_buffer(&arr.buffers[i]) {
                    return -1;
                }
            }
        }
        other => {
            eprint!("Error: Unknown operation {}\n", other as i32);
            return -1;
        }
    }

    0
}
