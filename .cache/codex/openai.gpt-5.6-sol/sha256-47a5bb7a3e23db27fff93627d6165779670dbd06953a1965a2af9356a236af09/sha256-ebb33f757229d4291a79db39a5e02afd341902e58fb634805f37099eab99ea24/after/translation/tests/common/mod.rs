#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::{Path, PathBuf};

pub struct Libraries {
    pub c: Library,
    pub rust: Library,
}

impl Libraries {
    pub unsafe fn load() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_dir.join("../c_src/build/liblz4.so");
        let rust_path = crate_dir.join("target/release/liblz4.so");
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );
        Self {
            c: unsafe { Library::new(c_path).expect("load C liblz4.so") },
            rust: unsafe { Library::new(rust_path).expect("load Rust liblz4.so") },
        }
    }

    pub unsafe fn pair<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        (
            unsafe {
                self.c
                    .get(name)
                    .unwrap_or_else(|e| panic!("C {:?}: {e}", name))
            },
            unsafe {
                self.rust
                    .get(name)
                    .unwrap_or_else(|e| panic!("Rust {:?}: {e}", name))
            },
        )
    }
}

pub fn manifest_path(relative: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
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

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64() as u8).collect()
    }
}

pub fn ptr_or_dangling(data: &[u8]) -> *const c_void {
    static EMPTY: u8 = 0;
    if data.is_empty() {
        (&EMPTY as *const u8).cast()
    } else {
        data.as_ptr().cast()
    }
}

pub fn mut_ptr_or_dangling(data: &mut [u8]) -> *mut c_void {
    if data.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr().cast()
    } else {
        data.as_mut_ptr().cast()
    }
}
