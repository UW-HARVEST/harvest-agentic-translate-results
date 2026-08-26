//! Phase C — error / rejection differential tests, one per `ERRORS.md` row.
//!
//! Rows that are fatal in C (NULL dereference, `assert()` → `abort()`) are run
//! in a *child process*: the `#[ignore]`d `child_*` case below is re-executed
//! twice (`DIFFTEST_CHILD_LIB=c` / `=rust`) and the parent compares the exit
//! signal, the exit code and the normalised `assert()` diagnostic.  "Both
//! crashed somehow" is never accepted — the *same* signal and the *same*
//! message are required.

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};

fn bin_cfg(elemsize: usize, keysize: usize) -> MapCfg {
    MapCfg {
        elemsize,
        keysize,
        key_is_ptr: false,
    }
}

fn str_cfg(elemsize: usize) -> MapCfg {
    MapCfg {
        elemsize,
        keysize: 8,
        key_is_ptr: true,
    }
}

// ===========================================================================
// Rows 1-9 — stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
struct ArrObs {
    is_null: bool,
    length: usize,
    capacity: usize,
    hash_table_null: bool,
    temp: isize,
}

unsafe fn arr_obs(a: *mut c_void) -> ArrObs {
    if a.is_null() {
        return ArrObs {
            is_null: true,
            length: 0,
            capacity: 0,
            hash_table_null: true,
            temp: 0,
        };
    }
    let h = header(a);
    ArrObs {
        is_null: false,
        length: (*h).length,
        capacity: (*h).capacity,
        hash_table_null: (*h).hash_table.is_null(),
        temp: (*h).temp,
    }
}

fn grow_both(a: [*mut c_void; 2], elemsize: usize, addlen: usize, min_cap: usize) -> [*mut c_void; 2] {
    let [c, r] = both();
    let ac = unsafe { (c.arrgrowf)(a[0], elemsize, addlen, min_cap) };
    let ar = unsafe { (r.arrgrowf)(a[1], elemsize, addlen, min_cap) };
    let oc = unsafe { arr_obs(ac) };
    let or_ = unsafe { arr_obs(ar) };
    assert_eq!(
        oc, or_,
        "arrgrowf(elemsize={elemsize}, addlen={addlen}, min_cap={min_cap})"
    );
    [ac, ar]
}

fn free_both(a: [*mut c_void; 2]) {
    let [c, r] = both();
    if !a[0].is_null() {
        unsafe { (c.arrfreef)(a[0]) };
    }
    if !a[1].is_null() {
        unsafe { (r.arrfreef)(a[1]) };
    }
}

#[test]
fn err_arrgrowf_null_a() {
    // Row 1
    for &elemsize in &[1usize, 8, 16, 24] {
        let a = grow_both([std::ptr::null_mut(); 2], elemsize, 0, 1);
        let o = unsafe { arr_obs(a[0]) };
        assert_eq!(o.length, 0);
        assert_eq!(o.temp, 0);
        assert!(o.hash_table_null);
        assert_eq!(o.capacity, 4, "min_cap 1 is bumped to the minimum 4");
        free_both(a);
    }
}

#[test]
fn err_arrgrowf_noop_returns_same_ptr() {
    // Row 2: min_cap <= arrcap -> the very same pointer comes back, untouched.
    let [c, r] = both();
    for &elemsize in &[1usize, 8, 16] {
        let a = grow_both([std::ptr::null_mut(); 2], elemsize, 0, 10);
        let cap = unsafe { (*header(a[0])).capacity };
        unsafe {
            (*header(a[0])).length = 3;
            (*header(a[1])).length = 3;
            (*header(a[0])).temp = 7;
            (*header(a[1])).temp = 7;
        }
        for &target in &[0usize, 1, cap - 1, cap] {
            let ac = unsafe { (c.arrgrowf)(a[0], elemsize, 0, target) };
            let ar = unsafe { (r.arrgrowf)(a[1], elemsize, 0, target) };
            assert_eq!(ac, a[0], "C must return the identical pointer");
            assert_eq!(ar, a[1], "Rust must return the identical pointer");
            let oc = unsafe { arr_obs(ac) };
            let or_ = unsafe { arr_obs(ar) };
            assert_eq!(oc, or_);
            assert_eq!(oc.capacity, cap, "capacity untouched");
            assert_eq!(oc.length, 3, "length untouched");
            assert_eq!(oc.temp, 7, "temp untouched");
        }
        free_both(a);
    }
}

#[test]
fn err_arrgrowf_min_cap_zero() {
    // Row 3: addlen == 0 && min_cap == 0 && a == NULL -> NULL comes straight
    // back (no allocation at all).
    let [c, r] = both();
    for &elemsize in &[0usize, 1, 8, 16, 1024] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        assert!(ac.is_null(), "C arrgrowf(NULL,{elemsize},0,0) must be NULL");
        assert!(ar.is_null(), "Rust arrgrowf(NULL,{elemsize},0,0) must be NULL");
    }
}

#[test]
fn err_arrgrowf_min_cap_below_4() {
    // Row 4: min_cap 1..=3 with a == NULL is bumped to 4.
    for &min_cap in &[1usize, 2, 3] {
        for &elemsize in &[1usize, 8, 16] {
            let a = grow_both([std::ptr::null_mut(); 2], elemsize, 0, min_cap);
            assert_eq!(unsafe { arr_obs(a[0]) }.capacity, 4);
            free_both(a);
        }
    }
    // min_cap 4 and 5 are used verbatim
    for &min_cap in &[4usize, 5, 100] {
        let a = grow_both([std::ptr::null_mut(); 2], 8, 0, min_cap);
        assert_eq!(unsafe { arr_obs(a[0]) }.capacity, min_cap);
        free_both(a);
    }
}

#[test]
fn err_arrgrowf_doubling() {
    // Row 5: cap < min_cap < 2*cap -> min_cap is raised to 2*cap.
    for &elemsize in &[1usize, 8, 16] {
        let mut a = grow_both([std::ptr::null_mut(); 2], elemsize, 0, 10);
        let cap = unsafe { (*header(a[0])).capacity };
        assert_eq!(cap, 10);
        for target in cap + 1..2 * cap {
            let b = grow_both(a, elemsize, 0, target);
            assert_eq!(
                unsafe { arr_obs(b[0]) }.capacity,
                2 * cap,
                "target={target} must be raised to 2*cap"
            );
            free_both(b);
            a = grow_both([std::ptr::null_mut(); 2], elemsize, 0, 10);
        }
        // >= 2*cap is used verbatim
        let b = grow_both(a, elemsize, 0, 2 * cap + 5);
        assert_eq!(unsafe { arr_obs(b[0]) }.capacity, 2 * cap + 5);
        free_both(b);
    }
}

#[test]
fn err_arrgrowf_addlen_wrap() {
    // Row 6: `arrlen(a) + addlen` wraps size_t.  With length == 4:
    //   addlen == SIZE_MAX-3 -> min_len wraps to 0   -> min_cap 0  <= cap -> identity
    //   addlen == SIZE_MAX   -> min_len wraps to 3   -> min_cap 3  <= cap -> identity
    //   addlen == SIZE_MAX-4 -> min_len == SIZE_MAX  -> min_cap SIZE_MAX, and
    //                           `elemsize*min_cap + 32` itself wraps to 24, so
    //                           realloc SHRINKS the block and the C then writes
    //                           `capacity = SIZE_MAX` into it.
    let [c, r] = both();
    for &(addlen, expect_identity) in &[
        (usize::MAX - 3, true),
        (usize::MAX, true),
        (usize::MAX - 1, true),
        (usize::MAX - 2, true),
    ] {
        let a = grow_both([std::ptr::null_mut(); 2], 8, 0, 8);
        unsafe {
            (*header(a[0])).length = 4;
            (*header(a[1])).length = 4;
            (*header(a[0])).temp = 33;
            (*header(a[1])).temp = 33;
        }
        let ac = unsafe { (c.arrgrowf)(a[0], 8, addlen, 0) };
        let ar = unsafe { (r.arrgrowf)(a[1], 8, addlen, 0) };
        let same_c = ac == a[0];
        let same_r = ar == a[1];
        assert_eq!(same_c, same_r, "addlen={addlen:#x}: identity return must agree");
        assert_eq!(same_c, expect_identity, "addlen={addlen:#x}");
        assert_eq!(unsafe { arr_obs(ac) }, unsafe { arr_obs(ar) });
        free_both([ac, ar]);
    }

    // The shrinking case: only `capacity` (offset 8 of the 24 byte block that
    // realloc leaves behind) is still inside the allocation, so only that is
    // compared — `length`/`temp` would be reads past the end of the block.
    for &addlen in &[usize::MAX - 4, usize::MAX - 5] {
        let a = grow_both([std::ptr::null_mut(); 2], 8, 0, 8);
        unsafe {
            (*header(a[0])).length = 4;
            (*header(a[1])).length = 4;
        }
        let ac = unsafe { (c.arrgrowf)(a[0], 8, addlen, 0) };
        let ar = unsafe { (r.arrgrowf)(a[1], 8, addlen, 0) };
        // (whether realloc shrinks in place is an allocator detail, so the
        // pointer identity is not compared here — `capacity` is.)
        let cap_c = unsafe { (*header(ac)).capacity };
        let cap_r = unsafe { (*header(ar)).capacity };
        assert_eq!(cap_c, cap_r, "wrapped capacity for addlen={addlen:#x}");
        assert_eq!(cap_c, 4usize.wrapping_add(addlen), "min_cap == min_len");
        // free the (shrunken) blocks
        free_both([ac, ar]);
    }
}

#[test]
fn err_arrgrowf_elemsize_zero() {
    // Row 7: elemsize == 0 -> only the 32 byte header is allocated.
    for &min_cap in &[1usize, 4, 100, 1 << 20] {
        let a = grow_both([std::ptr::null_mut(); 2], 0, 0, min_cap);
        let o = unsafe { arr_obs(a[0]) };
        assert_eq!(o.capacity, min_cap.max(4));
        assert_eq!(o.length, 0);
        free_both(a);
    }
}

#[test]
fn err_arrgrowf_oom_segv() {
    // Row 8: realloc fails -> the C writes through `NULL + 32`.
    assert_same_crash("child_arrgrowf_oom");
}

#[test]
#[ignore]
fn child_arrgrowf_oom() {
    let l = child_lib();
    // 2^20 * 2^40 == 2^60 bytes: realloc cannot satisfy this.
    let a = unsafe { (l.arrgrowf)(std::ptr::null_mut(), 1 << 20, 0, 1 << 40) };
    // Unreachable: the C dereferences (stbds_array_header*)(NULL+32) - 1.
    println!("unexpectedly survived: {a:p}");
}

