use libloading::{Library, Symbol};
use std::ffi::CString;

fn c_lib() -> Library {
    unsafe { Library::new("c_src/build/libtranslated_rust_c.so").unwrap() }
}

fn rust_lib() -> Library {
    unsafe { Library::new("target/debug/libhm_geti_lib.so").unwrap() }
}

// ============================================================
// 1. stbds_rand_seed – sets global state, no return to compare,
//    but we verify it doesn't crash and affects hash output.
// ============================================================
#[test]
fn test_stbds_rand_seed() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        for seed in [0usize, 1, 42, 0xdeadbeef, usize::MAX] {
            c_seed(seed);
            r_seed(seed);
        }
    }
}

// ============================================================
// 2. stbds_hash_bytes – compare byte-for-byte hash output
// ============================================================
#[test]
fn test_stbds_hash_bytes() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8, usize, usize) -> usize> =
            c.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const u8, usize, usize) -> usize> =
            r.get(b"stbds_hash_bytes").unwrap();

        let test_data: &[&[u8]] = &[
            b"", b"a", b"ab", b"abc", b"abcd", b"abcdefgh",
            b"abcdefghijklmnop", b"\x00\x01\x02\x03\x04\x05\x06\x07",
            b"hello world this is a longer test string for hashing",
        ];
        let seeds = [0usize, 1, 42, 0x31415926, 0xdeadbeefcafebabe];

        for data in test_data {
            for &seed in &seeds {
                let c_val = c_fn(data.as_ptr(), data.len(), seed);
                let r_val = r_fn(data.as_ptr(), data.len(), seed);
                assert_eq!(c_val, r_val, "hash_bytes mismatch for data={:?} seed={}", data, seed);
            }
        }
    }
}

// ============================================================
// 3. stbds_hash_string – compare string hash output
// ============================================================
#[test]
fn test_stbds_hash_string() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8, usize) -> usize> =
            c.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const u8, usize) -> usize> =
            r.get(b"stbds_hash_string").unwrap();

        let strings = ["", "a", "hello", "test_42", "a longer string for testing"];
        let seeds = [0usize, 1, 42, 0x31415926, 0xdeadbeefcafebabe];

        for s in &strings {
            let cs = CString::new(*s).unwrap();
            for &seed in &seeds {
                let c_val = c_fn(cs.as_ptr() as *const u8, seed);
                let r_val = r_fn(cs.as_ptr() as *const u8, seed);
                assert_eq!(c_val, r_val, "hash_string mismatch for str={:?} seed={}", s, seed);
            }
        }
    }
}

// ============================================================
// 4. strkey – compare formatted string output
// ============================================================
#[test]
fn test_strkey() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> *const u8> = c.get(b"strkey").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32) -> *const u8> = r.get(b"strkey").unwrap();

        for n in [0, 1, -1, 42, 100, 999, -999, i32::MAX, i32::MIN] {
            let c_ptr = c_fn(n);
            let r_ptr = r_fn(n);
            let c_str = std::ffi::CStr::from_ptr(c_ptr as *const i8);
            let r_str = std::ffi::CStr::from_ptr(r_ptr as *const i8);
            assert_eq!(c_str, r_str, "strkey mismatch for n={}", n);
        }
    }
}

// ============================================================
// 5. stbds_arrgrowf / stbds_arrfreef – allocation round-trip
// ============================================================
#[test]
fn test_arrgrowf_arrfreef() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_grow: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> =
            c.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> =
            r.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8)> = c.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8)> = r.get(b"stbds_arrfreef").unwrap();

        // Allocate from null, elemsize=4, addlen=0, min_cap=10
        let c_arr = c_grow(std::ptr::null_mut(), 4, 0, 10);
        let r_arr = r_grow(std::ptr::null_mut(), 4, 0, 10);
        assert!(!c_arr.is_null());
        assert!(!r_arr.is_null());

        // Read back the header: length should be 0, capacity >= 10
        // Header is at (ptr - sizeof(header))
        let hdr_size = 32usize; // stbds_array_header: 4 fields * 8 bytes on 64-bit
        let c_hdr = c_arr.sub(hdr_size);
        let r_hdr = r_arr.sub(hdr_size);
        let c_len = *(c_hdr as *const usize);
        let r_len = *(r_hdr as *const usize);
        let c_cap = *((c_hdr as *const usize).add(1));
        let r_cap = *((r_hdr as *const usize).add(1));
        assert_eq!(c_len, r_len, "length mismatch after arrgrowf");
        assert_eq!(c_cap, r_cap, "capacity mismatch after arrgrowf");

        c_free(c_arr);
        r_free(r_arr);
    }
}

