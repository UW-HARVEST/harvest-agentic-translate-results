// Integration tests that load both the C .so and the Rust .so via libloading
// and compare exported stb_ds primitives byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_void;

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/release/libsh_puts_lib.so";

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
    _pad: [u8; 6],
}

// Wrapper that loads symbols up-front to avoid repeating boilerplate.
struct LibFns {
    _lib: Library,
    rand_seed: unsafe extern "C" fn(seed: usize),
    hash_bytes: unsafe extern "C" fn(p: *mut c_void, len: usize, seed: usize) -> usize,
    hash_string: unsafe extern "C" fn(s: *mut i8, seed: usize) -> usize,
    arrgrowf: unsafe extern "C" fn(
        a: *mut c_void,
        elemsize: usize,
        addlen: usize,
        min_cap: usize,
    ) -> *mut c_void,
    arrfreef: unsafe extern "C" fn(a: *mut c_void),
    stralloc: unsafe extern "C" fn(arena: *mut StringArena, s: *mut i8) -> *mut i8,
    strreset: unsafe extern "C" fn(arena: *mut StringArena),
    hmput_default: unsafe extern "C" fn(a: *mut c_void, elemsize: usize) -> *mut c_void,
    hmget_key: unsafe extern "C" fn(
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut c_void,
    hmget_key_ts: unsafe extern "C" fn(
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        temp: *mut isize,
        mode: c_int,
    ) -> *mut c_void,
    hmput_key: unsafe extern "C" fn(
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        mode: c_int,
    ) -> *mut c_void,
    hmdel_key: unsafe extern "C" fn(
        a: *mut c_void,
        elemsize: usize,
        key: *mut c_void,
        keysize: usize,
        keyoffset: usize,
        mode: c_int,
    ) -> *mut c_void,
    hmfree_func: unsafe extern "C" fn(a: *mut c_void, elemsize: usize),
    shmode_func: unsafe extern "C" fn(elemsize: usize, mode: c_int) -> *mut c_void,
    strkey: unsafe extern "C" fn(n: c_int) -> *mut i8,
}

fn load(lib_path: &str) -> LibFns {
    unsafe {
        let lib = Library::new(lib_path).expect("library loads");
        let rand_seed: Symbol<unsafe extern "C" fn(usize)> =
            lib.get(b"stbds_rand_seed").unwrap();
        let hash_bytes: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            lib.get(b"stbds_hash_bytes").unwrap();
        let hash_string: Symbol<unsafe extern "C" fn(*mut i8, usize) -> usize> =
            lib.get(b"stbds_hash_string").unwrap();
        let arrgrowf: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void,
        > = lib.get(b"stbds_arrgrowf").unwrap();
        let arrfreef: Symbol<unsafe extern "C" fn(*mut c_void)> =
            lib.get(b"stbds_arrfreef").unwrap();
        let stralloc: Symbol<unsafe extern "C" fn(*mut StringArena, *mut i8) -> *mut i8> =
            lib.get(b"stbds_stralloc").unwrap();
        let strreset: Symbol<unsafe extern "C" fn(*mut StringArena)> =
            lib.get(b"stbds_strreset").unwrap();
        let hmput_default: Symbol<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void> =
            lib.get(b"stbds_hmput_default").unwrap();
        let hmget_key: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = lib.get(b"stbds_hmget_key").unwrap();
        let hmget_key_ts: Symbol<
            unsafe extern "C" fn(
                *mut c_void,
                usize,
                *mut c_void,
                usize,
                *mut isize,
                c_int,
            ) -> *mut c_void,
        > = lib.get(b"stbds_hmget_key_ts").unwrap();
        let hmput_key: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = lib.get(b"stbds_hmput_key").unwrap();
        let hmdel_key: Symbol<
            unsafe extern "C" fn(
                *mut c_void,
                usize,
                *mut c_void,
                usize,
                usize,
                c_int,
            ) -> *mut c_void,
        > = lib.get(b"stbds_hmdel_key").unwrap();
        let hmfree_func: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            lib.get(b"stbds_hmfree_func").unwrap();
        let shmode_func: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut c_void> =
            lib.get(b"stbds_shmode_func").unwrap();
        let strkey: Symbol<unsafe extern "C" fn(c_int) -> *mut i8> =
            lib.get(b"strkey").unwrap();

        let r = LibFns {
            rand_seed: *rand_seed,
            hash_bytes: *hash_bytes,
            hash_string: *hash_string,
            arrgrowf: *arrgrowf,
            arrfreef: *arrfreef,
            stralloc: *stralloc,
            strreset: *strreset,
            hmput_default: *hmput_default,
            hmget_key: *hmget_key,
            hmget_key_ts: *hmget_key_ts,
            hmput_key: *hmput_key,
            hmdel_key: *hmdel_key,
            hmfree_func: *hmfree_func,
            shmode_func: *shmode_func,
            strkey: *strkey,
            _lib: lib,
        };
        r
    }
}

// Force a deterministic seed before each test.
fn seed_both(c: &LibFns, r: &LibFns, seed: usize) {
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);
    }
}

