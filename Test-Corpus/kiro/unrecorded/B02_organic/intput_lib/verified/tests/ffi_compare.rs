use libloading::{Library, Symbol};
use std::ffi::CString;
use std::ptr;

const HDR_SIZE: usize = std::mem::size_of::<StbdsArrayHeader>();

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut u8,
    temp: isize,
}

#[repr(C)]
struct StbdsStringArena {
    storage: *mut u8,
    remaining: usize,
    block: u8,
    mode: u8,
}

unsafe fn hdr(t: *mut u8) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).offset(-1)
}

fn c_lib_path() -> String {
    std::env::current_dir()
        .unwrap()
        .join("c_src/build/libtranslated_rust.so")
        .to_str()
        .unwrap()
        .to_string()
}

fn rust_lib_path() -> String {
    // Find the Rust .so in target/debug
    let base = std::env::current_dir().unwrap().join("target/debug");
    for entry in std::fs::read_dir(&base).unwrap() {
        let p = entry.unwrap().path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("libintput_lib") && name.ends_with(".so") && !name.contains("deps") {
                return p.to_str().unwrap().to_string();
            }
        }
    }
    base.join("libintput_lib.so").to_str().unwrap().to_string()
}

#[test]
fn test_hash_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_hash_string: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();
        let r_hash_string: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            r_lib.get(b"stbds_hash_string").unwrap();

        // Both libs need same seed state - set seed first
        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> =
            c_lib.get(b"stbds_rand_seed").unwrap();
        let r_rand_seed: Symbol<unsafe extern "C" fn(usize)> =
            r_lib.get(b"stbds_rand_seed").unwrap();

        for seed_val in [0x31415926_usize, 0, 1, 0xDEADBEEF, usize::MAX] {
            c_rand_seed(seed_val);
            r_rand_seed(seed_val);

            let test_strings = ["", "hello", "test_42", "a", "abcdefghijklmnop", "stb_ds"];
            for s in &test_strings {
                let cs = CString::new(*s).unwrap();
                let c_result = c_hash_string(cs.as_ptr() as *mut u8, seed_val);
                let r_result = r_hash_string(cs.as_ptr() as *mut u8, seed_val);
                assert_eq!(
                    c_result, r_result,
                    "hash_string mismatch for {:?} seed={:#x}: C={:#x} Rust={:#x}",
                    s, seed_val, c_result, r_result
                );
            }
        }
    }
}

#[test]
fn test_hash_bytes() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_hash_bytes: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
            c_lib.get(b"stbds_hash_bytes").unwrap();
        let r_hash_bytes: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
            r_lib.get(b"stbds_hash_bytes").unwrap();

        let seeds = [0_usize, 1, 0x31415926, 0xDEADBEEF];
        // Test various lengths including edge cases for siphash
        let test_data: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![1, 2, 3],
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4, 5],
            vec![1, 2, 3, 4, 5, 6],
            vec![1, 2, 3, 4, 5, 6, 7],
            vec![0, 1, 2, 3, 4, 5, 6, 7],           // exactly 8 bytes
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8],         // 9 bytes
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15], // 16 bytes
            (0..17).collect(),                         // 17 bytes
            (0..32).collect(),                         // 32 bytes
        ];

        for seed in &seeds {
            for data in &test_data {
                let mut buf = data.clone();
                let c_result = c_hash_bytes(buf.as_mut_ptr(), buf.len(), *seed);
                let r_result = r_hash_bytes(buf.as_mut_ptr(), buf.len(), *seed);
                assert_eq!(
                    c_result, r_result,
                    "hash_bytes mismatch for len={} seed={:#x}: C={:#x} Rust={:#x}",
                    buf.len(), seed, c_result, r_result
                );
            }
        }
    }
}

#[test]
fn test_strkey() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_strkey: Symbol<unsafe extern "C" fn(i32) -> *mut u8> =
            c_lib.get(b"strkey").unwrap();
        let r_strkey: Symbol<unsafe extern "C" fn(i32) -> *mut u8> =
            r_lib.get(b"strkey").unwrap();

        for n in [0, 1, -1, 42, 1000, i32::MAX, i32::MIN] {
            let c_ptr = c_strkey(n);
            let r_ptr = r_strkey(n);
            let c_str = std::ffi::CStr::from_ptr(c_ptr as *const i8);
            let r_str = std::ffi::CStr::from_ptr(r_ptr as *const i8);
            assert_eq!(
                c_str, r_str,
                "strkey mismatch for n={}: C={:?} Rust={:?}",
                n, c_str, r_str
            );
        }
    }
}

