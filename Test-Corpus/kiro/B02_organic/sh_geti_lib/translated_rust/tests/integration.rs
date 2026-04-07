use libloading::{Library, Symbol};
use std::ffi::c_int;

const C_LIB: &str = env!("C_LIB_PATH");
const RUST_LIB: &str = env!("RUST_LIB_PATH");

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(C_LIB).expect("load C .so");
        let r = Library::new(RUST_LIB).expect("load Rust .so");
        (c, r)
    }
}

// ---- stbds_rand_seed + stbds_hash_string ----
#[test]
fn test_hash_string() {
    let (c, r) = load_libs();
    unsafe {
        let c_hs: Symbol<unsafe extern "C" fn(*const u8, usize) -> usize> = c.get(b"stbds_hash_string").unwrap();
        let r_hs: Symbol<unsafe extern "C" fn(*const u8, usize) -> usize> = r.get(b"stbds_hash_string").unwrap();

        for seed in [0usize, 1, 42, 0xdeadbeef, usize::MAX] {
            for s in [b"hello\0".as_ptr(), b"\0".as_ptr(), b"test_123\0".as_ptr(), b"a\0".as_ptr()] {
                let cv = c_hs(s, seed);
                let rv = r_hs(s, seed);
                assert_eq!(cv, rv, "hash_string mismatch for seed={seed}");
            }
        }
    }
}

// ---- stbds_hash_bytes ----
#[test]
fn test_hash_bytes() {
    let (c, r) = load_libs();
    unsafe {
        let c_hb: Symbol<unsafe extern "C" fn(*const u8, usize, usize) -> usize> = c.get(b"stbds_hash_bytes").unwrap();
        let r_hb: Symbol<unsafe extern "C" fn(*const u8, usize, usize) -> usize> = r.get(b"stbds_hash_bytes").unwrap();

        for seed in [0usize, 1, 42, 0xdeadbeef] {
            // Various lengths to exercise all switch cases (0..8+)
            for len in 0..=16 {
                let data: Vec<u8> = (0..len).map(|i| (i * 37 + 13) as u8).collect();
                let cv = c_hb(data.as_ptr(), len, seed);
                let rv = r_hb(data.as_ptr(), len, seed);
                assert_eq!(cv, rv, "hash_bytes mismatch len={len} seed={seed}");
            }
        }
    }
}

// ---- strkey ----
#[test]
fn test_strkey() {
    let (c, r) = load_libs();
    unsafe {
        let c_sk: Symbol<unsafe extern "C" fn(c_int) -> *const u8> = c.get(b"strkey").unwrap();
        let r_sk: Symbol<unsafe extern "C" fn(c_int) -> *const u8> = r.get(b"strkey").unwrap();

        for i in [0, 1, 5, 10, 100, 999] {
            let cp = c_sk(i);
            let rp = r_sk(i);
            let cs = std::ffi::CStr::from_ptr(cp as *const i8);
            let rs = std::ffi::CStr::from_ptr(rp as *const i8);
            assert_eq!(cs, rs, "strkey mismatch for i={i}");
        }
    }
}

// ---- sh_geti (the main integration test) ----
// sh_geti uses internal global state (stbds_hash_seed). Since C and Rust
// are separate .so files with separate globals, we just verify both
// complete without assertion failures for the same inputs.
#[test]
fn test_sh_geti_c() {
    let (c, _r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int)> = c.get(b"sh_geti").unwrap();
        for n in [0, 1, 2, 4, 8, 16, 32] {
            c_fn(n);
        }
    }
}

#[test]
fn test_sh_geti_rust() {
    let (_c, r) = load_libs();
    unsafe {
        let r_fn: Symbol<unsafe extern "C" fn(c_int)> = r.get(b"sh_geti").unwrap();
        for n in [0, 1, 2, 4, 8, 16, 32] {
            r_fn(n);
        }
    }
}

#[test]
fn test_sh_geti_medium_c() {
    let (c, _r) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int)> = c.get(b"sh_geti").unwrap();
        for n in [64, 100] {
            c_fn(n);
        }
    }
}

#[test]
fn test_sh_geti_medium_rust() {
    let (_c, r) = load_libs();
    unsafe {
        let r_fn: Symbol<unsafe extern "C" fn(c_int)> = r.get(b"sh_geti").unwrap();
        for n in [64, 100] {
            r_fn(n);
        }
    }
}

