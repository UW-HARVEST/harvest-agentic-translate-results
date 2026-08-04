use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::ptr;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libstr_put_lib.so")
}

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;



unsafe fn get_header(ptr: *mut c_void) -> *mut StbdsArrayHeader {
    (ptr as *mut StbdsArrayHeader).sub(1)
}

/// Helper: load both libraries and seed them identically.
struct DualLib {
    c_lib: Library,
    rust_lib: Library,
}

impl DualLib {
    fn load() -> Self {
        let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
        let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") };
        DualLib { c_lib, rust_lib }
    }

    unsafe fn seed_both(&self, seed: usize) {
        let c_fn: Symbol<unsafe extern "C" fn(usize)> =
            self.c_lib.get(b"stbds_rand_seed").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(usize)> =
            self.rust_lib.get(b"stbds_rand_seed").unwrap();
        c_fn(seed);
        r_fn(seed);
    }
}

// ============================================================
// Test 1: stbds_hash_string
// ============================================================
#[test]
fn test_hash_string() {
    let dl = DualLib::load();
    unsafe {
        dl.seed_both(42);

        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, usize) -> usize> =
            dl.c_lib.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char, usize) -> usize> =
            dl.rust_lib.get(b"stbds_hash_string").unwrap();

        let test_cases = ["hello", "world", "", "a", "test_12345", "foo bar baz"];
        let seeds = [0usize, 1, 42, 0x31415926, usize::MAX];

        for s in &test_cases {
            let cs = CString::new(*s).unwrap();
            for &seed in &seeds {
                let c_result = c_fn(cs.as_ptr(), seed);
                let r_result = r_fn(cs.as_ptr(), seed);
                assert_eq!(
                    c_result, r_result,
                    "hash_string mismatch for {:?} seed={}",
                    s, seed
                );
            }
        }
    }
}

// ============================================================
// Test 2: stbds_hash_bytes
// ============================================================
#[test]
fn test_hash_bytes() {
    let dl = DualLib::load();
    unsafe {
        dl.seed_both(42);

        let c_fn: Symbol<unsafe extern "C" fn(*const c_void, usize, usize) -> usize> =
            dl.c_lib.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_void, usize, usize) -> usize> =
            dl.rust_lib.get(b"stbds_hash_bytes").unwrap();

        let bufs: &[&[u8]] = &[
            b"hello",
            b"",
            b"\x00\x01\x02\x03",
            b"abcdefghijklmnop",       // 16 bytes, exactly 2 size_t
            b"abcdefghijklmnopq",      // 17 bytes
            &[0xffu8; 64],
        ];
        let seeds = [0usize, 1, 42, 0xdeadbeef];

        for buf in bufs {
            for &seed in &seeds {
                let c_result = c_fn(buf.as_ptr() as *const c_void, buf.len(), seed);
                let r_result = r_fn(buf.as_ptr() as *const c_void, buf.len(), seed);
                assert_eq!(
                    c_result, r_result,
                    "hash_bytes mismatch for len={} seed={}",
                    buf.len(),
                    seed
                );
            }
        }
    }
}

// ============================================================
// Test 3: strkey
// ============================================================
#[test]
fn test_strkey() {
    let dl = DualLib::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            dl.c_lib.get(b"strkey").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            dl.rust_lib.get(b"strkey").unwrap();

        for n in [0, 1, 5, 42, 100, 999] {
            // Call C, copy result immediately (global buffer)
            let c_ptr = c_fn(n);
            let c_str = CStr::from_ptr(c_ptr).to_str().unwrap().to_owned();

            let r_ptr = r_fn(n);
            let r_str = CStr::from_ptr(r_ptr).to_str().unwrap().to_owned();

            assert_eq!(c_str, r_str, "strkey mismatch for n={}", n);
            assert_eq!(c_str, format!("test_{}", n));
        }
    }
}