#[test]
fn test_arrgrowf_and_arrfreef() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_arrgrowf: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> =
            c_lib.get(b"stbds_arrgrowf").unwrap();
        let r_arrgrowf: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> =
            r_lib.get(b"stbds_arrgrowf").unwrap();
        let c_arrfreef: Symbol<unsafe extern "C" fn(*mut u8)> =
            c_lib.get(b"stbds_arrfreef").unwrap();
        let r_arrfreef: Symbol<unsafe extern "C" fn(*mut u8)> =
            r_lib.get(b"stbds_arrfreef").unwrap();

        // Test: allocate from null, check header fields
        let elemsize = 4_usize; // int-sized elements
        let c_arr = c_arrgrowf(ptr::null_mut(), elemsize, 0, 10);
        let r_arr = r_arrgrowf(ptr::null_mut(), elemsize, 0, 10);

        let c_hdr = hdr(c_arr);
        let r_hdr = hdr(r_arr);

        assert_eq!((*c_hdr).length, (*r_hdr).length, "length mismatch after initial grow");
        assert_eq!((*c_hdr).capacity, (*r_hdr).capacity, "capacity mismatch after initial grow");
        assert_eq!((*c_hdr).length, 0);
        assert!((*c_hdr).capacity >= 10);

        // Test: grow existing array
        (*c_hdr).length = 5;
        (*r_hdr).length = 5;
        let c_arr2 = c_arrgrowf(c_arr, elemsize, 10, 0);
        let r_arr2 = r_arrgrowf(r_arr, elemsize, 10, 0);
        let c_hdr2 = hdr(c_arr2);
        let r_hdr2 = hdr(r_arr2);
        assert_eq!((*c_hdr2).length, (*r_hdr2).length, "length mismatch after second grow");
        assert_eq!((*c_hdr2).capacity, (*r_hdr2).capacity, "capacity mismatch after second grow");

        // Test: grow with min_cap < 4 (should clamp to 4)
        let c_small = c_arrgrowf(ptr::null_mut(), elemsize, 1, 0);
        let r_small = r_arrgrowf(ptr::null_mut(), elemsize, 1, 0);
        assert_eq!((*hdr(c_small)).capacity, (*hdr(r_small)).capacity, "small alloc capacity mismatch");

        // Free all
        c_arrfreef(c_arr2);
        r_arrfreef(r_arr2);
        c_arrfreef(c_small);
        r_arrfreef(r_small);
    }
}

#[test]
fn test_intput() {
    // intput is a high-level test that exercises hmput/hmget internally
    // If it doesn't crash/assert, both implementations agree
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_intput: Symbol<unsafe extern "C" fn(i32)> = c_lib.get(b"intput").unwrap();
        let r_intput: Symbol<unsafe extern "C" fn(i32)> = r_lib.get(b"intput").unwrap();

        // Avoid 9 and 11 since intput uses those as keys internally,
        // causing C assertions to fail when num collides with them
        for num in [0, 1, -1, 42, 100, i32::MAX, i32::MIN] {
            c_intput(num);
            r_intput(num);
        }
    }
}

