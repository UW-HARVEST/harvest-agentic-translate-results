// Integration tests that compare the C .so against the Rust .so via libloading.
// We never call Rust functions directly — both implementations are loaded as
// dynamic libraries and called through their `process_buffer` C ABI symbol.
//
// The C `process_buffer` (specifically `compact_runs` with a small threshold)
// can transiently write to indices beyond `length` because it may double the
// number of output bytes per input run. Both impls therefore need a backing
// buffer that is larger than `length`. We allocate `BACKING` bytes and fill
// the trailing region with a deterministic pattern so that both
// implementations observe identical memory.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

const BACKING: usize = 2048;

type ProcessBufferFn = unsafe extern "C" fn(
    buffer: *mut u8,
    length: usize,
    flags: u32,
    param1: c_int,
    param2: c_int,
) -> usize;

fn c_so_path() -> PathBuf {
    PathBuf::from("/tmp/harvest-work-DdMYpi/c_build/libdriver_c.so")
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    if cfg!(debug_assertions) {
        p.push("debug");
    } else {
        p.push("release");
    }
    p.push("libdriver.so");
    p
}

struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: ProcessBufferFn,
    rust_fn: ProcessBufferFn,
}

impl Libs {
    fn load() -> Self {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to load C lib at {:?}: {}", c_path, e));
            let rust_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("failed to load Rust lib at {:?}: {}", r_path, e));
            let c_sym: Symbol<ProcessBufferFn> = c_lib
                .get(b"process_buffer\0")
                .expect("process_buffer not found in C .so");
            let r_sym: Symbol<ProcessBufferFn> = rust_lib
                .get(b"process_buffer\0")
                .expect("process_buffer not found in Rust .so");
            let c_fn = *c_sym;
            let rust_fn = *r_sym;
            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_fn,
                rust_fn,
            }
        }
    }

    fn run_pair(
        &self,
        input: &[u8],
        flags: u32,
        p1: i32,
        p2: i32,
    ) -> ((usize, Vec<u8>), (usize, Vec<u8>)) {
        let mut c_buf = vec![0u8; BACKING];
        let mut r_buf = vec![0u8; BACKING];
        // Fill the prefix with the input.
        c_buf[..input.len()].copy_from_slice(input);
        r_buf[..input.len()].copy_from_slice(input);
        // Trailing bytes beyond `input.len()` stay at their zero-initialized
        // value so both impls see identical memory.

        let c_len = unsafe {
            (self.c_fn)(
                c_buf.as_mut_ptr(),
                input.len(),
                flags,
                p1 as c_int,
                p2 as c_int,
            )
        };
        let r_len = unsafe {
            (self.rust_fn)(
                r_buf.as_mut_ptr(),
                input.len(),
                flags,
                p1 as c_int,
                p2 as c_int,
            )
        };
        ((c_len, c_buf), (r_len, r_buf))
    }
}

fn assert_pair_match(libs: &Libs, input: &[u8], flags: u32, p1: i32, p2: i32) {
    let ((c_len, c_buf), (r_len, r_buf)) = libs.run_pair(input, flags, p1, p2);
    assert_eq!(
        c_len, r_len,
        "length mismatch for input={:?}, flags=0x{:x}, p1={}, p2={}",
        input, flags, p1, p2
    );
    assert_eq!(
        &c_buf[..c_len],
        &r_buf[..r_len],
        "buffer mismatch for input={:?}, flags=0x{:x}, p1={}, p2={}\nC: {:?}\nR: {:?}",
        input,
        flags,
        p1,
        p2,
        &c_buf[..c_len],
        &r_buf[..r_len]
    );
}

// ------------------- Empty / degenerate inputs -------------------

#[test]
fn empty_buffer_returns_zero() {
    let libs = Libs::load();
    let mut c_buf = vec![0u8; BACKING];
    let mut r_buf = vec![0u8; BACKING];
    let c_len = unsafe { (libs.c_fn)(c_buf.as_mut_ptr(), 0, 0xFF, 5, 1) };
    let r_len = unsafe { (libs.rust_fn)(r_buf.as_mut_ptr(), 0, 0xFF, 5, 1) };
    assert_eq!(c_len, 0);
    assert_eq!(r_len, 0);
}

#[test]
fn zero_flags_no_op() {
    let libs = Libs::load();
    for input_len in [1usize, 2, 5, 10, 33, 100, 256] {
        let input: Vec<u8> = (0..input_len).map(|i| (i * 7 + 3) as u8).collect();
        assert_pair_match(&libs, &input, 0, 0, 0);
    }
}

// ------------------- Rotate (flag 0x01) -------------------

#[test]
fn rotate_various_offsets_and_lengths() {
    let libs = Libs::load();
    let lengths: &[usize] = &[1, 2, 3, 4, 8, 16, 32, 64, 100, 200, 256];
    for &len in lengths {
        let input: Vec<u8> = (0..len as u32).map(|x| (x as u8).wrapping_mul(13)).collect();
        for offset in [
            -10000i32, -300, -129, -64, -33, -10, -1, 0, 1, 7, 33, 64, 128, 200, 256, 300, 10000,
        ] {
            assert_pair_match(&libs, &input, 0x01, offset, 0);
        }
    }
}

#[test]
fn rotate_small_offset_path() {
    let libs = Libs::load();
    let input: Vec<u8> = (0..100u32).map(|x| x as u8).collect();
    for offset in 1..49 {
        assert_pair_match(&libs, &input, 0x01, offset, 0);
    }
}

#[test]
fn rotate_large_offset_path() {
    let libs = Libs::load();
    let input: Vec<u8> = (0..100u32).map(|x| (x as u8).wrapping_add(1)).collect();
    for offset in 50..=99 {
        assert_pair_match(&libs, &input, 0x01, offset, 0);
    }
}

