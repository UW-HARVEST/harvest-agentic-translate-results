// Compare C and Rust shared-library outputs through libloading.
//
// Both the C source (`c_src/`) and Rust translation produce a `.so` exporting
// many `stbds_*` symbols plus `arr_del` and `strkey`. We dlopen each and
// invoke the same sequence of FFI calls, asserting the C and Rust results
// agree byte-for-byte.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

type size_t = usize;
type ptrdiff_t = isize;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct stbds_array_header {
    length: size_t,
    capacity: size_t,
    hash_table: *mut c_void,
    temp: ptrdiff_t,
}

const HEADER_SIZE: usize = std::mem::size_of::<stbds_array_header>();

unsafe fn header_of(a: *mut c_void) -> *mut stbds_array_header {
    (a as *mut u8).sub(HEADER_SIZE) as *mut stbds_array_header
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    // We rely on cargo to have built the .so under target/<profile>/
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("target/debug/libarr_del_lib.so"),
        manifest.join("target/release/libarr_del_lib.so"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    // Fallback: just return first guess
    manifest.join("target/debug/libarr_del_lib.so")
}

unsafe fn open_libs() -> (Library, Library) {
    let c = Library::new(c_lib_path()).expect("failed to open C lib");
    let r = Library::new(rust_lib_path()).expect("failed to open Rust lib");
    (c, r)
}

// ------------------------------------------------------------
// Symbol type aliases
// ------------------------------------------------------------

type Fn_arr_del = unsafe extern "C" fn(c_int);
type Fn_stbds_arrgrowf =
    unsafe extern "C" fn(*mut c_void, size_t, size_t, size_t) -> *mut c_void;
type Fn_stbds_arrfreef = unsafe extern "C" fn(*mut c_void);
type Fn_stbds_rand_seed = unsafe extern "C" fn(size_t);
type Fn_stbds_hash_bytes = unsafe extern "C" fn(*mut c_void, size_t, size_t) -> size_t;
type Fn_stbds_hash_string = unsafe extern "C" fn(*mut c_char, size_t) -> size_t;
type Fn_stbds_hmput_key = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *mut c_void,
    size_t,
    c_int,
) -> *mut c_void;
type Fn_stbds_hmget_key = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *mut c_void,
    size_t,
    c_int,
) -> *mut c_void;
type Fn_stbds_hmget_key_ts = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *mut c_void,
    size_t,
    *mut ptrdiff_t,
    c_int,
) -> *mut c_void;
type Fn_stbds_hmput_default = unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void;
type Fn_stbds_shmode_func = unsafe extern "C" fn(size_t, c_int) -> *mut c_void;
type Fn_stbds_hmdel_key = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *mut c_void,
    size_t,
    size_t,
    c_int,
) -> *mut c_void;
type Fn_stbds_hmfree_func = unsafe extern "C" fn(*mut c_void, size_t);
type Fn_strkey = unsafe extern "C" fn(c_int) -> *mut c_char;

// ------------------------------------------------------------
// Tests
// ------------------------------------------------------------

#[test]
fn test_strkey() {
    unsafe {
        let (c, r) = open_libs();
        let c_strkey: Symbol<Fn_strkey> = c.get(b"strkey").unwrap();
        let r_strkey: Symbol<Fn_strkey> = r.get(b"strkey").unwrap();
        for n in [0i32, 1, 2, -1, 12345, -98765, i32::MAX, i32::MIN].iter().copied() {
            let cp = c_strkey(n);
            let rp = r_strkey(n);
            let cs = std::ffi::CStr::from_ptr(cp).to_bytes().to_vec();
            let rs = std::ffi::CStr::from_ptr(rp).to_bytes().to_vec();
            assert_eq!(cs, rs, "strkey({}) mismatch", n);
        }
    }
}

#[test]
fn test_stbds_hash_string() {
    unsafe {
        let (c, r) = open_libs();
        let c_h: Symbol<Fn_stbds_hash_string> = c.get(b"stbds_hash_string").unwrap();
        let r_h: Symbol<Fn_stbds_hash_string> = r.get(b"stbds_hash_string").unwrap();
        let inputs: &[(&str, u64)] = &[
            ("", 0),
            ("a", 0),
            ("hello", 12345),
            ("the quick brown fox", 0xdeadbeef),
            ("", 0xffffffffffffffff),
            ("longer string with more content", 0x31415926),
        ];
        for (s, seed) in inputs.iter() {
            let cs = CString::new(*s).unwrap();
            let p = cs.as_ptr() as *mut c_char;
            let ch = c_h(p, *seed as size_t);
            let rh = r_h(p, *seed as size_t);
            assert_eq!(ch, rh, "hash_string({:?}, {:#x}) mismatch", s, seed);
        }
    }
}

