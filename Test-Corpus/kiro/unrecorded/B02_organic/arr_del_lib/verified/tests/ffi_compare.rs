use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.join("target/debug/libarr_del_lib.so")
}

#[test]
fn test_strkey() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> *mut i8> = c.get(b"strkey").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32) -> *mut i8> = r.get(b"strkey").unwrap();
        for n in [0, 1, -1, 42, 999999, i32::MAX, i32::MIN] {
            let cs = CStr::from_ptr(c_fn(n));
            let rs = CStr::from_ptr(r_fn(n));
            assert_eq!(cs, rs, "strkey({n}) mismatch");
        }
    }
}

#[test]
fn test_hash_string() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*mut i8, usize) -> usize> = c.get(b"stbds_hash_string").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut i8, usize) -> usize> = r.get(b"stbds_hash_string").unwrap();
        for seed in [0usize, 1, 0x31415926, usize::MAX] {
            for s in [b"hello\0".as_ptr(), b"\0".as_ptr(), b"test_42\0".as_ptr(), b"a longer string for testing\0".as_ptr()] {
                let cv = c_fn(s as *mut i8, seed);
                let rv = r_fn(s as *mut i8, seed);
                assert_eq!(cv, rv, "hash_string mismatch for seed={seed}");
            }
        }
    }
}

#[test]
fn test_hash_bytes() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> = c.get(b"stbds_hash_bytes").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize, usize) -> usize> = r.get(b"stbds_hash_bytes").unwrap();
        for seed in [0usize, 1, 0x31415926, 0xdeadbeef] {
            for data in [
                &[][..], &[0u8][..], &[1, 2, 3, 4][..], &[0u8; 7][..], &[0xffu8; 8][..],
                &[1, 2, 3, 4, 5, 6, 7, 8, 9][..], &[0u8; 16][..], &[0xab; 31][..],
            ] {
                let mut buf = data.to_vec();
                let cv = c_fn(buf.as_mut_ptr(), buf.len(), seed);
                let rv = r_fn(buf.as_mut_ptr(), buf.len(), seed);
                assert_eq!(cv, rv, "hash_bytes mismatch for len={} seed={seed}", buf.len());
            }
        }
    }
}

#[test]
fn test_rand_seed() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();
        let c_hash: Symbol<unsafe extern "C" fn(*mut i8, usize) -> usize> = c.get(b"stbds_hash_string").unwrap();
        let r_hash: Symbol<unsafe extern "C" fn(*mut i8, usize) -> usize> = r.get(b"stbds_hash_string").unwrap();
        for seed_val in [0usize, 42, 0xdeadbeef] {
            c_seed(seed_val);
            r_seed(seed_val);
            let s = b"test\0".as_ptr() as *mut i8;
            assert_eq!(c_hash(s, 0), r_hash(s, 0), "hash after rand_seed({seed_val})");
        }
    }
}

#[test]
fn test_arrgrowf_and_arrfreef() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_grow: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> = c.get(b"stbds_arrgrowf").unwrap();
        let r_grow: Symbol<unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8> = r.get(b"stbds_arrgrowf").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8)> = c.get(b"stbds_arrfreef").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8)> = r.get(b"stbds_arrfreef").unwrap();

        // Grow from null
        let ca = c_grow(std::ptr::null_mut(), 4, 0, 1);
        let ra = r_grow(std::ptr::null_mut(), 4, 0, 1);
        assert!(!ca.is_null());
        assert!(!ra.is_null());

        // Check header fields match (length=0, capacity>=1)
        #[repr(C)]
        struct Hdr { length: usize, capacity: usize, hash_table: *mut u8, temp: isize }
        let ch = (ca as *mut Hdr).offset(-1);
        let rh = (ra as *mut Hdr).offset(-1);
        assert_eq!((*ch).length, (*rh).length, "length mismatch after grow from null");
        assert_eq!((*ch).capacity, (*rh).capacity, "capacity mismatch after grow from null");

        c_free(ca);
        r_free(ra);
    }
}

