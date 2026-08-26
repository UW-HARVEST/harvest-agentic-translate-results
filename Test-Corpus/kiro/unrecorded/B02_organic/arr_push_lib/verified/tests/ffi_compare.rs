use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::ptr;

fn rust_so_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libarr_push_lib.so", dir)
}

fn c_so_path() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libtranslated_rust.so", dir)
}

macro_rules! load_libs {
    () => {{
        let c = unsafe { Library::new(c_so_path()).expect("load C .so") };
        let r = unsafe { Library::new(rust_so_path()).expect("load Rust .so") };
        (c, r)
    }};
}

// ============================================================
// 1. stbds_rand_seed - just call it, no crash = pass
// ============================================================
#[test]
fn test_rand_seed() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        for seed in [0, 1, 42, 0xdeadbeef, usize::MAX] {
            c_fn(seed);
            r_fn(seed);
        }
    }
}

// ============================================================
// 2. strkey - compare output strings
// ============================================================
#[test]
fn test_strkey() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> *const u8> = c_lib.get(b"strkey").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32) -> *const u8> = r_lib.get(b"strkey").unwrap();
        for n in [0, 1, -1, 42, 1000, -999, i32::MAX, i32::MIN] {
            let c_str = CStr::from_ptr(c_fn(n) as *const i8);
            let r_str = CStr::from_ptr(r_fn(n) as *const i8);
            assert_eq!(c_str, r_str, "strkey({}) mismatch", n);
        }
    }
}

// ============================================================
// 3. stbds_hash_bytes - compare hash outputs
// ============================================================
#[test]
fn test_hash_bytes() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
            c_lib.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> =
            r_lib.get(b"stbds_hash_bytes").unwrap();

        let test_data: &[&[u8]] = &[
            b"", b"a", b"ab", b"abc", b"abcd", b"abcdefgh",
            b"hello world this is a longer string for testing",
            &[0u8; 64], &[0xFF; 7], &[1, 2, 3, 4, 5, 6, 7, 8, 9],
        ];
        let seeds = [0usize, 1, 42, 0x31415926, 0xdeadbeefcafebabe];

        for seed in seeds {
            for data in test_data {
                let mut buf = data.to_vec();
                let c_hash = c_fn(buf.as_mut_ptr(), buf.len(), seed);
                let r_hash = r_fn(buf.as_mut_ptr(), buf.len(), seed);
                assert_eq!(c_hash, r_hash,
                    "hash_bytes mismatch for data len={} seed={:#x}", buf.len(), seed);
            }
        }
    }
}

// ============================================================
// 4. stbds_hash_string - compare hash outputs
// ============================================================
#[test]
fn test_hash_string() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            c_lib.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> usize> =
            r_lib.get(b"stbds_hash_string").unwrap();

        let test_strings: &[&[u8]] = &[
            b"a\0", b"hello\0", b"test_42\0", b"\0",
            b"a longer string for testing hash\0",
            b"x\0", b"ab\0", b"abc\0",
        ];
        let seeds = [0usize, 1, 42, 0x31415926, 0xdeadbeefcafebabe];

        for seed in seeds {
            for s in test_strings {
                let mut buf = s.to_vec();
                let c_hash = c_fn(buf.as_mut_ptr(), seed);
                let r_hash = r_fn(buf.as_mut_ptr(), seed);
                assert_eq!(c_hash, r_hash,
                    "hash_string mismatch for {:?} seed={:#x}",
                    std::str::from_utf8(&buf[..buf.len()-1]).unwrap_or("?"), seed);
            }
        }
    }
}

// ============================================================
// 5. stbds_arrgrowf / stbds_arrfreef - array operations
// ============================================================
#[test]
fn test_arrgrowf() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        type GrowFn = unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8);

        let c_grow: Symbol<GrowFn> = c_lib.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<GrowFn> = r_lib.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"stbds_arrfreef").unwrap();

        // Header is 32 bytes: length(8) + capacity(8) + hash_table(8) + temp(8)
        let hdr_size = 32usize;

        // Test: grow from null
        let elemsize = 4usize; // int
        let c_arr = c_grow(ptr::null_mut(), elemsize, 0, 4);
        let r_arr = r_grow(ptr::null_mut(), elemsize, 0, 4);

        // Both should return non-null
        assert!(!c_arr.is_null(), "C arrgrowf returned null");
        assert!(!r_arr.is_null(), "Rust arrgrowf returned null");

        // Check header fields match
        let c_hdr = (c_arr as *mut usize).offset(-4);
        let r_hdr = (r_arr as *mut usize).offset(-4);

        // length should be 0
        assert_eq!(*c_hdr, *r_hdr, "length mismatch after grow from null");
        // capacity should be 4
        assert_eq!(*c_hdr.add(1), *r_hdr.add(1), "capacity mismatch after grow from null");
        assert_eq!(*r_hdr, 0, "length should be 0");
        assert_eq!(*r_hdr.add(1), 4, "capacity should be 4");

        // Test: grow again with addlen
        let c_arr2 = c_grow(c_arr, elemsize, 5, 0);
        let r_arr2 = r_grow(r_arr, elemsize, 5, 0);

        let c_hdr2 = (c_arr2 as *mut usize).offset(-4);
        let r_hdr2 = (r_arr2 as *mut usize).offset(-4);
        assert_eq!(*c_hdr2.add(1), *r_hdr2.add(1), "capacity mismatch after second grow");

        // Clean up
        c_free(c_arr2);
        r_free(r_arr2);
    }
}

