//! Phase B rows C58..C62 -- `strkey`, `str_put` (stdout compared byte-for-byte)
//! and the raw `shputs` pipeline `str_put` is built from.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// --- C58 --------------------------------------------------------------------
#[track_caller]
fn strkey_eq(c: &Api, rs: &Api, n: c_int) {
    unsafe {
        let pc = (c.strkey)(n);
        let pr = (rs.strkey)(n);
        let lc = strlen(pc);
        let lr = strlen(pr);
        let sc = std::slice::from_raw_parts(pc as *const u8, lc).to_vec();
        let sr = std::slice::from_raw_parts(pr as *const u8, lr).to_vec();
        assert_same(&format!("strkey({n})"), &sc, &sr);
        // and it really is `test_%d`
        let want = format!("test_{n}").into_bytes();
        assert_eq!(sc, want, "strkey({n}) content");
        assert!(lc < 256, "strkey must stay inside the 256-byte static buffer");
    }
}

#[test]
fn cfg_c58_strkey_values() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(58);
        for n in [
            0i32,
            1,
            2,
            9,
            10,
            11,
            99,
            100,
            999,
            1000,
            12345,
            -1,
            -9,
            -10,
            -99,
            -12345,
            c_int::MAX,
            c_int::MIN,
            c_int::MAX - 1,
            c_int::MIN + 1,
        ] {
            strkey_eq(c, rs, n);
        }
        for _ in 0..500 {
            strkey_eq(c, rs, rng.next_u64() as i32);
        }
        // the returned pointer must be the *same* static buffer every time
        unsafe {
            let a = (c.strkey)(1);
            let b = (c.strkey)(2);
            assert_eq!(a, b, "C strkey must return the same static buffer");
            let a = (rs.strkey)(1);
            let b = (rs.strkey)(2);
            assert_eq!(a, b, "RUST strkey must return the same static buffer");
        }
    });
}

// --- C59 --------------------------------------------------------------------
#[track_caller]
fn str_put_eq(c: &Api, rs: &Api, num: c_int) {
    let oc = capture_stdout(|| unsafe { (c.str_put)(num) });
    let or = capture_stdout(|| unsafe { (rs.str_put)(num) });
    if oc != or {
        panic!(
            "str_put({num}) stdout divergence\n  C   = {:?}\n  RUST= {:?}",
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or)
        );
    }
    // `shlen(strmap) == 1` after the single `shputs`, so exactly one line
    assert_eq!(
        oc,
        format!("a {num}\n").into_bytes(),
        "str_put({num}) must print `a <num>`"
    );
}

#[test]
fn cfg_c59_str_put_stdout() {
    with_libs(0x31415926, |c, rs| {
        // block-fill boundaries: "test_<i>" is 7..12 bytes, a 512-byte block
        // holds ~73 of the 7-byte ones.
        for num in [
            0i32, 1, 2, 3, 5, 8, 71, 72, 73, 74, 75, 100, 127, 128, 145, 146, 147, 200, 512, 1000,
            2000, 5000, 20000,
        ] {
            str_put_eq(c, rs, num);
        }
        for num in [-1i32, -2, -100, -12345, c_int::MIN, c_int::MIN + 1] {
            str_put_eq(c, rs, num);
        }
    });
}

#[test]
fn cfg_c59b_str_put_random() {
    with_libs(0x31415926, |c, rs| {
        let mut rng = Rng::new(59);
        for _ in 0..250 {
            let num = rng.below(3000) as c_int;
            str_put_eq(c, rs, num);
        }
        for _ in 0..100 {
            // full int range, but keep the loop bounded: negatives are free
            let n = rng.next_u64() as i32;
            if n > 20000 {
                continue;
            }
            str_put_eq(c, rs, n);
        }
    });
}