// Helper: reset both libs to same seed state
unsafe fn reset_seeds(
    c_seed_fn: &Symbol<unsafe extern "C" fn(usize)>,
    r_seed_fn: &Symbol<unsafe extern "C" fn(usize)>,
    seed: usize,
) {
    c_seed_fn(seed);
    r_seed_fn(seed);
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct IntKV {
    key: i32,
    value: i32,
}

const INTKV_SIZE: usize = std::mem::size_of::<IntKV>();

/// Test hmput_key + hmget_key for binary (int) keys, comparing C and Rust behavior
#[test]
fn test_hmput_hmget_binary() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_rand_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        let c_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmput_key").unwrap();
        let r_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmput_key").unwrap();
        let c_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmget_key").unwrap();
        let r_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmget_key").unwrap();
        let c_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r_lib.get(b"stbds_hmfree_func").unwrap();

        // Use same seed so hash tables have identical internal state
        reset_seeds(&c_rand_seed, &r_rand_seed, 0x12345678);

        let mut c_map: *mut u8 = ptr::null_mut();
        let mut r_map: *mut u8 = ptr::null_mut();

        // Insert several key-value pairs
        let pairs: Vec<(i32, i32)> = vec![(1, 10), (2, 20), (3, 30), (100, 999), (-5, 50)];
        for &(k, v) in &pairs {
            let mut key = k;
            c_map = c_hmput_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let c_idx = (*hdr(c_map.sub(INTKV_SIZE))).temp;
            let c_entry = c_map.add(INTKV_SIZE * c_idx as usize) as *mut IntKV;
            (*c_entry).key = k;
            (*c_entry).value = v;

            key = k;
            r_map = r_hmput_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let r_idx = (*hdr(r_map.sub(INTKV_SIZE))).temp;
            let r_entry = r_map.add(INTKV_SIZE * r_idx as usize) as *mut IntKV;
            (*r_entry).key = k;
            (*r_entry).value = v;
        }

        // Now get each key and compare
        for &(k, expected_v) in &pairs {
            let mut key = k;
            c_map = c_hmget_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let c_idx = (*hdr(c_map.sub(INTKV_SIZE))).temp;

            key = k;
            r_map = r_hmget_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let r_idx = (*hdr(r_map.sub(INTKV_SIZE))).temp;

            assert!(c_idx >= 0, "C: key {} not found", k);
            assert!(r_idx >= 0, "Rust: key {} not found", k);

            let c_entry = *(c_map.add(INTKV_SIZE * c_idx as usize) as *const IntKV);
            let r_entry = *(r_map.add(INTKV_SIZE * r_idx as usize) as *const IntKV);

            assert_eq!(c_entry.key, r_entry.key, "key mismatch for k={}", k);
            assert_eq!(c_entry.value, r_entry.value, "value mismatch for k={}", k);
            assert_eq!(c_entry.value, expected_v, "C value wrong for k={}", k);
        }

        // Get a key that doesn't exist
        let mut missing_key: i32 = 9999;
        c_map = c_hmget_key(c_map, INTKV_SIZE, &mut missing_key as *mut i32 as *mut u8, 4, 0);
        let c_miss = (*hdr(c_map.sub(INTKV_SIZE))).temp;
        r_map = r_hmget_key(r_map, INTKV_SIZE, &mut missing_key as *mut i32 as *mut u8, 4, 0);
        let r_miss = (*hdr(r_map.sub(INTKV_SIZE))).temp;
        assert_eq!(c_miss, r_miss, "missing key temp mismatch: C={} Rust={}", c_miss, r_miss);

        // Free
        c_hmfree(c_map.sub(INTKV_SIZE), INTKV_SIZE);
        r_hmfree(r_map.sub(INTKV_SIZE), INTKV_SIZE);
    }
}