#[test]
fn err_arrfreef_null_crashes() {
    // Row 9: free((stbds_array_header *) NULL - 1) == free((void *) -32).
    assert_same_crash("child_arrfreef_null");
}

#[test]
#[ignore]
fn child_arrfreef_null() {
    let l = child_lib();
    unsafe { (l.arrfreef)(std::ptr::null_mut()) };
    println!("unexpectedly survived");
}

// ===========================================================================
// Rows 10-15 — hashing
// ===========================================================================

#[test]
fn err_hash_string_empty() {
    // Row 10
    let [c, r] = both();
    let empty = b"\0";
    for &s in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
        let a = unsafe { (c.hash_string)(empty.as_ptr() as *mut c_char, s) };
        let b = unsafe { (r.hash_string)(empty.as_ptr() as *mut c_char, s) };
        assert_eq!(a, b, "hash_string(\"\", {s:#x})");
    }
}

#[test]
fn err_hash_string_null_segv() {
    // Row 11
    assert_same_crash("child_hash_string_null");
}

#[test]
#[ignore]
fn child_hash_string_null() {
    let l = child_lib();
    let h = unsafe { (l.hash_string)(std::ptr::null_mut(), 0) };
    println!("unexpectedly survived: {h:#x}");
}

#[test]
fn err_hash_bytes_zero_len_null_ptr() {
    // Row 12
    let [c, r] = both();
    for &s in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
        let a = unsafe { (c.hash_bytes)(std::ptr::null_mut(), 0, s) };
        let b = unsafe { (r.hash_bytes)(std::ptr::null_mut(), 0, s) };
        assert_eq!(a, b, "hash_bytes(NULL, 0, {s:#x})");
    }
}

#[test]
fn err_hash_bytes_tail_all_lengths() {
    // Row 13: all 7 `switch` fall-through cases, with the high bit of d[3] set
    // (the `int` shift sign-extends into the top 32 bits).
    let [c, r] = both();
    let mut rng = Rng::new(0xE001);
    for len in 0..=7usize {
        for _ in 0..64 {
            let mut buf = rng.bytes(8);
            buf[3] |= 0x80;
            buf[7] |= 0x80;
            for &s in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
                let p = buf.as_mut_ptr() as *mut c_void;
                let a = unsafe { (c.hash_bytes)(p, len, s) };
                let b = unsafe { (r.hash_bytes)(p, len, s) };
                assert_eq!(a, b, "tail len={len} seed={s:#x} buf={buf:02x?}");
            }
        }
    }
}

#[test]
fn err_hash_bytes_len_shift_wrap() {
    // Row 14: `data = len << 56` keeps only len & 0xff, so len and len+256 must
    // produce the same *initial* `data` (verified indirectly: both libraries
    // agree for every len around the 256 boundary).
    let [c, r] = both();
    let mut rng = Rng::new(0xE002);
    let mut buf = rng.bytes(1200);
    for &len in &[0usize, 1, 255, 256, 257, 511, 512, 513, 1023, 1024] {
        for &s in &[0usize, DEFAULT_SEED, usize::MAX] {
            let p = buf.as_mut_ptr() as *mut c_void;
            let a = unsafe { (c.hash_bytes)(p, len, s) };
            let b = unsafe { (r.hash_bytes)(p, len, s) };
            assert_eq!(a, b, "len={len} seed={s:#x}");
        }
    }
}

#[test]
fn err_hash_bytes_null_ptr_nonzero_len() {
    // Row 15
    assert_same_crash("child_hash_bytes_null");
}

#[test]
#[ignore]
fn child_hash_bytes_null() {
    let l = child_lib();
    let h = unsafe { (l.hash_bytes)(std::ptr::null_mut(), 8, DEFAULT_SEED) };
    println!("unexpectedly survived: {h:#x}");
}

// ===========================================================================
// Rows 16-22 — stbds_hmget_key / stbds_hmget_key_ts
// ===========================================================================

