use libloading::Library;
use std::ffi::{CStr, CString};
use std::ptr;

const C_LIB_PATH: &str = "/tmp/harvest-work-iHaAms/translated_rust/c_src/build/libtranslated_rust.so";
const RUST_LIB_PATH: &str = "/tmp/harvest-work-iHaAms/translated_rust/target/debug/libarr_ins_lib.so";

const _HEADER_SIZE: usize = 32; // sizeof(stbds_array_header) on 64-bit
const ARENA_SIZE: usize = 24;  // sizeof(stbds_string_arena) on 64-bit

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            Libs {
                c: Library::new(C_LIB_PATH).expect("Failed to load C library"),
                rust: Library::new(RUST_LIB_PATH).expect("Failed to load Rust library"),
            }
        }
    }
}

#[test]
fn test_strkey() {
    let libs = Libs::load();
    unsafe {
        let c_strkey = libs.c.get::<unsafe extern "C" fn(i32) -> *mut i8>(b"strkey\0").unwrap();
        let r_strkey = libs.rust.get::<unsafe extern "C" fn(i32) -> *mut i8>(b"strkey\0").unwrap();

        for &n in &[0, 42, -1] {
            let c_result = CStr::from_ptr(c_strkey(n));
            let r_result = CStr::from_ptr(r_strkey(n));
            assert_eq!(c_result, r_result, "strkey({}) mismatch", n);
        }
    }
}

#[test]
fn test_hash_string() {
    let libs = Libs::load();
    unsafe {
        let c_fn = libs.c.get::<unsafe extern "C" fn(*mut i8, usize) -> usize>(b"stbds_hash_string\0").unwrap();
        let r_fn = libs.rust.get::<unsafe extern "C" fn(*mut i8, usize) -> usize>(b"stbds_hash_string\0").unwrap();

        let strings = ["hello", "", "test_42", "a]longer string for testing hash functions!"];
        let seeds = [0usize, 1, 0x31415926];

        for s in &strings {
            let cs = CString::new(*s).unwrap();
            for &seed in &seeds {
                let c_result = c_fn(cs.as_ptr() as *mut i8, seed);
                let r_result = r_fn(cs.as_ptr() as *mut i8, seed);
                assert_eq!(c_result, r_result, "hash_string({:?}, {}) mismatch", s, seed);
            }
        }
    }
}

#[test]
fn test_hash_bytes() {
    let libs = Libs::load();
    unsafe {
        let c_fn = libs.c.get::<unsafe extern "C" fn(*mut u8, usize, usize) -> usize>(b"stbds_hash_bytes\0").unwrap();
        let r_fn = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize, usize) -> usize>(b"stbds_hash_bytes\0").unwrap();

        let test_cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![0x42],
            vec![1, 2, 3, 4, 5, 6, 7],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![0; 15],
            vec![0xFF; 16],
            (0..100).map(|i| (i & 0xFF) as u8).collect(),
        ];
        let seeds = [0usize, 1, 0x31415926];

        for data in &test_cases {
            for &seed in &seeds {
                let p = if data.is_empty() { ptr::null_mut() } else { data.as_ptr() as *mut u8 };
                let c_result = c_fn(p, data.len(), seed);
                let r_result = r_fn(p, data.len(), seed);
                assert_eq!(c_result, r_result, "hash_bytes(len={}, seed={}) mismatch", data.len(), seed);
            }
        }
    }
}

#[test]
fn test_rand_seed() {
    let libs = Libs::load();
    unsafe {
        let c_seed = libs.c.get::<unsafe extern "C" fn(usize)>(b"stbds_rand_seed\0").unwrap();
        let r_seed = libs.rust.get::<unsafe extern "C" fn(usize)>(b"stbds_rand_seed\0").unwrap();
        let c_hash = libs.c.get::<unsafe extern "C" fn(*mut i8, usize) -> usize>(b"stbds_hash_string\0").unwrap();
        let r_hash = libs.rust.get::<unsafe extern "C" fn(*mut i8, usize) -> usize>(b"stbds_hash_string\0").unwrap();

        c_seed(42);
        r_seed(42);

        let s = CString::new("test").unwrap();
        let c_result = c_hash(s.as_ptr() as *mut i8, 0);
        let r_result = r_hash(s.as_ptr() as *mut i8, 0);
        assert_eq!(c_result, r_result, "hash after rand_seed(42) mismatch");

        // Reset to default
        c_seed(0x31415926);
        r_seed(0x31415926);
    }
}