// --- C60 --------------------------------------------------------------------
#[test]
fn cfg_c60_str_put_repeated() {
    // Every `str_put` creates one fresh hash index, which consumes and advances
    // the process-global `stbds_hash_seed`.  Both libraries must stay in step.
    for start in [0usize, 1, 0x31415926, usize::MAX] {
        with_libs(start, |c, rs| {
            for i in 0..60 {
                str_put_eq(c, rs, i);
            }
            // after 60 calls the seeds must still agree: prove it by creating a
            // table in each and comparing the recorded seed
            unsafe {
                let tc = (c.shmode_func)(16, SH_ARENA);
                let tr = (rs.shmode_func)(16, SH_ARENA);
                assert_same(
                    "c60 seed still in step",
                    &snap_map(tc, 16, KeyKind::Binary),
                    &snap_map(tr, 16, KeyKind::Binary),
                );
                (c.hmfree_func)(hash_to_arr(tc, 16), 16);
                (rs.hmfree_func)(hash_to_arr(tr, 16), 16);
            }
        });
    }
}

// --- C62 --------------------------------------------------------------------
/// `struct { char *key; int value; }` -- exactly what `str_put` uses, with the
/// tail padding made *explicit*.
///
/// In the original C the 4 tail padding bytes of `s` are never initialised, and
/// `t[temp] = s` copies them; Rust likewise makes struct-assignment padding
/// undefined.  The library never reads those bytes (`hmput_key` only memcpy's
/// `keysize`, `is_key_equal` only looks at offset 0, and `str_put`'s `printf`
/// only consumes `%s`/`%d`), so they are not observable -- but a byte-for-byte
/// snapshot comparison would see them.  Modelling the padding as a real field
/// set to 0 keeps every compared byte defined.
#[repr(C)]
#[derive(Clone, Copy)]
struct Entry {
    key: *mut c_char,
    value: c_int,
    pad: c_int,
}

const E: usize = std::mem::size_of::<Entry>(); // 16
const K: usize = std::mem::size_of::<*mut c_char>(); // 8

/// Replays the `stbds_shputs(strmap, s)` macro through the raw exports:
///
/// ```text
/// (t) = stbds_hmput_key((t), sizeof *(t), (void*)(s).key, sizeof (s).key, STBDS_HM_STRING),
/// (t)[stbds_temp((t)-1)] = (s),
/// (t)[stbds_temp((t)-1)].key = stbds_temp_key((t)-1)
/// ```
#[track_caller]
unsafe fn shputs(api: &Api, t: *mut c_void, s: Entry) -> *mut c_void {
    let t = (api.hmput_key)(t, E, s.key as *mut c_void, K, HM_STRING);
    let arr = hash_to_arr(t, E);
    let idx = (*header_of(arr)).temp;
    let slot = (t as *mut Entry).offset(idx);
    *slot = s;
    (*slot).key = map_temp_key(t, E);
    t
}