// ============================================================
// 6. arr_push - the main public function
// ============================================================
#[test]
fn test_arr_push() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32)> = c_lib.get(b"arr_push").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32)> = r_lib.get(b"arr_push").unwrap();

        // Test with various values - arr_push just does internal work, no return value
        // If behavior differs, it would crash or assert-fail
        for num in [0, 1, 50, 100, 200, 500] {
            c_fn(num);
            r_fn(num);
        }
    }
}

// ============================================================
// 7. Hash map round-trip: put + get + del via hmput_key/hmget_key/hmdel_key
//    Uses binary (int) keys
// ============================================================
#[test]
fn test_hashmap_binary_roundtrip() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        // We need: stbds_hmput_key, stbds_hmget_key, stbds_hmdel_key, stbds_hmfree_func
        type PutFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type GetFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type DelFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_put: Symbol<PutFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<PutFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<GetFn> = c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<GetFn> = r_lib.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<DelFn> = c_lib.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<DelFn> = r_lib.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        // Use same seed for deterministic behavior
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(12345);
        r_seed(12345);

        // Element: struct { int key; int value; } => 8 bytes, keysize=4
        let elemsize = 8usize;
        let keysize = 4usize;

        let mut c_map: *mut u8 = ptr::null_mut();
        let mut r_map: *mut u8 = ptr::null_mut();

        // Insert several keys
        for i in 0i32..20 {
            let mut key = i;
            c_map = c_put(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            r_map = r_put(r_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);

            // Read back the header to check temp (index of inserted element)
            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, r_temp, "temp mismatch after put key={}", i);

            // Write value at the returned index
            let c_elem = c_map.add(elemsize * c_temp as usize);
            let r_elem = r_map.add(elemsize * r_temp as usize);
            *(c_elem as *mut i32) = i;
            *(c_elem.add(4) as *mut i32) = i * 10;
            *(r_elem as *mut i32) = i;
            *(r_elem.add(4) as *mut i32) = i * 10;
        }

        // Look up each key and compare temp index
        for i in 0i32..20 {
            let mut key = i;
            let c_res = c_get(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let r_res = r_get(r_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            c_map = c_res;
            r_map = r_res;

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, r_temp, "get temp mismatch for key={}", i);
            assert!(c_temp >= 0, "key {} not found in C map", i);

            // Compare stored values
            let c_val = *(c_map.add(elemsize * c_temp as usize + 4) as *mut i32);
            let r_val = *(r_map.add(elemsize * r_temp as usize + 4) as *mut i32);
            assert_eq!(c_val, r_val, "value mismatch for key={}", i);
        }

        // Look up a key that doesn't exist
        {
            let mut key = 999i32;
            let c_res = c_get(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let r_res = r_get(r_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            c_map = c_res;
            r_map = r_res;

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, r_temp, "missing key temp mismatch");
            assert_eq!(c_temp, -1, "missing key should return -1");
        }

        // Delete some keys
        for i in [0i32, 5, 10, 15] {
            let mut key = i;
            c_map = c_del(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0, 0);
            r_map = r_del(r_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0, 0);

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, r_temp, "del temp mismatch for key={}", i);
        }

        // Verify deleted keys are gone
        for i in [0i32, 5, 10, 15] {
            let mut key = i;
            c_map = c_get(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            r_map = r_get(r_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, -1, "C: deleted key {} still found", i);
            assert_eq!(r_temp, -1, "Rust: deleted key {} still found", i);
        }

        // Free
        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ============================================================
// 8. stbds_shmode_func + string hashmap roundtrip
// ============================================================
#[test]
fn test_string_hashmap_roundtrip() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        type ShModeFn = unsafe extern "C" fn(usize, i32) -> *mut u8;
        type PutFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type GetFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_shmode: Symbol<ShModeFn> = c_lib.get(b"stbds_shmode_func").unwrap();
        let r_shmode: Symbol<ShModeFn> = r_lib.get(b"stbds_shmode_func").unwrap();
        let c_put: Symbol<PutFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<PutFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<GetFn> = c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<GetFn> = r_lib.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(54321);
        r_seed(54321);

        // Element: struct { char *key; int value; } on 64-bit: 8 + 4 + 4(pad) = 16 bytes
        // keysize = sizeof(char*) = 8
        let elemsize = 16usize;
        let keysize = 8usize;

        // STBDS_SH_STRDUP = 2
        let mut c_map = c_shmode(elemsize, 2);
        let mut r_map = r_shmode(elemsize, 2);

        // Insert string keys
        let keys: Vec<Vec<u8>> = (0..10).map(|i| format!("key_{}\0", i).into_bytes()).collect();
        for (i, key) in keys.iter().enumerate() {
            c_map = c_put(c_map, elemsize, key.as_ptr() as *mut u8, keysize, 1); // STBDS_HM_STRING=1
            r_map = r_put(r_map, elemsize, key.as_ptr() as *mut u8, keysize, 1);

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, r_temp, "shput temp mismatch for key_{}", i);

            // Write value
            *(c_map.add(elemsize * c_temp as usize + 8) as *mut i32) = i as i32 * 100;
            *(r_map.add(elemsize * r_temp as usize + 8) as *mut i32) = i as i32 * 100;
        }

        // Look up each key
        for (i, key) in keys.iter().enumerate() {
            c_map = c_get(c_map, elemsize, key.as_ptr() as *mut u8, keysize, 1);
            r_map = r_get(r_map, elemsize, key.as_ptr() as *mut u8, keysize, 1);

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, r_temp, "shget temp mismatch for key_{}", i);
            assert!(c_temp >= 0, "key_{} not found", i);

            let c_val = *(c_map.add(elemsize * c_temp as usize + 8) as *mut i32);
            let r_val = *(r_map.add(elemsize * r_temp as usize + 8) as *mut i32);
            assert_eq!(c_val, r_val, "shget value mismatch for key_{}", i);
        }

        // Look up missing key
        {
            let missing = b"nonexistent\0";
            c_map = c_get(c_map, elemsize, missing.as_ptr() as *mut u8, keysize, 1);
            r_map = r_get(r_map, elemsize, missing.as_ptr() as *mut u8, keysize, 1);

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut usize).offset(-4).add(3) as *mut isize);
            let r_temp = *((r_raw as *mut usize).offset(-4).add(3) as *mut isize);
            assert_eq!(c_temp, -1, "C: missing key should be -1");
            assert_eq!(r_temp, -1, "Rust: missing key should be -1");
        }

        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ============================================================