#[test]
fn err_hmget_ts_null_a() {
    // Row 16: a == NULL -> the sentinel element is created, *temp = -1, and the
    // key is never looked at (it may even be NULL).
    let [c, r] = both();
    for &elemsize in &[8usize, 16, 24] {
        seed_both(DEFAULT_SEED);
        let mut tc: isize = 0x1234;
        let mut tr: isize = 0x1234;
        let ac = unsafe {
            (c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                std::ptr::null_mut(),
                8,
                &mut tc,
                STBDS_HM_BINARY,
            )
        };
        let ar = unsafe {
            (r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                std::ptr::null_mut(),
                8,
                &mut tr,
                STBDS_HM_BINARY,
            )
        };
        assert_eq!(tc, -1, "C *temp");
        assert_eq!(tr, -1, "Rust *temp");
        assert!(!ac.is_null() && !ar.is_null(), "a fresh array is returned");
        let sc = unsafe { snapshot(ac, elemsize, 8, false, false, &ExternRanges::default()) };
        let sr = unsafe { snapshot(ar, elemsize, 8, false, false, &ExternRanges::default()) };
        diff_snapshots("hmget_key_ts(NULL)", &sc, &sr);
        assert_eq!(sc.length, 1);
        assert!(!sc.has_table);
        let [lc, lr] = both();
        unsafe {
            (lc.hmfree_func)((ac as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (lr.hmfree_func)((ar as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn err_hmget_ts_no_table() {
    // Row 17: hash_table == 0 -> *temp = -1, no hashing, `a` unchanged.
    for &elemsize in &[8usize, 16, 24] {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new("hmget_ts no table".to_string(), bin_cfg(elemsize, 8.min(elemsize)));
        p.put_default();
        assert!(!p.snapshot(0).has_table);
        let before = p.t;
        // key == NULL is safe here: the C returns before touching it
        assert_eq!(p.get_ts(std::ptr::null_mut(), STBDS_HM_BINARY), -1);
        assert_eq!(before, p.t, "`a` must come back unchanged");
        p.check("after get_ts on table-less map");
        p.free();
    }
}

#[test]
fn err_hmget_ts_missing_key() {
    // Row 18: find_slot -> -1 -> *temp = STBDS_INDEX_EMPTY.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE003);
    let mut p = Pair::new("hmget_ts miss".to_string(), bin_cfg(16, 8));
    for _ in 0..20 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    for _ in 0..50 {
        let mut kb = rng.bytes(8);
        kb[0] ^= 0xFF;
        let k = p.intern(&kb) as *mut c_void;
        assert_eq!(p.get_ts(k, STBDS_HM_BINARY), -1);
        p.check("after missing get_ts");
    }
    p.free();
}

#[test]
fn err_hmget_null_a_sets_temp() {
    // Row 19: hmget_key also writes header->temp = -1 in the fresh array.
    let [c, r] = both();
    for &elemsize in &[8usize, 16, 24] {
        seed_both(DEFAULT_SEED);
        let ac = unsafe {
            (c.hmget_key)(std::ptr::null_mut(), elemsize, std::ptr::null_mut(), 8, STBDS_HM_BINARY)
        };
        let ar = unsafe {
            (r.hmget_key)(std::ptr::null_mut(), elemsize, std::ptr::null_mut(), 8, STBDS_HM_BINARY)
        };
        let sc = unsafe { snapshot(ac, elemsize, 8, false, false, &ExternRanges::default()) };
        let sr = unsafe { snapshot(ar, elemsize, 8, false, false, &ExternRanges::default()) };
        diff_snapshots("hmget_key(NULL)", &sc, &sr);
        assert_eq!(sc.temp, -1, "header->temp must be -1");
        unsafe {
            (c.hmfree_func)((ac as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((ar as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn err_hmget_missing_key_sets_temp() {
    // Row 20
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE004);
    let mut p = Pair::new("hmget miss".to_string(), bin_cfg(16, 8));
    for _ in 0..20 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    for _ in 0..50 {
        let mut kb = rng.bytes(8);
        kb[7] ^= 0xFF;
        let k = p.intern(&kb) as *mut c_void;
        assert_eq!(p.get(k, STBDS_HM_BINARY), -1);
        assert_eq!(p.snapshot(0).temp, -1);
        p.check("after missing get");
    }
    p.free();
}

const OUT_OF_RANGE_MODES: [c_int; 8] = [-2, -1, 0, 1, 2, 7, i32::MIN, i32::MAX];

#[test]
fn err_mode_out_of_range_get() {
    // Row 21: `mode` is never validated — `>= 1` means "string", `< 1` means
    // "binary", for *every* int.
    for &mode in &OUT_OF_RANGE_MODES {
        seed_both(DEFAULT_SEED);
        // a NULL map short-circuits before `mode` is even inspected
        let [c, r] = both();
        let mut tc: isize = 5;
        let mut tr: isize = 5;
        let ac = unsafe {
            (c.hmget_key_ts)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, &mut tc, mode)
        };
        let ar = unsafe {
            (r.hmget_key_ts)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, &mut tr, mode)
        };
        assert_eq!(tc, -1, "mode={mode}");
        assert_eq!(tr, -1, "mode={mode}");
        unsafe {
            (c.hmfree_func)((ac as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((ar as *mut u8).sub(16) as *mut c_void, 16);
        }

        // a populated map, looked up with the matching key shape
        let string_shaped = mode >= STBDS_HM_STRING;
        seed_both(DEFAULT_SEED);
        let cfg = if string_shaped { str_cfg(16) } else { bin_cfg(16, 8) };
        let mut p = Pair::new(format!("mode={mode} get"), cfg);
        let mut rng = Rng::new(0xE005);
        let mut ptrs = Vec::new();
        for _ in 0..5 {
            let mut kb = rng.ascii(7);
            kb.push(0);
            let k = if string_shaped {
                p.intern_cstr(&kb[..7]) as *mut c_void
            } else {
                p.intern(&kb) as *mut c_void
            };
            ptrs.push(k);
            assert!(p.put(k, mode, rng.next_u64()) >= 0, "mode={mode} put");
        }
        p.check(&format!("mode={mode} populated"));
        for &k in &ptrs {
            assert!(p.get(k, mode) >= 0, "mode={mode} hit");
            assert!(p.get_ts(k, mode) >= 0, "mode={mode} hit via _ts");
        }
        // a key that is definitely absent
        let mut kb = rng.ascii(7);
        kb.push(0);
        let miss = if string_shaped {
            p.intern_cstr(b"zzzzzzz") as *mut c_void
        } else {
            p.intern(&kb) as *mut c_void
        };
        assert_eq!(p.get(miss, mode), -1, "mode={mode} miss");
        p.check(&format!("mode={mode} miss"));
        p.free();
    }
}

#[test]
fn err_hmget_ts_null_temp_segv() {
    // Row 22
    assert_same_crash("child_hmget_ts_null_temp");
}

#[test]
#[ignore]
fn child_hmget_ts_null_temp() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let a = (l.hmget_key_ts)(
            std::ptr::null_mut(),
            16,
            std::ptr::null_mut(),
            8,
            std::ptr::null_mut(),
            STBDS_HM_BINARY,
        );
        println!("unexpectedly survived: {a:p}");
    }
}

// ===========================================================================
// Rows 23-25 — stbds_hmput_default
// ===========================================================================

#[test]
fn err_hmput_default_null_a() {
    // Row 23
    for &elemsize in &[8usize, 16, 24, 32] {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new("hmput_default NULL".to_string(), bin_cfg(elemsize, 8.min(elemsize)));
        p.put_default();
        let s = p.snapshot(0);
        assert_eq!(s.length, 1);
        assert_eq!(s.capacity, 4);
        assert!(!s.has_table);
        assert_eq!(s.temp, 0);
        assert_eq!(
            s.elems[0],
            ElemSnap::Bytes(vec![0u8; elemsize]),
            "element 0 must be zeroed"
        );
        p.check("hmput_default(NULL)");
        p.free();
    }
}

#[test]
fn err_hmput_default_zero_length() {
    // Row 24: a != NULL but header->length == 0 takes the same allocation path.
    let [c, r] = both();
    for &elemsize in &[8usize, 16, 24] {
        seed_both(DEFAULT_SEED);
        // hand-build an array with length == 0 and turn it into a hash pointer
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4) };
        let hc = unsafe { (ac as *mut u8).add(elemsize) as *mut c_void };
        let hr = unsafe { (ar as *mut u8).add(elemsize) as *mut c_void };
        let dc = unsafe { (c.hmput_default)(hc, elemsize) };
        let dr = unsafe { (r.hmput_default)(hr, elemsize) };
        let sc = unsafe { snapshot(dc, elemsize, elemsize, false, false, &ExternRanges::default()) };
        let sr = unsafe { snapshot(dr, elemsize, elemsize, false, false, &ExternRanges::default()) };
        diff_snapshots("hmput_default(length==0)", &sc, &sr);
        assert_eq!(sc.length, 1);
        unsafe {
            (c.hmfree_func)((dc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((dr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn err_hmput_default_idempotent() {
    // Row 25: length > 0 -> `a` is returned unchanged.
    for &elemsize in &[8usize, 16, 24] {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new("hmput_default idem".to_string(), bin_cfg(elemsize, 8.min(elemsize)));
        p.put_default();
        for _ in 0..5 {
            let before = p.t;
            p.put_default();
            assert_eq!(before, p.t, "must be a no-op");
            p.check("repeat hmput_default");
        }
        p.free();
    }
}

// ===========================================================================
// Rows 26-41 — stbds_hmput_key
// ===========================================================================

#[test]
fn err_hmput_null_a() {
    // Row 26 + Row 27: bootstrap + index creation, and `nt->string.mode` is
    // SH_DEFAULT for mode>=1 and 0 otherwise (C L707).
    for &mode in &[-1i32, 0, 1, 2, 7, i32::MAX] {
        seed_both(DEFAULT_SEED);
        let string_shaped = mode >= STBDS_HM_STRING;
        let cfg = if string_shaped { str_cfg(16) } else { bin_cfg(16, 8) };
        let mut p = Pair::new(format!("bootstrap mode={mode}"), cfg);
        let k = if string_shaped {
            p.intern_cstr(b"bootstrap") as *mut c_void
        } else {
            p.intern(b"bootstr\0") as *mut c_void
        };
        assert_eq!(p.put(k, mode, 1), 0);
        assert!(p.snapshot(0).has_table, "an 8-slot index must exist now");
        assert_eq!(
            p.snapshot(0).table.as_ref().unwrap().slot_count,
            8,
            "the first index has STBDS_BUCKET_LENGTH slots"
        );
        assert_eq!(
            p.string_mode(0),
            Some(if string_shaped { 1 } else { 0 }),
            "mode={mode} -> string.mode"
        );
        p.check(&format!("bootstrap mode={mode}"));
        p.free();
    }
}

#[test]
fn err_hmput_builds_index() {
    // Row 27 (thresholds of the freshly built 8-slot index)
    seed_both(DEFAULT_SEED);
    let mut p = Pair::new("index thresholds".to_string(), bin_cfg(16, 8));
    let k = p.intern(b"aaaaaaa\0") as *mut c_void;
    p.put(k, STBDS_HM_BINARY, 1);
    let t = p.snapshot(0).table.unwrap();
    assert_eq!(t.slot_count, 8);
    assert_eq!(t.slot_count_log2, 3);
    assert_eq!(t.used_count_threshold, 6, "8 - 8/4");
    assert_eq!(t.tombstone_count_threshold, 1, "8/8 + 8/16");
    assert_eq!(t.used_count_shrink_threshold, 0, "forced to 0 for slot_count<=8");
    assert_eq!(t.used_count, 1);
    p.check("thresholds");
    p.free();
}

#[test]
fn err_hmput_grow_threshold() {
    // Row 28: the insert that finds used_count >= used_count_threshold doubles
    // the index *before* inserting.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE006);
    let mut p = Pair::new("grow threshold".to_string(), bin_cfg(16, 8));
    let mut expect_slots = 8usize;
    for i in 0..200 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        let used_before = p
            .snapshot(0)
            .table
            .as_ref()
            .map(|t| t.used_count)
            .unwrap_or(0);
        let thr_before = p
            .snapshot(0)
            .table
            .as_ref()
            .map(|t| t.used_count_threshold)
            .unwrap_or(0);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
        let t = p.snapshot(0).table.unwrap();
        if i > 0 && used_before >= thr_before {
            expect_slots *= 2;
        }
        assert_eq!(t.slot_count, expect_slots, "insert {i}");
        p.check(&format!("grow {i}"));
    }
    p.free();
}

#[test]
fn err_hmput_duplicate_key() {
    // Row 29
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE007);
    let mut p = Pair::new("dup key".to_string(), bin_cfg(16, 8));
    let mut ptrs = Vec::new();
    for _ in 0..30 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        ptrs.push(k);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    let len = p.snapshot(0).length;
    let used = p.snapshot(0).table.unwrap().used_count;
    for (i, &k) in ptrs.iter().enumerate() {
        assert_eq!(p.put(k, STBDS_HM_BINARY, rng.next_u64()), i as isize);
        assert_eq!(p.snapshot(0).length, len, "no new element");
        assert_eq!(p.snapshot(0).table.unwrap().used_count, used, "no new slot");
        p.check(&format!("dup {i}"));
    }
    p.free();
}

#[test]
fn err_hmput_duplicate_wraparound_no_tempkey() {
    // Row 30: the duplicate branch of the *wrap-around* scan (C L746-751) sets
    // `temp` but — unlike the forward scan at L729-735 — does not refresh
    // `stbds_temp_key`.  Driving both scans with many keys and re-inserting each
    // one exercises whichever branch each key falls into; the resulting
    // `temp_key` is compared by the snapshot.
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0xE008 + s);
        let mut p = Pair::new(format!("wrap dup s={s}"), str_cfg(16));
        p.shmode(STBDS_SH_STRDUP);
        let mut ptrs = Vec::new();
        for i in 0..40 {
            let kb = format!("wrapkey{i:04}").into_bytes();
            let k = p.intern_cstr(&kb) as *mut c_void;
            ptrs.push(k);
            p.put(k, STBDS_HM_STRING, rng.next_u64());
        }
        p.check("populated");
        for (i, &k) in ptrs.iter().enumerate() {
            assert_eq!(p.put(k, STBDS_HM_STRING, rng.next_u64()), i as isize);
            p.check(&format!("wrap dup re-put {i}"));
        }
        p.free();
    }
}

#[test]
fn err_hmput_reserved_hash_bumped() {
    // Rows 31 + 83: `if (hash < 2) hash += 2` must be applied identically by
    // hmput_key and hm_find_slot, otherwise inserted keys become unfindable.
    // No key can be *made* to hash to 0/1 on demand, so instead every bucket
    // hash is compared directly (the snapshot does that) and it is asserted that
    // no live slot ever holds a reserved hash value with an in-use index.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE009);
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut p = Pair::new(format!("reserved hash s={s}"), bin_cfg(16, 8));
        for _ in 0..300 {
            let kb = rng.bytes(8);
            let k = p.intern(&kb) as *mut c_void;
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
        }
        p.check("populated");
        let t = p.snapshot(0).table.unwrap();
        for (i, &(h, idx)) in t.slots.iter().enumerate() {
            if idx >= 0 {
                assert!(h >= 2, "slot {i}: in-use entry with reserved hash {h}");
            }
        }
        p.free();
    }
}

#[test]
fn err_hmput_reuses_tombstone() {
    // Row 32: an insert that walks past a tombstone before finding the empty
    // slot re-uses the tombstone and decrements tombstone_count.
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0xE00A + s);
        let mut p = Pair::new(format!("tombstone reuse s={s}"), bin_cfg(16, 8));
        let mut ptrs = Vec::new();
        for _ in 0..40 {
            let kb = rng.bytes(8);
            let k = p.intern(&kb) as *mut c_void;
            ptrs.push(k);
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
        }
        // delete half of them to sprinkle tombstones around
        for i in (0..40).step_by(2) {
            assert_eq!(p.del(ptrs[i], 0, STBDS_HM_BINARY), 1);
        }
        p.check("tombstones planted");
        let tomb_before = p.snapshot(0).table.unwrap().tombstone_count;
        assert!(tomb_before > 0, "expected tombstones");
        for i in 0..40 {
            let kb = rng.bytes(8);
            let k = p.intern(&kb) as *mut c_void;
            assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
            p.check(&format!("reuse insert {i}"));
        }
        p.free();
    }
}

#[test]
fn err_assert_778_unreachable() {
    // Row 33: `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` (C L778) is a
    // tautology — L774-775 grows the array whenever i+1 > cap.  Hammer the
    // exact boundary (capacity == length after every insert) for both libraries
    // and require that neither aborts and both stay in step.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE00B);
    let mut p = Pair::new("assert778".to_string(), bin_cfg(16, 8));
    for i in 0..400 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
        let s = p.snapshot(0);
        assert!(s.length <= s.capacity, "insert {i}: length {} cap {}", s.length, s.capacity);
        p.check(&format!("assert778 insert {i}"));
    }
    p.free();
}

#[test]
fn err_hmput_string_mode_strdup() {
    // Row 34
    seed_both(DEFAULT_SEED);
    let mut p = Pair::new("strdup mode".to_string(), str_cfg(16));
    p.shmode(STBDS_SH_STRDUP);
    let k = p.intern_cstr(b"owned-by-lib") as *mut c_void;
    assert_eq!(p.put(k, STBDS_HM_STRING, 1), 0);
    p.check("strdup insert");
    // the stored pointer must NOT be the caller's buffer
    for i in 0..2 {
        unsafe {
            let stored = *(p.t[i] as *mut *mut c_char);
            assert_ne!(stored as *mut c_void, k, "lib {i} must own a copy");
            assert_eq!(cstr_bytes(stored), b"owned-by-lib");
            let tp = (*header((p.t[i] as *mut u8).sub(16) as *mut c_void)).hash_table
                as *mut HashIndex;
            assert_eq!((*tp).temp_key, stored, "temp_key must be the copy");
        }
    }
    p.free();
}

#[test]
fn err_hmput_string_mode_arena() {
    // Row 35
    seed_both(DEFAULT_SEED);
    let mut p = Pair::new("arena mode".to_string(), str_cfg(16));
    p.shmode(STBDS_SH_ARENA);
    let k = p.intern_cstr(b"in-the-arena") as *mut c_void;
    assert_eq!(p.put(k, STBDS_HM_STRING, 1), 0);
    p.check("arena insert");
    for i in 0..2 {
        unsafe {
            let stored = *(p.t[i] as *mut *mut c_char);
            assert_ne!(stored as *mut c_void, k, "lib {i} must own a copy");
            assert_eq!(cstr_bytes(stored), b"in-the-arena");
            let tp = (*header((p.t[i] as *mut u8).sub(16) as *mut c_void)).hash_table
                as *mut HashIndex;
            assert_eq!((*tp).temp_key, stored);
            assert!(!(*tp).string.storage.is_null(), "arena block allocated");
        }
    }
    p.free();
}

#[test]
fn err_hmput_string_mode_default() {
    // Row 36: SH_DEFAULT stores the caller's pointer verbatim.
    seed_both(DEFAULT_SEED);
    let mut p = Pair::new("default mode".to_string(), str_cfg(16));
    p.shmode(STBDS_SH_DEFAULT);
    let k = p.intern_cstr(b"caller-owned") as *mut c_void;
    assert_eq!(p.put(k, STBDS_HM_STRING, 1), 0);
    p.check("default insert");
    for i in 0..2 {
        unsafe {
            let stored = *(p.t[i] as *mut *mut c_char);
            assert_eq!(stored as *mut c_void, k, "lib {i} must store the pointer as-is");
        }
    }
    p.free();
}

#[test]
fn err_hmput_string_mode_out_of_range() {
    // Row 37: any other string.mode falls into the `switch` default and
    // memcpy's `keysize` bytes of the key *text* into the element.
    for &sh in &[0i32, 4, 5, 128, 255] {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new(format!("memcpy mode sh={sh}"), bin_cfg(16, 8));
        p.shmode(sh);
        assert_eq!(p.string_mode(0), Some(sh as u8));
        let k = p.intern_cstr(b"ABCDEFGH") as *mut c_void;
        assert_eq!(p.put(k, STBDS_HM_STRING, 1), 0);
        p.check(&format!("memcpy insert sh={sh}"));
        for i in 0..2 {
            unsafe {
                let bytes = std::slice::from_raw_parts(p.t[i] as *const u8, 8);
                assert_eq!(bytes, b"ABCDEFGH", "lib {i} must memcpy the key text");
            }
        }
        p.free();
    }
}

#[test]
fn err_memcpy_mode_lookup_segv() {
    // Row 37b: consequence of row 37 — a *string* lookup on such a table reads
    // the element's first 8 bytes as a `char *`, i.e. 0x4141414141414141.
    assert_same_crash("child_memcpy_mode_lookup");
}

#[test]
#[ignore]
fn child_memcpy_mode_lookup() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let key = b"AAAAAAAA\0";
        let kp = key.as_ptr() as *mut c_void;
        let mut t = (l.shmode_func)(16, STBDS_SH_NONE);
        t = (l.hmput_key)(t, 16, kp, 8, STBDS_HM_STRING);
        t = (l.hmget_key)(t, 16, kp, 8, STBDS_HM_STRING);
        println!("unexpectedly survived: {t:p}");
    }
}

#[test]
fn err_memcpy_mode_del_segv() {
    // Row 37c: same for hmdel_key (find_slot is shared).
    assert_same_crash("child_memcpy_mode_del");
}

#[test]
#[ignore]
fn child_memcpy_mode_del() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let key = b"AAAAAAAA\0";
        let kp = key.as_ptr() as *mut c_void;
        let mut t = (l.shmode_func)(16, STBDS_SH_NONE);
        t = (l.hmput_key)(t, 16, kp, 8, STBDS_HM_STRING);
        t = (l.hmdel_key)(t, 16, kp, 8, 0, STBDS_HM_STRING);
        println!("unexpectedly survived: {t:p}");
    }
}

#[test]
fn err_hmput_keysize_zero() {
    // Row 38
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE00C);
    let mut p = Pair::new("keysize 0".to_string(), bin_cfg(16, 0));
    for i in 0..20 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        assert_eq!(p.put(k, STBDS_HM_BINARY, rng.next_u64()), 0);
        assert_eq!(p.snapshot(0).length, 2, "insert {i}: only ever one element");
        assert_eq!(p.snapshot(0).table.unwrap().used_count, 1);
        p.check(&format!("keysize0 {i}"));
    }
    p.free();
}

#[test]
fn err_hmput_elemsize_zero() {
    // Row 39
    seed_both(DEFAULT_SEED);
    let mut p = Pair::new("elemsize 0".to_string(), bin_cfg(0, 0));
    for i in 0..10 {
        assert_eq!(p.put(std::ptr::null_mut(), STBDS_HM_BINARY, 0), 0);
        p.check(&format!("elemsize0 put {i}"));
    }
    assert_eq!(p.get(std::ptr::null_mut(), STBDS_HM_BINARY), 0);
    assert_eq!(p.get_ts(std::ptr::null_mut(), STBDS_HM_BINARY), 0);
    p.check("elemsize0 gets");
    assert_eq!(p.del(std::ptr::null_mut(), 0, STBDS_HM_BINARY), 1);
    p.check("elemsize0 del");
    assert_eq!(p.del(std::ptr::null_mut(), 0, STBDS_HM_BINARY), 0);
    p.check("elemsize0 del again");
    p.free();
}

#[test]
fn err_hmput_null_key_string_segv() {
    // Row 40
    assert_same_crash("child_hmput_null_key_string");
}

#[test]
#[ignore]
fn child_hmput_null_key_string() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let t = (l.hmput_key)(
            std::ptr::null_mut(),
            16,
            std::ptr::null_mut(),
            8,
            STBDS_HM_STRING,
        );
        println!("unexpectedly survived: {t:p}");
    }
}

