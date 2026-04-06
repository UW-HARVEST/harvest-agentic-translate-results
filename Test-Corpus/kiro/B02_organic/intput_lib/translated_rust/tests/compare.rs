use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libintput_lib.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/debug or target/release
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("libintput_lib.so");
    if !p.exists() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("target");
        p.push("release");
        p.push("libintput_lib.so");
    }
    p
}

// ============================================================
// 1. stbds_rand_seed + stbds_hash_string
// ============================================================
#[test]
fn test_hash_string() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        let c_hs: Symbol<unsafe extern "C" fn(*const u8, usize) -> usize> =
            c.get(b"stbds_hash_string").unwrap();
        let r_hs: Symbol<unsafe extern "C" fn(*const u8, usize) -> usize> =
            r.get(b"stbds_hash_string").unwrap();

        // Reset seeds to same value
        c_seed(0x12345678);
        r_seed(0x12345678);

        for seed in [0_usize, 1, 42, 0xdeadbeef, 0x31415926] {
            for s in [
                b"hello\0".as_ptr(),
                b"world\0".as_ptr(),
                b"\0".as_ptr(),
                b"test_42\0".as_ptr(),
                b"a\0".as_ptr(),
                b"abcdefghijklmnopqrstuvwxyz\0".as_ptr(),
            ] {
                let cv = c_hs(s, seed);
                let rv = r_hs(s, seed);
                assert_eq!(
                    cv, rv,
                    "stbds_hash_string mismatch for seed={seed:#x}, str={:?}",
                    std::ffi::CStr::from_ptr(s as *const i8)
                );
            }
        }
    }
}

// ============================================================
// 2. stbds_hash_bytes (siphash)
// ============================================================
#[test]
fn test_hash_bytes() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_hb: Symbol<unsafe extern "C" fn(*const u8, usize, usize) -> usize> =
            c.get(b"stbds_hash_bytes").unwrap();
        let r_hb: Symbol<unsafe extern "C" fn(*const u8, usize, usize) -> usize> =
            r.get(b"stbds_hash_bytes").unwrap();

        // Test various lengths and seeds
        let data: Vec<u8> = (0..64).collect();
        for seed in [0_usize, 1, 42, 0xdeadbeef, 0x31415926] {
            for len in 0..=32 {
                let cv = c_hb(data.as_ptr(), len, seed);
                let rv = r_hb(data.as_ptr(), len, seed);
                assert_eq!(
                    cv, rv,
                    "stbds_hash_bytes mismatch for seed={seed:#x}, len={len}"
                );
            }
        }

        // Test with specific int keys (like the hashmap uses)
        for key in [0_i32, 1, -1, 9, 11, 42, i32::MAX, i32::MIN] {
            let kb = key.to_ne_bytes();
            for seed in [0_usize, 0x31415926] {
                let cv = c_hb(kb.as_ptr(), 4, seed);
                let rv = r_hb(kb.as_ptr(), 4, seed);
                assert_eq!(
                    cv, rv,
                    "stbds_hash_bytes mismatch for int key={key}, seed={seed:#x}"
                );
            }
        }
    }
}

// ============================================================
// 3. stbds_arrgrowf / stbds_arrfreef
// ============================================================
#[test]
fn test_arrgrowf() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        type ArrgrowfFn = unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8;
        type ArrfreefFn = unsafe extern "C" fn(*mut u8);

        let c_grow: Symbol<ArrgrowfFn> = c.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<ArrgrowfFn> = r.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<ArrfreefFn> = c.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<ArrfreefFn> = r.get(b"stbds_arrfreef").unwrap();

        // Grow from null
        let ca = c_grow(std::ptr::null_mut(), 4, 0, 10);
        let ra = r_grow(std::ptr::null_mut(), 4, 0, 10);

        // Both should be non-null; header fields should match
        assert!(!ca.is_null());
        assert!(!ra.is_null());

        // Read header: length, capacity, hash_table, temp
        // Header is at (ptr - sizeof(header))
        let hdr_size = 32_usize; // 4 fields * 8 bytes on 64-bit
        let c_hdr = ca.sub(hdr_size);
        let r_hdr = ra.sub(hdr_size);

        let c_len = *(c_hdr as *const usize);
        let r_len = *(r_hdr as *const usize);
        assert_eq!(c_len, r_len, "length mismatch after grow from null");

        let c_cap = *(c_hdr.add(8) as *const usize);
        let r_cap = *(r_hdr.add(8) as *const usize);
        assert_eq!(c_cap, r_cap, "capacity mismatch after grow from null");

        c_free(ca);
        r_free(ra);
    }
}