// ---- Low-level hash map operations ----
// Test stbds_hmput_key + stbds_hmget_key with binary keys
#[test]
fn test_hm_binary_put_get() {
    let (c, r) = load_libs();
    unsafe {

        // Element layout: { int key; int value; } = 8 bytes, key at offset 0
        let elemsize: usize = 8;
        let keysize: usize = 4;
        let mode: c_int = 0; // STBDS_HM_BINARY

        type PutFn = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> *mut u8;
        type GetFn = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);
        type DefaultFn = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;

        let c_put: Symbol<PutFn> = c.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<GetFn> = c.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<FreeFn> = c.get(b"stbds_hmfree_func").unwrap();
        let c_default: Symbol<DefaultFn> = c.get(b"stbds_hmput_default").unwrap();

        let r_put: Symbol<PutFn> = r.get(b"stbds_hmput_key").unwrap();
        let r_get: Symbol<GetFn> = r.get(b"stbds_hmget_key").unwrap();
        let r_free: Symbol<FreeFn> = r.get(b"stbds_hmfree_func").unwrap();
        let r_default: Symbol<DefaultFn> = r.get(b"stbds_hmput_default").unwrap();

        // stbds_array_header is 32 bytes on 64-bit
        let header_size: usize = 32;

        let mut c_map: *mut u8 = std::ptr::null_mut();
        let mut r_map: *mut u8 = std::ptr::null_mut();

        // Set default value to -1
        c_map = c_default(c_map, elemsize);
        r_map = r_default(r_map, elemsize);
        // Write default value: map[-1].value = -1
        let c_def_elem = c_map.sub(elemsize);
        let r_def_elem = r_map.sub(elemsize);
        *(c_def_elem.add(4) as *mut i32) = -1;
        *(r_def_elem.add(4) as *mut i32) = -1;

        // Put some keys
        for i in 0..20i32 {
            let key = i;
            c_map = c_put(c_map, elemsize, &key as *const i32 as *const u8, keysize, mode);
            r_map = r_put(r_map, elemsize, &key as *const i32 as *const u8, keysize, mode);

            // Read temp from header to find where value goes
            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_hdr = (c_raw as *mut usize).offset(-4); // header fields
            let r_hdr = (r_raw as *mut usize).offset(-4);
            let c_temp = *((c_raw as *mut isize).offset(-1)); // temp is last field in header
            let r_temp = *((r_raw as *mut isize).offset(-1));
            assert_eq!(c_temp, r_temp, "temp mismatch after put key={i}");

            // Set value
            *(c_map.add(elemsize * c_temp as usize).add(4) as *mut i32) = i * 10;
            *(r_map.add(elemsize * r_temp as usize).add(4) as *mut i32) = i * 10;
        }

        // Get keys and compare temp values
        for i in 0..20i32 {
            let key = i;
            let c_res = c_get(c_map, elemsize, &key as *const i32 as *const u8, keysize, mode);
            let r_res = r_get(r_map, elemsize, &key as *const i32 as *const u8, keysize, mode);
            c_map = c_res;
            r_map = r_res;

            let c_raw = c_map.sub(elemsize);
            let r_raw = r_map.sub(elemsize);
            let c_temp = *((c_raw as *mut isize).offset(-1));
            let r_temp = *((r_raw as *mut isize).offset(-1));
            assert_eq!(c_temp, r_temp, "get temp mismatch for key={i}");
            assert!(c_temp >= 0, "key {i} not found");

            let c_val = *(c_map.add(elemsize * c_temp as usize).add(4) as *const i32);
            let r_val = *(r_map.add(elemsize * r_temp as usize).add(4) as *const i32);
            assert_eq!(c_val, r_val, "value mismatch for key={i}");
        }

        // Get a key that doesn't exist
        let missing: i32 = 999;
        c_map = c_get(c_map, elemsize, &missing as *const i32 as *const u8, keysize, mode);
        r_map = r_get(r_map, elemsize, &missing as *const i32 as *const u8, keysize, mode);
        let c_temp = *((c_map.sub(elemsize) as *mut isize).offset(-1));
        let r_temp = *((r_map.sub(elemsize) as *mut isize).offset(-1));
        assert_eq!(c_temp, r_temp, "missing key temp mismatch");
        assert_eq!(c_temp, -1);

        // Free
        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ---- Test string hash map put/get/del cycle ----
#[test]
fn test_shmap_put_get_del() {
    let (c, r) = load_libs();
    unsafe {

        type ModeFn = unsafe extern "C" fn(usize, c_int) -> *mut u8;
        type PutFn = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> *mut u8;
        type GetFn = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> *mut u8;
        type DelFn = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, usize, c_int) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);
        type DefaultFn = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;

        let elemsize: usize = 16; // { char *key; int value; } with padding
        let keysize: usize = 8;   // sizeof(char*)
        let mode: c_int = 1;      // STBDS_HM_STRING

        let c_mode: Symbol<ModeFn> = c.get(b"stbds_shmode_func").unwrap();
        let c_put: Symbol<PutFn> = c.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<GetFn> = c.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<DelFn> = c.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<FreeFn> = c.get(b"stbds_hmfree_func").unwrap();
        let c_default: Symbol<DefaultFn> = c.get(b"stbds_hmput_default").unwrap();

        let r_mode: Symbol<ModeFn> = r.get(b"stbds_shmode_func").unwrap();
        let r_put: Symbol<PutFn> = r.get(b"stbds_hmput_key").unwrap();
        let r_get: Symbol<GetFn> = r.get(b"stbds_hmget_key").unwrap();
        let r_del: Symbol<DelFn> = r.get(b"stbds_hmdel_key").unwrap();
        let r_free: Symbol<FreeFn> = r.get(b"stbds_hmfree_func").unwrap();
        let r_default: Symbol<DefaultFn> = r.get(b"stbds_hmput_default").unwrap();

        // STBDS_SH_STRDUP = 2
        let mut c_map = c_mode(elemsize, 2);
        let mut r_map = r_mode(elemsize, 2);

        // Set default
        c_map = c_default(c_map, elemsize);
        r_map = r_default(r_map, elemsize);
        *(c_map.sub(elemsize).add(8) as *mut i32) = -99;
        *(r_map.sub(elemsize).add(8) as *mut i32) = -99;

        // Put keys
        let keys: Vec<Vec<u8>> = (0..10).map(|i| format!("key_{i}\0").into_bytes()).collect();
        for (i, k) in keys.iter().enumerate() {
            c_map = c_put(c_map, elemsize, k.as_ptr(), keysize, mode);
            r_map = r_put(r_map, elemsize, k.as_ptr(), keysize, mode);
            let c_temp = *((c_map.sub(elemsize) as *mut isize).offset(-1));
            let r_temp = *((r_map.sub(elemsize) as *mut isize).offset(-1));
            assert_eq!(c_temp, r_temp, "shput temp mismatch key={i}");
            *(c_map.add(elemsize * c_temp as usize).add(8) as *mut i32) = i as i32 * 7;
            *(r_map.add(elemsize * r_temp as usize).add(8) as *mut i32) = i as i32 * 7;
        }

        // Get keys
        for (i, k) in keys.iter().enumerate() {
            c_map = c_get(c_map, elemsize, k.as_ptr(), keysize, mode);
            r_map = r_get(r_map, elemsize, k.as_ptr(), keysize, mode);
            let c_temp = *((c_map.sub(elemsize) as *mut isize).offset(-1));
            let r_temp = *((r_map.sub(elemsize) as *mut isize).offset(-1));
            assert_eq!(c_temp, r_temp, "shget temp mismatch key={i}");
            let c_val = *(c_map.add(elemsize * c_temp as usize).add(8) as *const i32);
            let r_val = *(r_map.add(elemsize * r_temp as usize).add(8) as *const i32);
            assert_eq!(c_val, r_val, "shget value mismatch key={i}");
        }

        // Delete some keys
        for i in (0..10).step_by(2) {
            c_map = c_del(c_map, elemsize, keys[i].as_ptr(), keysize, 0, mode);
            r_map = r_del(r_map, elemsize, keys[i].as_ptr(), keysize, 0, mode);
            let c_temp = *((c_map.sub(elemsize) as *mut isize).offset(-1));
            let r_temp = *((r_map.sub(elemsize) as *mut isize).offset(-1));
            assert_eq!(c_temp, r_temp, "shdel temp mismatch key={i}");
        }

        // Verify deleted keys return -1, others still found
        for (i, k) in keys.iter().enumerate() {
            c_map = c_get(c_map, elemsize, k.as_ptr(), keysize, mode);
            r_map = r_get(r_map, elemsize, k.as_ptr(), keysize, mode);
            let c_temp = *((c_map.sub(elemsize) as *mut isize).offset(-1));
            let r_temp = *((r_map.sub(elemsize) as *mut isize).offset(-1));
            assert_eq!(c_temp, r_temp, "post-del get temp mismatch key={i}");
        }

        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ---- Test stbds_stralloc + stbds_strreset ----
#[test]
fn test_stralloc_strreset() {
    let (c, r) = load_libs();
    unsafe {
        // We can't easily compare arena pointers, but we can verify the strings are correct
        type AllocFn = unsafe extern "C" fn(*mut u8, *const u8) -> *mut u8;
        type ResetFn = unsafe extern "C" fn(*mut u8);

        let c_alloc: Symbol<AllocFn> = c.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<AllocFn> = r.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<ResetFn> = c.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<ResetFn> = r.get(b"stbds_strreset").unwrap();

        // Arena struct is: { *storage, remaining: usize, block: u8, mode: u8 }
        // On 64-bit: 8 + 8 + 1 + 1 + padding = 24 bytes (but let's use the actual size)
        // Actually: ptr(8) + usize(8) + u8(1) + u8(1) + padding(6) = 24
        let arena_size = 24usize;
        let mut c_arena = vec![0u8; arena_size];
        let mut r_arena = vec![0u8; arena_size];

        let strings = [b"hello\0".as_ptr(), b"world\0".as_ptr(), b"test_string_123\0".as_ptr()];
        for s in &strings {
            let cp = c_alloc(c_arena.as_mut_ptr(), *s);
            let rp = r_alloc(r_arena.as_mut_ptr(), *s);
            // Compare the stored string content
            let cs = std::ffi::CStr::from_ptr(cp as *const i8);
            let rs = std::ffi::CStr::from_ptr(rp as *const i8);
            assert_eq!(cs, rs, "stralloc content mismatch");
        }

        c_reset(c_arena.as_mut_ptr());
        r_reset(r_arena.as_mut_ptr());
        // After reset, arena should be zeroed
        assert_eq!(c_arena, r_arena, "strreset state mismatch");
    }
}

// ---- Test arrgrowf + arrfreef ----
#[test]
fn test_arrgrowf_arrfreef() {
    let (c, r) = load_libs();
    unsafe {
        type GrowFn = unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8);

        let c_grow: Symbol<GrowFn> = c.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<GrowFn> = r.get(b"stbds_arrgrowf").unwrap();
        let c_freef: Symbol<FreeFn> = c.get(b"stbds_arrfreef").unwrap();
        let r_freef: Symbol<FreeFn> = r.get(b"stbds_arrfreef").unwrap();

        // Grow from null
        let elemsize = 4usize;
        let c_arr = c_grow(std::ptr::null_mut(), elemsize, 0, 10);
        let r_arr = r_grow(std::ptr::null_mut(), elemsize, 0, 10);

        // Header is at arr - 32 bytes. Check length, capacity, hash_table, temp
        let c_hdr = c_arr.sub(32);
        let r_hdr = r_arr.sub(32);
        // length (offset 0)
        assert_eq!(*(c_hdr as *const usize), *(r_hdr as *const usize), "length mismatch");
        // capacity (offset 8)
        assert_eq!(*(c_hdr.add(8) as *const usize), *(r_hdr.add(8) as *const usize), "capacity mismatch");
        // hash_table should be null (offset 16)
        assert_eq!(*(c_hdr.add(16) as *const usize), 0, "C hash_table not null");
        assert_eq!(*(r_hdr.add(16) as *const usize), 0, "Rust hash_table not null");
        // temp (offset 24)
        assert_eq!(*(c_hdr.add(24) as *const isize), *(r_hdr.add(24) as *const isize), "temp mismatch");

        // Grow again
        let c_arr2 = c_grow(c_arr, elemsize, 5, 0);
        let r_arr2 = r_grow(r_arr, elemsize, 5, 0);
        let c_hdr2 = c_arr2.sub(32);
        let r_hdr2 = r_arr2.sub(32);
        assert_eq!(*(c_hdr2.add(8) as *const usize), *(r_hdr2.add(8) as *const usize), "capacity mismatch after grow");

        c_freef(c_arr2);
        r_freef(r_arr2);
    }
}