// ============================================================
// Test 4: stbds_arrgrowf / stbds_arrfreef
// ============================================================
#[test]
fn test_arrgrowf() {
    let dl = DualLib::load();
    unsafe {
        let c_grow: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            dl.c_lib.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            dl.rust_lib.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            dl.c_lib.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut c_void)> =
            dl.rust_lib.get(b"stbds_arrfreef").unwrap();

        // Allocate from NULL with elemsize=4, addlen=0, min_cap=10
        let c_ptr = c_grow(ptr::null_mut(), 4, 0, 10);
        let r_ptr = r_grow(ptr::null_mut(), 4, 0, 10);

        assert!(!c_ptr.is_null());
        assert!(!r_ptr.is_null());

        let c_hdr = &*get_header(c_ptr);
        let r_hdr = &*get_header(r_ptr);

        assert_eq!(c_hdr.length, 0, "C length should be 0");
        assert_eq!(r_hdr.length, 0, "Rust length should be 0");
        assert_eq!(c_hdr.capacity, r_hdr.capacity, "Capacities should match");
        assert!(c_hdr.capacity >= 10, "Capacity should be >= 10");
        assert!(c_hdr.hash_table.is_null(), "hash_table should be null");
        assert!(r_hdr.hash_table.is_null(), "hash_table should be null");
        assert_eq!(c_hdr.temp, 0);
        assert_eq!(r_hdr.temp, 0);

        // Test grow from existing allocation
        let c_ptr2 = c_grow(c_ptr, 4, 5, 0);
        let r_ptr2 = r_grow(r_ptr, 4, 5, 0);

        let c_hdr2 = &*get_header(c_ptr2);
        let r_hdr2 = &*get_header(r_ptr2);
        assert_eq!(c_hdr2.capacity, r_hdr2.capacity);

        c_free(c_ptr2);
        r_free(r_ptr2);
    }
}

// ============================================================
// Test 5: stbds_stralloc / stbds_strreset
// ============================================================
#[test]
fn test_stralloc_strreset() {
    let dl = DualLib::load();
    unsafe {
        let c_alloc: Symbol<unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char> =
            dl.c_lib.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char> =
            dl.rust_lib.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<unsafe extern "C" fn(*mut c_void)> =
            dl.c_lib.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<unsafe extern "C" fn(*mut c_void)> =
            dl.rust_lib.get(b"stbds_strreset").unwrap();

        // stbds_string_arena is 24 bytes, zero-initialized
        let mut c_arena = [0u8; 24];
        let mut r_arena = [0u8; 24];

        let strings = ["hello", "world", "test_string", "a", "longer string for arena testing"];

        for s in &strings {
            let cs = CString::new(*s).unwrap();
            let c_ptr = c_alloc(c_arena.as_mut_ptr() as *mut c_void, cs.as_ptr());
            let r_ptr = r_alloc(r_arena.as_mut_ptr() as *mut c_void, cs.as_ptr());

            let c_result = CStr::from_ptr(c_ptr).to_str().unwrap();
            let r_result = CStr::from_ptr(r_ptr).to_str().unwrap();

            assert_eq!(c_result, *s, "C stralloc returned wrong string");
            assert_eq!(r_result, *s, "Rust stralloc returned wrong string");
            assert_eq!(c_result, r_result);
        }

        // Verify arena internal state matches (remaining field at offset 8)
        let c_remaining = usize::from_ne_bytes(c_arena[8..16].try_into().unwrap());
        let r_remaining = usize::from_ne_bytes(r_arena[8..16].try_into().unwrap());
        assert_eq!(c_remaining, r_remaining, "Arena remaining should match");

        // block field at offset 16
        assert_eq!(c_arena[16], r_arena[16], "Arena block should match");

        c_reset(c_arena.as_mut_ptr() as *mut c_void);
        r_reset(r_arena.as_mut_ptr() as *mut c_void);

        // After reset, arena should be zeroed
        assert_eq!(c_arena, [0u8; 24]);
        assert_eq!(r_arena, [0u8; 24]);
    }
}

