use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/<profile>/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libhelxo_lib.so");
    p
}

// ============================================================
// 1. stbds_hash_string
// ============================================================
#[test]
fn test_hash_string() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            r_lib.get(b"stbds_hash_string").unwrap();

        let test_strings = ["", "hello", "world", "test_123", "a", "abcdefghijklmnop"];
        let seeds = [0usize, 1, 42, 0x31415926, usize::MAX];

        for s in &test_strings {
            let cs = CString::new(*s).unwrap();
            for &seed in &seeds {
                let c_result = c_fn(cs.as_ptr() as *mut c_char, seed);
                let r_result = r_fn(cs.as_ptr() as *mut c_char, seed);
                assert_eq!(c_result, r_result,
                    "hash_string mismatch for {:?} seed={}: C={:#x} Rust={:#x}",
                    s, seed, c_result, r_result);
            }
        }
    }
}

// ============================================================
// 2. stbds_hash_bytes
// ============================================================
#[test]
fn test_hash_bytes() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            c_lib.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            r_lib.get(b"stbds_hash_bytes").unwrap();

        let test_data: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            vec![0xFF; 16],
            (0..255u8).collect(),
        ];
        let seeds = [0usize, 1, 42, 0x31415926];

        for data in &test_data {
            for &seed in &seeds {
                let c_result = c_fn(data.as_ptr() as *mut c_void, data.len(), seed);
                let r_result = r_fn(data.as_ptr() as *mut c_void, data.len(), seed);
                assert_eq!(c_result, r_result,
                    "hash_bytes mismatch for len={} seed={}: C={:#x} Rust={:#x}",
                    data.len(), seed, c_result, r_result);
            }
        }
    }
}

// ============================================================
// 3. strkey
// ============================================================
#[test]
fn test_strkey() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> =
            c_lib.get(b"strkey").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> =
            r_lib.get(b"strkey").unwrap();

        for n in [0, 1, -1, 42, 1000, -999, i32::MAX, i32::MIN] {
            let c_result = CStr::from_ptr(c_fn(n)).to_bytes().to_vec();
            let r_result = CStr::from_ptr(r_fn(n)).to_bytes().to_vec();
            assert_eq!(c_result, r_result,
                "strkey mismatch for n={}: C={:?} Rust={:?}",
                n, String::from_utf8_lossy(&c_result), String::from_utf8_lossy(&r_result));
        }
    }
}

// ============================================================
// 4. stbds_rand_seed — sets global state, verify hash outputs change consistently
// ============================================================
#[test]
fn test_rand_seed_affects_hash() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> =
            c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> =
            r_lib.get(b"stbds_rand_seed").unwrap();
        let c_hash: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();
        let r_hash: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            r_lib.get(b"stbds_hash_string").unwrap();

        // Set same seed, verify hash outputs match
        for seed_val in [0usize, 123, 0xDEADBEEF, usize::MAX] {
            c_seed(seed_val);
            r_seed(seed_val);
            let s = CString::new("test").unwrap();
            let c_result = c_hash(s.as_ptr() as *mut c_char, 99);
            let r_result = r_hash(s.as_ptr() as *mut c_char, 99);
            assert_eq!(c_result, r_result,
                "After rand_seed({}), hash mismatch: C={:#x} Rust={:#x}",
                seed_val, c_result, r_result);
        }
    }
}

// ============================================================
// Helper: stbds_array_header layout (matches C)
// ============================================================
#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

unsafe fn get_header(a: *mut c_void) -> *mut StbdsArrayHeader {
    (a as *mut StbdsArrayHeader).offset(-1)
}

