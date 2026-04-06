use libloading::{Library, Symbol};
use std::ffi::c_int;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libarr_push_lib.so");

fn rust_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{manifest}/target/debug/libarr_push_lib.so")
}

fn load_c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C .so") }
}

fn load_rust_lib() -> Library {
    unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") }
}

// ── 1. stbds_hash_string ──────────────────────────────────────────────
#[test]
fn test_hash_string() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();
    let seeds: &[usize] = &[0, 1, 42, 0x31415926, usize::MAX];
    let strings: &[&[u8]] = &[b"hello\0", b"world\0", b"\0", b"test_123\0", b"a\0"];

    for &seed in seeds {
        for &s in strings {
            unsafe {
                let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
                    c_lib.get(b"stbds_hash_string").unwrap();
                let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
                    r_lib.get(b"stbds_hash_string").unwrap();
                let c_result = c_fn(s.as_ptr() as *mut u8, seed);
                let r_result = r_fn(s.as_ptr() as *mut u8, seed);
                assert_eq!(c_result, r_result,
                    "hash_string mismatch for {:?} seed={seed}",
                    std::str::from_utf8(&s[..s.len()-1]));
            }
        }
    }
}

// ── 2. stbds_hash_bytes ───────────────────────────────────────────────
#[test]
fn test_hash_bytes() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();
    let seeds: &[usize] = &[0, 1, 42, 0x31415926];
    let inputs: &[&[u8]] = &[
        b"", b"a", b"ab", b"abc", b"abcd", b"abcde", b"abcdef", b"abcdefg",
        b"abcdefgh", b"abcdefghijklm",
        b"\x00\x01\x02\x03\x04\x05\x06\x07\x08",
    ];

    for &seed in seeds {
        for &data in inputs {
            unsafe {
                let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
                    c_lib.get(b"stbds_hash_bytes").unwrap();
                let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
                    r_lib.get(b"stbds_hash_bytes").unwrap();
                let c_result = c_fn(data.as_ptr() as *mut u8, data.len(), seed);
                let r_result = r_fn(data.as_ptr() as *mut u8, data.len(), seed);
                assert_eq!(c_result, r_result,
                    "hash_bytes mismatch for len={} seed={seed}", data.len());
            }
        }
    }
}

// ── 3. stbds_arrgrowf / stbds_arrfreef ────────────────────────────────
#[test]
fn test_arrgrowf_basic() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();

    unsafe {
        let c_grow: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> =
            c_lib.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8)> =
            c_lib.get(b"stbds_arrfreef").unwrap();
        let r_grow: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> =
            r_lib.get(b"stbds_arrgrowf").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8)> =
            r_lib.get(b"stbds_arrfreef").unwrap();

        // Grow from null with elemsize=4
        let c_arr = c_grow(std::ptr::null_mut(), 4, 1, 0);
        let r_arr = r_grow(std::ptr::null_mut(), 4, 1, 0);

        // Read header fields (length, capacity at offsets -4, -3 in usize units)
        let c_hdr = c_arr as *mut usize;
        let r_hdr = r_arr as *mut usize;
        let c_len = *c_hdr.offset(-4);
        let c_cap = *c_hdr.offset(-3);
        let r_len = *r_hdr.offset(-4);
        let r_cap = *r_hdr.offset(-3);

        assert_eq!(c_len, r_len, "length mismatch after grow from null");
        assert_eq!(c_cap, r_cap, "capacity mismatch after grow from null");
        assert_eq!(c_len, 0);
        assert!(c_cap >= 4);

        c_free(c_arr);
        r_free(r_arr);
    }
}

// ── 4. arr_push (the main exported function) ──────────────────────────
#[test]
fn test_arr_push() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();

    for &num in &[0, 1, 50, 100, 200] {
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"arr_push").unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"arr_push").unwrap();
            c_fn(num);
            r_fn(num);
        }
    }
    // If we get here without crashing, both behave the same
}

// ── 5. strkey ─────────────────────────────────────────────────────────
#[test]
fn test_strkey() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();

    for &n in &[0, 1, 42, 999] {
        unsafe {
            let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut u8> =
                c_lib.get(b"strkey").unwrap();
            let r_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut u8> =
                r_lib.get(b"strkey").unwrap();

            let c_ptr = c_fn(n);
            let r_ptr = r_fn(n);
            let c_str = std::ffi::CStr::from_ptr(c_ptr as *const i8);
            let r_str = std::ffi::CStr::from_ptr(r_ptr as *const i8);
            assert_eq!(c_str, r_str, "strkey mismatch for n={n}");
        }
    }
}

// ── 6. stbds_rand_seed + hash consistency ─────────────────────────────
#[test]
fn test_rand_seed_then_hash() {
    let c_lib = load_c_lib();
    let r_lib = load_rust_lib();

    unsafe {
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(12345);
        r_seed(12345);

        let c_hash: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();
        let r_hash: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            r_lib.get(b"stbds_hash_string").unwrap();

        let s = b"test\0";
        assert_eq!(
            c_hash(s.as_ptr() as *mut u8, 99),
            r_hash(s.as_ptr() as *mut u8, 99),
        );
    }
}