#[test]
fn err_mode_out_of_range_put() {
    // Row 41
    for &mode in &OUT_OF_RANGE_MODES {
        seed_both(DEFAULT_SEED);
        let string_shaped = mode >= STBDS_HM_STRING;
        let cfg = if string_shaped { str_cfg(16) } else { bin_cfg(16, 8) };
        let mut p = Pair::new(format!("put mode={mode}"), cfg);
        let mut rng = Rng::new(0xE00D);
        for i in 0..12 {
            let kb = format!("mkey{i:03}").into_bytes();
            let k = if string_shaped {
                p.intern_cstr(&kb) as *mut c_void
            } else {
                let mut v = kb.clone();
                v.push(0);
                p.intern(&v) as *mut c_void
            };
            assert!(p.put(k, mode, rng.next_u64()) >= 0, "mode={mode} put {i}");
            p.check(&format!("put mode={mode} i={i}"));
        }
        assert_eq!(
            p.string_mode(0),
            Some(if string_shaped { 1 } else { 0 }),
            "L707: string.mode for mode={mode}"
        );
        p.free();
    }
}

// ===========================================================================
// Rows 42-43 — stbds_shmode_func
// ===========================================================================

#[test]
fn err_shmode_out_of_range() {
    // Row 42: `(unsigned char) mode` truncation, no validation whatsoever.
    let cases: [(c_int, u8); 12] = [
        (-2, 254),
        (-1, 255),
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (255, 255),
        (256, 0),
        (257, 1),
        (i32::MIN, 0),
        (i32::MAX, 255),
    ];
    for &(mode, expect) in &cases {
        seed_both(DEFAULT_SEED);
        let key_is_ptr = matches!(expect, 1 | 2 | 3);
        let cfg = if key_is_ptr { str_cfg(16) } else { bin_cfg(16, 8) };
        let mut p = Pair::new(format!("shmode {mode}"), cfg);
        p.shmode(mode);
        assert_eq!(p.string_mode(0), Some(expect), "C shmode_func({mode})");
        assert_eq!(p.string_mode(1), Some(expect), "Rust shmode_func({mode})");
        p.check(&format!("shmode {mode}"));
        p.free();
    }
}

#[test]
fn err_shmode_elemsize_zero() {
    // Row 43
    for mode in 0..4i32 {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new(format!("shmode es0 m={mode}"), bin_cfg(0, 0));
        p.shmode(mode);
        let s = p.snapshot(0);
        assert_eq!(s.length, 1);
        assert_eq!(s.capacity, 4);
        assert!(s.has_table);
        assert_eq!(s.table.as_ref().unwrap().slot_count, 8);
        p.check(&format!("shmode es0 m={mode}"));
        p.free();
    }
}

// ===========================================================================
// Rows 44-57 — stbds_hmdel_key
// ===========================================================================

#[test]
fn err_hmdel_null_a() {
    // Row 44: returns NULL (0), so the `hmdel` macro yields 0.
    let [c, r] = both();
    for &mode in &OUT_OF_RANGE_MODES {
        let ac = unsafe {
            (c.hmdel_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, 0, mode)
        };
        let ar = unsafe {
            (r.hmdel_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, 0, mode)
        };
        assert!(ac.is_null(), "C hmdel_key(NULL, mode={mode})");
        assert!(ar.is_null(), "Rust hmdel_key(NULL, mode={mode})");
    }
}

#[test]
fn err_hmdel_no_table() {
    // Row 45: hash_table == 0 -> temp = 0, `a` unchanged.
    for &elemsize in &[8usize, 16, 24] {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new("del no table".to_string(), bin_cfg(elemsize, 8.min(elemsize)));
        p.put_default();
        let before = p.t;
        assert_eq!(p.del(std::ptr::null_mut(), 0, STBDS_HM_BINARY), 0);
        assert_eq!(before, p.t);
        p.check("del on table-less map");
        p.free();
    }
}

#[test]
fn err_hmdel_missing_key() {
    // Row 46
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE00E);
    let mut p = Pair::new("del missing".to_string(), bin_cfg(16, 8));
    for _ in 0..20 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    let len = p.snapshot(0).length;
    let used = p.snapshot(0).table.unwrap().used_count;
    for _ in 0..50 {
        let mut kb = rng.bytes(8);
        kb[0] ^= 0xFF;
        let k = p.intern(&kb) as *mut c_void;
        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 0);
        assert_eq!(p.snapshot(0).length, len);
        assert_eq!(p.snapshot(0).table.unwrap().used_count, used);
        p.check("del missing");
    }
    p.free();
}