// ------------------- Compact runs (flag 0x02) -------------------

#[test]
fn compact_runs_basic() {
    let libs = Libs::load();
    let inputs: Vec<Vec<u8>> = vec![
        vec![1, 1, 1, 1, 2, 3, 3, 3, 4],
        vec![5, 5, 5, 5, 5, 5, 5, 5],
        vec![1, 2, 3, 4, 5],
        (0..100u32).map(|i| (i / 10) as u8).collect(),
        std::iter::repeat(7u8).take(256).collect(),
        vec![],
    ];
    for input in inputs {
        for threshold in [1i32, 2, 3, 4, 5, 10, 100, 255, 256, 0, -5] {
            assert_pair_match(&libs, &input, 0x02, threshold, 0);
        }
    }
}

#[test]
fn compact_runs_long_run_capped_at_255() {
    let libs = Libs::load();
    let input: Vec<u8> = std::iter::repeat(42u8).take(256).collect();
    for threshold in [1i32, 2, 3, 200] {
        assert_pair_match(&libs, &input, 0x02, threshold, 0);
    }
}

// ------------------- Remove duplicates (flag 0x04) -------------------

#[test]
fn remove_duplicates_preserve_order() {
    let libs = Libs::load();
    let inputs: Vec<Vec<u8>> = vec![
        vec![1, 2, 3, 1, 2, 3],
        vec![5, 5, 5, 5, 5],
        (0..200u32).map(|i| (i % 10) as u8).collect(),
        (0..256u32).map(|i| i as u8).collect(),
        vec![1],
        vec![],
    ];
    for input in inputs {
        assert_pair_match(&libs, &input, 0x04, 0, 1);
    }
}

#[test]
fn remove_duplicates_no_preserve_order() {
    let libs = Libs::load();
    let inputs: Vec<Vec<u8>> = vec![
        vec![1, 2, 3, 1, 2, 3],
        vec![5, 5, 5, 5, 5],
        (0..200u32).map(|i| (i % 10) as u8).collect(),
        (0..256u32).map(|i| i as u8).collect(),
        vec![1],
        vec![],
    ];
    for input in inputs {
        assert_pair_match(&libs, &input, 0x04, 0, 0);
    }
}

// ------------------- Interleave (flag 0x08) -------------------

#[test]
fn interleave_various_lengths() {
    let libs = Libs::load();
    for len in [2usize, 3, 4, 5, 10, 11, 32, 100, 200, 255, 256] {
        let input: Vec<u8> = (0..len as u32).map(|x| (x as u8).wrapping_mul(3)).collect();
        assert_pair_match(&libs, &input, 0x08, 0, 0);
    }
}

#[test]
fn interleave_too_short() {
    let libs = Libs::load();
    let input = vec![42u8];
    assert_pair_match(&libs, &input, 0x08, 0, 0);
}

// ------------------- Reverse segments (flag 0x10) -------------------

#[test]
fn reverse_segments_various() {
    let libs = Libs::load();
    let lengths: &[usize] = &[4, 5, 8, 9, 16, 17, 31, 32, 64, 100, 256];
    for &len in lengths {
        let input: Vec<u8> = (0..len as u32).map(|x| x as u8).collect();
        for seg_size in [1i32, 2, 3, 4, 5, 7, 8, 16, 32, 64, 100, 256, 0, -3] {
            assert_pair_match(&libs, &input, 0x10, seg_size, 0);
        }
    }
}

#[test]
fn reverse_segments_too_short() {
    let libs = Libs::load();
    let input = vec![1u8, 2, 3];
    assert_pair_match(&libs, &input, 0x10, 4, 0);
}

// ------------------- Combined flags -------------------

#[test]
fn combined_all_flags() {
    let libs = Libs::load();
    let inputs: Vec<Vec<u8>> = vec![
        (0..100u32).map(|i| ((i * 7) % 11) as u8).collect(),
        (0..50u32).map(|i| (i / 5) as u8).collect(),
        std::iter::repeat(0u8).take(200).collect(),
        (0..200u32).map(|i| (i ^ 0xA5) as u8).collect(),
        vec![1, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5],
    ];
    for input in inputs {
        for flags in 0u32..32u32 {
            for p1 in [-5i32, 0, 1, 3, 4, 7, 16, 100] {
                for p2 in [0i32, 1] {
                    assert_pair_match(&libs, &input, flags, p1, p2);
                }
            }
        }
    }
}

#[test]
fn pseudorandom_inputs() {
    let libs = Libs::load();
    let mut state: u32 = 0xDEAD_BEEF;
    let mut next = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state
    };

    for _ in 0..200 {
        let len = (next() as usize) % 257; // 0..=256
        let input: Vec<u8> = (0..len).map(|_| (next() & 0xFF) as u8).collect();
        let flags = next() & 0x1F;
        let p1 = (next() % 512) as i32 - 256;
        let p2 = (next() & 1) as i32;
        assert_pair_match(&libs, &input, flags, p1, p2);
    }
}

// ------------------- Symbol export check -------------------
//
// Sanity check that mirrors `nm -D`: we pull both .so files' export tables
// and confirm the Rust .so exports the C-defined function symbol. (libloading
// has no enumeration API, so we just assert the lookup succeeds.)
#[test]
fn rust_so_exports_process_buffer() {
    unsafe {
        let lib = Library::new(rust_so_path()).expect("Rust .so loads");
        let _: Symbol<ProcessBufferFn> = lib
            .get(b"process_buffer\0")
            .expect("Rust .so must export process_buffer with C ABI");
    }
}
