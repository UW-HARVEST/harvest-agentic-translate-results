use libloading::{Library, Symbol};
use std::ffi::CString;

fn c_lib() -> Library {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let so = manifest.join("target/debug/libarr_del_lib.so");
    unsafe { Library::new(so).expect("Failed to load Rust .so") }
}

// ============================================================
// Test: stbds_hash_string
// ============================================================
#[test]
fn test_hash_string() {
    let c = c_lib();
    let r = rust_lib();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            c.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            r.get(b"stbds_hash_string").unwrap();

        for seed in [0usize, 1, 42, 0x31415926, usize::MAX] {
            for s in &["", "hello", "test_123", "a", "abcdefghijklmnop"] {
                let cs = CString::new(*s).unwrap();
                let ptr = cs.as_ptr() as *mut u8;
                let c_val = c_fn(ptr, seed);
                let r_val = r_fn(ptr, seed);
                assert_eq!(c_val, r_val, "hash_string mismatch for {:?} seed={}", s, seed);
            }
        }
    }
}

// ============================================================
// Test: stbds_hash_bytes
// ============================================================
#[test]
fn test_hash_bytes() {
    let c = c_lib();
    let r = rust_lib();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
            c.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
            r.get(b"stbds_hash_bytes").unwrap();

        for seed in [0usize, 1, 42, 0x31415926] {
            // Various lengths to test all switch cases (0..8+)
            for len in 0..=20 {
                let data: Vec<u8> = (0..len).map(|i| (i * 37 + 13) as u8).collect();
                let mut c_data = data.clone();
                let mut r_data = data.clone();
                let c_val = c_fn(c_data.as_mut_ptr(), len as usize, seed);
                let r_val = r_fn(r_data.as_mut_ptr(), len as usize, seed);
                assert_eq!(c_val, r_val, "hash_bytes mismatch len={} seed={}", len, seed);
            }
        }
    }
}

// ============================================================
// Test: stbds_arrgrowf + stbds_arrfreef
// ============================================================
#[test]
fn test_arrgrowf() {
    let c = c_lib();
    let r = rust_lib();

    unsafe {
        type ArrgrowfFn = unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8;
        type ArrfreefFn = unsafe extern "C" fn(*mut u8);

        let c_grow: Symbol<ArrgrowfFn> = c.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<ArrgrowfFn> = r.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<ArrfreefFn> = c.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<ArrfreefFn> = r.get(b"stbds_arrfreef").unwrap();

        // Allocate from null, elemsize=4 (int), addlen=0, min_cap=10
        let c_arr = c_grow(std::ptr::null_mut(), 4, 0, 10);
        let r_arr = r_grow(std::ptr::null_mut(), 4, 0, 10);

        // Both should be non-null
        assert!(!c_arr.is_null(), "C arrgrowf returned null");
        assert!(!r_arr.is_null(), "Rust arrgrowf returned null");

        // Read header fields: length should be 0, capacity should be 10
        // Header is at (ptr - sizeof(header))
        #[repr(C)]
        struct Header {
            length: usize,
            capacity: usize,
            hash_table: *mut u8,
            temp: isize,
        }
        let c_hdr = &*((c_arr as *const Header).offset(-1));
        let r_hdr = &*((r_arr as *const Header).offset(-1));

        assert_eq!(c_hdr.length, r_hdr.length, "length mismatch after arrgrowf");
        assert_eq!(c_hdr.capacity, r_hdr.capacity, "capacity mismatch after arrgrowf");

        c_free(c_arr);
        r_free(r_arr);
    }
}

// ============================================================
// Test: strkey
// ============================================================
#[test]
fn test_strkey() {
    let c = c_lib();
    let r = rust_lib();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> *mut i8> =
            c.get(b"strkey").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32) -> *mut i8> =
            r.get(b"strkey").unwrap();

        for n in [0, 1, -1, 42, 1000, -999] {
            let c_val = c_fn(n);
            let r_val = r_fn(n);
            let c_str = std::ffi::CStr::from_ptr(c_val).to_str().unwrap();
            let r_str = std::ffi::CStr::from_ptr(r_val).to_str().unwrap();
            assert_eq!(c_str, r_str, "strkey mismatch for n={}", n);
        }
    }
}

// ============================================================
// Test: arr_del (the main function - just ensure no crash and
// both run identically; it has no return value but exercises
// the array operations internally)
// ============================================================
#[test]
fn test_arr_del() {
    let c = c_lib();
    let r = rust_lib();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32)> = c.get(b"arr_del").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32)> = r.get(b"arr_del").unwrap();

        // arr_del doesn't return anything or print, but we verify no crash
        // and that both complete successfully
        for num in [0, 1, -1, 42, 100, -999] {
            c_fn(num);
            r_fn(num);
        }
    }
}

// ============================================================
// Test: stbds_rand_seed (just verify no crash, it's a setter)
// ============================================================
#[test]
fn test_rand_seed() {
    let c = c_lib();
    let r = rust_lib();

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();

        for seed in [0usize, 1, 42, 0xdeadbeef] {
            c_fn(seed);
            r_fn(seed);
        }
    }
}