#[test]
fn hash_bytes_matches() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    let inputs: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"the quick brown fox jumps over the lazy dog".to_vec(),
        (0u8..32).collect(),
        (0u8..255).collect(),
    ];
    let seeds: &[usize] = &[0, 1, 0xdeadbeef, 0x123456789abcdef0, usize::MAX];
    for input in &inputs {
        for &seed in seeds {
            let p = input.as_ptr() as *mut c_void;
            let cv = unsafe { (c.hash_bytes)(p, input.len(), seed) };
            let rv = unsafe { (r.hash_bytes)(p, input.len(), seed) };
            assert_eq!(cv, rv, "hash_bytes mismatch len={} seed={:#x}", input.len(), seed);
        }
    }
}

#[test]
fn hash_string_matches() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    let strs = [
        &b"\0"[..],
        &b"a\0"[..],
        &b"hello\0"[..],
        &b"the quick brown fox\0"[..],
        &b"longer string with !@#%^&*()_+ chars\0"[..],
    ];
    let seeds = [0usize, 1, 0xdead_beef, 0x1234_5678_9abc_def0];
    for s in strs {
        for &seed in &seeds {
            let p = s.as_ptr() as *mut i8;
            let cv = unsafe { (c.hash_string)(p, seed) };
            let rv = unsafe { (r.hash_string)(p, seed) };
            assert_eq!(cv, rv, "hash_string mismatch s={:?} seed={:#x}", s, seed);
        }
    }
}

#[test]
fn arrgrow_and_free_match() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    unsafe {
        // Grow capacities deterministically and verify header layout matches.
        for elemsize in [1usize, 4, 8, 16, 24] {
            for &(addlen, min_cap) in &[(0usize, 0usize), (1, 0), (5, 16), (0, 32), (10, 4)] {
                let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ch = (ca as *mut u8).sub(32) as *const [usize; 4];
                let rh = (ra as *mut u8).sub(32) as *const [usize; 4];
                // Compare length, capacity, temp; skip hash_table pointer (idx 2).
                assert_eq!((*ch)[0], (*rh)[0], "length mismatch elemsize={} addlen={} min_cap={}", elemsize, addlen, min_cap);
                assert_eq!((*ch)[1], (*rh)[1], "capacity mismatch elemsize={} addlen={} min_cap={}", elemsize, addlen, min_cap);
                assert_eq!((*ch)[3], (*rh)[3], "temp mismatch elemsize={} addlen={} min_cap={}", elemsize, addlen, min_cap);
                (c.arrfreef)(ca);
                (r.arrfreef)(ra);
            }
        }
    }
}

#[test]
fn stralloc_and_strreset_match() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    unsafe {
        let mut ca: StringArena = std::mem::zeroed();
        let mut ra: StringArena = std::mem::zeroed();
        for n in 0..50 {
            let s = format!("string number {}\0", n);
            let cp = (c.stralloc)(&mut ca, s.as_ptr() as *mut i8);
            let rp = (r.stralloc)(&mut ra, s.as_ptr() as *mut i8);
            // Verify the contents match (pointers will differ).
            let cs = std::ffi::CStr::from_ptr(cp);
            let rs = std::ffi::CStr::from_ptr(rp);
            assert_eq!(cs.to_bytes(), rs.to_bytes(), "stralloc content mismatch at n={}", n);
            // Verify the arena bookkeeping (remaining/block/mode) matches.
            assert_eq!(ca.remaining, ra.remaining, "remaining mismatch at n={}", n);
            assert_eq!(ca.block, ra.block, "block mismatch at n={}", n);
            assert_eq!(ca.mode, ra.mode, "mode mismatch at n={}", n);
        }
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        // After reset, all bookkeeping should be zero in both.
        assert_eq!(ca.remaining, 0);
        assert_eq!(ra.remaining, 0);
        assert_eq!(ca.block, 0);
        assert_eq!(ra.block, 0);
    }
}