#[test]
fn test_stralloc_strreset() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_alloc: Symbol<unsafe extern "C" fn(*mut u8, *mut i8) -> *mut i8> = c.get(b"stbds_stralloc").unwrap();
        let r_alloc: Symbol<unsafe extern "C" fn(*mut u8, *mut i8) -> *mut i8> = r.get(b"stbds_stralloc").unwrap();
        let c_reset: Symbol<unsafe extern "C" fn(*mut u8)> = c.get(b"stbds_strreset").unwrap();
        let r_reset: Symbol<unsafe extern "C" fn(*mut u8)> = r.get(b"stbds_strreset").unwrap();

        // Arena struct: storage ptr, remaining usize, block u8, mode u8
        let mut c_arena = [0u8; 64];
        let mut r_arena = [0u8; 64];

        let strings = [b"hello\0".as_ptr(), b"world\0".as_ptr(), b"test_string\0".as_ptr()];
        for s in &strings {
            let cp = c_alloc(c_arena.as_mut_ptr(), *s as *mut i8);
            let rp = r_alloc(r_arena.as_mut_ptr(), *s as *mut i8);
            let cs = CStr::from_ptr(cp);
            let rs = CStr::from_ptr(rp);
            assert_eq!(cs, rs, "stralloc content mismatch");
        }

        c_reset(c_arena.as_mut_ptr());
        r_reset(r_arena.as_mut_ptr());
        // After reset, arena should be zeroed
        assert_eq!(&c_arena[..24], &r_arena[..24], "arena state mismatch after reset");
    }
}

#[test]
fn test_arr_del() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(i32)> = c.get(b"arr_del").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32)> = r.get(b"arr_del").unwrap();
        // Both should run without crashing for various inputs
        for num in [0, 1, -1, 42, 100, i32::MAX, i32::MIN] {
            c_fn(num);
            r_fn(num);
        }
    }
}

#[test]
fn test_hmput_hmget_hmdel_cycle() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();

        type GrowFn = unsafe extern "C" fn(*mut u8, usize, usize, usize) -> *mut u8;
        type PutKeyFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type GetKeyFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, i32) -> *mut u8;
        type DelKeyFn = unsafe extern "C" fn(*mut u8, usize, *mut u8, usize, usize, i32) -> *mut u8;
        type FreeFn = unsafe extern "C" fn(*mut u8, usize);

        let c_put: Symbol<PutKeyFn> = c.get(b"stbds_hmput_key").unwrap();
        let r_put: Symbol<PutKeyFn> = r.get(b"stbds_hmput_key").unwrap();
        let c_get: Symbol<GetKeyFn> = c.get(b"stbds_hmget_key").unwrap();
        let r_get: Symbol<GetKeyFn> = r.get(b"stbds_hmget_key").unwrap();
        let c_del: Symbol<DelKeyFn> = c.get(b"stbds_hmdel_key").unwrap();
        let r_del: Symbol<DelKeyFn> = r.get(b"stbds_hmdel_key").unwrap();
        let c_free: Symbol<FreeFn> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<FreeFn> = r.get(b"stbds_hmfree_func").unwrap();
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();

        // Use same seed for deterministic behavior
        c_seed(12345);
        r_seed(12345);

        // struct { int key, value; } - elemsize=8, keysize=4
        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        struct KV { key: i32, value: i32 }
        #[repr(C)]
        struct Hdr { length: usize, capacity: usize, hash_table: *mut u8, temp: isize }

        let elemsize = std::mem::size_of::<KV>();
        let keysize = std::mem::size_of::<i32>();

        let mut ca: *mut u8 = std::ptr::null_mut();
        let mut ra: *mut u8 = std::ptr::null_mut();

        // Put several keys
        for i in 0..20i32 {
            let mut key = i;
            ca = c_put(ca, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let c_hdr = (ca.sub(elemsize) as *mut Hdr).offset(-1);
            let c_temp = (*c_hdr).temp;
            // Write value
            let c_entry = ca.add(elemsize * c_temp as usize) as *mut KV;
            (*c_entry).key = i;
            (*c_entry).value = i * 10;

            key = i;
            ra = r_put(ra, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let r_hdr = (ra.sub(elemsize) as *mut Hdr).offset(-1);
            let r_temp = (*r_hdr).temp;
            let r_entry = ra.add(elemsize * r_temp as usize) as *mut KV;
            (*r_entry).key = i;
            (*r_entry).value = i * 10;

            assert_eq!(c_temp, r_temp, "temp mismatch after put key={i}");
        }

        // Get keys and compare temp values
        for i in 0..20i32 {
            let mut key = i;
            ca = c_get(ca, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let c_hdr = (ca.sub(elemsize) as *mut Hdr).offset(-1);
            let c_temp = (*c_hdr).temp;

            key = i;
            ra = r_get(ra, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let r_hdr = (ra.sub(elemsize) as *mut Hdr).offset(-1);
            let r_temp = (*r_hdr).temp;

            assert_eq!(c_temp, r_temp, "get temp mismatch for key={i}");
            if c_temp >= 0 {
                let cv = (ca.add(elemsize * c_temp as usize) as *mut KV).read();
                let rv = (ra.add(elemsize * r_temp as usize) as *mut KV).read();
                assert_eq!(cv.key, rv.key, "key mismatch for key={i}");
                assert_eq!(cv.value, rv.value, "value mismatch for key={i}");
            }
        }

        // Delete some keys
        for i in [0, 5, 10, 15] {
            let mut key: i32 = i;
            ca = c_del(ca, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0, 0);
            key = i;
            ra = r_del(ra, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0, 0);
            let c_hdr = (ca.sub(elemsize) as *mut Hdr).offset(-1);
            let r_hdr = (ra.sub(elemsize) as *mut Hdr).offset(-1);
            assert_eq!((*c_hdr).temp, (*r_hdr).temp, "del temp mismatch for key={i}");
        }

        // Verify deleted keys return -1
        for i in [0, 5, 10, 15] {
            let mut key: i32 = i;
            ca = c_get(ca, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let c_hdr = (ca.sub(elemsize) as *mut Hdr).offset(-1);
            key = i;
            ra = r_get(ra, elemsize, &mut key as *mut i32 as *mut u8, keysize, 0);
            let r_hdr = (ra.sub(elemsize) as *mut Hdr).offset(-1);
            assert_eq!((*c_hdr).temp, -1, "C: deleted key {i} should return -1");
            assert_eq!((*r_hdr).temp, -1, "Rust: deleted key {i} should return -1");
        }

        // Free
        c_free(ca.sub(elemsize), elemsize);
        r_free(ra.sub(elemsize), elemsize);
    }
}

#[test]
fn test_hmput_default() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> *mut u8> = c.get(b"stbds_hmput_default").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut u8, usize) -> *mut u8> = r.get(b"stbds_hmput_default").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> = r.get(b"stbds_hmfree_func").unwrap();

        #[repr(C)]
        struct Hdr { length: usize, capacity: usize, hash_table: *mut u8, temp: isize }

        let elemsize = 8usize;
        let ca = c_fn(std::ptr::null_mut(), elemsize);
        let ra = r_fn(std::ptr::null_mut(), elemsize);
        assert!(!ca.is_null());
        assert!(!ra.is_null());

        let ch = (ca.sub(elemsize) as *mut Hdr).offset(-1);
        let rh = (ra.sub(elemsize) as *mut Hdr).offset(-1);
        assert_eq!((*ch).length, (*rh).length, "hmput_default length mismatch");

        // Calling again should return same pointer (already has default)
        let ca2 = c_fn(ca, elemsize);
        let ra2 = r_fn(ra, elemsize);
        assert_eq!(ca, ca2);
        assert_eq!(ra, ra2);

        c_free(ca.sub(elemsize), elemsize);
        r_free(ra.sub(elemsize), elemsize);
    }
}