#[test]
fn cfg_c62_shputs_pipeline() {
    let mut rng = Rng::new(62);
    for seed in [0usize, 0x31415926, usize::MAX] {
        with_libs(seed, |c, rs| unsafe {
            // (a) the exact single-entry sequence from `str_put`
            for _ in 0..200 {
                let key = b"a\0".to_vec();
                let num = rng.next_u64() as c_int;
                let s = Entry { key: key.as_ptr() as *mut c_char, value: num, pad: 0 };
                let tc = shputs(c, std::ptr::null_mut(), s);
                let tr = shputs(rs, std::ptr::null_mut(), s);
                assert_same(
                    "c62 single shputs",
                    &snap_map(tc, E, KeyKind::Binary),
                    &snap_map(tr, E, KeyKind::Binary),
                );
                // the three asserts `str_put` makes
                for (name, t) in [("C", tc), ("RUST", tr)] {
                    let e = *(t as *const Entry);
                    assert_eq!(*(e.key as *const u8), b'a', "{name}: *strmap[0].key");
                    assert_eq!(e.key, s.key, "{name}: strmap[0].key == s.key");
                    assert_eq!(e.value, s.value, "{name}: strmap[0].value");
                }
                assert_eq!(map_len(tc, E), 1);
                assert_eq!(map_len(tr, E), 1);
                (c.hmfree_func)(hash_to_arr(tc, E), E);
                (rs.hmfree_func)(hash_to_arr(tr, E), E);
            }

            // (b) many randomized entries through the same pipeline
            for _ in 0..40 {
                let keys = Keys::random(&mut rng, 300, 20);
                let mut tc: *mut c_void = std::ptr::null_mut();
                let mut tr: *mut c_void = std::ptr::null_mut();
                for i in 0..keys.len() {
                    let s = Entry { key: keys.cptr(i), value: (i as c_int) ^ 0x5A5A, pad: 0 };
                    tc = shputs(c, tc, s);
                    tr = shputs(rs, tr, s);
                    assert_same(
                        &format!("c62 shputs #{i}"),
                        &snap_map(tc, E, KeyKind::Binary),
                        &snap_map(tr, E, KeyKind::Binary),
                    );
                }
                // and re-put every key (duplicate/update path of shputs)
                for i in 0..keys.len() {
                    let s = Entry { key: keys.cptr(i), value: -(i as c_int), pad: 0 };
                    tc = shputs(c, tc, s);
                    tr = shputs(rs, tr, s);
                    assert_same(
                        &format!("c62 shputs dup #{i}"),
                        &snap_map(tc, E, KeyKind::Binary),
                        &snap_map(tr, E, KeyKind::Binary),
                    );
                }
                assert_eq!(map_len(tc, E), keys.len() as isize);
                (c.hmfree_func)(hash_to_arr(tc, E), E);
                (rs.hmfree_func)(hash_to_arr(tr, E), E);
            }
        });
    }
}

#[test]
fn cfg_c62b_shputs_then_shdel() {
    // full pipeline: shputs / shgeti / shdel, mirroring the stb_ds macros
    let mut rng = Rng::new(620);
    with_libs(0x31415926, |c, rs| unsafe {
        for _ in 0..30 {
            let keys = Keys::random(&mut rng, 150, 18);
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            for i in 0..keys.len() {
                let s = Entry { key: keys.cptr(i), value: i as c_int, pad: 0 };
                tc = shputs(c, tc, s);
                tr = shputs(rs, tr, s);
            }
            // shgeti
            for i in 0..keys.len() {
                let a = (c.hmget_key)(tc, E, keys.ptr(i), K, HM_STRING);
                let b = (rs.hmget_key)(tr, E, keys.ptr(i), K, HM_STRING);
                tc = a;
                tr = b;
                assert_same(
                    "c62b shgeti temp",
                    &map_temp(tc, E),
                    &map_temp(tr, E),
                );
            }
            // shdel, in a shuffled order (keyoffset == STBDS_OFFSETOF(t,key) == 0)
            let mut order: Vec<usize> = (0..keys.len()).collect();
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for (n, &i) in order.iter().enumerate() {
                let a = (c.hmdel_key)(tc, E, keys.ptr(i), K, 0, HM_STRING);
                let b = (rs.hmdel_key)(tr, E, keys.ptr(i), K, 0, HM_STRING);
                assert_same("c62b shdel nullness", &a.is_null(), &b.is_null());
                tc = a;
                tr = b;
                assert_same("c62b shdel temp", &map_temp(tc, E), &map_temp(tr, E));
                assert_same(
                    &format!("c62b shdel #{n}"),
                    &snap_map(tc, E, KeyKind::Binary),
                    &snap_map(tr, E, KeyKind::Binary),
                );
            }
            assert_eq!(map_len(tc, E), 0);
            (c.hmfree_func)(hash_to_arr(tc, E), E);
            (rs.hmfree_func)(hash_to_arr(tr, E), E);
        }
    });
}