// ============================================================
// 4. strkey
// ============================================================
#[test]
fn test_strkey() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path());

        let c_strkey: Symbol<unsafe extern "C" fn(i32) -> *const u8> =
            c.get(b"strkey").unwrap();

        // strkey may or may not be in Rust lib; test C behavior
        // and if Rust exports it, compare
        for n in [0, 1, 42, -1, 100, 999] {
            let c_ptr = c_strkey(n);
            let c_str = std::ffi::CStr::from_ptr(c_ptr as *const i8);
            let expected = format!("test_{n}");
            assert_eq!(c_str.to_str().unwrap(), expected, "C strkey({n}) wrong");
        }

        if let Ok(ref r) = r_lib {
            if let Ok(r_strkey) = r.get::<unsafe extern "C" fn(i32) -> *const u8>(b"strkey") {
                for n in [0, 1, 42, -1, 100] {
                    let c_ptr = c_strkey(n);
                    let r_ptr = r_strkey(n);
                    let cs = std::ffi::CStr::from_ptr(c_ptr as *const i8);
                    let rs = std::ffi::CStr::from_ptr(r_ptr as *const i8);
                    assert_eq!(cs, rs, "strkey({n}) mismatch");
                }
            }
        }
    }
}

// ============================================================
// 5. intput (top-level) — should not panic/crash
// ============================================================
#[test]
fn test_intput() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_intput: Symbol<unsafe extern "C" fn(i32)> = c.get(b"intput").unwrap();
        let r_intput: Symbol<unsafe extern "C" fn(i32)> = r.get(b"intput").unwrap();

        // Both must reset seed to same value before calling intput
        // since intput uses the global hash seed internally
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();

        for num in [0, 1, 5, 42, -1, 100, i32::MAX, i32::MIN] {
            // Reset seeds to identical state
            c_seed(0x31415926);
            r_seed(0x31415926);

            // Both should complete without panic/crash
            c_intput(num);
            r_intput(num);
        }
    }
}

// ============================================================
// 6. Full hashmap round-trip: put, get, delete
// ============================================================
#[test]
fn test_hmput_hmget_hmdel_roundtrip() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        type SeedFn = unsafe extern "C" fn(usize);
        type PutKeyFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type GetKeyFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type DelKeyFn =
            unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_seed: Symbol<SeedFn> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<SeedFn> = r.get(b"stbds_rand_seed").unwrap();
        let c_put: Symbol<PutKeyFn> = c.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<PutKeyFn> = r.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<GetKeyFn> = c.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<GetKeyFn> = r.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<DelKeyFn> = c.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<DelKeyFn> = r.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<FreeFn> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r.get(b"stbds_hmfree_func").unwrap();

        // struct { int key; int value; } — 8 bytes
        let elemsize: usize = 8;
        let keysize: usize = 4;

        c_seed(0x31415926);
        r_seed(0x31415926);

        let mut c_map: *mut u8 = std::ptr::null_mut();
        let mut r_map: *mut u8 = std::ptr::null_mut();

        // Header is at ptr - 32 bytes (sizeof stbds_array_header)
        let hdr_size: usize = 32;

        // Helper to read temp from header
        let read_temp = |map: *mut u8| -> isize {
            let raw = map.sub(elemsize);
            let hdr = raw.sub(hdr_size);
            *(hdr.add(24) as *const isize) // temp is 4th field
        };

        // Put keys 1..=5
        for k in 1..=5_i32 {
            let v = k * 10;
            c_map = c_put(c_map, elemsize, &k as *const i32 as *mut u8, keysize, 0);
            let c_idx = read_temp(c_map);
            let c_entry = c_map.add(elemsize * c_idx as usize) as *mut i32;
            *c_entry = k;
            *c_entry.add(1) = v;

            r_map = r_put(r_map, elemsize, &k as *const i32 as *mut u8, keysize, 0);
            let r_idx = read_temp(r_map);
            let r_entry = r_map.add(elemsize * r_idx as usize) as *mut i32;
            *r_entry = k;
            *r_entry.add(1) = v;
        }

        // Get and compare
        for k in 1..=5_i32 {
            c_map = c_get(c_map, elemsize, &k as *const i32 as *mut u8, keysize, 0);
            let c_idx = read_temp(c_map);

            r_map = r_get(r_map, elemsize, &k as *const i32 as *mut u8, keysize, 0);
            let r_idx = read_temp(r_map);

            assert!(c_idx >= 0, "C: key {k} not found");
            assert!(r_idx >= 0, "Rust: key {k} not found");

            let c_val = *(c_map.add(elemsize * c_idx as usize + 4) as *const i32);
            let r_val = *(r_map.add(elemsize * r_idx as usize + 4) as *const i32);
            assert_eq!(c_val, r_val, "value mismatch for key {k}");
            assert_eq!(c_val, k * 10);
        }

        // Delete key 3
        let k3: i32 = 3;
        c_map = c_del(c_map, elemsize, &k3 as *const i32 as *mut u8, keysize, 0, 0);
        r_map = r_del(r_map, elemsize, &k3 as *const i32 as *mut u8, keysize, 0, 0);

        // Verify key 3 is gone
        c_map = c_get(c_map, elemsize, &k3 as *const i32 as *mut u8, keysize, 0);
        let c_idx3 = read_temp(c_map);
        r_map = r_get(r_map, elemsize, &k3 as *const i32 as *mut u8, keysize, 0);
        let r_idx3 = read_temp(r_map);
        assert_eq!(c_idx3, r_idx3, "delete result mismatch for key 3");
        assert_eq!(c_idx3, -1, "key 3 should be deleted");

        // Verify other keys still present
        for k in [1, 2, 4, 5] {
            c_map = c_get(c_map, elemsize, &k as *const i32 as *mut u8, keysize, 0);
            let ci = read_temp(c_map);
            r_map = r_get(r_map, elemsize, &k as *const i32 as *mut u8, keysize, 0);
            let ri = read_temp(r_map);
            assert!(ci >= 0, "C: key {k} missing after delete");
            assert!(ri >= 0, "Rust: key {k} missing after delete");
        }

        // Free
        c_free(c_map.sub(elemsize), elemsize);
        r_free(r_map.sub(elemsize), elemsize);
    }
}

