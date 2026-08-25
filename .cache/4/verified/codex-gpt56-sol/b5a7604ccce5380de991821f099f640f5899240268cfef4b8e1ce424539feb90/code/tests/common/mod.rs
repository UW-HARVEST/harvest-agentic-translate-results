#![allow(dead_code)]

use libloading::{Library, Symbol};
use lz4 as _;
use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use std::path::{Path, PathBuf};

pub const LZ4_MAX_INPUT_SIZE: c_int = 0x7e00_0000;
pub const LZ4F_VERSION: c_uint = 100;

pub struct Libraries {
    pub c: Library,
    pub rust: Library,
}

impl Libraries {
    pub unsafe fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/liblz4.so");
        let rust_path = rust_library_path();
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        Self {
            c: unsafe { Library::new(c_path).expect("load C library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust library") },
        }
    }

    pub unsafe fn pair<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        (
            unsafe { self.c.get(name).expect("resolve C symbol") },
            unsafe { self.rust.get(name).expect("resolve Rust symbol") },
        )
    }
}

fn rust_library_path() -> PathBuf {
    let deps = std::env::current_exe()
        .expect("current test executable")
        .parent()
        .expect("deps directory")
        .to_path_buf();
    let in_deps = deps.join("liblz4.so");
    if in_deps.is_file() {
        in_deps
    } else {
        deps.parent()
            .expect("target profile directory")
            .join("liblz4.so")
    }
}

#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64() as u8).collect()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameInfo {
    pub block_size_id: c_int,
    pub block_mode: c_int,
    pub content_checksum_flag: c_int,
    pub frame_type: c_int,
    pub content_size: c_ulonglong,
    pub dict_id: c_uint,
    pub block_checksum_flag: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preferences {
    pub frame_info: FrameInfo,
    pub compression_level: c_int,
    pub auto_flush: c_uint,
    pub favor_dec_speed: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CompressOptions {
    pub stable_src: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DecompressOptions {
    pub stable_dst: c_uint,
    pub skip_checksums: c_uint,
    pub reserved1: c_uint,
    pub reserved0: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CustomMem {
    pub custom_alloc: *mut c_void,
    pub custom_calloc: *mut c_void,
    pub custom_free: *mut c_void,
    pub opaque_state: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HcMatch {
    pub off: c_int,
    pub len: c_int,
    pub back: c_int,
}

pub type Compress = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
pub type CompressFast =
    unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
pub type Decompress = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;

pub fn patterned(len: usize) -> Vec<u8> {
    (0..len).map(|i| b"abcd1234"[i % 8]).collect()
}

pub unsafe fn compress_with(
    function: &Symbol<'_, Compress>,
    input: &[u8],
    capacity: usize,
) -> (c_int, Vec<u8>) {
    let mut output = vec![0xa5; capacity.max(1)];
    let result = unsafe {
        function(
            input.as_ptr().cast(),
            output.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity as c_int,
        )
    };
    if result > 0 {
        output.truncate(result as usize);
    } else {
        output.clear();
    }
    (result, output)
}

pub unsafe fn decompress_with(
    function: &Symbol<'_, Decompress>,
    input: &[u8],
    capacity: usize,
) -> (c_int, Vec<u8>) {
    let mut output = vec![0xa5; capacity.max(1)];
    let result = unsafe {
        function(
            input.as_ptr().cast(),
            output.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity as c_int,
        )
    };
    if result >= 0 {
        output.truncate(result as usize);
    } else {
        output.clear();
    }
    (result, output)
}