// ============================================================
// 5. stbds_arrgrowf — test array allocation behavior
// ============================================================
#[test]
fn test_arrgrowf() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type ArrgrowfFn = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
        type ArrfreefFn = unsafe extern "C" fn(*mut c_void);

        let c_grow: Symbol<ArrgrowfFn> = c_lib.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<ArrgrowfFn> = r_lib.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<ArrfreefFn> = c_lib.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<ArrfreefFn> = r_lib.get(b"stbds_arrfreef").unwrap();

        let elemsize = 8usize; // like i64

        // Allocate from null
        let c_a = c_grow(std::ptr::null_mut(), elemsize, 0, 1);
        let r_a = r_grow(std::ptr::null_mut(), elemsize, 0, 1);

        let c_hdr = &*get_header(c_a);
        let r_hdr = &*get_header(r_a);
        assert_eq!(c_hdr.length, r_hdr.length, "length mismatch after initial grow");
        assert_eq!(c_hdr.capacity, r_hdr.capacity, "capacity mismatch after initial grow");
        assert_eq!(c_hdr.temp, r_hdr.temp, "temp mismatch after initial grow");
        assert!(c_hdr.hash_table.is_null() && r_hdr.hash_table.is_null());

        // Grow with addlen
        let c_b = c_grow(c_a, elemsize, 10, 0);
        let r_b = r_grow(r_a, elemsize, 10, 0);
        let c_hdr2 = &*get_header(c_b);
        let r_hdr2 = &*get_header(r_b);
        assert_eq!(c_hdr2.capacity, r_hdr2.capacity, "capacity mismatch after grow(10)");

        // Grow with min_cap
        let c_c = c_grow(c_b, elemsize, 0, 100);
        let r_c = r_grow(r_b, elemsize, 0, 100);
        let c_hdr3 = &*get_header(c_c);
        let r_hdr3 = &*get_header(r_c);
        assert_eq!(c_hdr3.capacity, r_hdr3.capacity, "capacity mismatch after grow(min_cap=100)");

        c_free(c_c);
        r_free(r_c);
    }
}