#[test]
fn err_hmdel_present_key() {
    // Row 47
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE00F);
    let mut p = Pair::new("del present".to_string(), bin_cfg(16, 8));
    let mut ptrs = Vec::new();
    for _ in 0..20 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        ptrs.push(k);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    for (i, &k) in ptrs.iter().enumerate() {
        let before = p.snapshot(0);
        let bt = before.table.as_ref().unwrap();
        let (used, tomb, len) = (bt.used_count, bt.tombstone_count, before.length);
        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1, "del {i}");
        let after = p.snapshot(0);
        let at = after.table.as_ref().unwrap();
        assert_eq!(after.length, len - 1, "length must drop by one");
        if at.slot_count == bt.slot_count && at.tombstone_count >= tomb {
            assert_eq!(at.used_count, used - 1);
            assert_eq!(at.tombstone_count, tomb + 1);
        }
        p.check(&format!("del present {i}"));
    }
    p.free();
}

#[test]
fn err_assert_828_unreachable() {
    // Row 48: `slot < (ptrdiff_t) table->slot_count` (C L828) cannot fail —
    // hm_find_slot masks `pos` with slot_count-1 and returns
    // `(pos & ~7) + i` with i < 8.  Verified by hammering deletes on tables of
    // every slot_count from 8 to 1024 and checking that no bucket index ever
    // exceeds slot_count.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE010);
    let mut p = Pair::new("assert828".to_string(), bin_cfg(16, 8));
    let mut ptrs = Vec::new();
    for _ in 0..800 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        ptrs.push(k);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    for (i, &k) in ptrs.iter().enumerate() {
        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1, "del {i}");
        let t = p.snapshot(0).table.unwrap();
        for &(_, idx) in &t.slots {
            assert!(idx < t.slot_count as isize, "bucket index out of range");
        }
        if i % 11 == 0 {
            p.check(&format!("assert828 del {i}"));
        }
    }
    p.check("assert828 emptied");
    p.free();
}

#[test]
fn err_hmdel_last_element_no_swap() {
    // Row 49
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE011);
    for s in 0..16u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut p = Pair::new(format!("del last s={s}"), bin_cfg(16, 8));
        let mut ptrs = Vec::new();
        for _ in 0..6 {
            let kb = rng.bytes(8);
            let k = p.intern(&kb) as *mut c_void;
            ptrs.push(k);
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
        }
        // element bytes of the survivors must be untouched by a last-element del
        let before: Vec<_> = p.snapshot(0).elems[..6].to_vec();
        assert_eq!(p.del(ptrs[5], 0, STBDS_HM_BINARY), 1);
        let after = p.snapshot(0);
        assert_eq!(after.elems.len(), 6, "sentinel + 5 survivors");
        assert_eq!(&after.elems[..6], &before[..6], "no data movement");
        p.check(&format!("del last s={s}"));
        p.free();
    }
}

#[test]
fn err_hmdel_mode_two_strdup_quirk() {
    // Row 50: `mode == 2` on a SH_STRDUP table.  The exact `mode == 1` tests at
    // C L836/L842 mean the strdup'ed key is NOT freed and the re-find takes the
    // *binary* branch.  With a single element (old_index == final_index) the
    // re-find is skipped, so this is observable without aborting.
    for &mode in &[2i32, 3, 7, i32::MAX] {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new(format!("del mode={mode} strdup"), str_cfg(16));
        p.shmode(STBDS_SH_STRDUP);
        let k = p.intern_cstr(b"only-one") as *mut c_void;
        assert_eq!(p.put(k, STBDS_HM_STRING, 1), 0);
        p.check(&format!("mode={mode} populated"));
        // old_index == final_index -> no memmove, no re-find, no assert
        assert_eq!(p.del(k, 0, mode), 1, "mode={mode} delete");
        p.check(&format!("mode={mode} deleted"));
        assert_eq!(p.snapshot(0).length, 1);
        p.free();
    }
}

#[test]
fn err_assert_846_via_mode2() {
    // Row 51: `mode == 2` delete of a *non-last* element of a string table.
    // C L842 takes the binary branch, so find_slot hashes `&elem.key` (the
    // pointer bytes) as a string, finds nothing and `STBDS_ASSERT(slot >= 0)`
    // fires.
    assert_same_crash("child_assert_846_mode2");
}

#[test]
#[ignore]
fn child_assert_846_mode2() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let elemsize = 16usize;
        let mut t = (l.shmode_func)(elemsize, STBDS_SH_STRDUP);
        let keys: [&[u8]; 3] = [b"aaa\0", b"bbb\0", b"ccc\0"];
        for k in keys {
            t = (l.hmput_key)(t, elemsize, k.as_ptr() as *mut c_void, 8, STBDS_HM_STRING);
        }
        // delete the element at index 0 with mode == 2
        t = (l.hmdel_key)(t, elemsize, keys[0].as_ptr() as *mut c_void, 8, 0, 2);
        println!("unexpectedly survived: {t:p}");
    }
}

#[test]
fn err_assert_849_corrupt_index() {
    // Row 52: `STBDS_ASSERT(b->index[i] == final_index)` (C L849).
    assert_same_crash("child_assert_849_corrupt");
}

#[test]
#[ignore]
fn child_assert_849_corrupt() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let elemsize = 16usize;
        let keys: [&[u8]; 3] = [b"k0aaaaa\0", b"k1bbbbb\0", b"k2ccccc\0"];
        let mut t: *mut c_void = std::ptr::null_mut();
        for k in keys {
            t = (l.hmput_key)(t, elemsize, k.as_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
        }
        let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
        let ti = (*header(raw)).hash_table as *mut HashIndex;
        // Point the *last* element's bucket entry at index 0 instead of 2.  The
        // memmove in hmdel_key copies element 2 over element 0, so the re-find
        // still matches the key at index 0 — but b->index[i] != final_index.
        let mut patched = false;
        for i in 0..(*ti).slot_count {
            let b = (*ti).storage.add(i >> 3);
            if (*b).index[i & 7] == 2 {
                (*b).index[i & 7] = 0;
                patched = true;
                break;
            }
        }
        assert!(patched, "could not find the bucket entry for index 2");
        t = (l.hmdel_key)(t, elemsize, keys[0].as_ptr() as *mut c_void, 8, 0, STBDS_HM_BINARY);
        println!("unexpectedly survived: {t:p}");
    }
}

#[test]
fn err_hmdel_shrink() {
    // Row 53
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE012);
    let mut p = Pair::new("del shrink".to_string(), bin_cfg(16, 8));
    let mut ptrs = Vec::new();
    for _ in 0..200 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        ptrs.push(k);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    let start_slots = p.snapshot(0).table.unwrap().slot_count;
    let mut shrank = false;
    for (i, &k) in ptrs.iter().enumerate() {
        let before = p.snapshot(0).table.unwrap().slot_count;
        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1);
        let after = p.snapshot(0).table.unwrap();
        if after.slot_count < before {
            shrank = true;
            assert_eq!(after.slot_count, before / 2, "shrink halves the index");
            assert_eq!(after.tombstone_count, 0, "tombstones purged by the rebuild");
        }
        assert!(after.slot_count >= 8, "never below STBDS_BUCKET_LENGTH");
        p.check(&format!("shrink del {i}"));
    }
    assert!(shrank, "expected at least one shrink (start={start_slots})");
    assert_eq!(
        p.snapshot(0).table.unwrap().slot_count,
        8,
        "an emptied table ends at the minimum size"
    );
    p.free();
}

#[test]
fn err_hmdel_rebuild_tombstones() {
    // Row 54: tombstone_count > tombstone_count_threshold triggers a same-size
    // rebuild (the `else if` at C L858).
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE013);
    let mut p = Pair::new("del rebuild".to_string(), bin_cfg(16, 8));
    let mut keep = Vec::new();
    for _ in 0..100 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        keep.push(k);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    let mut rebuilt = false;
    for i in 0..300 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        assert!(p.put(k, STBDS_HM_BINARY, rng.next_u64()) >= 0);
        let before = p.snapshot(0).table.unwrap();
        let (slots, tomb) = (before.slot_count, before.tombstone_count);
        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1);
        let after = p.snapshot(0).table.unwrap();
        if after.slot_count == slots && after.tombstone_count == 0 && tomb > 0 {
            rebuilt = true;
        }
        p.check(&format!("rebuild cycle {i}"));
    }
    assert!(rebuilt, "expected at least one same-size rebuild");
    for &k in &keep {
        assert!(p.get(k, STBDS_HM_BINARY) >= 0);
    }
    p.check("rebuild survivors");
    p.free();
}

#[test]
fn err_hmdel_keyoffset_nonzero() {
    // Row 55
    for &(elemsize, keyoffset) in &[(24usize, 8usize), (32, 16), (32, 24)] {
        seed_both(DEFAULT_SEED);
        let mut rng = Rng::new(0xE014);
        let mut p = Pair::new(
            format!("del keyoffset es={elemsize} ko={keyoffset}"),
            bin_cfg(elemsize, 8),
        );
        let mut keys = Vec::new();
        for _ in 0..10 {
            let kb = rng.bytes(8);
            let k = p.intern(&kb) as *mut c_void;
            let t = p.put(k, STBDS_HM_BINARY, rng.next_u64());
            for i in 0..2 {
                unsafe {
                    let e = (p.t[i] as *mut u8).add(elemsize * t as usize).add(keyoffset);
                    std::ptr::copy_nonoverlapping(kb.as_ptr(), e, 8);
                }
            }
            keys.push(k);
        }
        p.check("keyoffset populated");
        // A wrong keyoffset does not match -> temp = 0.  (Offset 0 *would* match,
        // because hmput_key always stores the key there as well, so pick an
        // offset that only holds value bytes.)
        let bogus = if keyoffset == 8 { 16 } else { 8 };
        assert_eq!(
            p.del(keys[0], bogus, STBDS_HM_BINARY),
            0,
            "wrong keyoffset {bogus} must not match"
        );
        p.check("keyoffset mismatch");
        for (i, &k) in keys.iter().enumerate() {
            assert_eq!(p.del(k, keyoffset, STBDS_HM_BINARY), 1, "del {i}");
            p.check(&format!("keyoffset del {i}"));
        }
        p.free();
    }
}