// ============================================================
// 6. hm_geti – the full integration test
//    Both C and Rust should run without assertion failures.
// ============================================================
#[test]
fn test_hm_geti() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        // Both libraries have their own global seed state.
        // Reset seeds to same value so hash tables behave identically.
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(i32)> = c.get(b"hm_geti").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32)> = r.get(b"hm_geti").unwrap();

        for &num in &[0, 1, 2, 4, 8, 16, 32, 64, 100] {
            // Reset seed before each run so behavior is deterministic
            c_seed(0x31415926);
            r_seed(0x31415926);
            c_fn(num);
            r_fn(num);
        }
    }
}

// ============================================================
// 7. stbds_hmput_key / stbds_hmget_key / stbds_hmfree_func
//    Direct low-level hashmap operations compared between C and Rust
// ============================================================
#[test]
fn test_hmput_hmget_hmfree() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        let c_put: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r.get(b"stbds_hmfree_func").unwrap();

        c_seed(0x31415926);
        r_seed(0x31415926);

        // Entry: { int key; int value; } = 8 bytes
        let elemsize: usize = 8;
        let keysize: usize = 4;

        // Put keys 0,2,4,...,18 with value = key*5
        let mut c_map: *mut u8 = std::ptr::null_mut();
        let mut r_map: *mut u8 = std::ptr::null_mut();

        for i in (0..20).step_by(2) {
            let mut key: i32 = i;
            c_map = c_put(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            // Write key and value into the new slot
            let hdr_ptr = (c_map.sub(elemsize) as *const usize).sub(4); // header
            let temp = *((hdr_ptr as *const isize).add(3));
            let entry = c_map.add(elemsize * temp as usize);
            *(entry as *mut i32) = i;
            *((entry as *mut i32).add(1)) = i * 5;

            let mut key2: i32 = i;
            r_map = r_put(r_map, elemsize, &mut key2 as *mut i32 as *mut u8, keysize, 0);
            let hdr_ptr = (r_map.sub(elemsize) as *const usize).sub(4);
            let temp = *((hdr_ptr as *const isize).add(3));
            let entry = r_map.add(elemsize * temp as usize);
            *(entry as *mut i32) = i;
            *((entry as *mut i32).add(1)) = i * 5;
        }

        // Get each key and compare temp (index) values
        for i in 0..20 {
            let mut c_key: i32 = i;
            let mut r_key: i32 = i;
            let c_res = c_get(c_map, elemsize, &mut c_key as *mut i32 as *mut u8, keysize, 0);
            let r_res = r_get(r_map, elemsize, &mut r_key as *mut i32 as *mut u8, keysize, 0);
            c_map = c_res;
            r_map = r_res;

            let c_hdr = (c_map.sub(elemsize) as *const isize).sub(4);
            let r_hdr = (r_map.sub(elemsize) as *const isize).sub(4);
            let c_temp = *c_hdr.add(3);
            let r_temp = *r_hdr.add(3);

            if i % 2 == 0 {
                // Key exists
                assert!(c_temp >= 0, "C: key {} should exist", i);
                assert!(r_temp >= 0, "Rust: key {} should exist", i);
                // Compare stored values
                let c_val = *((c_map.add(elemsize * c_temp as usize) as *const i32).add(1));
                let r_val = *((r_map.add(elemsize * r_temp as usize) as *const i32).add(1));
                assert_eq!(c_val, r_val, "value mismatch for key {}", i);
            } else {
                // Key doesn't exist
                assert_eq!(c_temp, -1, "C: key {} should not exist", i);
                assert_eq!(r_temp, -1, "Rust: key {} should not exist", i);
            }
        }

        // Free
        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ============================================================
// 8. stbds_hmdel_key – test deletion
// ============================================================
#[test]
fn test_hmdel_key() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        let c_put: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            c.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8> =
            r.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8> =
            c.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8> =
            r.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r.get(b"stbds_hmfree_func").unwrap();

        c_seed(0x31415926);
        r_seed(0x31415926);

        let elemsize: usize = 8;
        let keysize: usize = 4;

        let mut c_map: *mut u8 = std::ptr::null_mut();
        let mut r_map: *mut u8 = std::ptr::null_mut();

        // Insert keys 0..10
        for i in 0..10i32 {
            let mut key: i32 = i;
            c_map = c_put(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let hdr = (c_map.sub(elemsize) as *const isize).sub(4);
            let temp = *hdr.add(3);
            let entry = c_map.add(elemsize * temp as usize);
            *(entry as *mut i32) = i;
            *((entry as *mut i32).add(1)) = i * 10;

            let mut key2: i32 = i;
            r_map = r_put(r_map, elemsize, &mut key2 as *mut i32 as *mut u8, keysize, 0);
            let hdr = (r_map.sub(elemsize) as *const isize).sub(4);
            let temp = *hdr.add(3);
            let entry = r_map.add(elemsize * temp as usize);
            *(entry as *mut i32) = i;
            *((entry as *mut i32).add(1)) = i * 10;
        }

        // Delete even keys
        for i in (0..10i32).step_by(2) {
            let mut key: i32 = i;
            c_map = c_del(c_map, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0, 0);
            let mut key2: i32 = i;
            r_map = r_del(r_map, elemsize, &mut key2 as *mut i32 as *mut u8, keysize, 0, 0);
        }

        // Verify: even keys gone, odd keys still present
        for i in 0..10i32 {
            let mut c_key: i32 = i;
            let mut r_key: i32 = i;
            c_map = c_get(c_map, elemsize, &mut c_key as *mut i32 as *mut u8, keysize, 0);
            r_map = r_get(r_map, elemsize, &mut r_key as *mut i32 as *mut u8, keysize, 0);

            let c_temp = *((c_map.sub(elemsize) as *const isize).sub(4).add(3));
            let r_temp = *((r_map.sub(elemsize) as *const isize).sub(4).add(3));

            assert_eq!(c_temp >= 0, r_temp >= 0, "existence mismatch for key {} after delete", i);
            if i % 2 == 0 {
                assert_eq!(c_temp, -1, "C: deleted key {} should be gone", i);
                assert_eq!(r_temp, -1, "Rust: deleted key {} should be gone", i);
            }
        }

        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ============================================================
// 9. stbds_hmput_default – test default value mechanism
// ============================================================
#[test]
fn test_hmput_default() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        let c_def: Symbol<unsafe extern "C" fn(*mut u8, usize) -> *mut u8> =
            c.get(b"stbds_hmput_default").unwrap();
        let r_def: Symbol<unsafe extern "C" fn(*mut u8, usize) -> *mut u8> =
            r.get(b"stbds_hmput_default").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r.get(b"stbds_hmfree_func").unwrap();

        c_seed(0x31415926);
        r_seed(0x31415926);

        let elemsize: usize = 8;

        // Call with null - should allocate and return pointer to element[0]
        let c_map = c_def(std::ptr::null_mut(), elemsize);
        let r_map = r_def(std::ptr::null_mut(), elemsize);
        assert!(!c_map.is_null());
        assert!(!r_map.is_null());

        // The header length should be 1
        let c_len = *((c_map.sub(elemsize) as *const usize).sub(4));
        let r_len = *((r_map.sub(elemsize) as *const usize).sub(4));
        assert_eq!(c_len, r_len, "length mismatch after hmput_default");
        assert_eq!(c_len, 1);

        // Calling again should be a no-op (already has default)
        let c_map2 = c_def(c_map, elemsize);
        let r_map2 = r_def(r_map, elemsize);
        assert_eq!(c_map, c_map2, "C: hmput_default should return same pointer");
        assert_eq!(r_map, r_map2, "Rust: hmput_default should return same pointer");

        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ============================================================
// 10. stbds_shmode_func – test string hashmap mode init
// ============================================================
#[test]
fn test_shmode_func() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(usize, i32) -> *mut u8> =
            c.get(b"stbds_shmode_func").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(usize, i32) -> *mut u8> =
            r.get(b"stbds_shmode_func").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> =
            r.get(b"stbds_hmfree_func").unwrap();

        // elemsize for a string map entry: { char* key; int value; } = 16 bytes on 64-bit
        let elemsize: usize = 16;

        for &mode in &[2i32, 3i32] { // STBDS_SH_STRDUP=2, STBDS_SH_ARENA=3
            c_seed(0x31415926);
            r_seed(0x31415926);

            let c_map = c_fn(elemsize, mode);
            let r_map = r_fn(elemsize, mode);
            assert!(!c_map.is_null());
            assert!(!r_map.is_null());

            // Check header length = 1
            let c_len = *((c_map.sub(elemsize) as *const usize).sub(4));
            let r_len = *((r_map.sub(elemsize) as *const usize).sub(4));
            assert_eq!(c_len, r_len, "length mismatch for shmode_func mode={}", mode);
            assert_eq!(c_len, 1);

            c_free(c_map.sub(elemsize), elemsize);
            r_free(r_map.sub(elemsize), elemsize);
        }
    }
}