/// Test hmdel_key for binary keys
#[test]
fn test_hmdel_binary() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_rand_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        let c_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmput_key").unwrap();
        let r_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmput_key").unwrap();
        let c_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmget_key").unwrap();
        let r_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmget_key").unwrap();
        let c_hmdel_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmdel_key").unwrap();
        let r_hmdel_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmdel_key").unwrap();
        let c_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r_lib.get(b"stbds_hmfree_func").unwrap();

        reset_seeds(&c_rand_seed, &r_rand_seed, 0xABCDEF01);

        let mut c_map: *mut u8 = ptr::null_mut();
        let mut r_map: *mut u8 = ptr::null_mut();

        // Insert keys 0..10
        for i in 0..10_i32 {
            let mut key = i;
            c_map = c_hmput_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let idx = (*hdr(c_map.sub(INTKV_SIZE))).temp;
            let e = c_map.add(INTKV_SIZE * idx as usize) as *mut IntKV;
            (*e).key = i;
            (*e).value = i * 10;

            key = i;
            r_map = r_hmput_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let idx = (*hdr(r_map.sub(INTKV_SIZE))).temp;
            let e = r_map.add(INTKV_SIZE * idx as usize) as *mut IntKV;
            (*e).key = i;
            (*e).value = i * 10;
        }

        // Delete key 5
        let mut key: i32 = 5;
        // keyoffset for IntKV.key is 0
        c_map = c_hmdel_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0, 0);
        let c_del_temp = (*hdr(c_map.sub(INTKV_SIZE))).temp;
        key = 5;
        r_map = r_hmdel_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0, 0);
        let r_del_temp = (*hdr(r_map.sub(INTKV_SIZE))).temp;
        assert_eq!(c_del_temp, r_del_temp, "delete temp mismatch for key 5");
        assert_eq!(c_del_temp, 1, "expected temp=1 after successful delete");

        // Verify key 5 is gone
        key = 5;
        c_map = c_hmget_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
        let c_idx = (*hdr(c_map.sub(INTKV_SIZE))).temp;
        key = 5;
        r_map = r_hmget_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
        let r_idx = (*hdr(r_map.sub(INTKV_SIZE))).temp;
        assert_eq!(c_idx, -1, "C: key 5 should be deleted");
        assert_eq!(r_idx, -1, "Rust: key 5 should be deleted");

        // Verify other keys still present
        for i in [0, 3, 7, 9] {
            key = i;
            c_map = c_hmget_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let c_idx = (*hdr(c_map.sub(INTKV_SIZE))).temp;
            key = i;
            r_map = r_hmget_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let r_idx = (*hdr(r_map.sub(INTKV_SIZE))).temp;
            assert!(c_idx >= 0, "C: key {} should exist", i);
            assert!(r_idx >= 0, "Rust: key {} should exist", i);
            let c_val = (*(c_map.add(INTKV_SIZE * c_idx as usize) as *const IntKV)).value;
            let r_val = (*(r_map.add(INTKV_SIZE * r_idx as usize) as *const IntKV)).value;
            assert_eq!(c_val, r_val, "value mismatch after delete for key {}", i);
        }

        // Delete non-existent key
        key = 999;
        c_map = c_hmdel_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0, 0);
        let c_t = (*hdr(c_map.sub(INTKV_SIZE))).temp;
        key = 999;
        r_map = r_hmdel_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0, 0);
        let r_t = (*hdr(r_map.sub(INTKV_SIZE))).temp;
        assert_eq!(c_t, r_t, "delete non-existent key temp mismatch");

        c_hmfree(c_map.sub(INTKV_SIZE), INTKV_SIZE);
        r_hmfree(r_map.sub(INTKV_SIZE), INTKV_SIZE);
    }
}

/// String-keyed hash map entry: { char* key; int value; }
#[repr(C)]
#[derive(Clone, Copy)]
struct StrKV {
    key: *mut u8,
    value: i32,
}

const STRKV_SIZE: usize = std::mem::size_of::<StrKV>();