#[test]
fn test_arrgrowf_and_arrfreef() {
    let libs = Libs::load();
    unsafe {
        let c_grow = libs.c.get::<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8>(b"stbds_arrgrowf\0").unwrap();
        let r_grow = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8>(b"stbds_arrgrowf\0").unwrap();
        let c_free = libs.c.get::<unsafe extern "C" fn(*mut u8)>(b"stbds_arrfreef\0").unwrap();
        let r_free = libs.rust.get::<unsafe extern "C" fn(*mut u8)>(b"stbds_arrfreef\0").unwrap();

        // Allocate from null
        let c_ptr = c_grow(ptr::null_mut(), 4, 0, 10);
        let r_ptr = r_grow(ptr::null_mut(), 4, 0, 10);
        assert!(!c_ptr.is_null(), "C arrgrowf returned null");
        assert!(!r_ptr.is_null(), "Rust arrgrowf returned null");

        // Check header: length should be 0, capacity >= 10
        let c_hdr = (c_ptr as *mut usize).sub(4); // header is 32 bytes before
        let r_hdr = (r_ptr as *mut usize).sub(4);
        assert_eq!(*c_hdr, 0, "C length should be 0");
        assert_eq!(*r_hdr, 0, "Rust length should be 0");
        assert!(*c_hdr.add(1) >= 10, "C capacity should be >= 10");
        assert!(*r_hdr.add(1) >= 10, "Rust capacity should be >= 10");
        assert_eq!(*c_hdr.add(1), *r_hdr.add(1), "Capacities should match");

        // Write some data
        *(c_ptr as *mut i32) = 123;
        *(r_ptr as *mut i32) = 123;

        // Grow existing array
        let c_ptr2 = c_grow(c_ptr, 4, 0, 20);
        let r_ptr2 = r_grow(r_ptr, 4, 0, 20);
        assert!(!c_ptr2.is_null());
        assert!(!r_ptr2.is_null());

        let c_hdr2 = (c_ptr2 as *mut usize).sub(4);
        let r_hdr2 = (r_ptr2 as *mut usize).sub(4);
        assert!(*c_hdr2.add(1) >= 20);
        assert!(*r_hdr2.add(1) >= 20);
        assert_eq!(*c_hdr2.add(1), *r_hdr2.add(1), "Grown capacities should match");

        c_free(c_ptr2);
        r_free(r_ptr2);
    }
}

#[test]
fn test_arr_ins() {
    let libs = Libs::load();
    unsafe {
        let c_fn = libs.c.get::<unsafe extern "C" fn(i32)>(b"arr_ins\0").unwrap();
        let r_fn = libs.rust.get::<unsafe extern "C" fn(i32)>(b"arr_ins\0").unwrap();

        for &n in &[42, 0, -1] {
            c_fn(n);
            r_fn(n);
        }
    }
}