#[test]
fn err_mode_out_of_range_del() {
    // Row 56: `mode` is compared with `>=` in find_slot but with `==` twice in
    // hmdel_key.  Delete the last element so the (aborting) re-find is skipped.
    for &mode in &OUT_OF_RANGE_MODES {
        seed_both(DEFAULT_SEED);
        let string_shaped = mode >= STBDS_HM_STRING;
        let cfg = if string_shaped { str_cfg(16) } else { bin_cfg(16, 8) };
        let mut p = Pair::new(format!("del mode={mode}"), cfg);
        let mut ptrs = Vec::new();
        for i in 0..5 {
            let kb = format!("dkey{i:03}").into_bytes();
            let k = if string_shaped {
                p.intern_cstr(&kb) as *mut c_void
            } else {
                let mut v = kb.clone();
                v.push(0);
                p.intern(&v) as *mut c_void
            };
            ptrs.push(k);
            assert!(p.put(k, mode, 1) >= 0);
        }
        p.check(&format!("del mode={mode} populated"));
        for i in (0..5).rev() {
            assert_eq!(p.del(ptrs[i], 0, mode), 1, "mode={mode} del {i}");
            p.check(&format!("del mode={mode} i={i}"));
        }
        assert_eq!(p.del(ptrs[0], 0, mode), 0, "mode={mode} del missing");
        p.check(&format!("del mode={mode} missing"));
        p.free();
    }
}

#[test]
fn err_hmdel_null_key_segv() {
    // Row 57
    assert_same_crash("child_hmdel_null_key");
}

#[test]
#[ignore]
fn child_hmdel_null_key() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let elemsize = 16usize;
        let mut t = (l.shmode_func)(elemsize, STBDS_SH_STRDUP);
        t = (l.hmput_key)(
            t,
            elemsize,
            b"present\0".as_ptr() as *mut c_void,
            8,
            STBDS_HM_STRING,
        );
        t = (l.hmdel_key)(t, elemsize, std::ptr::null_mut(), 8, 0, STBDS_HM_STRING);
        println!("unexpectedly survived: {t:p}");
    }
}

// ===========================================================================
// Rows 58-61 — stbds_hmfree_func
// ===========================================================================

#[test]
fn err_hmfree_null_a() {
    // Row 58: the only explicit NULL guard in the whole file.
    let [c, r] = both();
    for &elemsize in &[0usize, 1, 8, 16, usize::MAX] {
        unsafe { (c.hmfree_func)(std::ptr::null_mut(), elemsize) };
        unsafe { (r.hmfree_func)(std::ptr::null_mut(), elemsize) };
    }
}

#[test]
fn err_hmfree_no_table() {
    // Row 59: hash_table == NULL -> the key/arena cleanup is skipped but the
    // header is still released.  A wrong free() here aborts the process.
    for &elemsize in &[8usize, 16, 24] {
        seed_both(DEFAULT_SEED);
        let mut p = Pair::new("hmfree no table".to_string(), bin_cfg(elemsize, 8.min(elemsize)));
        p.put_default();
        p.check("before free");
        p.free();
        p.check("after free");
    }
}

#[test]
fn err_hmfree_strdup_frees_keys() {
    // Row 60: for SH_STRDUP every key of elements 1..length is freed.  Rebuild
    // and free the same table many times: a missed or double free trips glibc.
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0xE015 + s);
        for &n in &[0usize, 1, 2, 7, 30] {
            let mut p = Pair::new(format!("hmfree strdup n={n}"), str_cfg(16));
            p.shmode(STBDS_SH_STRDUP);
            for i in 0..n {
                let kb = format!("free{i:04}").into_bytes();
                let k = p.intern_cstr(&kb) as *mut c_void;
                assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
            }
            p.check(&format!("hmfree strdup n={n} before"));
            p.free();
        }
    }
}

#[test]
fn err_hmfree_non_strdup() {
    // Row 61: string.mode != SH_STRDUP -> the keys are NOT freed (the caller or
    // the arena owns them); the arena chain is still released by strreset.
    for &sh in &[0i32, 1, 3, 4, 255] {
        for &n in &[0usize, 1, 5, 20] {
            seed_both(DEFAULT_SEED);
            let mut rng = Rng::new(0xE016);
            let key_is_ptr = matches!(sh, 1 | 3);
            let cfg = if key_is_ptr { str_cfg(16) } else { bin_cfg(16, 8) };
            let mut p = Pair::new(format!("hmfree sh={sh} n={n}"), cfg);
            p.shmode(sh);
            for i in 0..n {
                let kb = format!("nonstrdup{i:04}").into_bytes();
                let k = p.intern_cstr(&kb) as *mut c_void;
                assert!(p.put(k, STBDS_HM_STRING, rng.next_u64()) >= 0);
            }
            p.check(&format!("hmfree sh={sh} n={n} before"));
            p.free();
            // the caller's buffers are still intact afterwards
            for i in 0..n {
                let kb = format!("nonstrdup{i:04}").into_bytes();
                assert_eq!(kb.len(), 13);
            }
        }
    }
}

// ===========================================================================
// Rows 62-75 — string arena
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
struct ArenaObs {
    remaining: usize,
    block: u8,
    mode: u8,
    blocks: usize,
    content: Vec<u8>,
    at_head_start: bool,
    at_head_bump: bool,
    at_second_start: bool,
}

unsafe fn arena_obs(a: &StringArena, p: *mut c_char) -> ArenaObs {
    let head = if a.storage.is_null() {
        std::ptr::null_mut()
    } else {
        std::ptr::addr_of_mut!((*a.storage).storage) as *mut c_char
    };
    let second = if a.storage.is_null() || (*a.storage).next.is_null() {
        std::ptr::null_mut()
    } else {
        std::ptr::addr_of_mut!((*(*a.storage).next).storage) as *mut c_char
    };
    ArenaObs {
        remaining: a.remaining,
        block: a.block,
        mode: a.mode,
        blocks: count_blocks(a.storage),
        content: if p.is_null() { Vec::new() } else { cstr_bytes(p) },
        at_head_start: !p.is_null() && p == head,
        at_head_bump: !p.is_null() && !head.is_null() && p == head.wrapping_add(a.remaining),
        at_second_start: !p.is_null() && p == second,
    }
}

struct Arenas([StringArena; 2]);

impl Arenas {
    fn new() -> Self {
        Arenas([StringArena::zeroed(); 2])
    }
    fn alloc(&mut self, body: &[u8], ctx: &str) -> ArenaObs {
        let [c, r] = both();
        let mut buf = body.to_vec();
        buf.push(0);
        let p = buf.as_mut_ptr() as *mut c_char;
        let pc = unsafe { (c.stralloc)(&mut self.0[0], p) };
        let pr = unsafe { (r.stralloc)(&mut self.0[1], p) };
        let oc = unsafe { arena_obs(&self.0[0], pc) };
        let or_ = unsafe { arena_obs(&self.0[1], pr) };
        assert_eq!(oc, or_, "stralloc {ctx}");
        assert_eq!(oc.content, body, "content {ctx}");
        oc
    }
}

impl Drop for Arenas {
    fn drop(&mut self) {
        let [c, r] = both();
        unsafe { (c.strreset)(&mut self.0[0]) };
        unsafe { (r.strreset)(&mut self.0[1]) };
    }
}

#[test]
fn err_stralloc_fits() {
    // Row 62: len <= remaining -> no allocation, pure bump.
    let mut a = Arenas::new();
    a.alloc(b"first", "seed");
    let before = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) };
    a.alloc(b"second", "fits");
    let after = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) };
    assert_eq!(after.blocks, before.blocks, "no new block");
    assert_eq!(after.remaining, before.remaining - 7);
    assert_eq!(after.block, before.block, "block counter unchanged");
}

#[test]
fn err_stralloc_new_block() {
    // Row 63: len > remaining but <= blocksize -> a fresh 512<<(block>>1) block
    // becomes the head and `remaining` is set to the whole block.
    let mut a = Arenas::new();
    a.alloc(b"x", "first block");
    let o = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) };
    assert_eq!(o.blocks, 1);
    assert_eq!(o.remaining, 512 - 2);
    assert_eq!(o.block, 1, "block was incremented");
    // exhaust it
    for i in 0..300 {
        a.alloc(&vec![b'y'; 3], &format!("fill {i}"));
    }
    let o2 = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) };
    assert!(o2.blocks > 1, "more blocks must have been chained");
}

#[test]
fn err_stralloc_oversize_empty_arena() {
    // Row 64: len > blocksize && storage == NULL -> `remaining` forced to 0.
    for &len in &[512usize, 513, 1000, 100_000] {
        let mut a = Arenas::new();
        let o = a.alloc(&vec![b'z'; len], &format!("oversize-empty len={len}"));
        assert!(len + 1 > 512, "len={len} must exceed the 512 byte blocksize");
        assert_eq!(o.remaining, 0, "remaining forced to 0 (len={len})");
        assert!(o.at_head_start, "the returned pointer is the block start");
        assert_eq!(o.blocks, 1);
        assert_eq!(o.block, 1, "the block counter is still incremented");
    }
}

#[test]
fn err_stralloc_oversize_nonempty_arena() {
    // Row 65: len > blocksize && storage != NULL -> spliced as storage->next,
    // `remaining` preserved.
    let mut a = Arenas::new();
    let before = a.alloc(b"seed", "seed");
    let after = a.alloc(&vec![b'q'; 5000], "oversize-nonempty");
    assert_eq!(
        after.remaining, before.remaining,
        "remaining must be preserved on the oversized non-empty path"
    );
    assert_eq!(after.blocks, before.blocks + 1);
    assert!(after.at_second_start, "the new block is storage->next");
    // and the arena keeps bump-allocating from the *original* head afterwards
    let third = a.alloc(b"third", "after oversize-nonempty");
    assert_eq!(third.blocks, after.blocks, "no new block was needed");
    assert!(third.at_head_bump, "still bump-allocating in the head block");
}

#[test]
fn err_stralloc_block_saturates() {
    // Row 66: `a->block` stops growing once 512<<(block>>1) reaches 1<<20.
    let mut a = Arenas::new();
    a.0[0].block = 22;
    a.0[1].block = 22;
    a.alloc(b"m", "block=22");
    let o = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) };
    assert_eq!(o.block, 22, "512<<11 == 1<<20 is not < 1<<20 -> no increment");
    assert_eq!(o.remaining, (1 << 20) - 2);

    let mut b = Arenas::new();
    b.0[0].block = 21;
    b.0[1].block = 21;
    b.alloc(b"m", "block=21");
    let o2 = unsafe { arena_obs(&b.0[0], std::ptr::null_mut()) };
    assert_eq!(o2.block, 22, "512<<10 < 1<<20 -> incremented");
    assert_eq!(o2.remaining, (1 << 19) - 2);
}

#[test]
fn err_stralloc_block_shift_wrap() {
    // Row 67: `512 << (block>>1)` with a shift count >= 64.  x86-64 masks the
    // count to 6 bits, so e.g. block=255 gives 512<<63 == 0 and the dedicated
    // block path is taken; `++a->block` then wraps 255 -> 0.
    for &blk in &[110u8, 118, 120, 127, 128, 130, 140, 148, 255] {
        let mut a = Arenas::new();
        a.0[0].block = blk;
        a.0[1].block = blk;
        a.alloc(b"wrapped", &format!("block={blk}"));
        a.alloc(b"again", &format!("block={blk} second"));
    }
}