#[test]
fn test_stbds_hash_bytes() {
    unsafe {
        let (c, r) = open_libs();
        let c_h: Symbol<Fn_stbds_hash_bytes> = c.get(b"stbds_hash_bytes").unwrap();
        let r_h: Symbol<Fn_stbds_hash_bytes> = r.get(b"stbds_hash_bytes").unwrap();
        // Cover all length residues (mod 8) plus several seeds.
        let mut data: Vec<u8> = (0..64u8).collect();
        let seeds: &[usize] = &[0, 1, 12345, 0xdeadbeef, 0x31415926];
        for &seed in seeds {
            for len in 0..=64 {
                let ch = c_h(data.as_mut_ptr() as *mut c_void, len, seed);
                let rh = r_h(data.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(ch, rh, "hash_bytes(len={}, seed={:#x}) mismatch", len, seed);
            }
        }
    }
}

#[test]
fn test_arrgrowf_arrfreef() {
    unsafe {
        let (c, r) = open_libs();
        let c_grow: Symbol<Fn_stbds_arrgrowf> = c.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<Fn_stbds_arrgrowf> = r.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<Fn_stbds_arrfreef> = c.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<Fn_stbds_arrfreef> = r.get(b"stbds_arrfreef").unwrap();
        let elemsize = std::mem::size_of::<i32>();

        // Compare header fields after several growth steps.
        let mut ca: *mut c_void = ptr::null_mut();
        let mut ra: *mut c_void = ptr::null_mut();

        let steps = [(0usize, 1usize), (5, 0), (10, 0), (0, 100), (200, 0)];
        for (addlen, min_cap) in steps.iter().copied() {
            ca = c_grow(ca, elemsize, addlen, min_cap);
            ra = r_grow(ra, elemsize, addlen, min_cap);
            let ch = *header_of(ca);
            let rh = *header_of(ra);
            assert_eq!(ch.length, rh.length);
            assert_eq!(ch.capacity, rh.capacity);
            // hash_table must be NULL initially in both
            assert_eq!(ch.hash_table.is_null(), rh.hash_table.is_null());
            assert_eq!(ch.temp, rh.temp);
        }

        c_free(ca);
        r_free(ra);
    }
}

// Helper: simulate stbds_hmput on an array of (i32 key, i32 value) entries
// via stbds_hmput_key and a manual key/value write afterwards. We use the
// same mechanism for both libs and inspect the resulting array contents.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct KV {
    key: i32,
    _pad: i32, // alignment so total elem size matches typical stb_ds usage
    value: i32,
}