#[test]
fn test_hmput_hmget_hmdel() {
    let libs = Libs::load();
    unsafe {
        let c_seed = libs.c.get::<unsafe extern "C" fn(usize)>(b"stbds_rand_seed\0").unwrap();
        let r_seed = libs.rust.get::<unsafe extern "C" fn(usize)>(b"stbds_rand_seed\0").unwrap();
        let c_hmput = libs.c.get::<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8>(b"stbds_hmput_key\0").unwrap();
        let r_hmput = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8>(b"stbds_hmput_key\0").unwrap();
        let c_hmget = libs.c.get::<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8>(b"stbds_hmget_key\0").unwrap();
        let r_hmget = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8>(b"stbds_hmget_key\0").unwrap();
        let c_hmdel = libs.c.get::<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8>(b"stbds_hmdel_key\0").unwrap();
        let r_hmdel = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8>(b"stbds_hmdel_key\0").unwrap();
        let c_hmfree = libs.c.get::<unsafe extern "C" fn(*mut u8, usize)>(b"stbds_hmfree_func\0").unwrap();
        let r_hmfree = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize)>(b"stbds_hmfree_func\0").unwrap();

        // Use same seed for deterministic behavior
        c_seed(12345);
        r_seed(12345);

        let elemsize: usize = 16; // { key: i32, pad: i32, value: i64 }
        let keysize: usize = 4;
        let mode: i32 = 0; // STBDS_HM_BINARY

        // Insert key=42
        let mut key: i32 = 42;
        let c_a = c_hmput(ptr::null_mut(), elemsize, &mut key as *mut i32 as *mut u8, keysize, mode);
        let r_a = r_hmput(ptr::null_mut(), elemsize, &mut key as *mut i32 as *mut u8, keysize, mode);
        assert!(!c_a.is_null(), "C hmput returned null");
        assert!(!r_a.is_null(), "Rust hmput returned null");

        // Read back the temp value from header (index of inserted element)
        let c_raw = c_a.sub(elemsize);
        let r_raw = r_a.sub(elemsize);
        let c_temp = *((c_raw as *mut usize).sub(4).add(3) as *mut isize);
        let r_temp = *((r_raw as *mut usize).sub(4).add(3) as *mut isize);
        assert_eq!(c_temp, r_temp, "temp after hmput should match");

        // Lookup key=42
        let c_a2 = c_hmget(c_a, elemsize, &mut key as *mut i32 as *mut u8, keysize, mode);
        let r_a2 = r_hmget(r_a, elemsize, &mut key as *mut i32 as *mut u8, keysize, mode);
        let c_raw2 = c_a2.sub(elemsize);
        let r_raw2 = r_a2.sub(elemsize);
        let c_temp2 = *((c_raw2 as *mut usize).sub(4).add(3) as *mut isize);
        let r_temp2 = *((r_raw2 as *mut usize).sub(4).add(3) as *mut isize);
        assert!(c_temp2 >= 0, "C hmget should find key=42");
        assert!(r_temp2 >= 0, "Rust hmget should find key=42");
        assert_eq!(c_temp2, r_temp2, "temp after hmget should match");

        // Lookup non-existent key=99
        let mut key2: i32 = 99;
        let c_a3 = c_hmget(c_a2, elemsize, &mut key2 as *mut i32 as *mut u8, keysize, mode);
        let r_a3 = r_hmget(r_a2, elemsize, &mut key2 as *mut i32 as *mut u8, keysize, mode);
        let c_raw3 = c_a3.sub(elemsize);
        let r_raw3 = r_a3.sub(elemsize);
        let c_temp3 = *((c_raw3 as *mut usize).sub(4).add(3) as *mut isize);
        let r_temp3 = *((r_raw3 as *mut usize).sub(4).add(3) as *mut isize);
        assert!(c_temp3 < 0, "C hmget should not find key=99");
        assert!(r_temp3 < 0, "Rust hmget should not find key=99");

        // Delete key=42
        let c_a4 = c_hmdel(c_a3, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0, mode);
        let r_a4 = r_hmdel(r_a3, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0, mode);
        let c_raw4 = c_a4.sub(elemsize);
        let r_raw4 = r_a4.sub(elemsize);
        let c_temp4 = *((c_raw4 as *mut usize).sub(4).add(3) as *mut isize);
        let r_temp4 = *((r_raw4 as *mut usize).sub(4).add(3) as *mut isize);
        // temp=1 means deleted, temp=0 means not found
        assert_eq!(c_temp4, r_temp4, "temp after hmdel should match");
        assert_eq!(c_temp4, 1, "hmdel should indicate deletion");

        // Verify key=42 is gone
        let c_a5 = c_hmget(c_a4, elemsize, &mut key as *mut i32 as *mut u8, keysize, mode);
        let r_a5 = r_hmget(r_a4, elemsize, &mut key as *mut i32 as *mut u8, keysize, mode);
        let c_raw5 = c_a5.sub(elemsize);
        let r_raw5 = r_a5.sub(elemsize);
        let c_temp5 = *((c_raw5 as *mut usize).sub(4).add(3) as *mut isize);
        let r_temp5 = *((r_raw5 as *mut usize).sub(4).add(3) as *mut isize);
        assert!(c_temp5 < 0, "C: key=42 should be gone after delete");
        assert!(r_temp5 < 0, "Rust: key=42 should be gone after delete");

        // Cleanup
        c_hmfree(c_raw5, elemsize);
        r_hmfree(r_raw5, elemsize);
    }
}

#[test]
fn test_hmput_default() {
    let libs = Libs::load();
    unsafe {
        let c_fn = libs.c.get::<unsafe extern "C" fn(*mut u8, usize) -> *mut u8>(b"stbds_hmput_default\0").unwrap();
        let r_fn = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize) -> *mut u8>(b"stbds_hmput_default\0").unwrap();

        let elemsize: usize = 16;
        let c_a = c_fn(ptr::null_mut(), elemsize);
        let r_a = r_fn(ptr::null_mut(), elemsize);
        assert!(!c_a.is_null(), "C hmput_default returned null");
        assert!(!r_a.is_null(), "Rust hmput_default returned null");

        // Check that length is 1 (the default element)
        let c_raw = c_a.sub(elemsize);
        let r_raw = r_a.sub(elemsize);
        let c_len = *((c_raw as *mut usize).sub(4));
        let r_len = *((r_raw as *mut usize).sub(4));
        assert_eq!(c_len, 1);
        assert_eq!(r_len, 1);

        // Calling again should be idempotent
        let c_a2 = c_fn(c_a, elemsize);
        let r_a2 = r_fn(r_a, elemsize);
        assert_eq!(c_a, c_a2, "C hmput_default should be idempotent");
        assert_eq!(r_a, r_a2, "Rust hmput_default should be idempotent");

        // Cleanup - these don't have hash tables, just free the array
        let c_free = libs.c.get::<unsafe extern "C" fn(*mut u8)>(b"stbds_arrfreef\0").unwrap();
        let r_free = libs.rust.get::<unsafe extern "C" fn(*mut u8)>(b"stbds_arrfreef\0").unwrap();
        c_free(c_raw);
        r_free(r_raw);
    }
}