/// Test shmode_func + hmput_key/hmget_key with string keys (mode=1)
#[test]
fn test_string_hashmap() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_rand_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        let c_shmode: Symbol<unsafe extern "C" fn(usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_shmode_func").unwrap();
        let r_shmode: Symbol<unsafe extern "C" fn(usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_shmode_func").unwrap();
        let c_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmput_key").unwrap();
        let r_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmput_key").unwrap();
        let c_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmget_key").unwrap();
        let r_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmget_key").unwrap();
        let c_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r_lib.get(b"stbds_hmfree_func").unwrap();

        reset_seeds(&c_rand_seed, &r_rand_seed, 0x55555555);

        // STBDS_SH_STRDUP = 2
        let mut c_map = c_shmode(STRKV_SIZE, 2);
        let mut r_map = r_shmode(STRKV_SIZE, 2);

        // Insert string keys
        let keys_vals: Vec<(&str, i32)> = vec![
            ("hello", 1), ("world", 2), ("foo", 3), ("bar", 4), ("test_42", 5),
        ];
        let cstrings: Vec<CString> = keys_vals.iter().map(|(k, _)| CString::new(*k).unwrap()).collect();

        for (i, &(_, v)) in keys_vals.iter().enumerate() {
            // mode=1 = STBDS_HM_STRING, keysize = sizeof(char*) = 8
            c_map = c_hmput_key(c_map, STRKV_SIZE, cstrings[i].as_ptr() as *mut u8, 8, 1);
            let c_idx = (*hdr(c_map.sub(STRKV_SIZE))).temp;
            let c_entry = c_map.add(STRKV_SIZE * c_idx as usize) as *mut StrKV;
            (*c_entry).value = v;

            r_map = r_hmput_key(r_map, STRKV_SIZE, cstrings[i].as_ptr() as *mut u8, 8, 1);
            let r_idx = (*hdr(r_map.sub(STRKV_SIZE))).temp;
            let r_entry = r_map.add(STRKV_SIZE * r_idx as usize) as *mut StrKV;
            (*r_entry).value = v;
        }

        // Look up each key
        for (i, &(k, expected_v)) in keys_vals.iter().enumerate() {
            c_map = c_hmget_key(c_map, STRKV_SIZE, cstrings[i].as_ptr() as *mut u8, 8, 1);
            let c_idx = (*hdr(c_map.sub(STRKV_SIZE))).temp;
            r_map = r_hmget_key(r_map, STRKV_SIZE, cstrings[i].as_ptr() as *mut u8, 8, 1);
            let r_idx = (*hdr(r_map.sub(STRKV_SIZE))).temp;

            assert!(c_idx >= 0, "C: string key {:?} not found", k);
            assert!(r_idx >= 0, "Rust: string key {:?} not found", k);

            let c_val = (*(c_map.add(STRKV_SIZE * c_idx as usize) as *const StrKV)).value;
            let r_val = (*(r_map.add(STRKV_SIZE * r_idx as usize) as *const StrKV)).value;
            assert_eq!(c_val, expected_v, "C value wrong for key {:?}", k);
            assert_eq!(r_val, expected_v, "Rust value wrong for key {:?}", k);
        }

        // Look up missing key
        let missing = CString::new("nonexistent").unwrap();
        c_map = c_hmget_key(c_map, STRKV_SIZE, missing.as_ptr() as *mut u8, 8, 1);
        let c_miss = (*hdr(c_map.sub(STRKV_SIZE))).temp;
        r_map = r_hmget_key(r_map, STRKV_SIZE, missing.as_ptr() as *mut u8, 8, 1);
        let r_miss = (*hdr(r_map.sub(STRKV_SIZE))).temp;
        assert_eq!(c_miss, r_miss, "string missing key temp mismatch");
        assert_eq!(c_miss, -1);

        c_hmfree(c_map.sub(STRKV_SIZE), STRKV_SIZE);
        r_hmfree(r_map.sub(STRKV_SIZE), STRKV_SIZE);
    }
}

/// Test hmput_default
#[test]
fn test_hmput_default() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_hmput_default: Symbol<unsafe extern "C" fn(*mut u8, usize) -> *mut u8> =
            c_lib.get(b"stbds_hmput_default").unwrap();
        let r_hmput_default: Symbol<unsafe extern "C" fn(*mut u8, usize) -> *mut u8> =
            r_lib.get(b"stbds_hmput_default").unwrap();
        let c_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r_lib.get(b"stbds_hmfree_func").unwrap();

        // Call with null - should allocate and return pointer to element[0] area
        let c_map = c_hmput_default(ptr::null_mut(), INTKV_SIZE);
        let r_map = r_hmput_default(ptr::null_mut(), INTKV_SIZE);

        // Check that the header was initialized
        let c_hdr = hdr(c_map.sub(INTKV_SIZE));
        let r_hdr = hdr(r_map.sub(INTKV_SIZE));
        assert_eq!((*c_hdr).length, (*r_hdr).length, "hmput_default length mismatch");
        assert_eq!((*c_hdr).length, 1);

        // Calling again should be a no-op (length already > 0)
        let c_map2 = c_hmput_default(c_map, INTKV_SIZE);
        let r_map2 = r_hmput_default(r_map, INTKV_SIZE);
        assert_eq!((*hdr(c_map2.sub(INTKV_SIZE))).length, (*hdr(r_map2.sub(INTKV_SIZE))).length);

        c_hmfree(c_map2.sub(INTKV_SIZE), INTKV_SIZE);
        r_hmfree(r_map2.sub(INTKV_SIZE), INTKV_SIZE);
    }
}