// ============================================================
// 7. stbds_stralloc / stbds_strreset
// ============================================================
#[test]
fn test_stralloc_strreset() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        type StrAllocFn = unsafe extern "C" fn(*mut u8, *mut u8) -> *mut u8;
        type StrResetFn = unsafe extern "C" fn(*mut u8);

        let c_alloc: Symbol<StrAllocFn> = c.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<StrAllocFn> = r.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<StrResetFn> = c.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<StrResetFn> = r.get(b"stbds_strreset").unwrap();

        // stbds_string_arena is 24 bytes: *storage(8), remaining(8), block(1), mode(1) + padding
        let arena_size = 24_usize;
        let mut c_arena = vec![0u8; arena_size];
        let mut r_arena = vec![0u8; arena_size];

        let strings = [
            b"hello\0".as_ptr(),
            b"world\0".as_ptr(),
            b"test_string_123\0".as_ptr(),
            b"a\0".as_ptr(),
        ];

        for s in &strings {
            let cp = c_alloc(c_arena.as_mut_ptr(), *s as *mut u8);
            let rp = r_alloc(r_arena.as_mut_ptr(), *s as *mut u8);

            // Both should return valid strings matching input
            let cs = std::ffi::CStr::from_ptr(cp as *const i8);
            let rs = std::ffi::CStr::from_ptr(rp as *const i8);
            let orig = std::ffi::CStr::from_ptr(*s as *const i8);
            assert_eq!(cs, orig, "C stralloc returned wrong string");
            assert_eq!(rs, orig, "Rust stralloc returned wrong string");
        }

        c_reset(c_arena.as_mut_ptr());
        r_reset(r_arena.as_mut_ptr());
    }
}

// ============================================================
// 8. stbds_shmode_func
// ============================================================
#[test]
fn test_shmode_func() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        type ShmodeFn = unsafe extern "C" fn(usize, i32) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_shmode: Symbol<ShmodeFn> = c.get(b"stbds_shmode_func").unwrap();
        let r_shmode: Symbol<ShmodeFn> = r.get(b"stbds_shmode_func").unwrap();
        let c_free: Symbol<FreeFn> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r.get(b"stbds_hmfree_func").unwrap();

        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();

        // struct { char *key; int value; } — 16 bytes on 64-bit
        let elemsize = 16_usize;

        for mode in [1, 2, 3] {
            // STBDS_SH_DEFAULT=1, STRDUP=2, ARENA=3
            c_seed(0x31415926);
            r_seed(0x31415926);

            let ca = c_shmode(elemsize, mode);
            let ra = r_shmode(elemsize, mode);

            assert!(!ca.is_null(), "C shmode returned null for mode {mode}");
            assert!(!ra.is_null(), "Rust shmode returned null for mode {mode}");

            // Free them
            c_free(ca.sub(elemsize), elemsize);
            r_free(ra.sub(elemsize), elemsize);
        }
    }
}

// ============================================================
// 9. stbds_hmput_default
// ============================================================
#[test]
fn test_hmput_default() {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");

        type PutDefaultFn = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_pd: Symbol<PutDefaultFn> = c.get(b"stbds_hmput_default").unwrap();
        let r_pd: Symbol<PutDefaultFn> = r.get(b"stbds_hmput_default").unwrap();
        let c_free: Symbol<FreeFn> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r.get(b"stbds_hmfree_func").unwrap();

        let elemsize = 8_usize;

        // From null
        let ca = c_pd(std::ptr::null_mut(), elemsize);
        let ra = r_pd(std::ptr::null_mut(), elemsize);
        assert!(!ca.is_null());
        assert!(!ra.is_null());

        // Calling again should return same pointer (already has default)
        let ca2 = c_pd(ca, elemsize);
        let ra2 = r_pd(ra, elemsize);
        assert_eq!(ca, ca2, "C hmput_default should return same ptr");
        assert_eq!(ra, ra2, "Rust hmput_default should return same ptr");

        // Read header length — should be 1 for both
        let hdr_size = 32_usize;
        let c_len = *(ca.sub(elemsize).sub(hdr_size) as *const usize);
        let r_len = *(ra.sub(elemsize).sub(hdr_size) as *const usize);
        assert_eq!(c_len, r_len, "hmput_default length mismatch");
        assert_eq!(c_len, 1);

        c_free(ca.sub(elemsize), elemsize);
        r_free(ra.sub(elemsize), elemsize);
    }
}
