//! Integration tests that load both the C-built .so and the Rust-built .so
//! and compare their behavior through the FFI boundary.

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

use libloading::{Library, Symbol};

const C_SO_REL: &str = "c_src/build/libtranslated_rust.so";
const RUST_SO_REL: &str = "target/release/libarr_push_lib.so";

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_libs() -> (Library, Library) {
    let root = project_root();
    let c_path = root.join(C_SO_REL);
    let rust_path = root.join(RUST_SO_REL);
    let c = unsafe { Library::new(&c_path).expect("failed to load C .so") };
    let rust = unsafe { Library::new(&rust_path).expect("failed to load Rust .so") };
    (c, rust)
}

// ---------------------------------------------------------------------------
// Header layout that matches the C side. Used for inspecting array headers.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

// All hash-table state fields except `seed` and `string` (which are
// non-deterministic between fresh tables) must compare equal.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct HashIndex {
    temp_key: *mut c_char,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string_storage: *mut c_void,
    string_remaining: usize,
    string_block: u8,
    string_mode: u8,
    storage: *mut c_void,
}

unsafe fn header(p: *mut c_void) -> Header {
    let h = unsafe { (p as *mut Header).sub(1) };
    unsafe { *h }
}

/// For a hash-table return pointer (an "array-to-hash" pointer that points
/// `elemsize` past the underlying array data), produce the address of the
/// `stbds_array_header` that precedes the array data.
unsafe fn arr_header_of_hash(p: *mut c_void, elemsize: usize) -> *mut Header {
    unsafe {
        let arr = (p as *mut u8).sub(elemsize);
        (arr as *mut Header).sub(1)
    }
}

// ---------------------------------------------------------------------------
// Test: stbds_rand_seed (just exercises the symbol exists / is callable).
// ---------------------------------------------------------------------------

