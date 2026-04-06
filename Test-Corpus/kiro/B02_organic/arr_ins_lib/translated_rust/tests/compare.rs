use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libarr_ins_lib.so");

fn load_c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C library") }
}

// ── Test: stbds_rand_seed + stbds_hash_string ─────────────────────────
// After seeding, hash_string should produce identical results.
#[test]
fn test_hash_string() {
    let c_lib = load_c_lib();
    unsafe {
        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> =
            c_lib.get(b"stbds_rand_seed").unwrap();
        let c_hash_string: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();

        let seeds = [0usize, 1, 42, 0x31415926, 0xDEADBEEF, usize::MAX];
        let strings = ["", "hello", "test_42", "a", "abcdefghijklmnop", "stb_ds"];

        for &seed in &seeds {
            for &s in &strings {
                // Reset both to same seed
                c_rand_seed(seed);
                arr_ins_lib::stbds_rand_seed(seed);

                let cs = CString::new(s).unwrap();
                let c_result = c_hash_string(cs.as_ptr() as *mut c_char, seed);
                let rust_result =
                    arr_ins_lib::stbds_hash_string(cs.as_ptr() as *mut c_char, seed);

                assert_eq!(
                    c_result, rust_result,
                    "hash_string mismatch for '{}' seed={}: C={:#x} Rust={:#x}",
                    s, seed, c_result, rust_result
                );
            }
        }
    }
}

// ── Test: stbds_hash_bytes ────────────────────────────────────────────
#[test]
fn test_hash_bytes() {
    let c_lib = load_c_lib();
    unsafe {
        let c_hash_bytes: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            c_lib.get(b"stbds_hash_bytes").unwrap();

        let test_data: &[&[u8]] = &[
            b"",
            b"a",
            b"ab",
            b"abc",
            b"abcd",
            b"abcde",
            b"abcdef",
            b"abcdefg",
            b"abcdefgh",
            b"abcdefghijklmnop",
            &[0u8; 64],
            &[0xFF; 17],
        ];
        let seeds = [0usize, 1, 42, 0x31415926, 0xDEADBEEF];

        for &seed in &seeds {
            for &data in test_data {
                let c_result =
                    c_hash_bytes(data.as_ptr() as *mut c_void, data.len(), seed);
                let rust_result =
                    arr_ins_lib::stbds_hash_bytes(data.as_ptr() as *mut c_void, data.len(), seed);

                assert_eq!(
                    c_result, rust_result,
                    "hash_bytes mismatch for len={} seed={}: C={:#x} Rust={:#x}",
                    data.len(),
                    seed,
                    c_result,
                    rust_result
                );
            }
        }
    }
}

// ── Test: stbds_arrgrowf / stbds_arrfreef ─────────────────────────────
#[test]
fn test_arrgrowf() {
    let c_lib = load_c_lib();
    unsafe {
        let c_arrgrowf: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
        > = c_lib.get(b"stbds_arrgrowf").unwrap();
        let c_arrfreef: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"stbds_arrfreef").unwrap();

        // Test: grow from null
        let elemsize = std::mem::size_of::<i32>();
        let c_arr = c_arrgrowf(std::ptr::null_mut(), elemsize, 0, 4);
        let r_arr = arr_ins_lib::stbds_arrgrowf(std::ptr::null_mut(), elemsize, 0, 4);

        // Both should have header with length=0, capacity>=4
        #[repr(C)]
        struct Header {
            length: usize,
            capacity: usize,
            hash_table: *mut c_void,
            temp: isize,
        }
        let c_hdr = &*((c_arr as *mut Header).offset(-1));
        let r_hdr = &*((r_arr as *mut Header).offset(-1));

        assert_eq!(c_hdr.length, r_hdr.length, "length mismatch after growf from null");
        assert_eq!(c_hdr.capacity, r_hdr.capacity, "capacity mismatch after growf from null");

        // Test: grow existing
        let c_arr2 = c_arrgrowf(c_arr, elemsize, 5, 0);
        let r_arr2 = arr_ins_lib::stbds_arrgrowf(r_arr, elemsize, 5, 0);
        let c_hdr2 = &*((c_arr2 as *mut Header).offset(-1));
        let r_hdr2 = &*((r_arr2 as *mut Header).offset(-1));
        assert_eq!(c_hdr2.capacity, r_hdr2.capacity, "capacity mismatch after second growf");

        c_arrfreef(c_arr2);
        arr_ins_lib::stbds_arrfreef(r_arr2);
    }
}

