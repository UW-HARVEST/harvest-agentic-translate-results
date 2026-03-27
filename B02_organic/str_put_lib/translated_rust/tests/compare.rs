use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libstr_put_lib.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the built Rust cdylib
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try debug first
    let debug = p.join("debug").join("libstr_put_lib.so");
    if debug.exists() {
        return debug;
    }
    let release = p.join("release").join("libstr_put_lib.so");
    if release.exists() {
        return release;
    }
    debug // fallback
}

// ============================================================
// 1. stbds_rand_seed - just verify it doesn't crash
// ============================================================
#[test]
fn test_rand_seed() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(usize)> =
            c_lib.get(b"stbds_rand_seed").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(usize)> =
            r_lib.get(b"stbds_rand_seed").unwrap();

        c_fn(42);
        r_fn(42);
        // No crash = pass
    }
}

// ============================================================
// 2. stbds_hash_string - compare outputs for several inputs
// ============================================================
#[test]
fn test_hash_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            r_lib.get(b"stbds_hash_string").unwrap();

        let test_cases = [
            ("", 0usize),
            ("hello", 0),
            ("hello", 12345),
            ("test_0", 0x31415926),
            ("a", 1),
            ("abcdefghijklmnopqrstuvwxyz", 999),
        ];

        for (s, seed) in &test_cases {
            let cs = CString::new(*s).unwrap();
            let c_result = c_fn(cs.as_ptr() as *mut c_char, *seed);
            let r_result = r_fn(cs.as_ptr() as *mut c_char, *seed);
            assert_eq!(
                c_result, r_result,
                "hash_string mismatch for ({:?}, {}): C={:#x} Rust={:#x}",
                s, seed, c_result, r_result
            );
        }
    }
}

// ============================================================
// 3. stbds_hash_bytes - compare outputs for several inputs
// ============================================================
#[test]
fn test_hash_bytes() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            c_lib.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            r_lib.get(b"stbds_hash_bytes").unwrap();

        let test_inputs: Vec<(Vec<u8>, usize)> = vec![
            (vec![], 0),
            (vec![1], 0),
            (vec![1, 2, 3, 4], 0),
            (vec![1, 2, 3, 4, 5, 6, 7, 8], 0),
            (vec![1, 2, 3, 4, 5, 6, 7, 8, 9], 0),
            (vec![0; 16], 42),
            (b"hello world".to_vec(), 12345),
            ((0..255u8).collect(), 0x31415926),
        ];

        for (data, seed) in &test_inputs {
            let mut buf = data.clone();
            let ptr = if buf.is_empty() {
                std::ptr::null_mut()
            } else {
                buf.as_mut_ptr() as *mut c_void
            };
            let c_result = c_fn(ptr, data.len(), *seed);

            let mut buf2 = data.clone();
            let ptr2 = if buf2.is_empty() {
                std::ptr::null_mut()
            } else {
                buf2.as_mut_ptr() as *mut c_void
            };
            let r_result = r_fn(ptr2, data.len(), *seed);

            assert_eq!(
                c_result, r_result,
                "hash_bytes mismatch for len={} seed={}: C={:#x} Rust={:#x}",
                data.len(),
                seed,
                c_result,
                r_result
            );
        }
    }
}

// ============================================================
// 4. strkey - compare output strings
// ============================================================
#[test]
fn test_strkey() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> =
            c_lib.get(b"strkey").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> =
            r_lib.get(b"strkey").unwrap();

        for n in [0, 1, 10, 100, 999] {
            let c_ptr = c_fn(n);
            let r_ptr = r_fn(n);
            let c_str = std::ffi::CStr::from_ptr(c_ptr).to_str().unwrap();
            let r_str = std::ffi::CStr::from_ptr(r_ptr).to_str().unwrap();
            assert_eq!(
                c_str, r_str,
                "strkey mismatch for n={}: C={:?} Rust={:?}",
                n, c_str, r_str
            );
        }
    }
}

// ============================================================
// 5. stbds_arrgrowf - test basic allocation behavior
// ============================================================
#[test]
fn test_arrgrowf() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type ArrgrowfFn = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
        type ArrfreefFn = unsafe extern "C" fn(*mut c_void);

        let c_grow: Symbol<ArrgrowfFn> = c_lib.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<ArrgrowfFn> = r_lib.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<ArrfreefFn> = c_lib.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<ArrfreefFn> = r_lib.get(b"stbds_arrfreef").unwrap();

        // Allocate from null with elemsize=4, addlen=0, min_cap=10
        let c_arr = c_grow(std::ptr::null_mut(), 4, 0, 10);
        let r_arr = r_grow(std::ptr::null_mut(), 4, 0, 10);

        // Both should be non-null
        assert!(!c_arr.is_null(), "C arrgrowf returned null");
        assert!(!r_arr.is_null(), "Rust arrgrowf returned null");

        // Check header fields match: length, capacity, hash_table, temp
        // Header is at (ptr - sizeof(header))
        let hdr_size = 32usize; // 4 fields * 8 bytes on 64-bit
        let c_hdr = (c_arr as *const u8).sub(hdr_size);
        let r_hdr = (r_arr as *const u8).sub(hdr_size);

        let c_len = *(c_hdr as *const usize);
        let r_len = *(r_hdr as *const usize);
        assert_eq!(c_len, r_len, "length mismatch: C={} Rust={}", c_len, r_len);

        let c_cap = *((c_hdr as *const usize).add(1));
        let r_cap = *((r_hdr as *const usize).add(1));
        assert_eq!(c_cap, r_cap, "capacity mismatch: C={} Rust={}", c_cap, r_cap);

        c_free(c_arr);
        r_free(r_arr);
    }
}