/// Test hmget_key_ts
#[test]
fn test_hmget_key_ts() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_rand_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        let c_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmput_key").unwrap();
        let r_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmput_key").unwrap();
        let c_hmget_key_ts: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, *mut isize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmget_key_ts").unwrap();
        let r_hmget_key_ts: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, *mut isize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmget_key_ts").unwrap();
        let c_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r_lib.get(b"stbds_hmfree_func").unwrap();

        reset_seeds(&c_rand_seed, &r_rand_seed, 0x99887766);

        // Test with null - should initialize
        let mut c_temp: isize = 0;
        let mut r_temp: isize = 0;
        let mut key: i32 = 42;
        let c_map = c_hmget_key_ts(ptr::null_mut(), INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, &mut c_temp, 0);
        key = 42;
        let r_map = r_hmget_key_ts(ptr::null_mut(), INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, &mut r_temp, 0);
        assert_eq!(c_temp, r_temp, "hmget_key_ts null init temp mismatch");
        assert_eq!(c_temp, -1);

        // Insert a key, then look it up with _ts
        let mut c_map2 = c_map;
        let mut r_map2 = r_map;
        key = 42;
        c_map2 = c_hmput_key(c_map2, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
        let idx = (*hdr(c_map2.sub(INTKV_SIZE))).temp;
        let e = c_map2.add(INTKV_SIZE * idx as usize) as *mut IntKV;
        (*e).key = 42;
        (*e).value = 100;

        key = 42;
        r_map2 = r_hmput_key(r_map2, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
        let idx = (*hdr(r_map2.sub(INTKV_SIZE))).temp;
        let e = r_map2.add(INTKV_SIZE * idx as usize) as *mut IntKV;
        (*e).key = 42;
        (*e).value = 100;

        // Now get with _ts
        key = 42;
        c_map2 = c_hmget_key_ts(c_map2, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, &mut c_temp, 0);
        key = 42;
        r_map2 = r_hmget_key_ts(r_map2, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, &mut r_temp, 0);
        assert_eq!(c_temp, r_temp, "hmget_key_ts found temp mismatch");
        assert!(c_temp >= 0);

        let c_val = (*(c_map2.add(INTKV_SIZE * c_temp as usize) as *const IntKV)).value;
        let r_val = (*(r_map2.add(INTKV_SIZE * r_temp as usize) as *const IntKV)).value;
        assert_eq!(c_val, r_val);
        assert_eq!(c_val, 100);

        c_hmfree(c_map2.sub(INTKV_SIZE), INTKV_SIZE);
        r_hmfree(r_map2.sub(INTKV_SIZE), INTKV_SIZE);
    }
}

/// Stress test: insert many keys, delete some, verify all lookups match
#[test]
fn test_stress_insert_delete() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_rand_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_rand_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        let c_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmput_key").unwrap();
        let r_hmput_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmput_key").unwrap();
        let c_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmget_key").unwrap();
        let r_hmget_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmget_key").unwrap();
        let c_hmdel_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8> =
            c_lib.get(b"stbds_hmdel_key").unwrap();
        let r_hmdel_key: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8> =
            r_lib.get(b"stbds_hmdel_key").unwrap();
        let c_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_hmfree: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r_lib.get(b"stbds_hmfree_func").unwrap();

        reset_seeds(&c_rand_seed, &r_rand_seed, 0xFEDCBA98);

        let mut c_map: *mut u8 = ptr::null_mut();
        let mut r_map: *mut u8 = ptr::null_mut();

        let n = 100;
        // Insert 0..n
        for i in 0..n as i32 {
            let mut key = i;
            c_map = c_hmput_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let idx = (*hdr(c_map.sub(INTKV_SIZE))).temp;
            let e = c_map.add(INTKV_SIZE * idx as usize) as *mut IntKV;
            (*e).key = i;
            (*e).value = i * 3;

            key = i;
            r_map = r_hmput_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let idx = (*hdr(r_map.sub(INTKV_SIZE))).temp;
            let e = r_map.add(INTKV_SIZE * idx as usize) as *mut IntKV;
            (*e).key = i;
            (*e).value = i * 3;
        }

        // Delete even keys
        for i in (0..n as i32).step_by(2) {
            let mut key = i;
            c_map = c_hmdel_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0, 0);
            key = i;
            r_map = r_hmdel_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0, 0);
        }

        // Verify: even keys gone, odd keys present with correct values
        for i in 0..n as i32 {
            let mut key = i;
            c_map = c_hmget_key(c_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let c_idx = (*hdr(c_map.sub(INTKV_SIZE))).temp;
            key = i;
            r_map = r_hmget_key(r_map, INTKV_SIZE, &mut key as *mut i32 as *mut u8, 4, 0);
            let r_idx = (*hdr(r_map.sub(INTKV_SIZE))).temp;

            assert_eq!(c_idx, r_idx, "stress: index mismatch for key {}: C={} Rust={}", i, c_idx, r_idx);
            if i % 2 == 0 {
                assert_eq!(c_idx, -1, "stress: even key {} should be deleted", i);
            } else {
                assert!(c_idx >= 0, "stress: odd key {} should exist", i);
                let c_val = (*(c_map.add(INTKV_SIZE * c_idx as usize) as *const IntKV)).value;
                let r_val = (*(r_map.add(INTKV_SIZE * r_idx as usize) as *const IntKV)).value;
                assert_eq!(c_val, r_val, "stress: value mismatch for key {}", i);
            }
        }

        c_hmfree(c_map.sub(INTKV_SIZE), INTKV_SIZE);
        r_hmfree(r_map.sub(INTKV_SIZE), INTKV_SIZE);
    }
}

