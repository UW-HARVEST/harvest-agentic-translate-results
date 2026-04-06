use libloading::{Library, Symbol};
use std::ffi::CString;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libhm_geti_lib.so");

fn c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C library") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path = format!("{}/target/debug/libhm_geti_lib.so", manifest);
    unsafe { Library::new(&path).expect("Failed to load Rust library") }
}

// ============================================================
// 1. stbds_hash_string
// ============================================================
#[test]
fn test_hash_string() {
    let c = c_lib();
    let r = rust_lib();
    let cases = ["", "hello", "test_0", "test_999", "a", "abcdefghijklmnop"];
    let seeds: &[usize] = &[0, 1, 0x31415926, 0xDEADBEEF, usize::MAX];

    for &s in &cases {
        let cs = CString::new(s).unwrap();
        for &seed in seeds {
            unsafe {
                let cf: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
                    c.get(b"stbds_hash_string").unwrap();
                let rf: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
                    r.get(b"stbds_hash_string").unwrap();
                let cv = cf(cs.as_ptr() as *mut u8, seed);
                let rv = rf(cs.as_ptr() as *mut u8, seed);
                assert_eq!(cv, rv, "hash_string mismatch for {:?} seed={:#x}", s, seed);
            }
        }
    }
}

// ============================================================
// 2. stbds_hash_bytes
// ============================================================
#[test]
fn test_hash_bytes() {
    let c = c_lib();
    let r = rust_lib();
    let test_data: &[&[u8]] = &[
        &[],
        &[0],
        &[1, 2, 3, 4],
        &[0, 1, 2, 3, 4, 5, 6, 7],
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        b"hello world",
        &[0xFF; 32],
    ];
    let seeds: &[usize] = &[0, 1, 0x31415926, 0xCAFEBABE];

    for data in test_data {
        for &seed in seeds {
            unsafe {
                let cf: Symbol<unsafe extern "C" fn(*mut libc::c_void, usize, usize) -> usize> =
                    c.get(b"stbds_hash_bytes").unwrap();
                let rf: Symbol<unsafe extern "C" fn(*mut libc::c_void, usize, usize) -> usize> =
                    r.get(b"stbds_hash_bytes").unwrap();
                let cv = cf(data.as_ptr() as *mut libc::c_void, data.len(), seed);
                let rv = rf(data.as_ptr() as *mut libc::c_void, data.len(), seed);
                assert_eq!(
                    cv, rv,
                    "hash_bytes mismatch for len={} seed={:#x}",
                    data.len(),
                    seed
                );
            }
        }
    }
}

// ============================================================
// 3. stbds_rand_seed + hash consistency
// ============================================================
#[test]
fn test_rand_seed() {
    let c = c_lib();
    let r = rust_lib();
    let probe = CString::new("probe").unwrap();

    for &seed in &[0usize, 42, 0x31415926, 0xFFFFFFFF] {
        unsafe {
            let c_set: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
            let r_set: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
            let c_hash: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
                c.get(b"stbds_hash_string").unwrap();
            let r_hash: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
                r.get(b"stbds_hash_string").unwrap();

            c_set(seed);
            r_set(seed);
            let cv = c_hash(probe.as_ptr() as *mut u8, seed);
            let rv = r_hash(probe.as_ptr() as *mut u8, seed);
            assert_eq!(cv, rv, "rand_seed/hash mismatch for seed={:#x}", seed);
        }
    }
}

// ============================================================
// 4. hm_geti - main function (both should not crash)
// ============================================================
#[test]
fn test_hm_geti() {
    let c = c_lib();
    let r = rust_lib();

    for &num in &[0, 1, 2, 10, 50, 100, 200] {
        unsafe {
            let c_set: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
            let r_set: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
            let c_fn: Symbol<unsafe extern "C" fn(i32)> = c.get(b"hm_geti").unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(i32)> = r.get(b"hm_geti").unwrap();

            c_set(0x31415926);
            c_fn(num);

            r_set(0x31415926);
            r_fn(num);
        }
    }
}

// ============================================================
// 5. strkey
// ============================================================
#[test]
fn test_strkey() {
    let c = c_lib();
    let r = rust_lib();

    for n in [0, 1, 42, 999] {
        unsafe {
            let cf: Symbol<unsafe extern "C" fn(i32) -> *mut u8> = c.get(b"strkey").unwrap();
            let rf: Symbol<unsafe extern "C" fn(i32) -> *mut u8> = r.get(b"strkey").unwrap();

            let c_ptr = cf(n);
            let c_str = std::ffi::CStr::from_ptr(c_ptr as *const i8)
                .to_str()
                .unwrap()
                .to_owned();

            let r_ptr = rf(n);
            let r_str = std::ffi::CStr::from_ptr(r_ptr as *const i8)
                .to_str()
                .unwrap();

            assert_eq!(c_str, r_str, "strkey mismatch for n={}", n);
        }
    }
}
