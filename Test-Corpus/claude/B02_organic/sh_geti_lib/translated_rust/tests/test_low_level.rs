//! Low-level FFI tests: hash functions, array growth, string arena.

mod common;

use common::{c_lib_path, ensure_libs_built, rust_lib_path};
use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

// Repr-equivalent layout types matching the C header for testing purposes.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
    _pad: [u8; 6],
}

#[test]
fn test_hash_string_matches() {
    ensure_libs_built();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type Fn = unsafe extern "C" fn(*mut c_char, usize) -> usize;
        let c_fn: Symbol<Fn> = c_lib.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<Fn> = r_lib.get(b"stbds_hash_string").unwrap();

        let test_strings: &[&[u8]] = &[
            b"\0",
            b"a\0",
            b"ab\0",
            b"abc\0",
            b"hello\0",
            b"hello world\0",
            b"a much longer string with several words to hash\0",
            b"test_0\0",
            b"test_1\0",
            b"test_42\0",
            b"foo\0",
        ];

        let seeds = [0usize, 1, 0x31415926, 0xdeadbeef, 0xffffffffffffffff];

        for s in test_strings {
            for &seed in &seeds {
                let mut buf = s.to_vec();
                let p = buf.as_mut_ptr() as *mut c_char;
                let c_v = c_fn(p, seed);
                let r_v = r_fn(p, seed);
                assert_eq!(
                    c_v, r_v,
                    "stbds_hash_string mismatch for {:?} seed={:#x}",
                    String::from_utf8_lossy(s),
                    seed
                );
            }
        }
    }
}

#[test]
fn test_hash_bytes_matches() {
    ensure_libs_built();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type Fn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
        let c_fn: Symbol<Fn> = c_lib.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<Fn> = r_lib.get(b"stbds_hash_bytes").unwrap();

        // Test with various byte buffers, lengths, and seeds.
        let mut data: Vec<u8> = (0..=255u8).collect();
        let seeds = [0usize, 1, 0x31415926, 0xdeadbeef, 0xffffffffffffffff];

        for &len in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 32, 64, 128, 200] {
            if len > data.len() {
                continue;
            }
            for &seed in &seeds {
                let p = data.as_mut_ptr() as *mut c_void;
                let c_v = c_fn(p, len, seed);
                let r_v = r_fn(p, len, seed);
                assert_eq!(
                    c_v, r_v,
                    "stbds_hash_bytes mismatch len={} seed={:#x}",
                    len, seed
                );
            }
        }
    }
}

#[test]
fn test_string_arena_alloc() {
    ensure_libs_built();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type AllocFn =
            unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
        type ResetFn = unsafe extern "C" fn(*mut StringArena);
        let c_alloc: Symbol<AllocFn> = c_lib.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<AllocFn> = r_lib.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<ResetFn> = c_lib.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<ResetFn> = r_lib.get(b"stbds_strreset").unwrap();

        let mut c_arena = StringArena::default();
        let mut r_arena = StringArena::default();

        // Same input strings -> same returned text.
        let inputs: &[&[u8]] = &[
            b"hello\0",
            b"world\0",
            b"foo\0",
            b"bar\0",
            b"a longer string for the arena\0",
            b"x\0",
        ];

        for s in inputs {
            let mut buf = s.to_vec();
            let cp = c_alloc(&mut c_arena, buf.as_mut_ptr() as *mut c_char);
            let rp = r_alloc(&mut r_arena, buf.as_mut_ptr() as *mut c_char);
            // Compare the contents at the returned pointers.
            let cs = std::ffi::CStr::from_ptr(cp);
            let rs = std::ffi::CStr::from_ptr(rp);
            assert_eq!(cs, rs);
        }
        // Compare arena bookkeeping after allocations (pointer fields will
        // differ because allocations are independent).
        assert_eq!(c_arena.remaining, r_arena.remaining);
        assert_eq!(c_arena.block, r_arena.block);
        assert_eq!(c_arena.mode, r_arena.mode);

        c_reset(&mut c_arena);
        r_reset(&mut r_arena);
        assert_eq!(c_arena.remaining, r_arena.remaining);
        assert_eq!(c_arena.block, r_arena.block);
    }
}

#[test]
fn test_arrgrowf_and_free() {
    ensure_libs_built();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type GrowFn = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
        type FreeFn = unsafe extern "C" fn(*mut c_void);

        let c_grow: Symbol<GrowFn> = c_lib.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<GrowFn> = r_lib.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"stbds_arrfreef").unwrap();

        // Just verify that growing produces an array with correct capacity in
        // both implementations. We can read the header (24 bytes preceding
        // the returned pointer).
        for &(addlen, mincap) in &[(0usize, 1usize), (1, 0), (5, 0), (0, 100)] {
            let cp = c_grow(std::ptr::null_mut(), 4, addlen, mincap);
            let rp = r_grow(std::ptr::null_mut(), 4, addlen, mincap);

            let c_hdr = (cp as *const u8).sub(24) as *const [usize; 2];
            let r_hdr = (rp as *const u8).sub(24) as *const [usize; 2];
            // length = 0, capacity = same.
            assert_eq!((*c_hdr)[0], (*r_hdr)[0], "length mismatch");
            assert_eq!(
                (*c_hdr)[1],
                (*r_hdr)[1],
                "capacity mismatch addlen={} mincap={}",
                addlen,
                mincap
            );

            c_free(cp);
            r_free(rp);
        }
    }
}

#[test]
fn test_strkey() {
    ensure_libs_built();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type Fn = unsafe extern "C" fn(c_int) -> *mut c_char;
        let c_fn: Symbol<Fn> = c_lib.get(b"strkey").unwrap();
        let r_fn: Symbol<Fn> = r_lib.get(b"strkey").unwrap();

        for n in [0, 1, 7, 42, 99, 1234, -1, -42, 0x7fffffff] {
            let cp = c_fn(n);
            let rp = r_fn(n);
            let cs = std::ffi::CStr::from_ptr(cp);
            let rs = std::ffi::CStr::from_ptr(rp);
            assert_eq!(cs, rs, "strkey({}) differs", n);
        }
    }
}

#[test]
fn test_rand_seed() {
    // We can't easily observe the seed directly, but we can call the function
    // to verify it links and accepts the argument without crashing.
    ensure_libs_built();
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type Fn = unsafe extern "C" fn(usize);
        let c_fn: Symbol<Fn> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_fn: Symbol<Fn> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_fn(0xdeadbeef);
        r_fn(0xdeadbeef);
    }
}