// 9. stbds_hmput_default
// ============================================================
#[test]
fn test_hmput_default() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        type DefaultFn = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_fn: Symbol<DefaultFn> = c_lib.get(b"stbds_hmput_default").unwrap();
        let r_fn: Symbol<DefaultFn> = r_lib.get(b"stbds_hmput_default").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize = 8usize;

        // From null
        let c_res = c_fn(ptr::null_mut(), elemsize);
        let r_res = r_fn(ptr::null_mut(), elemsize);
        assert!(!c_res.is_null());
        assert!(!r_res.is_null());

        // Check header length = 1
        let c_raw = c_res.sub(elemsize);
        let r_raw = r_res.sub(elemsize);
        let c_len = *((c_raw as *mut usize).offset(-4));
        let r_len = *((r_raw as *mut usize).offset(-4));
        assert_eq!(c_len, r_len, "hmput_default length mismatch");
        assert_eq!(c_len, 1);

        // Calling again should not change length
        let c_res2 = c_fn(c_res, elemsize);
        let r_res2 = r_fn(r_res, elemsize);
        let c_raw2 = c_res2.sub(elemsize);
        let r_raw2 = r_res2.sub(elemsize);
        let c_len2 = *((c_raw2 as *mut usize).offset(-4));
        let r_len2 = *((r_raw2 as *mut usize).offset(-4));
        assert_eq!(c_len2, r_len2, "hmput_default second call length mismatch");
        assert_eq!(c_len2, 1);

        c_free(c_raw2, elemsize);
        r_free(r_raw2, elemsize);
    }
}