// ── Test: strkey ──────────────────────────────────────────────────────
#[test]
fn test_strkey() {
    let c_lib = load_c_lib();
    unsafe {
        let c_strkey: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> =
            c_lib.get(b"strkey").unwrap();

        for n in [0, 1, -1, 42, 100, 999, -999, i32::MAX, i32::MIN] {
            let c_ptr = c_strkey(n);
            let r_ptr = arr_ins_lib::strkey(n);

            let c_str = std::ffi::CStr::from_ptr(c_ptr).to_bytes();
            let r_str = std::ffi::CStr::from_ptr(r_ptr).to_bytes();

            assert_eq!(
                c_str, r_str,
                "strkey mismatch for n={}: C={:?} Rust={:?}",
                n,
                std::str::from_utf8(c_str),
                std::str::from_utf8(r_str)
            );
        }
    }
}

// ── Test: arr_ins ─────────────────────────────────────────────────────
// arr_ins doesn't return anything, it just asserts internally.
// If either panics/aborts, the test fails.
#[test]
fn test_arr_ins() {
    let c_lib = load_c_lib();
    unsafe {
        let c_arr_ins: Symbol<unsafe extern "C" fn(c_int)> =
            c_lib.get(b"arr_ins").unwrap();

        for num in [0, 1, -1, 42, 100, i32::MAX, i32::MIN] {
            c_arr_ins(num);
            arr_ins_lib::arr_ins(num);
        }
    }
}

// ── Test: stbds_stralloc / stbds_strreset ─────────────────────────────
#[test]
fn test_stralloc_strreset() {
    let c_lib = load_c_lib();
    unsafe {
        let c_stralloc: Symbol<unsafe extern "C" fn(*mut c_void, *mut c_char) -> *mut c_char> =
            c_lib.get(b"stbds_stralloc").unwrap();
        let c_strreset: Symbol<unsafe extern "C" fn(*mut c_void)> =
            c_lib.get(b"stbds_strreset").unwrap();

        // Create two arenas (zeroed)
        #[repr(C)]
        struct Arena {
            storage: *mut c_void,
            remaining: usize,
            block: u8,
            mode: u8,
        }
        let mut c_arena: Arena = std::mem::zeroed();
        let mut r_arena: arr_ins_lib::stbds_string_arena = std::mem::zeroed();

        let strings = ["hello", "world", "test_string_123", "a", ""];
        for s in &strings {
            let cs = CString::new(*s).unwrap();
            let c_ptr = c_stralloc(
                &mut c_arena as *mut Arena as *mut c_void,
                cs.as_ptr() as *mut c_char,
            );
            let r_ptr = arr_ins_lib::stbds_stralloc(
                &mut r_arena as *mut arr_ins_lib::stbds_string_arena,
                cs.as_ptr() as *mut c_char,
            );

            let c_result = std::ffi::CStr::from_ptr(c_ptr).to_bytes();
            let r_result = std::ffi::CStr::from_ptr(r_ptr).to_bytes();
            assert_eq!(
                c_result, r_result,
                "stralloc content mismatch for '{}'",
                s
            );

            // Check arena state matches
            assert_eq!(
                c_arena.remaining, r_arena.remaining,
                "arena remaining mismatch after '{}'",
                s
            );
            assert_eq!(
                c_arena.block, r_arena.block,
                "arena block mismatch after '{}'",
                s
            );
        }

        c_strreset(&mut c_arena as *mut Arena as *mut c_void);
        arr_ins_lib::stbds_strreset(&mut r_arena as *mut arr_ins_lib::stbds_string_arena);
    }
}