#[test]
fn test_shmode_func() {
    let libs = Libs::load();
    unsafe {
        let c_seed = libs.c.get::<unsafe extern "C" fn(usize)>(b"stbds_rand_seed\0").unwrap();
        let r_seed = libs.rust.get::<unsafe extern "C" fn(usize)>(b"stbds_rand_seed\0").unwrap();
        let c_fn = libs.c.get::<unsafe extern "C" fn(usize, i32) -> *mut u8>(b"stbds_shmode_func\0").unwrap();
        let r_fn = libs.rust.get::<unsafe extern "C" fn(usize, i32) -> *mut u8>(b"stbds_shmode_func\0").unwrap();
        let c_hmfree = libs.c.get::<unsafe extern "C" fn(*mut u8, usize)>(b"stbds_hmfree_func\0").unwrap();
        let r_hmfree = libs.rust.get::<unsafe extern "C" fn(*mut u8, usize)>(b"stbds_hmfree_func\0").unwrap();

        c_seed(99999);
        r_seed(99999);

        for &mode in &[1i32, 2] {
            let elemsize: usize = 16;
            let c_a = c_fn(elemsize, mode);
            let r_a = r_fn(elemsize, mode);
            assert!(!c_a.is_null(), "C shmode_func({}, {}) returned null", elemsize, mode);
            assert!(!r_a.is_null(), "Rust shmode_func({}, {}) returned null", elemsize, mode);

            // Check length is 1
            let c_raw = c_a.sub(elemsize);
            let r_raw = r_a.sub(elemsize);
            let c_len = *((c_raw as *mut usize).sub(4));
            let r_len = *((r_raw as *mut usize).sub(4));
            assert_eq!(c_len, 1, "C shmode length");
            assert_eq!(r_len, 1, "Rust shmode length");

            // Hash table should be set
            let c_ht = *((c_raw as *mut usize).sub(4).add(2));
            let r_ht = *((r_raw as *mut usize).sub(4).add(2));
            assert_ne!(c_ht, 0, "C hash table should be non-null");
            assert_ne!(r_ht, 0, "Rust hash table should be non-null");

            c_hmfree(c_raw, elemsize);
            r_hmfree(r_raw, elemsize);
        }
    }
}

#[test]
fn test_stralloc_strreset() {
    let libs = Libs::load();
    unsafe {
        let c_alloc = libs.c.get::<unsafe extern "C" fn(*mut u8, *mut i8) -> *mut i8>(b"stbds_stralloc\0").unwrap();
        let r_alloc = libs.rust.get::<unsafe extern "C" fn(*mut u8, *mut i8) -> *mut i8>(b"stbds_stralloc\0").unwrap();
        let c_reset = libs.c.get::<unsafe extern "C" fn(*mut u8)>(b"stbds_strreset\0").unwrap();
        let r_reset = libs.rust.get::<unsafe extern "C" fn(*mut u8)>(b"stbds_strreset\0").unwrap();

        // Create zeroed arenas (24 bytes each)
        let mut c_arena = [0u8; ARENA_SIZE];
        let mut r_arena = [0u8; ARENA_SIZE];

        let strings = ["hello", "world", "test_string_123", ""];
        for s in &strings {
            let cs = CString::new(*s).unwrap();
            let c_result = c_alloc(c_arena.as_mut_ptr(), cs.as_ptr() as *mut i8);
            let r_result = r_alloc(r_arena.as_mut_ptr(), cs.as_ptr() as *mut i8);
            assert!(!c_result.is_null(), "C stralloc returned null for {:?}", s);
            assert!(!r_result.is_null(), "Rust stralloc returned null for {:?}", s);

            let c_str = CStr::from_ptr(c_result);
            let r_str = CStr::from_ptr(r_result);
            assert_eq!(c_str.to_bytes(), s.as_bytes(), "C stralloc content mismatch");
            assert_eq!(r_str.to_bytes(), s.as_bytes(), "Rust stralloc content mismatch");
            assert_eq!(c_str, r_str, "stralloc results differ for {:?}", s);
        }

        // Reset both arenas
        c_reset(c_arena.as_mut_ptr());
        r_reset(r_arena.as_mut_ptr());

        // Verify arenas are zeroed after reset
        assert_eq!(c_arena, [0u8; ARENA_SIZE], "C arena should be zeroed after reset");
        assert_eq!(r_arena, [0u8; ARENA_SIZE], "Rust arena should be zeroed after reset");
    }
}