// ============================================================
// 6. stbds_stralloc / stbds_strreset - test string arena
// ============================================================
#[test]
fn test_stralloc_strreset() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type StrallocFn = unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char;
        type StrresetFn = unsafe extern "C" fn(*mut c_void);

        let c_alloc: Symbol<StrallocFn> = c_lib.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<StrallocFn> = r_lib.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<StrresetFn> = c_lib.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<StrresetFn> = r_lib.get(b"stbds_strreset").unwrap();

        // stbds_string_arena is: { *storage, remaining(usize), block(u8), mode(u8) }
        // On 64-bit: ptr(8) + usize(8) + u8 + u8 + padding = 24 bytes
        // Let's use a zeroed buffer
        let arena_size = 24usize;
        let mut c_arena = vec![0u8; arena_size];
        let mut r_arena = vec![0u8; arena_size];

        let test_strings = ["hello", "world", "test_string_123", "a"];

        for s in &test_strings {
            let cs = CString::new(*s).unwrap();
            let c_ptr = c_alloc(c_arena.as_mut_ptr() as *mut c_void, cs.as_ptr() as *mut c_char);
            let r_ptr = r_alloc(r_arena.as_mut_ptr() as *mut c_void, cs.as_ptr() as *mut c_char);

            // Both should return valid strings equal to input
            let c_result = std::ffi::CStr::from_ptr(c_ptr).to_str().unwrap();
            let r_result = std::ffi::CStr::from_ptr(r_ptr).to_str().unwrap();
            assert_eq!(
                c_result, r_result,
                "stralloc mismatch for {:?}: C={:?} Rust={:?}",
                s, c_result, r_result
            );
            assert_eq!(c_result, *s);
        }

        // Check arena state matches (remaining, block fields)
        // remaining is at offset 8
        let c_remaining = *(c_arena.as_ptr().add(8) as *const usize);
        let r_remaining = *(r_arena.as_ptr().add(8) as *const usize);
        assert_eq!(
            c_remaining, r_remaining,
            "arena remaining mismatch: C={} Rust={}",
            c_remaining, r_remaining
        );

        // block is at offset 16
        assert_eq!(
            c_arena[16], r_arena[16],
            "arena block mismatch: C={} Rust={}",
            c_arena[16], r_arena[16]
        );

        c_reset(c_arena.as_mut_ptr() as *mut c_void);
        r_reset(r_arena.as_mut_ptr() as *mut c_void);

        // After reset, arena should be zeroed
        assert_eq!(c_arena, r_arena, "arena state mismatch after reset");
    }
}

// ============================================================
// 7. str_put - capture stdout and compare
// ============================================================
#[test]
fn test_str_put_output() {
    use std::process::Command;

    // We'll write small C and Rust programs that call str_put and capture output
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = c_lib_path();
    let r_so = rust_lib_path();

    // Use a helper program via LD_PRELOAD or dlopen. Simpler: write a tiny C program.
    let tmp_dir = std::env::temp_dir();
    let helper_c = tmp_dir.join("str_put_helper.c");
    let helper_bin = tmp_dir.join("str_put_helper");

    std::fs::write(
        &helper_c,
        r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: %s <lib.so> <num>\n", argv[0]); return 1; }
    void *lib = dlopen(argv[1], RTLD_NOW);
    if (!lib) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
    void (*fn)(int) = dlsym(lib, "str_put");
    if (!fn) { fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }
    fn(atoi(argv[2]));
    dlclose(lib);
    return 0;
}
"#,
    )
    .unwrap();

    let compile = Command::new("gcc")
        .args([
            helper_c.to_str().unwrap(),
            "-o",
            helper_bin.to_str().unwrap(),
            "-ldl",
        ])
        .output()
        .expect("gcc");
    assert!(compile.status.success(), "Failed to compile helper: {:?}", String::from_utf8_lossy(&compile.stderr));

    for num in [1, 5] {
        let c_out = Command::new(helper_bin.to_str().unwrap())
            .args([c_so.to_str().unwrap(), &num.to_string()])
            .output()
            .expect("run C");

        let r_out = Command::new(helper_bin.to_str().unwrap())
            .args([r_so.to_str().unwrap(), &num.to_string()])
            .output()
            .expect("run Rust");

        let c_stdout = String::from_utf8_lossy(&c_out.stdout);
        let r_stdout = String::from_utf8_lossy(&r_out.stdout);

        assert_eq!(
            c_stdout, r_stdout,
            "str_put({}) stdout mismatch:\nC:    {:?}\nRust: {:?}\nC stderr: {:?}\nRust stderr: {:?}",
            num, c_stdout, r_stdout,
            String::from_utf8_lossy(&c_out.stderr),
            String::from_utf8_lossy(&r_out.stderr),
        );
    }
}