#[test]
fn err_stralloc_block_realloc_fail_segv() {
    // Row 67b: block == 108 gives blocksize == 2^63, so `realloc` fails and the
    // C writes `sb->next` through NULL.
    assert_same_crash("child_stralloc_block_realloc_fail");
}

#[test]
#[ignore]
fn child_stralloc_block_realloc_fail() {
    let l = child_lib();
    let mut a = StringArena::zeroed();
    a.block = 108; // 512 << (108>>1 == 54) == 2^63
    let s = b"x\0";
    let p = unsafe { (l.stralloc)(&mut a, s.as_ptr() as *mut c_char) };
    println!("unexpectedly survived: {p:p}");
}

#[test]
fn err_assert_913_unreachable() {
    // Row 68: `STBDS_ASSERT(len <= a->remaining)` (C L913).  Both preceding
    // branches guarantee it: the oversized path returns early and the new-block
    // path sets remaining = blocksize >= len.  Hammer the exact boundary
    // (len == remaining, len == remaining+1) for many block sizes.
    for &blk in &[0u8, 1, 2, 3, 4, 5] {
        let mut a = Arenas::new();
        a.0[0].block = blk;
        a.0[1].block = blk;
        a.alloc(b"prime", &format!("prime blk={blk}"));
        for _ in 0..50 {
            let rem = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) }.remaining;
            // len == remaining exactly
            if rem >= 1 {
                a.alloc(&vec![b'e'; rem - 1], &format!("exact blk={blk} rem={rem}"));
                let after = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) }.remaining;
                assert_eq!(after, 0, "exact fit must consume everything");
            }
            // len == remaining + 1 -> a new block
            a.alloc(b"o", &format!("overflow blk={blk}"));
        }
    }
}

#[test]
fn err_stralloc_null_storage_segv() {
    // Row 69: an inconsistent arena (storage == NULL but remaining >= len)
    // skips the whole `if` and dereferences a->storage.
    assert_same_crash("child_stralloc_null_storage");
}

#[test]
#[ignore]
fn child_stralloc_null_storage() {
    let l = child_lib();
    let mut a = StringArena::zeroed();
    a.remaining = 1000;
    let s = b"hello\0";
    let p = unsafe { (l.stralloc)(&mut a, s.as_ptr() as *mut c_char) };
    println!("unexpectedly survived: {p:p}");
}

#[test]
fn err_stralloc_null_str_segv() {
    // Row 70
    assert_same_crash("child_stralloc_null_str");
}

#[test]
#[ignore]
fn child_stralloc_null_str() {
    let l = child_lib();
    let mut a = StringArena::zeroed();
    let p = unsafe { (l.stralloc)(&mut a, std::ptr::null_mut()) };
    println!("unexpectedly survived: {p:p}");
}

#[test]
fn err_stralloc_null_arena_segv() {
    // Row 71
    assert_same_crash("child_stralloc_null_arena");
}

#[test]
#[ignore]
fn child_stralloc_null_arena() {
    let l = child_lib();
    let s = b"x\0";
    let p = unsafe { (l.stralloc)(std::ptr::null_mut(), s.as_ptr() as *mut c_char) };
    println!("unexpectedly survived: {p:p}");
}

#[test]
fn err_stralloc_empty_string() {
    // Row 72: "" still consumes the NUL byte.
    let mut a = Arenas::new();
    a.alloc(b"", "empty");
    let o = unsafe { arena_obs(&a.0[0], std::ptr::null_mut()) };
    assert_eq!(o.remaining, 511, "one byte consumed for the NUL");
    for i in 0..600 {
        a.alloc(b"", &format!("empty {i}"));
    }
}

#[test]
fn err_strreset_empty() {
    // Row 73
    let [c, r] = both();
    let mut ac = StringArena::zeroed();
    let mut ar = StringArena::zeroed();
    for _ in 0..3 {
        unsafe { (c.strreset)(&mut ac) };
        unsafe { (r.strreset)(&mut ar) };
        let oc = unsafe { arena_obs(&ac, std::ptr::null_mut()) };
        let or_ = unsafe { arena_obs(&ar, std::ptr::null_mut()) };
        assert_eq!(oc, or_);
        assert_eq!(oc.blocks, 0);
        assert_eq!(oc.remaining, 0);
    }
}

#[test]
fn err_strreset_null_segv() {
    // Row 74
    assert_same_crash("child_strreset_null");
}

#[test]
#[ignore]
fn child_strreset_null() {
    let l = child_lib();
    unsafe { (l.strreset)(std::ptr::null_mut()) };
    println!("unexpectedly survived");
}

#[test]
fn err_strreset_multi_block() {
    // Row 75: the whole chain is freed and all 24 bytes are zeroed (including
    // `block` and `mode`).
    let [c, r] = both();
    for &n in &[1usize, 2, 5, 20] {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        ac.mode = 3;
        ar.mode = 3;
        for i in 0..n {
            let mut s = vec![b'a'; if i % 3 == 2 { 2000 } else { 600 }];
            s.push(0);
            unsafe { (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char) };
            unsafe { (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char) };
        }
        let oc = unsafe { arena_obs(&ac, std::ptr::null_mut()) };
        let or_ = unsafe { arena_obs(&ar, std::ptr::null_mut()) };
        assert_eq!(oc, or_, "arena state before reset (n={n})");
        unsafe { (c.strreset)(&mut ac) };
        unsafe { (r.strreset)(&mut ar) };
        for a in [&ac, &ar] {
            assert!(a.storage.is_null());
            assert_eq!(a.remaining, 0);
            assert_eq!(a.block, 0);
            assert_eq!(a.mode, 0, "strreset memsets the *whole* struct");
        }
    }
}

// ===========================================================================
// Rows 76-79 — stbds_make_hash_index (reached through the public API)
// ===========================================================================

#[test]
fn err_make_index_no_shrink_at_8() {
    // Row 76: an 8-slot index has used_count_shrink_threshold == 0, so it can
    // never shrink below STBDS_BUCKET_LENGTH.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE017);
    let mut p = Pair::new("no shrink at 8".to_string(), bin_cfg(16, 8));
    let mut ptrs = Vec::new();
    for _ in 0..5 {
        let kb = rng.bytes(8);
        let k = p.intern(&kb) as *mut c_void;
        ptrs.push(k);
        p.put(k, STBDS_HM_BINARY, rng.next_u64());
    }
    let t = p.snapshot(0).table.unwrap();
    assert_eq!(t.slot_count, 8);
    assert_eq!(t.used_count_shrink_threshold, 0);
    for (i, &k) in ptrs.iter().enumerate() {
        assert_eq!(p.del(k, 0, STBDS_HM_BINARY), 1);
        assert_eq!(
            p.snapshot(0).table.unwrap().slot_count,
            8,
            "must stay at 8 slots after del {i}"
        );
        p.check(&format!("no-shrink del {i}"));
    }
    p.free();
}

#[test]
fn err_assert_401_slot_count_2() {
    // Row 77: `used_count_threshold + tombstone_count_threshold < slot_count`
    // (C L401) fails for slot_count == 2.  Reachable by corrupting
    // table->slot_count to 1 and forcing a grow.
    assert_same_crash("child_assert_401");
}

#[test]
#[ignore]
fn child_assert_401() {
    let l = child_lib();
    unsafe {
        (l.rand_seed)(DEFAULT_SEED);
        let elemsize = 16usize;
        let k1 = b"first00\0";
        let k2 = b"second0\0";
        let mut t = (l.hmput_key)(
            std::ptr::null_mut(),
            elemsize,
            k1.as_ptr() as *mut c_void,
            8,
            STBDS_HM_BINARY,
        );
        let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
        let ti = (*header(raw)).hash_table as *mut HashIndex;
        (*ti).slot_count = 1;
        (*ti).used_count_threshold = 0;
        // used_count(1) >= threshold(0) -> make_hash_index(1*2 == 2, table)
        t = (l.hmput_key)(t, elemsize, k2.as_ptr() as *mut c_void, 8, STBDS_HM_BINARY);
        println!("unexpectedly survived: {t:p}");
    }
}

#[test]
fn err_make_index_inherits_seed() {
    // Row 78: a rehash (`ot != NULL`) inherits `seed` and `string` and does NOT
    // advance the global seed.
    seed_both(DEFAULT_SEED);
    let mut rng = Rng::new(0xE018);
    let mut p = Pair::new("inherit seed".to_string(), str_cfg(16));
    p.shmode(STBDS_SH_ARENA);
    let seed0 = p.snapshot(0).table.unwrap().seed;
    let mut slots = p.snapshot(0).table.unwrap().slot_count;
    let mut grew = 0;
    for i in 0..60 {
        let kb = format!("inherit{i:04}").into_bytes();
        let k = p.intern_cstr(&kb) as *mut c_void;
        p.put(k, STBDS_HM_STRING, rng.next_u64());
        let t = p.snapshot(0).table.unwrap();
        assert_eq!(t.seed, seed0, "the per-table seed must survive rehashes");
        assert_eq!(t.string_mode, 3, "string.mode must survive rehashes");
        if t.slot_count != slots {
            grew += 1;
            slots = t.slot_count;
        }
        p.check(&format!("inherit {i}"));
    }
    assert!(grew >= 2, "expected several rehashes, saw {grew}");
    p.free();
}

#[test]
fn err_make_index_advances_global_seed() {
    // Row 79: `ot == NULL` snapshots the global seed and then advances it by
    // seed = seed*A + B.
    for &start in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
        seed_both(start);
        let mut seeds_c = Vec::new();
        let mut seeds_r = Vec::new();
        for round in 0..8 {
            let mut p = Pair::new(format!("advance start={start:#x}"), bin_cfg(16, 8));
            let k = p.intern(b"advance\0") as *mut c_void;
            p.put(k, STBDS_HM_BINARY, 1);
            p.check(&format!("advance start={start:#x} round={round}"));
            seeds_c.push(p.snapshot(0).table.unwrap().seed);
            seeds_r.push(p.snapshot(1).table.unwrap().seed);
            p.free();
        }
        assert_eq!(seeds_c, seeds_r, "seed advance sequence for start={start:#x}");
        assert_eq!(seeds_c[0], start, "the first table gets the current seed");
        assert!(
            seeds_c.windows(2).all(|w| w[0] != w[1]),
            "the seed must actually advance: {seeds_c:?}"
        );
    }
}

// ===========================================================================
// Rows 80-83 — stbds_hm_find_slot
// ===========================================================================