/// Test stralloc and strreset
#[test]
fn test_stralloc_strreset() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_stralloc: Symbol<unsafe extern "C" fn(*mut StbdsStringArena, *mut u8) -> *mut u8> =
            c_lib.get(b"stbds_stralloc").unwrap();
        let r_stralloc: Symbol<unsafe extern "C" fn(*mut StbdsStringArena, *mut u8) -> *mut u8> =
            r_lib.get(b"stbds_stralloc").unwrap();
        let c_strreset: Symbol<unsafe extern "C" fn(*mut StbdsStringArena)> =
            c_lib.get(b"stbds_strreset").unwrap();
        let r_strreset: Symbol<unsafe extern "C" fn(*mut StbdsStringArena)> =
            r_lib.get(b"stbds_strreset").unwrap();

        // Create zero-initialized arenas
        let mut c_arena: StbdsStringArena = std::mem::zeroed();
        let mut r_arena: StbdsStringArena = std::mem::zeroed();

        let test_strings = ["hello", "world", "a longer string for testing", "x", ""];
        for s in &test_strings {
            let cs = CString::new(*s).unwrap();
            let c_ptr = c_stralloc(&mut c_arena, cs.as_ptr() as *mut u8);
            let r_ptr = r_stralloc(&mut r_arena, cs.as_ptr() as *mut u8);

            // Compare the stored string content
            let c_stored = std::ffi::CStr::from_ptr(c_ptr as *const i8);
            let r_stored = std::ffi::CStr::from_ptr(r_ptr as *const i8);
            assert_eq!(c_stored, r_stored, "stralloc content mismatch for {:?}", s);

            // Arena state should match
            assert_eq!(c_arena.remaining, r_arena.remaining, "arena remaining mismatch after {:?}", s);
            assert_eq!(c_arena.block, r_arena.block, "arena block mismatch after {:?}", s);
        }

        // Test with a large string that exceeds blocksize
        let big = "A".repeat(1024);
        let cs_big = CString::new(big.as_str()).unwrap();
        let c_ptr = c_stralloc(&mut c_arena, cs_big.as_ptr() as *mut u8);
        let r_ptr = r_stralloc(&mut r_arena, cs_big.as_ptr() as *mut u8);
        let c_stored = std::ffi::CStr::from_ptr(c_ptr as *const i8);
        let r_stored = std::ffi::CStr::from_ptr(r_ptr as *const i8);
        assert_eq!(c_stored, r_stored, "stralloc big string mismatch");

        // Reset
        c_strreset(&mut c_arena);
        r_strreset(&mut r_arena);

        // After reset, arenas should be zeroed
        assert_eq!(c_arena.remaining, r_arena.remaining);
        assert_eq!(c_arena.block, r_arena.block);
        assert!(c_arena.storage.is_null());
        assert!(r_arena.storage.is_null());
    }
}