#[test]
fn stbds_rand_seed_callable() {
    let (c, rust) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(usize)> = rust.get(b"stbds_rand_seed").unwrap();
        // Set both to the same seed so subsequent tests are deterministic.
        f_c(0x31415926);
        f_r(0x31415926);
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_string.
// ---------------------------------------------------------------------------

#[test]
fn hash_string_matches() {
    let (c, rust) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            c.get(b"stbds_hash_string").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*mut c_char, usize) -> usize> =
            rust.get(b"stbds_hash_string").unwrap();

        for s in &["", "a", "ab", "hello", "test_42", "0123456789abcdef"] {
            let mut buf = s.as_bytes().to_vec();
            buf.push(0);
            for seed in &[0usize, 1, 0x31415926, 0xdeadbeefcafebabe] {
                let cv = f_c(buf.as_mut_ptr() as *mut c_char, *seed);
                let rv = f_r(buf.as_mut_ptr() as *mut c_char, *seed);
                assert_eq!(cv, rv, "hash_string({s:?}, {seed:#x}) mismatch");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_hash_bytes.
// ---------------------------------------------------------------------------

#[test]
fn hash_bytes_matches() {
    let (c, rust) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            c.get(b"stbds_hash_bytes").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize> =
            rust.get(b"stbds_hash_bytes").unwrap();

        let cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![1],
            vec![1, 2, 3],
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            (0..16u8).collect(),
            (0..32u8).collect(),
            b"the quick brown fox".to_vec(),
        ];
        for c_bytes in &cases {
            let mut buf = c_bytes.clone();
            for seed in &[0usize, 1, 0x31415926, 0xdeadbeefcafebabe] {
                let cv = f_c(buf.as_mut_ptr() as *mut c_void, buf.len(), *seed);
                let rv = f_r(buf.as_mut_ptr() as *mut c_void, buf.len(), *seed);
                assert_eq!(
                    cv, rv,
                    "hash_bytes({:?}, len={}, seed={:#x}) mismatch",
                    buf,
                    buf.len(),
                    *seed
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef.
// ---------------------------------------------------------------------------

#[test]
fn arrgrowf_initial_grow_matches() {
    let (c, rust) = load_libs();
    unsafe {
        let g_c: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            c.get(b"stbds_arrgrowf").unwrap();
        let g_r: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            rust.get(b"stbds_arrgrowf").unwrap();
        let f_c: Symbol<unsafe extern "C" fn(*mut c_void)> = c.get(b"stbds_arrfreef").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*mut c_void)> = rust.get(b"stbds_arrfreef").unwrap();

        // Grow from NULL with various (elemsize, addlen, min_cap) tuples and
        // confirm the resulting header capacity/length match.
        // Skip the (0, 0) case since arrgrowf returns NULL unchanged for that.
        for elemsize in [1usize, 4, 8, 16] {
            for (addlen, min_cap) in [(1usize, 0usize), (0, 1), (5, 0), (0, 10), (50, 50)] {
                let pc = g_c(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let pr = g_r(std::ptr::null_mut(), elemsize, addlen, min_cap);
                assert!(!pc.is_null());
                assert!(!pr.is_null());
                let hc = header(pc);
                let hr = header(pr);
                assert_eq!(
                    (hc.length, hc.capacity),
                    (hr.length, hr.capacity),
                    "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap}) header mismatch"
                );
                f_c(pc);
                f_r(pr);
            }
        }
    }
}

#[test]
fn arrgrowf_repeated_grow_matches() {
    let (c, rust) = load_libs();
    unsafe {
        let g_c: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            c.get(b"stbds_arrgrowf").unwrap();
        let g_r: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void> =
            rust.get(b"stbds_arrgrowf").unwrap();
        let f_c: Symbol<unsafe extern "C" fn(*mut c_void)> = c.get(b"stbds_arrfreef").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*mut c_void)> = rust.get(b"stbds_arrfreef").unwrap();

        // Build up an array of ints by repeatedly growing+pushing values, then
        // confirm both implementations produced identical capacity sequences
        // and identical contents.
        let mut pc: *mut c_void = std::ptr::null_mut();
        let mut pr: *mut c_void = std::ptr::null_mut();
        let mut len = 0usize;
        let elemsize = std::mem::size_of::<c_int>();

        for value in 0..200i32 {
            // grow if needed
            let need_grow = pc.is_null() || header(pc).length + 1 > header(pc).capacity;
            if need_grow {
                pc = g_c(pc, elemsize, 1, 0);
            }
            let need_grow_r = pr.is_null() || header(pr).length + 1 > header(pr).capacity;
            if need_grow_r {
                pr = g_r(pr, elemsize, 1, 0);
            }

            assert_eq!(
                header(pc).capacity,
                header(pr).capacity,
                "capacity diverged at value {value}"
            );

            // header.length++; arr[len-1] = value;
            unsafe {
                let hc = (pc as *mut Header).sub(1);
                (*hc).length += 1;
                let hr = (pr as *mut Header).sub(1);
                (*hr).length += 1;
                let pc_int = pc as *mut c_int;
                let pr_int = pr as *mut c_int;
                *pc_int.add(len) = value;
                *pr_int.add(len) = value;
            }
            len += 1;
        }

        // Compare every byte
        let pc_bytes = std::slice::from_raw_parts(pc as *const u8, elemsize * len);
        let pr_bytes = std::slice::from_raw_parts(pr as *const u8, elemsize * len);
        assert_eq!(pc_bytes, pr_bytes);

        f_c(pc);
        f_r(pr);
    }
}

// ---------------------------------------------------------------------------
// stbds_hmput_key / stbds_hmget_key (binary key mode).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct IntPair {
    key: i32,
    value: i32,
}

#[test]
fn hmput_get_int_keys_match() {
    let (c, rust) = load_libs();
    unsafe {
        let seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        seed(0x31415926);
        let seed_r: Symbol<unsafe extern "C" fn(usize)> = rust.get(b"stbds_rand_seed").unwrap();
        seed_r(0x31415926);

        let put_c: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c.get(b"stbds_hmput_key").unwrap();
        let put_r: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = rust.get(b"stbds_hmput_key").unwrap();
        let get_c: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c.get(b"stbds_hmget_key").unwrap();
        let get_r: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = rust.get(b"stbds_hmget_key").unwrap();
        let free_c: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            c.get(b"stbds_hmfree_func").unwrap();
        let free_r: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            rust.get(b"stbds_hmfree_func").unwrap();

        let elemsize = std::mem::size_of::<IntPair>();
        let keysize = std::mem::size_of::<i32>();
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();

        // Insert keys 0..50, mirroring stbds_hmput's macro behavior.
        // hmput conceptually writes (key,value) at index temp((t)-1) + 1.
        // Through the FFI we call hmput_key, then write key/value ourselves.
        for k in 0..50i32 {
            let mut key = k;
            tc = put_c(tc, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);
            tr = put_r(tr, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);

            // tc/tr now point to the array. Find where to write.
            let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
            let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
            assert_eq!(temp_c, temp_r, "temp differs at insert {k}");

            // Write (key, value) at slot temp + 1 (offset by the [-1] header element).
            let pair = IntPair {
                key: k,
                value: k * 2 + 7,
            };
            let pc_pair = (tc as *mut IntPair).offset(temp_c as isize);
            let pr_pair = (tr as *mut IntPair).offset(temp_r as isize);
            *pc_pair = pair;
            *pr_pair = pair;
        }

        // Now look up every key and verify.
        for k in 0..50i32 {
            let mut key = k;
            tc = get_c(tc, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);
            tr = get_r(tr, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);
            let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
            let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
            assert_eq!(temp_c, temp_r, "lookup temp diverges at key {k}");
            assert!(temp_c >= 0, "key {k} not found in C side");

            let pc_pair = *((tc as *mut IntPair).offset(temp_c as isize));
            let pr_pair = *((tr as *mut IntPair).offset(temp_r as isize));
            assert_eq!(pc_pair, pr_pair, "lookup payload diverges at key {k}");
            assert_eq!(pc_pair.key, k);
            assert_eq!(pc_pair.value, k * 2 + 7);
        }

        // Look up a missing key.
        let mut missing: i32 = 1234;
        tc = get_c(
            tc,
            elemsize,
            &mut missing as *mut i32 as *mut c_void,
            keysize,
            0,
        );
        tr = get_r(
            tr,
            elemsize,
            &mut missing as *mut i32 as *mut c_void,
            keysize,
            0,
        );
        let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
        let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
        assert_eq!(temp_c, -1);
        assert_eq!(temp_r, -1);

        let raw_c = (tc as *mut u8).sub(elemsize) as *mut c_void;
        let raw_r = (tr as *mut u8).sub(elemsize) as *mut c_void;
        free_c(raw_c, elemsize);
        free_r(raw_r, elemsize);
    }
}

// ---------------------------------------------------------------------------
// stbds_hmdel_key.
// ---------------------------------------------------------------------------

#[test]
fn hmdel_int_keys_match() {
    let (c, rust) = load_libs();
    unsafe {
        let seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        seed(0x31415926);
        let seed_r: Symbol<unsafe extern "C" fn(usize)> = rust.get(b"stbds_rand_seed").unwrap();
        seed_r(0x31415926);

        let put_c: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c.get(b"stbds_hmput_key").unwrap();
        let put_r: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = rust.get(b"stbds_hmput_key").unwrap();
        let del_c: Symbol<
            unsafe extern "C" fn(
                *mut c_void,
                usize,
                *mut c_void,
                usize,
                usize,
                c_int,
            ) -> *mut c_void,
        > = c.get(b"stbds_hmdel_key").unwrap();
        let del_r: Symbol<
            unsafe extern "C" fn(
                *mut c_void,
                usize,
                *mut c_void,
                usize,
                usize,
                c_int,
            ) -> *mut c_void,
        > = rust.get(b"stbds_hmdel_key").unwrap();
        let get_c: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c.get(b"stbds_hmget_key").unwrap();
        let get_r: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = rust.get(b"stbds_hmget_key").unwrap();
        let free_c: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            c.get(b"stbds_hmfree_func").unwrap();
        let free_r: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            rust.get(b"stbds_hmfree_func").unwrap();

        let elemsize = std::mem::size_of::<IntPair>();
        let keysize = std::mem::size_of::<i32>();
        let keyoffset = 0usize; // key is the first field
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();

        // Insert 30 keys, then delete every other one.
        for k in 0..30i32 {
            let mut key = k;
            tc = put_c(tc, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);
            tr = put_r(tr, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);
            let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
            let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
            assert_eq!(temp_c, temp_r);
            let pair = IntPair { key: k, value: k };
            *(tc as *mut IntPair).offset(temp_c as isize) = pair;
            *(tr as *mut IntPair).offset(temp_r as isize) = pair;
        }

        for k in (0..30i32).step_by(2) {
            let mut key = k;
            tc = del_c(
                tc,
                elemsize,
                &mut key as *mut i32 as *mut c_void,
                keysize,
                keyoffset,
                0,
            );
            tr = del_r(
                tr,
                elemsize,
                &mut key as *mut i32 as *mut c_void,
                keysize,
                keyoffset,
                0,
            );
            let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
            let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
            assert_eq!(temp_c, temp_r, "del temp differs at key {k}");
        }

        // Look up the surviving keys; they should still match.
        for k in (1..30i32).step_by(2) {
            let mut key = k;
            tc = get_c(tc, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);
            tr = get_r(tr, elemsize, &mut key as *mut i32 as *mut c_void, keysize, 0);
            let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
            let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
            assert_eq!(temp_c, temp_r);
            assert!(temp_c >= 0, "surviving key {k} should still be findable");
            let pc_pair = *((tc as *mut IntPair).offset(temp_c as isize));
            let pr_pair = *((tr as *mut IntPair).offset(temp_r as isize));
            assert_eq!(pc_pair, pr_pair);
        }
        // Look up a deleted key - should miss.
        let mut deleted_key: i32 = 0;
        tc = get_c(
            tc,
            elemsize,
            &mut deleted_key as *mut i32 as *mut c_void,
            keysize,
            0,
        );
        tr = get_r(
            tr,
            elemsize,
            &mut deleted_key as *mut i32 as *mut c_void,
            keysize,
            0,
        );
        let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
        let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
        assert_eq!(temp_c, -1);
        assert_eq!(temp_r, -1);

        let raw_c = (tc as *mut u8).sub(elemsize) as *mut c_void;
        let raw_r = (tr as *mut u8).sub(elemsize) as *mut c_void;
        free_c(raw_c, elemsize);
        free_r(raw_r, elemsize);
    }
}

// ---------------------------------------------------------------------------
// stbds_shmode_func + stbds_hmput_key (string mode).
// ---------------------------------------------------------------------------

#[repr(C)]
struct StrPair {
    key: *mut c_char,
    value: i32,
}

#[test]
fn shmode_func_strdup_basic_match() {
    let (c, rust) = load_libs();
    unsafe {
        let seed: Symbol<unsafe extern "C" fn(usize)> = c.get(b"stbds_rand_seed").unwrap();
        seed(0x31415926);
        let seed_r: Symbol<unsafe extern "C" fn(usize)> = rust.get(b"stbds_rand_seed").unwrap();
        seed_r(0x31415926);

        let sh_c: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut c_void> =
            c.get(b"stbds_shmode_func").unwrap();
        let sh_r: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut c_void> =
            rust.get(b"stbds_shmode_func").unwrap();
        let put_c: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c.get(b"stbds_hmput_key").unwrap();
        let put_r: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = rust.get(b"stbds_hmput_key").unwrap();
        let get_c: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = c.get(b"stbds_hmget_key").unwrap();
        let get_r: Symbol<
            unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
        > = rust.get(b"stbds_hmget_key").unwrap();
        let free_c: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            c.get(b"stbds_hmfree_func").unwrap();
        let free_r: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
            rust.get(b"stbds_hmfree_func").unwrap();

        let elemsize = std::mem::size_of::<StrPair>();
        let keysize = std::mem::size_of::<*mut c_char>();
        let mut tc = sh_c(elemsize, 2 /* STBDS_SH_STRDUP */);
        let mut tr = sh_r(elemsize, 2);

        for k in 0..15i32 {
            let s = format!("test_{k}\0");
            let mut buf = s.as_bytes().to_vec();
            tc = put_c(
                tc,
                elemsize,
                buf.as_mut_ptr() as *mut c_void,
                keysize,
                1, /* STBDS_HM_STRING */
            );
            tr = put_r(tr, elemsize, buf.as_mut_ptr() as *mut c_void, keysize, 1);
            let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
            let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
            assert_eq!(temp_c, temp_r);
            let pc_pair = (tc as *mut StrPair).offset(temp_c as isize);
            let pr_pair = (tr as *mut StrPair).offset(temp_r as isize);
            (*pc_pair).value = k * 100;
            (*pr_pair).value = k * 100;
        }

        for k in 0..15i32 {
            let s = format!("test_{k}\0");
            let mut buf = s.as_bytes().to_vec();
            tc = get_c(
                tc,
                elemsize,
                buf.as_mut_ptr() as *mut c_void,
                keysize,
                1,
            );
            tr = get_r(tr, elemsize, buf.as_mut_ptr() as *mut c_void, keysize, 1);
            let temp_c = (*arr_header_of_hash(tc, elemsize)).temp;
            let temp_r = (*arr_header_of_hash(tr, elemsize)).temp;
            assert_eq!(temp_c, temp_r);
            assert!(temp_c >= 0);
            let pc_v = (*(tc as *mut StrPair).offset(temp_c as isize)).value;
            let pr_v = (*(tr as *mut StrPair).offset(temp_r as isize)).value;
            assert_eq!(pc_v, pr_v);
            assert_eq!(pc_v, k * 100);
        }

        let raw_c = (tc as *mut u8).sub(elemsize) as *mut c_void;
        let raw_r = (tr as *mut u8).sub(elemsize) as *mut c_void;
        free_c(raw_c, elemsize);
        free_r(raw_r, elemsize);
    }
}

// ---------------------------------------------------------------------------
// stbds_stralloc / stbds_strreset.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[test]
fn stralloc_strreset_match() {
    let (c, rust) = load_libs();
    unsafe {
        let alloc_c: Symbol<unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char> =
            c.get(b"stbds_stralloc").unwrap();
        let alloc_r: Symbol<unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char> =
            rust.get(b"stbds_stralloc").unwrap();
        let reset_c: Symbol<unsafe extern "C" fn(*mut StringArena)> =
            c.get(b"stbds_strreset").unwrap();
        let reset_r: Symbol<unsafe extern "C" fn(*mut StringArena)> =
            rust.get(b"stbds_strreset").unwrap();

        let mut a_c = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut a_r = a_c;

        for s in [
            "hi\0".as_bytes().to_vec(),
            "longer string\0".as_bytes().to_vec(),
            "abc\0".as_bytes().to_vec(),
            "x\0".as_bytes().to_vec(),
            vec![b'A'; 1023].into_iter().chain([0u8]).collect::<Vec<u8>>(),
        ] {
            let mut sc = s.clone();
            let mut sr = s.clone();
            let pc = alloc_c(&mut a_c, sc.as_mut_ptr() as *mut c_char);
            let pr = alloc_r(&mut a_r, sr.as_mut_ptr() as *mut c_char);

            // Both should hold strcmp-equal contents to the input.
            let len = s.len();
            let pc_slice = std::slice::from_raw_parts(pc as *const u8, len);
            let pr_slice = std::slice::from_raw_parts(pr as *const u8, len);
            assert_eq!(pc_slice, pr_slice);
            assert_eq!(pc_slice, s.as_slice());

            // Bookkeeping fields should also match.
            assert_eq!(a_c.remaining, a_r.remaining);
            assert_eq!(a_c.block, a_r.block);
        }

        reset_c(&mut a_c);
        reset_r(&mut a_r);
        assert!(a_c.storage.is_null());
        assert!(a_r.storage.is_null());
        assert_eq!(a_c.remaining, 0);
        assert_eq!(a_r.remaining, 0);
        assert_eq!(a_c.block, 0);
        assert_eq!(a_r.block, 0);
        assert_eq!(a_c.mode, 0);
        assert_eq!(a_r.mode, 0);
    }
}

// ---------------------------------------------------------------------------
// strkey: writes "test_%d" into a static buffer.
// ---------------------------------------------------------------------------

#[test]
fn strkey_returns_test_n_string() {
    let (c, rust) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> = c.get(b"strkey").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(c_int) -> *mut c_char> = rust.get(b"strkey").unwrap();
        for n in [0, 1, 42, -7, 12345] {
            let pc = f_c(n);
            let pr = f_r(n);
            let sc = std::ffi::CStr::from_ptr(pc).to_bytes();
            let sr = std::ffi::CStr::from_ptr(pr).to_bytes();
            assert_eq!(sc, sr);
            let expected = format!("test_{n}");
            assert_eq!(sc, expected.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// arr_push: the only public C API symbol (declared in lib.h).
// ---------------------------------------------------------------------------

#[test]
fn arr_push_returns_no_panic_for_various_sizes() {
    let (c, rust) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(c_int)> = c.get(b"arr_push").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(c_int)> = rust.get(b"arr_push").unwrap();
        for n in [0, 1, 50, 100, 250, 500, 1000] {
            f_c(n);
            f_r(n);
        }
    }
}