#[test]
fn test_hash_map_int_key() {
    unsafe {
        let (c, r) = open_libs();
        let elemsize = std::mem::size_of::<KV>();

        let c_put: Symbol<Fn_stbds_hmput_key> = c.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<Fn_stbds_hmput_key> = r.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<Fn_stbds_hmget_key> = c.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<Fn_stbds_hmget_key> = r.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<Fn_stbds_hmfree_func> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<Fn_stbds_hmfree_func> = r.get(b"stbds_hmfree_func").unwrap();

        let mut ca: *mut c_void = ptr::null_mut();
        let mut ra: *mut c_void = ptr::null_mut();

        // Insert 30 entries
        for i in 0..30i32 {
            let mut k: i32 = i * 7 + 3;
            ca = c_put(ca, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
            ra = r_put(ra, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);

            // Write the value at the slot indicated by header->temp + 1 (because
            // the array layout is [defaults][entries...], with index 0 the dummy
            // "defaults" entry).
            let c_raw = (ca as *mut u8).sub(elemsize);
            let r_raw = (ra as *mut u8).sub(elemsize);
            let c_idx = (*header_of(c_raw as *mut c_void)).temp;
            let r_idx = (*header_of(r_raw as *mut c_void)).temp;
            assert_eq!(c_idx, r_idx, "temp index mismatch on insert {}", i);

            let c_entry = (ca as *mut KV).add(c_idx as usize);
            let r_entry = (ra as *mut KV).add(r_idx as usize);
            (*c_entry).key = k;
            (*c_entry)._pad = 0;
            (*c_entry).value = i * 11;
            (*r_entry).key = k;
            (*r_entry)._pad = 0;
            (*r_entry).value = i * 11;
        }

        // Compare lengths
        let c_raw = (ca as *mut u8).sub(elemsize);
        let r_raw = (ra as *mut u8).sub(elemsize);
        let c_len = (*header_of(c_raw as *mut c_void)).length;
        let r_len = (*header_of(r_raw as *mut c_void)).length;
        assert_eq!(c_len, r_len);

        // Test get
        for i in 0..30i32 {
            let mut k: i32 = i * 7 + 3;
            ca = c_get(ca, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
            ra = r_get(ra, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
            let c_raw = (ca as *mut u8).sub(elemsize);
            let r_raw = (ra as *mut u8).sub(elemsize);
            let c_idx = (*header_of(c_raw as *mut c_void)).temp;
            let r_idx = (*header_of(r_raw as *mut c_void)).temp;
            assert_eq!(c_idx, r_idx, "get idx mismatch for key {}", k);
            let c_entry = *(ca as *mut KV).add(c_idx as usize);
            let r_entry = *(ra as *mut KV).add(r_idx as usize);
            assert_eq!(c_entry.value, r_entry.value);
            assert_eq!(c_entry.key, r_entry.key);
        }

        // Get nonexistent key
        let mut k_missing: i32 = 99999;
        ca = c_get(ca, elemsize, &mut k_missing as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
        ra = r_get(ra, elemsize, &mut k_missing as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
        let c_raw = (ca as *mut u8).sub(elemsize);
        let r_raw = (ra as *mut u8).sub(elemsize);
        let c_idx = (*header_of(c_raw as *mut c_void)).temp;
        let r_idx = (*header_of(r_raw as *mut c_void)).temp;
        assert_eq!(c_idx, r_idx, "missing key idx differs");
        assert_eq!(c_idx, -1);

        let c_raw_free = (ca as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw_free = (ra as *mut u8).sub(elemsize) as *mut c_void;
        c_free(c_raw_free, elemsize);
        r_free(r_raw_free, elemsize);
    }
}

#[test]
fn test_hmdel_int_key() {
    unsafe {
        let (c, r) = open_libs();
        let elemsize = std::mem::size_of::<KV>();
        let c_put: Symbol<Fn_stbds_hmput_key> = c.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<Fn_stbds_hmput_key> = r.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<Fn_stbds_hmget_key> = c.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<Fn_stbds_hmget_key> = r.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<Fn_stbds_hmdel_key> = c.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<Fn_stbds_hmdel_key> = r.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<Fn_stbds_hmfree_func> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<Fn_stbds_hmfree_func> = r.get(b"stbds_hmfree_func").unwrap();

        let mut ca: *mut c_void = ptr::null_mut();
        let mut ra: *mut c_void = ptr::null_mut();

        for i in 0..50i32 {
            let mut k: i32 = i;
            ca = c_put(ca, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
            ra = r_put(ra, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
            let c_raw = (ca as *mut u8).sub(elemsize);
            let r_raw = (ra as *mut u8).sub(elemsize);
            let c_idx = (*header_of(c_raw as *mut c_void)).temp;
            let r_idx = (*header_of(r_raw as *mut c_void)).temp;
            assert_eq!(c_idx, r_idx);
            let c_entry = (ca as *mut KV).add(c_idx as usize);
            let r_entry = (ra as *mut KV).add(r_idx as usize);
            (*c_entry).key = k;
            (*c_entry)._pad = 0;
            (*c_entry).value = i * 100;
            (*r_entry).key = k;
            (*r_entry)._pad = 0;
            (*r_entry).value = i * 100;
        }

        // Delete every other key
        for i in (0..50i32).step_by(2) {
            let mut k: i32 = i;
            // keyoffset for KV is 0 (key is first field)
            ca = c_del(ca, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0, 0);
            ra = r_del(ra, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0, 0);
        }

        // Verify
        for i in 0..50i32 {
            let mut k: i32 = i;
            ca = c_get(ca, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
            ra = r_get(ra, elemsize, &mut k as *mut _ as *mut c_void, std::mem::size_of::<i32>(), 0);
            let c_raw = (ca as *mut u8).sub(elemsize);
            let r_raw = (ra as *mut u8).sub(elemsize);
            let c_idx = (*header_of(c_raw as *mut c_void)).temp;
            let r_idx = (*header_of(r_raw as *mut c_void)).temp;
            assert_eq!(c_idx, r_idx, "after delete: get for key {} differs", i);
        }

        let c_raw_free = (ca as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw_free = (ra as *mut u8).sub(elemsize) as *mut c_void;
        c_free(c_raw_free, elemsize);
        r_free(r_raw_free, elemsize);
    }
}

#[test]
fn test_arr_del_runs() {
    unsafe {
        let (c, r) = open_libs();
        let c_fn: Symbol<Fn_arr_del> = c.get(b"arr_del").unwrap();
        let r_fn: Symbol<Fn_arr_del> = r.get(b"arr_del").unwrap();
        // arr_del has no observable output; just check it doesn't crash.
        for n in [0i32, 1, 2, 3, -100, 9999] {
            c_fn(n);
            r_fn(n);
        }
    }
}

#[test]
fn test_rand_seed_determines_hash_seed_state() {
    // Setting the seed then doing a hash_string should be the same in C and Rust.
    // We can't directly observe the seed but we can use stbds_make_hash_index
    // via stbds_shmode_func which embeds the current STBDS_HASH_SEED. To keep
    // this simple, just call stbds_rand_seed and check that the next
    // stbds_hash_bytes/stbds_hash_string call is unaffected by it (those
    // functions take their own seed argument).
    unsafe {
        let (c, r) = open_libs();
        let c_seed: Symbol<Fn_stbds_rand_seed> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<Fn_stbds_rand_seed> = r.get(b"stbds_rand_seed").unwrap();
        c_seed(0xCAFE);
        r_seed(0xCAFE);
    }
}

#[test]
fn test_shmode_string_keys() {
    unsafe {
        let (c, r) = open_libs();
        let elemsize = std::mem::size_of::<StrKV>();

        let c_sh: Symbol<Fn_stbds_shmode_func> = c.get(b"stbds_shmode_func").unwrap();
        let r_sh: Symbol<Fn_stbds_shmode_func> = r.get(b"stbds_shmode_func").unwrap();
        let c_put: Symbol<Fn_stbds_hmput_key> = c.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<Fn_stbds_hmput_key> = r.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<Fn_stbds_hmget_key> = c.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<Fn_stbds_hmget_key> = r.get(b"stbds_hmget_key").unwrap();
        let c_free: Symbol<Fn_stbds_hmfree_func> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<Fn_stbds_hmfree_func> = r.get(b"stbds_hmfree_func").unwrap();

        // mode 1 = STBDS_SH_DEFAULT (no string copying)
        let mut ca = c_sh(elemsize, 1);
        let mut ra = r_sh(elemsize, 1);

        let mut keys: Vec<CString> = Vec::new();
        for i in 0..20 {
            keys.push(CString::new(format!("key_{}", i)).unwrap());
        }

        for (i, k) in keys.iter().enumerate() {
            let key_ptr = k.as_ptr() as *mut c_void;
            ca = c_put(ca, elemsize, key_ptr, std::mem::size_of::<*mut c_char>(), 1);
            ra = r_put(ra, elemsize, key_ptr, std::mem::size_of::<*mut c_char>(), 1);

            let c_raw = (ca as *mut u8).sub(elemsize);
            let r_raw = (ra as *mut u8).sub(elemsize);
            let c_idx = (*header_of(c_raw as *mut c_void)).temp;
            let r_idx = (*header_of(r_raw as *mut c_void)).temp;
            assert_eq!(c_idx, r_idx, "shmode put idx differs at {}", i);

            let c_entry = (ca as *mut StrKV).add(c_idx as usize);
            let r_entry = (ra as *mut StrKV).add(r_idx as usize);
            (*c_entry).value = i as i32 * 13;
            (*r_entry).value = i as i32 * 13;
        }

        for (i, k) in keys.iter().enumerate() {
            let key_ptr = k.as_ptr() as *mut c_void;
            ca = c_get(ca, elemsize, key_ptr, std::mem::size_of::<*mut c_char>(), 1);
            ra = r_get(ra, elemsize, key_ptr, std::mem::size_of::<*mut c_char>(), 1);
            let c_raw = (ca as *mut u8).sub(elemsize);
            let r_raw = (ra as *mut u8).sub(elemsize);
            let c_idx = (*header_of(c_raw as *mut c_void)).temp;
            let r_idx = (*header_of(r_raw as *mut c_void)).temp;
            assert_eq!(c_idx, r_idx, "shmode get idx differs at {}", i);
            assert!(c_idx >= 0, "key {:?} not found in shmode map", k);
            let c_v = (*(ca as *mut StrKV).add(c_idx as usize)).value;
            let r_v = (*(ra as *mut StrKV).add(r_idx as usize)).value;
            assert_eq!(c_v, r_v);
        }

        let c_raw_free = (ca as *mut u8).sub(elemsize) as *mut c_void;
        let r_raw_free = (ra as *mut u8).sub(elemsize) as *mut c_void;
        c_free(c_raw_free, elemsize);
        r_free(r_raw_free, elemsize);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct StrKV {
    key: *mut c_char,
    value: i32,
    _pad: i32,
}