#[test]
fn err_find_slot_wraparound_miss() {
    // Rows 80-81: both `return -1` sites (forward scan and wrap-around scan).
    // Many misses over tables of every size force both.
    for s in 0..8u64 {
        seed_both(DEFAULT_SEED ^ s as usize);
        let mut rng = Rng::new(0xE019 + s);
        let mut p = Pair::new(format!("find_slot miss s={s}"), bin_cfg(16, 8));
        for n in 0..120 {
            let kb = rng.bytes(8);
            let k = p.intern(&kb) as *mut c_void;
            p.put(k, STBDS_HM_BINARY, rng.next_u64());
            // probe a handful of absent keys at every table size
            for _ in 0..3 {
                let mut mb = rng.bytes(8);
                mb[0] |= 0x80;
                mb[3] ^= 0x5A;
                let mk = p.intern(&mb) as *mut c_void;
                assert_eq!(p.get(mk, STBDS_HM_BINARY), -1);
                assert_eq!(p.get_ts(mk, STBDS_HM_BINARY), -1);
                assert_eq!(p.del(mk, 0, STBDS_HM_BINARY), 0);
            }
            if n % 13 == 0 {
                p.check(&format!("find_slot miss n={n}"));
            }
        }
        p.check("find_slot misses done");
        p.free();
    }
}

// ===========================================================================
// Rows 84-87 — strkey / str_dups
// ===========================================================================

#[test]
fn err_strkey_extremes() {
    // Row 84: the longest possible output is "test_-2147483648" (17 bytes with
    // the NUL) — the 256 byte static buffer can never overflow.
    let [c, r] = both();
    for &n in &[0i32, 1, -1, i32::MAX, i32::MIN, i32::MIN + 1, -2147483647] {
        let a = unsafe { cstr_bytes((c.strkey)(n)) };
        let b = unsafe { cstr_bytes((r.strkey)(n)) };
        assert_eq!(a, b, "strkey({n})");
        assert_eq!(a, format!("test_{n}").into_bytes());
        assert!(a.len() + 1 <= 256, "must fit the static buffer");
    }
    assert_eq!(
        unsafe { cstr_bytes((c.strkey)(i32::MIN)) }.len() + 1,
        17,
        "longest output is 17 bytes"
    );
}

#[test]
fn err_strkey_shared_buffer() {
    // Row 85
    let [c, r] = both();
    let pc = unsafe { (c.strkey)(1) };
    let pr = unsafe { (r.strkey)(1) };
    assert_eq!(unsafe { cstr_bytes(pc) }, b"test_1");
    assert_eq!(unsafe { cstr_bytes(pr) }, b"test_1");
    let pc2 = unsafe { (c.strkey)(22222) };
    let pr2 = unsafe { (r.strkey)(22222) };
    assert_eq!(pc, pc2, "C must reuse the same static buffer");
    assert_eq!(pr, pr2, "Rust must reuse the same static buffer");
    assert_eq!(unsafe { cstr_bytes(pc) }, b"test_22222");
    assert_eq!(unsafe { cstr_bytes(pr) }, b"test_22222");
}

#[test]
fn err_str_dups_non_positive() {
    // Row 86: num <= 0 skips the arena loop but still runs the strdup block.
    let [c, r] = both();
    for &num in &[0i32, -1, -2, -12345, i32::MIN] {
        seed_both(DEFAULT_SEED);
        let oc = capture_stdout(&format!("errc{num}"), || unsafe { (c.str_dups)(num) });
        seed_both(DEFAULT_SEED);
        let or_ = capture_stdout(&format!("errr{num}"), || unsafe { (r.str_dups)(num) });
        assert_eq!(oc, or_, "str_dups({num})");
        assert_eq!(oc, format!("a {num}\n").into_bytes());
    }
}

#[test]
fn err_str_dups_asserts_hold() {
    // Row 87: the three asserts at C L960-962 always hold, so `str_dups` never
    // aborts for any `num`.  (If either library aborted, this test process would
    // die and the test would fail.)
    let [c, r] = both();
    let mut rng = Rng::new(0xE01A);
    let mut nums: Vec<i32> = vec![0, 1, 2, 3, 7, 29, 30, 31, 32, 63, 64, 65, 100, 511, 512, 513];
    for _ in 0..16 {
        nums.push((rng.next_u32() % 2000) as i32);
    }
    for n in nums {
        seed_both(DEFAULT_SEED);
        let oc = capture_stdout("assc", || unsafe { (c.str_dups)(n) });
        seed_both(DEFAULT_SEED);
        let or_ = capture_stdout("assr", || unsafe { (r.str_dups)(n) });
        assert_eq!(oc, or_, "str_dups({n})");
        assert_eq!(oc, format!("a {n}\n").into_bytes());
    }
}

// ===========================================================================
// Generic FFI-boundary boundaries (required even though not in ERRORS.md)
// ===========================================================================

#[test]
fn err_generic_null_pointers() {
    // Every entry point that documents a NULL-tolerant argument.
    let [c, r] = both();
    unsafe {
        // hash_bytes(NULL, 0, seed)
        assert_eq!(
            (c.hash_bytes)(std::ptr::null_mut(), 0, 1),
            (r.hash_bytes)(std::ptr::null_mut(), 0, 1)
        );
        // hmfree_func(NULL, _)
        (c.hmfree_func)(std::ptr::null_mut(), 16);
        (r.hmfree_func)(std::ptr::null_mut(), 16);
        // hmdel_key(NULL, ...) -> NULL
        assert!((c.hmdel_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, 0, 0).is_null());
        assert!((r.hmdel_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, 0, 0).is_null());
        // arrgrowf(NULL, _, 0, 0) -> NULL
        assert!((c.arrgrowf)(std::ptr::null_mut(), 16, 0, 0).is_null());
        assert!((r.arrgrowf)(std::ptr::null_mut(), 16, 0, 0).is_null());
        // rand_seed accepts anything
        (c.rand_seed)(0);
        (r.rand_seed)(0);
        (c.rand_seed)(usize::MAX);
        (r.rand_seed)(usize::MAX);
    }
}

#[test]
fn err_generic_zero_and_oversized_lengths() {
    let [c, r] = both();
    let mut buf = vec![0xA5u8; 4096];
    unsafe {
        for &len in &[0usize, 1, 7, 8, 9, 4095, 4096] {
            let p = buf.as_mut_ptr() as *mut c_void;
            assert_eq!(
                (c.hash_bytes)(p, len, DEFAULT_SEED),
                (r.hash_bytes)(p, len, DEFAULT_SEED),
                "hash_bytes len={len}"
            );
        }
        // hmfree_func with a bogus elemsize on an index-less map: the loop over
        // keys is skipped because string.mode != SH_STRDUP.
        for &elemsize in &[0usize, 1, 8, 16] {
            (c.rand_seed)(DEFAULT_SEED);
            (r.rand_seed)(DEFAULT_SEED);
            let ac = (c.hmput_default)(std::ptr::null_mut(), elemsize.max(1));
            let ar = (r.hmput_default)(std::ptr::null_mut(), elemsize.max(1));
            (c.hmfree_func)((ac as *mut u8).sub(elemsize.max(1)) as *mut c_void, elemsize);
            (r.hmfree_func)((ar as *mut u8).sub(elemsize.max(1)) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn err_generic_one_past_valid_enum() {
    // Out-of-range enum values across the FFI boundary, one step past both ends
    // of every documented range.
    //   mode:            valid {0,1}          -> test -1 and 2
    //   shmode_func:     valid {0,1,2,3}      -> test -1 and 4
    let [c, r] = both();
    for &mode in &[-1i32, 0, 1, 2] {
        seed_both(DEFAULT_SEED);
        let mut tc: isize = 9;
        let mut tr: isize = 9;
        let ac = unsafe {
            (c.hmget_key_ts)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, &mut tc, mode)
        };
        let ar = unsafe {
            (r.hmget_key_ts)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 8, &mut tr, mode)
        };
        assert_eq!(tc, tr, "mode={mode}");
        unsafe {
            (c.hmfree_func)((ac as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((ar as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
    for &sh in &[-1i32, 0, 1, 2, 3, 4] {
        seed_both(DEFAULT_SEED);
        let key_is_ptr = matches!(sh, 1 | 2 | 3);
        let cfg = if key_is_ptr { str_cfg(16) } else { bin_cfg(16, 8) };
        let mut p = Pair::new(format!("enum sh={sh}"), cfg);
        p.shmode(sh);
        assert_eq!(p.string_mode(0), p.string_mode(1), "shmode_func({sh})");
        p.check(&format!("enum sh={sh}"));
        p.free();
    }
}

// ===========================================================================
// ABI layout parity (see SYMBOLS.md)
// ===========================================================================

#[test]
fn abi_layout_matches_c() {
    // Derive the header layout from live behaviour instead of trusting the
    // Rust-side struct definitions: `arrgrowf` must write `capacity` at
    // offset -16 of the returned pointer, `length` at -32, etc.
    assert_eq!(std::mem::size_of::<ArrayHeader>(), 32);
    assert_eq!(std::mem::size_of::<StringArena>(), 24);
    assert_eq!(std::mem::size_of::<StringBlock>(), 16);
    assert_eq!(std::mem::size_of::<HashBucket>(), 128);
    assert_eq!(std::mem::size_of::<HashIndex>(), 104);

    let [c, r] = both();
    for &elemsize in &[8usize, 16, 24] {
        let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7) };
        let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7) };
        for a in [ac, ar] {
            unsafe {
                let words = (a as *const usize).sub(4);
                assert_eq!(*words.add(0), 0, "length at offset -32");
                assert_eq!(*words.add(1), 7, "capacity at offset -24");
                assert_eq!(*words.add(2), 0, "hash_table at offset -16");
                assert_eq!(*words.add(3), 0, "temp at offset -8");
            }
        }
        unsafe { (c.arrfreef)(ac) };
        unsafe { (r.arrfreef)(ar) };
    }

    // and the hash index layout, through shmode_func
    for mode in 0..4i32 {
        seed_both(DEFAULT_SEED);
        let tc = unsafe { (c.shmode_func)(16, mode) };
        let tr = unsafe { (r.shmode_func)(16, mode) };
        for t in [tc, tr] {
            unsafe {
                let raw = (t as *mut u8).sub(16) as *mut c_void;
                let ti = (*header(raw)).hash_table as *mut HashIndex;
                assert_eq!((*ti).slot_count, 8);
                assert_eq!((*ti).slot_count_log2, 3);
                assert_eq!((*ti).string.mode, mode as u8);
                // `string` starts at byte 72 and `storage` at byte 96
                let bytes = ti as *const u8;
                assert_eq!(
                    *(bytes.add(72 + 16 + 1) as *const u8),
                    mode as u8,
                    "string.mode at offset 89"
                );
                assert_eq!(
                    *(bytes.add(96) as *const usize),
                    (*ti).storage as usize,
                    "storage pointer at offset 96"
                );
            }
        }
        unsafe {
            (c.hmfree_func)((tc as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((tr as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}
