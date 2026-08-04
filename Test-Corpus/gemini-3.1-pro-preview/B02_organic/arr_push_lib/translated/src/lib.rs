use std::os::raw::{c_char, c_int, c_void};

pub const STBDS_BUCKET_LENGTH: usize = 8;

#[repr(C)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

#[repr(C)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct stbds_hash_index {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: stbds_string_arena,
    pub storage: *mut stbds_hash_bucket,
}

#[repr(C)]
pub struct stbds_struct {
    pub key: c_int,
    pub b: c_int,
    pub c: c_int,
    pub d: c_int,
}

#[repr(C)]
pub struct stbds_struct2 {
    pub key: [c_int; 2],
    pub b: c_int,
    pub c: c_int,
    pub d: c_int,
}

static mut BUFFER: [u8; 256] = [0; 256];

#[unsafe(no_mangle)]
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    let s = format!("test_{}\0", n);
    let bytes = s.as_bytes();
    let len = bytes.len().min(256);
    unsafe {
        BUFFER[..len].copy_from_slice(&bytes[..len]);
        BUFFER.as_mut_ptr() as *mut c_char
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_push(num: c_int) {
    let mut arr: Vec<c_int> = Vec::new();
    assert!(arr.is_empty());
    
    let mut i = 0;
    while i < num {
        for j in 0..i {
            arr.push(j);
        }
        arr = Vec::new();
        i += 50;
    }
}