// ============================================================
// 8. Higher-level: stbds_hmput_key / stbds_hmget_key round-trip
//    with seeded hash to ensure deterministic behavior
// ============================================================
#[test]
fn test_hmput_hmget_roundtrip() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        type RandSeedFn = unsafe extern "C" fn(usize);
        type HmputKeyFn =
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmgetKeyFn =
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_seed: Symbol<RandSeedFn> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<RandSeedFn> = r_lib.get(b"stbds_rand_seed").unwrap();
        let c_put: Symbol<HmputKeyFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmputKeyFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<HmgetKeyFn> = c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<HmgetKeyFn> = r_lib.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<HmfreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmfreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        // Set same seed
        c_seed(0x12345678);
        r_seed(0x12345678);

        // Use struct { int key; int value; } => elemsize = 8
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct KV {
            key: c_int,
            value: c_int,
        }
        let elemsize = std::mem::size_of::<KV>();

        let mut c_map: *mut c_void = std::ptr::null_mut();
        let mut r_map: *mut c_void = std::ptr::null_mut();

        // Insert several keys
        for i in 0..10i32 {
            let mut k = i;
            c_map = c_put(
                c_map,
                elemsize,
                &mut k as *mut i32 as *mut c_void,
                std::mem::size_of::<c_int>(),
                0, // STBDS_HM_BINARY
            );
            let mut k2 = i;
            r_map = r_put(
                r_map,
                elemsize,
                &mut k2 as *mut i32 as *mut c_void,
                std::mem::size_of::<c_int>(),
                0,
            );

            // After put, check temp field matches
            let c_raw = (c_map as *mut u8).sub(elemsize) as *mut c_void;
            let r_raw = (r_map as *mut u8).sub(elemsize) as *mut c_void;
            let c_hdr = (c_raw as *mut u8).sub(32);
            let r_hdr = (r_raw as *mut u8).sub(32);
            let c_temp = *((c_hdr as *const isize).add(3));
            let r_temp = *((r_hdr as *const isize).add(3));
            assert_eq!(
                c_temp, r_temp,
                "temp mismatch after put({}): C={} Rust={}",
                i, c_temp, r_temp
            );

            // Write value at the returned index
            let c_entry = (c_map as *mut u8).offset(elemsize as isize * c_temp) as *mut KV;
            (*c_entry).key = i;
            (*c_entry).value = i * 100;
            let r_entry = (r_map as *mut u8).offset(elemsize as isize * r_temp) as *mut KV;
            (*r_entry).key = i;
            (*r_entry).value = i * 100;
        }

        // Now get each key and verify temp matches
        for i in 0..10i32 {
            let mut k = i;
            c_map = c_get(
                c_map,
                elemsize,
                &mut k as *mut i32 as *mut c_void,
                std::mem::size_of::<c_int>(),
                0,
            );
            let mut k2 = i;
            r_map = r_get(
                r_map,
                elemsize,
                &mut k2 as *mut i32 as *mut c_void,
                std::mem::size_of::<c_int>(),
                0,
            );

            let c_raw = (c_map as *mut u8).sub(elemsize) as *mut c_void;
            let r_raw = (r_map as *mut u8).sub(elemsize) as *mut c_void;
            let c_hdr = (c_raw as *mut u8).sub(32);
            let r_hdr = (r_raw as *mut u8).sub(32);
            let c_temp = *((c_hdr as *const isize).add(3));
            let r_temp = *((r_hdr as *const isize).add(3));
            assert_eq!(
                c_temp, r_temp,
                "temp mismatch after get({}): C={} Rust={}",
                i, c_temp, r_temp
            );

            // Verify value
            let c_entry = &*((c_map as *const u8).offset(elemsize as isize * c_temp) as *const KV);
            let r_entry = &*((r_map as *const u8).offset(elemsize as isize * r_temp) as *const KV);
            assert_eq!(c_entry.value, r_entry.value, "value mismatch for key {}", i);
        }

        // Free
        let c_raw = (c_map as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_map as *mut u8).sub(elemsize) as *mut c_void;
        c_free(c_raw, elemsize);
        r_free(r_raw, elemsize);
    }
}