// ============================================================
// Test 6: stbds_hmput_key / stbds_hmget_key (BINARY mode)
// ============================================================
#[test]
fn test_hmput_hmget_binary() {
    let dl = DualLib::load();
    unsafe {
        dl.seed_both(12345);

        type HmPutKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmGetKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmFreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_put: Symbol<HmPutKeyFn> = dl.c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmPutKeyFn> = dl.rust_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<HmGetKeyFn> = dl.c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<HmGetKeyFn> = dl.rust_lib.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<HmFreeFn> = dl.c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmFreeFn> = dl.rust_lib.get(b"stbds_hmfree_func").unwrap();

        // Struct: { key: i32, value: i32 } => elemsize=8, keysize=4
        let elemsize: usize = 8;
        let keysize: usize = 4;

        // Start with NULL (no map yet)
        let mut c_a: *mut c_void = ptr::null_mut();
        let mut r_a: *mut c_void = ptr::null_mut();

        // Insert key=10, value=100
        let mut key: i32 = 10;
        c_a = c_put(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        r_a = r_put(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);

        // c_a and r_a now point to arr+elemsize (the STBDS_ARR_TO_HASH result)
        // The raw array is at c_a - elemsize
        // header->temp gives the index where the element was placed
        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;

        let c_temp = (*get_header(c_raw)).temp;
        let r_temp = (*get_header(r_raw)).temp;
        assert_eq!(c_temp, r_temp, "temp index should match after put");

        // Write the element at the index: key=10, value=100
        let c_elem = (c_a as *mut u8).offset(elemsize as isize * c_temp) as *mut i32;
        *c_elem = 10;
        *c_elem.add(1) = 100;

        let r_elem = (r_a as *mut u8).offset(elemsize as isize * r_temp) as *mut i32;
        *r_elem = 10;
        *r_elem.add(1) = 100;

        // Insert key=20, value=200
        key = 20;
        c_a = c_put(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        r_a = r_put(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_temp = (*get_header(c_raw)).temp;
        let r_temp = (*get_header(r_raw)).temp;
        assert_eq!(c_temp, r_temp, "temp index should match after second put");

        let c_elem = (c_a as *mut u8).offset(elemsize as isize * c_temp) as *mut i32;
        *c_elem = 20;
        *c_elem.add(1) = 200;

        let r_elem = (r_a as *mut u8).offset(elemsize as isize * r_temp) as *mut i32;
        *r_elem = 20;
        *r_elem.add(1) = 200;

        // Now look up key=10
        key = 10;
        c_a = c_get(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        r_a = r_get(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_idx = (*get_header(c_raw)).temp;
        let r_idx = (*get_header(r_raw)).temp;
        assert_eq!(c_idx, r_idx, "get index should match for key=10");
        assert!(c_idx >= 0, "key=10 should be found");

        // Read value at index
        let c_val = *((c_a as *mut u8).offset(elemsize as isize * c_idx) as *mut i32).add(1);
        let r_val = *((r_a as *mut u8).offset(elemsize as isize * r_idx) as *mut i32).add(1);
        assert_eq!(c_val, 100);
        assert_eq!(r_val, 100);

        // Look up key=20
        key = 20;
        c_a = c_get(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        r_a = r_get(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_idx = (*get_header(c_raw)).temp;
        let r_idx = (*get_header(r_raw)).temp;
        assert_eq!(c_idx, r_idx);
        let c_val = *((c_a as *mut u8).offset(elemsize as isize * c_idx) as *mut i32).add(1);
        let r_val = *((r_a as *mut u8).offset(elemsize as isize * r_idx) as *mut i32).add(1);
        assert_eq!(c_val, 200);
        assert_eq!(r_val, 200);

        // Look up non-existent key=99
        key = 99;
        c_a = c_get(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        r_a = r_get(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_idx = (*get_header(c_raw)).temp;
        let r_idx = (*get_header(r_raw)).temp;
        assert_eq!(c_idx, r_idx);
        assert_eq!(c_idx, -1, "non-existent key should return -1");

        // Cleanup
        c_free((c_a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        r_free((r_a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ============================================================
// Test 7: stbds_hmput_key / stbds_hmget_key (STRING mode)
// ============================================================
#[test]
fn test_hmput_hmget_string() {
    let dl = DualLib::load();
    unsafe {
        dl.seed_both(99999);

        type HmPutKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmGetKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmFreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_put: Symbol<HmPutKeyFn> = dl.c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmPutKeyFn> = dl.rust_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<HmGetKeyFn> = dl.c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<HmGetKeyFn> = dl.rust_lib.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<HmFreeFn> = dl.c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmFreeFn> = dl.rust_lib.get(b"stbds_hmfree_func").unwrap();

        // Struct: { key: *const c_char, value: i32 } => elemsize=16 (8+4+4pad), keysize=8
        let elemsize: usize = 16;
        let keysize: usize = 8;

        let mut c_a: *mut c_void = ptr::null_mut();
        let mut r_a: *mut c_void = ptr::null_mut();

        // Insert "hello" => 42
        let key1 = CString::new("hello").unwrap();
        c_a = c_put(c_a, elemsize, key1.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);
        r_a = r_put(r_a, elemsize, key1.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_temp = (*get_header(c_raw)).temp;
        let r_temp = (*get_header(r_raw)).temp;
        assert_eq!(c_temp, r_temp, "string put temp should match");

        // Write value at index (value is at offset 8 within the element)
        let c_val_ptr = (c_a as *mut u8).offset(elemsize as isize * c_temp).add(8) as *mut i32;
        *c_val_ptr = 42;
        let r_val_ptr = (r_a as *mut u8).offset(elemsize as isize * r_temp).add(8) as *mut i32;
        *r_val_ptr = 42;

        // Insert "world" => 99
        let key2 = CString::new("world").unwrap();
        c_a = c_put(c_a, elemsize, key2.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);
        r_a = r_put(r_a, elemsize, key2.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_temp = (*get_header(c_raw)).temp;
        let r_temp = (*get_header(r_raw)).temp;
        assert_eq!(c_temp, r_temp);

        let c_val_ptr = (c_a as *mut u8).offset(elemsize as isize * c_temp).add(8) as *mut i32;
        *c_val_ptr = 99;
        let r_val_ptr = (r_a as *mut u8).offset(elemsize as isize * r_temp).add(8) as *mut i32;
        *r_val_ptr = 99;

        // Look up "hello"
        c_a = c_get(c_a, elemsize, key1.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);
        r_a = r_get(r_a, elemsize, key1.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_idx = (*get_header(c_raw)).temp;
        let r_idx = (*get_header(r_raw)).temp;
        assert_eq!(c_idx, r_idx, "get index for 'hello' should match");
        assert!(c_idx >= 0);

        let c_val = *((c_a as *mut u8).offset(elemsize as isize * c_idx).add(8) as *mut i32);
        let r_val = *((r_a as *mut u8).offset(elemsize as isize * r_idx).add(8) as *mut i32);
        assert_eq!(c_val, 42);
        assert_eq!(r_val, 42);

        // Look up non-existent "missing"
        let key_miss = CString::new("missing").unwrap();
        c_a = c_get(c_a, elemsize, key_miss.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);
        r_a = r_get(r_a, elemsize, key_miss.as_ptr() as *mut c_void, keysize, STBDS_HM_STRING);

        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).temp, (*get_header(r_raw)).temp);
        assert_eq!((*get_header(c_raw)).temp, -1);

        c_free((c_a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        r_free((r_a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ============================================================
// Test 8: stbds_hmdel_key (BINARY mode)
// ============================================================
#[test]
fn test_hmdel() {
    let dl = DualLib::load();
    unsafe {
        dl.seed_both(77777);

        type HmPutKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmGetKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
        type HmDelKeyFn = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
        type HmFreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_put: Symbol<HmPutKeyFn> = dl.c_lib.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<HmPutKeyFn> = dl.rust_lib.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<HmGetKeyFn> = dl.c_lib.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<HmGetKeyFn> = dl.rust_lib.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<HmDelKeyFn> = dl.c_lib.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<HmDelKeyFn> = dl.rust_lib.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<HmFreeFn> = dl.c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmFreeFn> = dl.rust_lib.get(b"stbds_hmfree_func").unwrap();

        let elemsize: usize = 8; // {i32 key, i32 value}
        let keysize: usize = 4;
        let keyoffset: usize = 0;

        let mut c_a: *mut c_void = ptr::null_mut();
        let mut r_a: *mut c_void = ptr::null_mut();

        // Insert keys 1, 2, 3
        for k in [1i32, 2, 3] {
            let mut key = k;
            c_a = c_put(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
            r_a = r_put(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);

            let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
            let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
            let c_temp = (*get_header(c_raw)).temp;
            let r_temp = (*get_header(r_raw)).temp;

            // Write key and value
            let c_elem = (c_a as *mut u8).offset(elemsize as isize * c_temp) as *mut i32;
            *c_elem = k;
            *c_elem.add(1) = k * 10;
            let r_elem = (r_a as *mut u8).offset(elemsize as isize * r_temp) as *mut i32;
            *r_elem = k;
            *r_elem.add(1) = k * 10;
        }

        // Delete key=2
        let mut key: i32 = 2;
        c_a = c_del(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, keyoffset, STBDS_HM_BINARY);
        r_a = r_del(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, keyoffset, STBDS_HM_BINARY);

        // temp should be 1 (deleted=true) for both
        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).temp, (*get_header(r_raw)).temp);
        assert_eq!((*get_header(c_raw)).temp, 1, "del should return 1 for found key");

        // Verify key=2 is gone
        key = 2;
        c_a = c_get(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        r_a = r_get(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).temp, -1, "key=2 should be gone in C");
        assert_eq!((*get_header(r_raw)).temp, -1, "key=2 should be gone in Rust");

        // Verify key=1 still exists
        key = 1;
        c_a = c_get(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        r_a = r_get(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, STBDS_HM_BINARY);
        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        let c_idx = (*get_header(c_raw)).temp;
        let r_idx = (*get_header(r_raw)).temp;
        assert!(c_idx >= 0, "key=1 should still exist in C");
        assert!(r_idx >= 0, "key=1 should still exist in Rust");

        // Delete non-existent key=99
        key = 99;
        c_a = c_del(c_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, keyoffset, STBDS_HM_BINARY);
        r_a = r_del(r_a, elemsize, &mut key as *mut i32 as *mut c_void, keysize, keyoffset, STBDS_HM_BINARY);
        let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;
        assert_eq!((*get_header(c_raw)).temp, 0, "del non-existent should return 0");
        assert_eq!((*get_header(r_raw)).temp, 0);

        c_free((c_a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        r_free((r_a as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ============================================================
// Test 9: stbds_shmode_func
// ============================================================
#[test]
fn test_shmode_func() {
    let dl = DualLib::load();
    unsafe {
        dl.seed_both(55555);

        type ShModeFn = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
        type HmFreeFn = unsafe extern "C" fn(*mut c_void, usize);

        let c_shmode: Symbol<ShModeFn> = dl.c_lib.get(b"stbds_shmode_func").unwrap();
        let r_shmode: Symbol<ShModeFn> = dl.rust_lib.get(b"stbds_shmode_func").unwrap();
        let c_free: Symbol<HmFreeFn> = dl.c_lib.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<HmFreeFn> = dl.rust_lib.get(b"stbds_hmfree_func").unwrap();

        // elemsize=16 for {char* key, int value} with padding
        let elemsize: usize = 16;

        for mode in [1i32, 2, 3] {
            // STBDS_SH_DEFAULT=1, STBDS_SH_STRDUP=2, STBDS_SH_ARENA=3
            let c_a = c_shmode(elemsize, mode);
            let r_a = r_shmode(elemsize, mode);

            assert!(!c_a.is_null());
            assert!(!r_a.is_null());

            // The returned pointer is ARR_TO_HASH(a, elemsize) = a + elemsize
            // raw_a = returned - elemsize
            let c_raw = (c_a as *mut u8).sub(elemsize) as *mut c_void;
            let r_raw = (r_a as *mut u8).sub(elemsize) as *mut c_void;

            let c_hdr = &*get_header(c_raw);
            let r_hdr = &*get_header(r_raw);

            assert_eq!(c_hdr.length, r_hdr.length, "length should match for mode={}", mode);
            assert_eq!(c_hdr.length, 1, "length should be 1 (default element)");
            assert!(!c_hdr.hash_table.is_null(), "hash_table should be set");
            assert!(!r_hdr.hash_table.is_null(), "hash_table should be set");

            c_free(c_raw, elemsize);
            r_free(r_raw, elemsize);
        }
    }
}

// ============================================================
// Test 10: str_put (high-level, smoke test)
// ============================================================
#[test]
fn test_str_put() {
    let dl = DualLib::load();
    unsafe {
        dl.seed_both(11111);

        let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
            dl.c_lib.get(b"str_put").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int)> =
            dl.rust_lib.get(b"str_put").unwrap();

        // C version should work correctly
        c_fn(3);
        c_fn(0);

        // Rust version: call in a subprocess to detect crashes without aborting
        // the test runner. str_put uses shputs which involves complex macro
        // expansion; the Rust translation may have bugs here.
        use std::process::Command;
        let _rust_lib = rust_lib_path();
        let test_bin = std::env::current_exe().unwrap();

        // We use a helper env var to run the Rust str_put in a child process
        if std::env::var("__FFI_TEST_STR_PUT_RUST").is_ok() {
            r_fn(3);
            return;
        }

        let output = Command::new(&test_bin)
            .env("__FFI_TEST_STR_PUT_RUST", "1")
            .arg("--test-threads=1")
            .arg("test_str_put")
            .arg("--exact")
            .output()
            .expect("failed to spawn subprocess");

        if !output.status.success() {
            eprintln!(
                "NOTE: Rust str_put crashed (exit={:?}). This indicates a translation bug in str_put.",
                output.status.code()
            );
            // Fail the test to flag the translation discrepancy
            panic!(
                "Rust str_put diverges from C: subprocess exited with {:?}",
                output.status
            );
        }
    }
}