// ── Test: hash map round-trip (hmput_key + hmget_key) ─────────────────
#[test]
fn test_hmput_hmget_round_trip() {
    let c_lib = load_c_lib();
    unsafe {
        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> =
            c_lib.get(b"stbds_rand_seed").unwrap();
        let c_hmput_key: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c_lib.get(b"stbds_hmput_key").unwrap();
        let c_hmget_key: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c_lib.get(b"stbds_hmget_key").unwrap();
        let c_hmfree: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            c_lib.get(b"stbds_hmfree_func").unwrap();

        // Use a struct { int key; int value; }
        #[repr(C)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct KV {
            key: i32,
            value: i32,
        }
        #[repr(C)]
        struct Header {
            length: usize,
            capacity: usize,
            hash_table: *mut c_void,
            temp: isize,
        }

        let elemsize = std::mem::size_of::<KV>();
        let seed = 12345usize;

        // Test C side
        c_rand_seed(seed);
        let mut c_map: *mut c_void = std::ptr::null_mut();
        let keys: [i32; 5] = [10, 20, 30, 40, 50];
        for &k in &keys {
            let mut key_val = k;
            c_map = c_hmput_key(
                c_map,
                elemsize,
                &mut key_val as *mut i32 as *mut c_void,
                std::mem::size_of::<i32>(),
                0, // STBDS_HM_BINARY
            );
            // Set value at temp index
            let raw = (c_map as *mut u8).sub(elemsize);
            let hdr = &*(raw as *mut Header).offset(-1);
            let idx = hdr.temp;
            let entry = &mut *((c_map as *mut KV).offset(idx));
            entry.key = k;
            entry.value = k * 10;
        }

        // Test Rust side
        arr_ins_lib::stbds_rand_seed(seed);
        let mut r_map: *mut c_void = std::ptr::null_mut();
        for &k in &keys {
            let mut key_val = k;
            r_map = arr_ins_lib::stbds_hmput_key(
                r_map,
                elemsize,
                &mut key_val as *mut i32 as *mut c_void,
                std::mem::size_of::<i32>(),
                0,
            );
            let raw = (r_map as *mut u8).sub(elemsize);
            let hdr = &*(raw as *mut Header).offset(-1);
            let idx = hdr.temp;
            let entry = &mut *((r_map as *mut KV).offset(idx));
            entry.key = k;
            entry.value = k * 10;
        }

        // Now get each key and compare temp indices
        for &k in &keys {
            let mut key_val = k;
            let c_result = c_hmget_key(
                c_map,
                elemsize,
                &mut key_val as *mut i32 as *mut c_void,
                std::mem::size_of::<i32>(),
                0,
            );
            let c_raw = (c_result as *mut u8).sub(elemsize);
            let c_hdr = &*(c_raw as *mut Header).offset(-1);
            let c_idx = c_hdr.temp;

            let mut key_val2 = k;
            let r_result = arr_ins_lib::stbds_hmget_key(
                r_map,
                elemsize,
                &mut key_val2 as *mut i32 as *mut c_void,
                std::mem::size_of::<i32>(),
                0,
            );
            let r_raw = (r_result as *mut u8).sub(elemsize);
            let r_hdr = &*(r_raw as *mut Header).offset(-1);
            let r_idx = r_hdr.temp;

            // Both should find the key (idx >= 0)
            assert!(c_idx >= 0, "C didn't find key {}", k);
            assert!(r_idx >= 0, "Rust didn't find key {}", k);

            // Compare the stored values
            let c_entry = &*((c_result as *mut KV).offset(c_idx));
            let r_entry = &*((r_result as *mut KV).offset(r_idx));
            assert_eq!(
                c_entry.value, r_entry.value,
                "hmget value mismatch for key {}: C={} Rust={}",
                k, c_entry.value, r_entry.value
            );
        }

        // Cleanup
        c_hmfree((c_map as *mut KV).offset(-1) as *mut c_void, elemsize);
        arr_ins_lib::stbds_hmfree_func(
            (r_map as *mut KV).offset(-1) as *mut c_void,
            elemsize,
        );
    }
}