// ============================================================
// 10. stbds_stralloc / stbds_strreset
// ============================================================
#[test]
fn test_stralloc_strreset() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        // We need to create a zeroed stbds_string_arena struct for each
        // stbds_string_arena is: storage(8) + remaining(8) + block(1) + mode(1) + padding
        // Total size = 24 bytes (with alignment)
        let arena_size = 24usize;

        let c_arena = libc::calloc(1, arena_size) as *mut u8;
        let r_arena = libc::calloc(1, arena_size) as *mut u8;

        type AllocFn = unsafe extern "C" fn(*mut u8, *mut u8) -> *mut u8;
        type ResetFn = unsafe extern "C" fn(*mut u8);

        let c_alloc: Symbol<AllocFn> = c_lib.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<AllocFn> = r_lib.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<ResetFn> = c_lib.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<ResetFn> = r_lib.get(b"stbds_strreset").unwrap();

        // Allocate several strings and verify content matches
        let strings: &[&[u8]] = &[
            b"hello\0", b"world\0", b"test_string_123\0",
            b"a\0", b"longer string that might need a new block\0",
        ];

        for s in strings {
            let c_ptr = c_alloc(c_arena, s.as_ptr() as *mut u8);
            let r_ptr = r_alloc(r_arena, s.as_ptr() as *mut u8);

            let c_str = CStr::from_ptr(c_ptr as *const i8);
            let r_str = CStr::from_ptr(r_ptr as *const i8);
            assert_eq!(c_str, r_str, "stralloc content mismatch for {:?}",
                std::str::from_utf8(&s[..s.len()-1]).unwrap_or("?"));
        }

        // Check arena state matches
        // remaining field is at offset 8
        let c_remaining = *(c_arena.add(8) as *const usize);
        let r_remaining = *(r_arena.add(8) as *const usize);
        assert_eq!(c_remaining, r_remaining, "arena remaining mismatch");

        // block field at offset 16
        let c_block = *c_arena.add(16);
        let r_block = *r_arena.add(16);
        assert_eq!(c_block, r_block, "arena block mismatch");

        // Reset
        c_reset(c_arena);
        r_reset(r_arena);

        // After reset, arena should be zeroed
        let c_storage = *(c_arena as *const usize);
        let r_storage = *(r_arena as *const usize);
        assert_eq!(c_storage, 0, "C arena storage not null after reset");
        assert_eq!(r_storage, 0, "Rust arena storage not null after reset");

        libc::free(c_arena as *mut libc::c_void);
        libc::free(r_arena as *mut libc::c_void);
    }
}

// ============================================================
// 11. stbds_hmget_key_ts
// ============================================================
#[test]
fn test_hmget_key_ts() {
    let (c_lib, r_lib) = load_libs!();
    unsafe {
        type GetTsFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, *mut isize, i32) -> *mut u8;
        type PutFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_get_ts: Symbol<GetTsFn> = c_lib.get(b"stbds_hmget_key_ts").unwrap();
        let r_get_ts: Symbol<GetTsFn> = r_lib.get(b"stbds_hmget_key_ts").unwrap();
        let c_put: Symbol<PutFn> = c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<PutFn> = r_lib.get(b"stbds_hmput_key").unwrap();
        let c_free: Symbol<FreeFn> = c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r_lib.get(b"stbds_hmfree_func").unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c_lib.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r_lib.get(b"stbds_rand_seed").unwrap();
        c_seed(99999);
        r_seed(99999);

        let elemsize = 8usize;
        let keysize = 4usize;

        // Test from null - should initialize
        let mut c_temp: isize = 0;
        let mut r_temp: isize = 0;
        let mut key = 42i32;
        let mut c_map = c_get_ts(ptr::null_mut(), elemsize, &mut key as *mut i32 as *mut u8, keysize, &mut c_temp, 0);
        let mut r_map = r_get_ts(ptr::null_mut(), elemsize, &mut key as *mut i32 as *mut u8, keysize, &mut r_temp, 0);
        assert_eq!(c_temp, r_temp, "get_ts from null temp mismatch");
        assert_eq!(c_temp, -1);

        // Insert a key
        key = 42;
        c_map = c_put(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
        r_map = r_put(r_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);

        // Now get_ts should find it
        key = 42;
        c_map = c_get_ts(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, &mut c_temp, 0);
        r_map = r_get_ts(r_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, &mut r_temp, 0);
        assert_eq!(c_temp, r_temp, "get_ts after put temp mismatch");
        assert!(c_temp >= 0, "key should be found");

        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}