// A binary-keyed hash map test.  Key is i32, value is i32. We fix the hash
// seed before each map creation to make the comparison deterministic.
#[test]
fn binary_hashmap_matches() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    unsafe {
        seed_both(&c, &r, 0xcafef00d);

        #[repr(C)]
        struct Pair { key: i32, value: i32 }
        let elemsize = std::mem::size_of::<Pair>();
        let mode = 0i32; // STBDS_HM_BINARY

        let mut cmap: *mut c_void = std::ptr::null_mut();
        let mut rmap: *mut c_void = std::ptr::null_mut();

        // Insert.
        for k in 0..32i32 {
            let mut key = k;
            cmap = (c.hmput_key)(cmap, elemsize, &mut key as *mut _ as *mut c_void, 4, mode);
            rmap = (r.hmput_key)(rmap, elemsize, &mut key as *mut _ as *mut c_void, 4, mode);
            // Set value at the slot indicated by temp.
            let ct = *((cmap as *mut u8).sub(elemsize).cast::<[usize; 4]>());
            let rt = *((rmap as *mut u8).sub(elemsize).cast::<[usize; 4]>());
            // length must match.
            assert_eq!(ct[0], rt[0], "length mismatch after insert k={}", k);
            // Read out temp index (4th field).
            let c_temp = ct[3] as isize;
            let r_temp = rt[3] as isize;
            assert_eq!(c_temp, r_temp, "insert temp mismatch k={}", k);
            let c_slot = (cmap as *mut Pair).offset(c_temp);
            let r_slot = (rmap as *mut Pair).offset(r_temp);
            (*c_slot).key = k;
            (*c_slot).value = k * 10;
            (*r_slot).key = k;
            (*r_slot).value = k * 10;
        }

        // Lookup all keys.
        for k in 0..32i32 {
            let mut key = k;
            let mut ct: isize = 0;
            let mut rt: isize = 0;
            let _ = (c.hmget_key_ts)(cmap, elemsize, &mut key as *mut _ as *mut c_void, 4, &mut ct, mode);
            let _ = (r.hmget_key_ts)(rmap, elemsize, &mut key as *mut _ as *mut c_void, 4, &mut rt, mode);
            assert_eq!(ct, rt, "hmget_key_ts temp mismatch k={}", k);
            let cv = (*(cmap as *mut Pair).offset(ct)).value;
            let rv = (*(rmap as *mut Pair).offset(rt)).value;
            assert_eq!(cv, rv, "value mismatch k={}", k);
        }

        // Delete half of the keys.
        for k in (0..32i32).step_by(2) {
            let mut key = k;
            cmap = (c.hmdel_key)(cmap, elemsize, &mut key as *mut _ as *mut c_void, 4, 0, mode);
            rmap = (r.hmdel_key)(rmap, elemsize, &mut key as *mut _ as *mut c_void, 4, 0, mode);
            let ct = *((cmap as *mut u8).sub(elemsize).cast::<[usize; 4]>());
            let rt = *((rmap as *mut u8).sub(elemsize).cast::<[usize; 4]>());
            assert_eq!(ct[0], rt[0], "length mismatch after delete k={}", k);
        }

        // Lookup again.
        for k in 0..32i32 {
            let mut key = k;
            let mut ct: isize = 0;
            let mut rt: isize = 0;
            let _ = (c.hmget_key_ts)(cmap, elemsize, &mut key as *mut _ as *mut c_void, 4, &mut ct, mode);
            let _ = (r.hmget_key_ts)(rmap, elemsize, &mut key as *mut _ as *mut c_void, 4, &mut rt, mode);
            assert_eq!(ct, rt, "post-delete hmget_key_ts mismatch k={}", k);
            // For surviving keys, also compare value.
            if ct >= 0 && rt >= 0 {
                let cv = (*(cmap as *mut Pair).offset(ct)).value;
                let rv = (*(rmap as *mut Pair).offset(rt)).value;
                assert_eq!(cv, rv, "post-delete value mismatch k={}", k);
            }
        }

        (c.hmfree_func)((cmap as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((rmap as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn shmode_and_strkey_match() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    unsafe {
        for n in [-3i32, 0, 1, 5, 100] {
            let cs = (c.strkey)(n);
            let rs = (r.strkey)(n);
            let cstr = std::ffi::CStr::from_ptr(cs);
            let rstr = std::ffi::CStr::from_ptr(rs);
            assert_eq!(cstr.to_bytes(), rstr.to_bytes(), "strkey mismatch for n={}", n);
        }

        // shmode_func with STBDS_SH_ARENA (3) — exercise it returns a non-null
        // pointer with length=1 in both implementations.
        let elemsize = std::mem::size_of::<(i64, i32, [u8; 4])>();
        let cmap = (c.shmode_func)(elemsize, 3);
        let rmap = (r.shmode_func)(elemsize, 3);
        let ch = (cmap as *mut u8).sub(elemsize).cast::<[usize; 4]>();
        let rh = (rmap as *mut u8).sub(elemsize).cast::<[usize; 4]>();
        assert_eq!((*ch)[0], (*rh)[0], "shmode_func length mismatch");
        // hmfree to clean up.
        (c.hmfree_func)((cmap as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((rmap as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn hmput_default_match() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    unsafe {
        let elemsize = 16usize;
        let cmap = (c.hmput_default)(std::ptr::null_mut(), elemsize);
        let rmap = (r.hmput_default)(std::ptr::null_mut(), elemsize);
        let ch = (cmap as *mut u8).sub(elemsize).cast::<[usize; 4]>();
        let rh = (rmap as *mut u8).sub(elemsize).cast::<[usize; 4]>();
        assert_eq!((*ch)[0], (*rh)[0]);
        (c.arrfreef)((cmap as *mut u8).sub(elemsize) as *mut c_void);
        (r.arrfreef)((rmap as *mut u8).sub(elemsize) as *mut c_void);
    }
}

#[test]
fn hmget_key_match() {
    let c = load(C_LIB);
    let r = load(RUST_LIB);
    unsafe {
        seed_both(&c, &r, 0x1234_5678);
        #[repr(C)]
        struct Pair { key: i32, value: i32 }
        let elemsize = std::mem::size_of::<Pair>();
        let mode = 0i32;

        let mut cmap: *mut c_void = std::ptr::null_mut();
        let mut rmap: *mut c_void = std::ptr::null_mut();
        for k in 0..16i32 {
            let mut key = k;
            cmap = (c.hmput_key)(cmap, elemsize, &mut key as *mut _ as *mut c_void, 4, mode);
            rmap = (r.hmput_key)(rmap, elemsize, &mut key as *mut _ as *mut c_void, 4, mode);
            let ct = (*((cmap as *mut u8).sub(elemsize).cast::<[usize; 4]>()))[3] as isize;
            let rt = (*((rmap as *mut u8).sub(elemsize).cast::<[usize; 4]>()))[3] as isize;
            (*(cmap as *mut Pair).offset(ct)).key = k;
            (*(cmap as *mut Pair).offset(ct)).value = k * 7;
            (*(rmap as *mut Pair).offset(rt)).key = k;
            (*(rmap as *mut Pair).offset(rt)).value = k * 7;
        }

        for k in -2..18i32 {
            let mut key = k;
            let _ = (c.hmget_key)(cmap, elemsize, &mut key as *mut _ as *mut c_void, 4, mode);
            let _ = (r.hmget_key)(rmap, elemsize, &mut key as *mut _ as *mut c_void, 4, mode);
            let ct = (*((cmap as *mut u8).sub(elemsize).cast::<[usize; 4]>()))[3] as isize;
            let rt = (*((rmap as *mut u8).sub(elemsize).cast::<[usize; 4]>()))[3] as isize;
            assert_eq!(ct, rt, "hmget_key temp mismatch k={}", k);
        }
        (c.hmfree_func)((cmap as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)((rmap as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}