// ============================================================
// 6. Full hashmap workflow: hmput_key, hmget_key, hmdel_key, hmfree_func
//    Test with binary (int) keys
// ============================================================
#[test]
fn test_hashmap_binary_keys() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        // Reset seeds to same value so hash tables behave identically
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(0x12345678);
        r_seed(0x12345678);

        type HmputKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmgetKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmdelKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
        type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_put: Symbol<HmputKeyFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmputKeyFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<HmgetKeyFn> = c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<HmgetKeyFn> = r_lib.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<HmdelKeyFn> = c_lib.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<HmdelKeyFn> = r_lib.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<HmfreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmfreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        // Element: struct { i32 key; i32 value; } — 8 bytes, key at offset 0
        let elemsize = 8usize;
        let keysize = 4usize;
        let mode = 0i32; // STBDS_HM_BINARY

        let mut c_hm: *mut c_void = std::ptr::null_mut();
        let mut r_hm: *mut c_void = std::ptr::null_mut();

        // Insert 20 entries
        for i in 0..20i32 {
            let mut key = i;
            c_hm = c_put(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
            let c_temp = (*get_header(c_raw)).temp;
            // Write value at key offset + 4
            *((c_hm as *mut u8).offset(c_temp as isize * elemsize as isize + 4) as *mut i32) = i * 100;

            key = i;
            r_hm = r_put(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
            let r_temp = (*get_header(r_raw)).temp;
            *((r_hm as *mut u8).offset(r_temp as isize * elemsize as isize + 4) as *mut i32) = i * 100;

            // Verify temp indices match
            assert_eq!(c_temp, r_temp, "temp mismatch after put key={}", i);
        }

        // Verify lengths match
        let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
        let c_len = (*get_header(c_raw)).length;
        let r_len = (*get_header(r_raw)).length;
        assert_eq!(c_len, r_len, "length mismatch after 20 puts");

        // Lookup all 20 entries
        for i in 0..20i32 {
            let mut key = i;
            c_hm = c_get(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
            let c_temp = (*get_header(c_raw)).temp;

            key = i;
            r_hm = r_get(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
            let r_temp = (*get_header(r_raw)).temp;

            assert_eq!(c_temp, r_temp, "get temp mismatch for key={}", i);
            assert!(c_temp >= 0, "key {} not found in C", i);

            // Compare stored values
            let c_val = *((c_hm as *mut u8).offset(c_temp as isize * elemsize as isize + 4) as *mut i32);
            let r_val = *((r_hm as *mut u8).offset(r_temp as isize * elemsize as isize + 4) as *mut i32);
            assert_eq!(c_val, r_val, "value mismatch for key={}: C={} Rust={}", i, c_val, r_val);
        }

        // Lookup non-existent key
        let mut key = 999i32;
        c_hm = c_get(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
        let c_temp = (*get_header((c_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
        key = 999;
        r_hm = r_get(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
        let r_temp = (*get_header((r_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
        assert_eq!(c_temp, r_temp, "non-existent key temp mismatch");
        assert_eq!(c_temp, -1, "non-existent key should return -1");

        // Delete some entries
        for i in [0, 5, 10, 15, 19] {
            let mut key = i as i32;
            c_hm = c_del(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0, mode);
            let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
            let c_temp = (*get_header(c_raw)).temp;

            key = i as i32;
            r_hm = r_del(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0, mode);
            let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
            let r_temp = (*get_header(r_raw)).temp;

            assert_eq!(c_temp, r_temp, "del temp mismatch for key={}", i);
        }

        // Verify length after deletes
        let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).length, (*get_header(r_raw)).length,
            "length mismatch after deletes");

        // Free
        c_free((c_hm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        r_free((r_hm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ============================================================
// 7. String hashmap workflow (shput/shget/shdel pattern)
// ============================================================
#[test]
fn test_hashmap_string_keys() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(0xABCDEF01);
        r_seed(0xABCDEF01);

        type HmputKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmgetKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmdelKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
        type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_put: Symbol<HmputKeyFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmputKeyFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<HmgetKeyFn> = c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<HmgetKeyFn> = r_lib.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<HmdelKeyFn> = c_lib.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<HmdelKeyFn> = r_lib.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<HmfreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmfreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        // Element: struct { char *key; char value; } — on 64-bit: 8 + 1 + 7 padding = 16 bytes
        // Actually in C: struct { char *key; char value; } is 8+1 = 9, padded to 16
        // Let's use the same layout as helxo: pointer + char
        // sizeof(char*) = 8, sizeof(char) = 1, struct padding to 16
        let elemsize = 16usize;
        let keysize = 8usize; // sizeof(char*)
        let mode = 1i32; // STBDS_HM_STRING

        let mut c_hm: *mut c_void = std::ptr::null_mut();
        let mut r_hm: *mut c_void = std::ptr::null_mut();

        let keys = ["bob", "sally", "fred", "jen", "doug", "alice", "charlie"];
        let values: Vec<u8> = vec![b'h', b'e', b'l', b'x', b'o', b'a', b'c'];

        // Insert entries
        for (idx, &k) in keys.iter().enumerate() {
            let cs = CString::new(k).unwrap();
            c_hm = c_put(c_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, mode);
            let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
            let c_temp = (*get_header(c_raw)).temp;
            *((c_hm as *mut u8).offset(c_temp as isize * elemsize as isize + 8) as *mut u8) = values[idx];

            r_hm = r_put(r_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, mode);
            let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
            let r_temp = (*get_header(r_raw)).temp;
            *((r_hm as *mut u8).offset(r_temp as isize * elemsize as isize + 8) as *mut u8) = values[idx];

            assert_eq!(c_temp, r_temp, "string put temp mismatch for key={}", k);
        }

        // Verify lengths
        let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).length, (*get_header(r_raw)).length,
            "string hm length mismatch");

        // Lookup all entries
        for (idx, &k) in keys.iter().enumerate() {
            let cs = CString::new(k).unwrap();
            c_hm = c_get(c_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, mode);
            let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
            let c_temp = (*get_header(c_raw)).temp;

            r_hm = r_get(r_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, mode);
            let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
            let r_temp = (*get_header(r_raw)).temp;

            assert_eq!(c_temp, r_temp, "string get temp mismatch for key={}", k);
            assert!(c_temp >= 0, "key {} not found", k);

            let c_val = *((c_hm as *mut u8).offset(c_temp as isize * elemsize as isize + 8) as *mut u8);
            let r_val = *((r_hm as *mut u8).offset(r_temp as isize * elemsize as isize + 8) as *mut u8);
            assert_eq!(c_val, r_val, "string value mismatch for key={}: C={} Rust={}", k, c_val, r_val);
        }

        // Lookup non-existent
        let cs = CString::new("nonexistent").unwrap();
        c_hm = c_get(c_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, mode);
        let c_temp = (*get_header((c_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
        r_hm = r_get(r_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, mode);
        let r_temp = (*get_header((r_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
        assert_eq!(c_temp, r_temp, "nonexistent string key temp mismatch");

        // Delete some
        for &k in &["bob", "fred", "doug"] {
            let cs = CString::new(k).unwrap();
            c_hm = c_del(c_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, 0, mode);
            let c_temp = (*get_header((c_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
            r_hm = r_del(r_hm, elemsize, cs.as_ptr() as *mut c_void, keysize, 0, mode);
            let r_temp = (*get_header((r_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
            assert_eq!(c_temp, r_temp, "string del temp mismatch for key={}", k);
        }

        // Verify length after deletes
        let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).length, (*get_header(r_raw)).length,
            "string hm length mismatch after deletes");

        // Free
        c_free((c_hm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        r_free((r_hm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ============================================================
// 8. stbds_hmput_default
// ============================================================
#[test]
fn test_hmput_default() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type HmputDefaultFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
        type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_fn: Symbol<HmputDefaultFn> = c_lib.get(b"stbds_hmput_default").unwrap();
        let r_fn: Symbol<HmputDefaultFn> = r_lib.get(b"stbds_hmput_default").unwrap();
        let c_free: Symbol<HmfreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmfreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize = 8usize;

        // Call with null — should allocate
        let c_a = c_fn(std::ptr::null_mut(), elemsize);
        let r_a = r_fn(std::ptr::null_mut(), elemsize);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).length, (*get_header(r_raw)).length,
            "hmput_default length mismatch");

        // Call again — should be no-op since length > 0
        let c_b = c_fn(c_a, elemsize);
        let r_b = r_fn(r_a, elemsize);
        let c_raw2 = (c_b as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw2 = (r_b as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw2)).length, (*get_header(r_raw2)).length,
            "hmput_default second call length mismatch");

        c_free(c_raw2, elemsize);
        r_free(r_raw2, elemsize);
    }
}

// ============================================================
// 9. stbds_hmget_key_ts
// ============================================================
#[test]
fn test_hmget_key_ts() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(0x99887766);
        r_seed(0x99887766);

        type HmputKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmgetKeyTsFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
        type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_put: Symbol<HmputKeyFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmputKeyFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_get_ts: Symbol<HmgetKeyTsFn> = c_lib.get(b"stbds_hmget_key_ts").unwrap();
        let r_get_ts: Symbol<HmgetKeyTsFn> = r_lib.get(b"stbds_hmget_key_ts").unwrap();
        let c_free: Symbol<HmfreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmfreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize = 8usize;
        let keysize = 4usize;
        let mode = 0i32;

        // Test with null — should allocate
        let mut c_temp: isize = 0;
        let mut r_temp: isize = 0;
        let mut key = 42i32;
        let c_a = c_get_ts(std::ptr::null_mut(), elemsize, &mut key as *mut i32 as *mut c_void, keysize, &mut c_temp, mode);
        key = 42;
        let r_a = r_get_ts(std::ptr::null_mut(), elemsize, &mut key as *mut i32 as *mut c_void, keysize, &mut r_temp, mode);
        assert_eq!(c_temp, r_temp, "hmget_key_ts null temp mismatch");
        assert_eq!(c_temp, -1, "hmget_key_ts null should return -1");

        // Put a key, then get_ts
        let mut c_hm = c_a;
        let mut r_hm = r_a;
        key = 42;
        c_hm = c_put(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
        key = 42;
        r_hm = r_put(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);

        key = 42;
        c_hm = c_get_ts(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, &mut c_temp, mode);
        key = 42;
        r_hm = r_get_ts(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, &mut r_temp, mode);
        assert_eq!(c_temp, r_temp, "hmget_key_ts found temp mismatch");
        assert!(c_temp >= 0, "key should be found");

        c_free((c_hm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        r_free((r_hm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ============================================================
// 10. stbds_shmode_func
// ============================================================
#[test]
fn test_shmode_func() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(0x55443322);
        r_seed(0x55443322);

        type ShmodeFn = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
        type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_fn: Symbol<ShmodeFn> = c_lib.get(b"stbds_shmode_func").unwrap();
        let r_fn: Symbol<ShmodeFn> = r_lib.get(b"stbds_shmode_func").unwrap();
        let c_free: Symbol<HmfreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmfreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize = 16usize;

        for mode in [2i32, 3i32] { // STBDS_SH_STRDUP=2, STBDS_SH_ARENA=3
            let c_a = c_fn(elemsize, mode);
            let r_a = r_fn(elemsize, mode);

            let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
            let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;

            assert_eq!((*get_header(c_raw)).length, (*get_header(r_raw)).length,
                "shmode_func length mismatch for mode={}", mode);

            c_free(c_raw, elemsize);
            r_free(r_raw, elemsize);
        }
    }
}

// ============================================================
// 11. stbds_stralloc / stbds_strreset
// ============================================================
#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[test]
fn test_stralloc_strreset() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type StrallocFn = unsafe extern "C" fn(*mut StbdsStringArena, *mut c_char) -> *mut c_char;
        type StrresetFn = unsafe extern "C" fn(*mut StbdsStringArena);

        let c_alloc: Symbol<StrallocFn> = c_lib.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<StrallocFn> = r_lib.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<StrresetFn> = c_lib.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<StrresetFn> = r_lib.get(b"stbds_strreset").unwrap();

        let mut c_arena: StbdsStringArena = std::mem::zeroed();
        let mut r_arena: StbdsStringArena = std::mem::zeroed();

        let test_strings = ["hello", "world", "a", "longer string for testing", "x"];

        for s in &test_strings {
            let cs = CString::new(*s).unwrap();
            let c_result = c_alloc(&mut c_arena, cs.as_ptr() as *mut c_char);
            let r_result = r_alloc(&mut r_arena, cs.as_ptr() as *mut c_char);

            // Compare the stored string content
            let c_str = CStr::from_ptr(c_result).to_bytes();
            let r_str = CStr::from_ptr(r_result).to_bytes();
            assert_eq!(c_str, r_str, "stralloc content mismatch for {:?}", s);

            // Compare arena state
            assert_eq!(c_arena.remaining, r_arena.remaining,
                "arena remaining mismatch after {:?}", s);
            assert_eq!(c_arena.block, r_arena.block,
                "arena block mismatch after {:?}", s);
        }

        // Allocate a very large string to trigger different block path
        let big = "A".repeat(600);
        let cs = CString::new(big.as_str()).unwrap();
        let c_result = c_alloc(&mut c_arena, cs.as_ptr() as *mut c_char);
        let r_result = r_alloc(&mut r_arena, cs.as_ptr() as *mut c_char);
        let c_str = CStr::from_ptr(c_result).to_bytes();
        let r_str = CStr::from_ptr(r_result).to_bytes();
        assert_eq!(c_str, r_str, "stralloc big string content mismatch");
        assert_eq!(c_arena.remaining, r_arena.remaining, "arena remaining mismatch after big");

        c_reset(&mut c_arena);
        r_reset(&mut r_arena);

        // After reset, arenas should be zeroed
        assert_eq!(c_arena.remaining, r_arena.remaining, "remaining mismatch after reset");
        assert!(c_arena.storage.is_null() && r_arena.storage.is_null(), "storage not null after reset");
    }
}

// ============================================================
// 12. helxo — compare stdout output
// ============================================================
#[test]
fn test_helxo() {
    use std::process::Command;

    // Write a small C program that calls helxo from the C .so
    let dir = std::env::temp_dir().join("helxo_test");
    std::fs::create_dir_all(&dir).unwrap();

    let c_so = c_lib_path();
    let r_so = rust_lib_path();

    // Use a helper program that dlopen's and calls helxo
    let helper_src = format!(r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
int main(int argc, char **argv) {{
    void *lib = dlopen(argv[1], RTLD_NOW);
    if (!lib) {{ fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }}
    void (*fn)(char) = dlsym(lib, "helxo");
    if (!fn) {{ fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }}
    fn(argv[2][0]);
    dlclose(lib);
    return 0;
}}
"#);

    let helper_c = dir.join("helper.c");
    let helper_bin = dir.join("helper");
    std::fs::write(&helper_c, helper_src).unwrap();

    let status = Command::new("gcc")
        .args([helper_c.to_str().unwrap(), "-o", helper_bin.to_str().unwrap(), "-ldl"])
        .status().unwrap();
    assert!(status.success(), "Failed to compile helper");

    for letter in [b'Z', b'!', b'A'] {
        let c_out = Command::new(helper_bin.to_str().unwrap())
            .args([c_so.to_str().unwrap(), &String::from(letter as char)])
            .output().unwrap();
        let r_out = Command::new(helper_bin.to_str().unwrap())
            .args([r_so.to_str().unwrap(), &String::from(letter as char)])
            .output().unwrap();

        assert_eq!(c_out.stdout, r_out.stdout,
            "helxo output mismatch for letter='{}'\nC:    {:?}\nRust: {:?}",
            letter as char,
            String::from_utf8_lossy(&c_out.stdout),
            String::from_utf8_lossy(&r_out.stdout));
    }
}

// ============================================================
// 13. Stress test: many inserts + deletes + re-inserts
// ============================================================
#[test]
fn test_hashmap_stress() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(0xFEDCBA98);
        r_seed(0xFEDCBA98);

        type HmputKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmgetKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmdelKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
        type HmfreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_put: Symbol<HmputKeyFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmputKeyFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<HmgetKeyFn> = c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<HmgetKeyFn> = r_lib.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<HmdelKeyFn> = c_lib.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<HmdelKeyFn> = r_lib.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<HmfreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmfreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize = 8usize;
        let keysize = 4usize;
        let mode = 0i32;

        let mut c_hm: *mut c_void = std::ptr::null_mut();
        let mut r_hm: *mut c_void = std::ptr::null_mut();

        // Insert 100 entries
        for i in 0..100i32 {
            let mut key = i;
            c_hm = c_put(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            key = i;
            r_hm = r_put(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
        }

        // Delete even entries
        for i in (0..100i32).step_by(2) {
            let mut key = i;
            c_hm = c_del(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0, mode);
            key = i;
            r_hm = r_del(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0, mode);
        }

        // Re-insert deleted entries with different values
        for i in (0..100i32).step_by(2) {
            let mut key = i;
            c_hm = c_put(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            key = i;
            r_hm = r_put(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
        }

        // Verify all 100 entries exist
        for i in 0..100i32 {
            let mut key = i;
            c_hm = c_get(c_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            let c_temp = (*get_header((c_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
            key = i;
            r_hm = r_get(r_hm, elemsize, &mut key as *mut i32 as *mut c_void, keysize, mode);
            let r_temp = (*get_header((r_hm as *mut u8).sub(elemsize) as *mut c_void)).temp;
            assert_eq!(c_temp, r_temp, "stress get temp mismatch for key={}", i);
            assert!(c_temp >= 0, "stress: key {} not found", i);
        }

        // Verify lengths
        let c_raw = (c_hm as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_hm as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).length, (*get_header(r_raw)).length,
            "stress length mismatch");

        c_free(c_raw, elemsize);
        r_free(r_raw, elemsize);
    }
}