#[test]
fn test_shmode_func() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(usize, i32) -> *mut u8> = c.get(b"stbds_shmode_func").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(usize, i32) -> *mut u8> = r.get(b"stbds_shmode_func").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> = c.get(b"stbds_hmfree_func").unwrap();
        let r_free: Symbol<unsafe extern "C" fn(*mut u8, usize)> = r.get(b"stbds_hmfree_func").unwrap();
        let c_seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let r_seed: Symbol<unsafe extern "C" fn(usize)> = r.get(b"stbds_rand_seed").unwrap();

        #[repr(C)]
        struct Hdr { length: usize, capacity: usize, hash_table: *mut u8, temp: isize }

        for mode in [2, 3] { // STBDS_SH_STRDUP, STBDS_SH_ARENA
            c_seed(0x31415926);
            r_seed(0x31415926);
            let elemsize = 16usize; // typical string hash map entry
            let ca = c_fn(elemsize, mode);
            let ra = r_fn(elemsize, mode);
            assert!(!ca.is_null());
            assert!(!ra.is_null());
            let ch = (ca.sub(elemsize) as *mut Hdr).offset(-1);
            let rh = (ra.sub(elemsize) as *mut Hdr).offset(-1);
            assert_eq!((*ch).length, (*rh).length, "shmode_func length mismatch mode={mode}");
            c_free(ca.sub(elemsize), elemsize);
            r_free(ra.sub(elemsize), elemsize);
        }
    }
}
