//! Phase B rows 72–76: `strkey`, `helxo` (stdout captured), cross-library
//! interop, and the full end-to-end pipeline.

mod common;
use common::*;
use std::ffi::{c_void, CStr};

// --- row 72: strkey ------------------------------------------------------

#[test]
fn row72_strkey() {
    let (c, r) = libs();
    let mut ns: Vec<i32> = vec![0, 1, -1, 42, -42, 9, 10, 99, 100, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    let mut rng = Rng::new(0x7201);
    for _ in 0..200 {
        ns.push(rng.next_u32() as i32);
    }
    for n in ns {
        unsafe {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert!(!pc.is_null() && !pr.is_null());
            let sc = CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = CStr::from_ptr(pr).to_bytes().to_vec();
            assert_eq!(sc, sr, "row72: strkey({n})");
            assert_eq!(sc, format!("test_{n}").into_bytes(), "row72: strkey({n}) format");
        }
    }
    // the returned pointer is the same static buffer on every call
    unsafe {
        let p1 = (c.strkey)(1);
        let p2 = (c.strkey)(2);
        assert_eq!(p1, p2, "row72: C must reuse the static buffer");
        let q1 = (r.strkey)(1);
        let q2 = (r.strkey)(2);
        assert_eq!(q1, q2, "row72: Rust must reuse the static buffer");
        // ...so the earlier result is clobbered
        assert_eq!(CStr::from_ptr(p1).to_bytes(), b"test_2");
        assert_eq!(CStr::from_ptr(q1).to_bytes(), b"test_2");
    }
}

// --- row 75: cross-library interop ---------------------------------------

/// A table built by one library must be fully usable by the other: this can only
/// work if the array header, hash index, bucket layout, hash function and probe
/// sequence are all byte-identical.
#[test]
fn row75_cross_library_interop_binary() {
    let (c, r) = libs();
    let _g = serial();
    let es = 16usize;
    let ks = 8usize;
    for &(builder_is_c, reader_is_c) in &[(true, false), (false, true)] {
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x7500 ^ builder_is_c as u64);
        let n = 500;
        let mut keys = Keys::binary(&mut rng, n, ks);
        let put = if builder_is_c { c.hmput_key } else { r.hmput_key };
        let get = if reader_is_c { c.hmget_key } else { r.hmget_key };
        let del = if reader_is_c { c.hmdel_key } else { r.hmdel_key };
        let free = if builder_is_c { c.hmfree_func } else { r.hmfree_func };
        unsafe {
            let mut t: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                t = put(t, es, keys.ptr(i), ks, HM_BINARY);
            }
            let tag = format!("row75 built_by={} read_by={}", if builder_is_c { "C" } else { "RUST" }, if reader_is_c { "C" } else { "RUST" });
            // every key must be found by the *other* library
            for i in 0..n {
                let t2 = get(t, es, keys.ptr(i), ks, HM_BINARY);
                assert_eq!(t2, t, "{tag}: hmget_key must not move the table");
                let temp = (*((t as *mut u8).sub(es) as *mut ArrayHeader).offset(-1)).temp;
                assert_eq!(temp, i as isize, "{tag}: key {i} not found by the other library");
            }
            // and deleting through the other library must keep it consistent
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                order.swap(i, rng.below(i + 1));
            }
            for (step, &i) in order.iter().enumerate() {
                t = del(t, es, keys.ptr(i), ks, 0, HM_BINARY);
                assert!(!t.is_null());
                let temp = (*((t as *mut u8).sub(es) as *mut ArrayHeader).offset(-1)).temp;
                assert_eq!(temp, 1, "{tag}: cross-library delete of key {i} at step {step}");
            }
            assert_eq!(snapshot(t, es).length, 1, "{tag}: all deleted");
            free((t as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

#[test]
fn row75b_cross_library_interop_string_modes() {
    let (c, r) = libs();
    let es = 16usize;
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &(builder_is_c, reader_is_c) in &[(true, false), (false, true)] {
            let _g = serial();
            set_seed(DEFAULT_SEED);
            let mut rng = Rng::new(0x75b0 ^ (sh as u64) << 4 ^ builder_is_c as u64);
            let n = 300;
            let mut keys = Keys::strings(&mut rng, n, 30, ASCII);
            let mk = if builder_is_c { c.shmode_func } else { r.shmode_func };
            let put = if builder_is_c { c.hmput_key } else { r.hmput_key };
            let get = if reader_is_c { c.hmget_key } else { r.hmget_key };
            let del = if reader_is_c { c.hmdel_key } else { r.hmdel_key };
            let free = if builder_is_c { c.hmfree_func } else { r.hmfree_func };
            unsafe {
                let mut t = mk(es, sh);
                for i in 0..keys.len() {
                    t = put(t, es, keys.ptr(i), 8, HM_STRING);
                }
                let tag = format!(
                    "row75b sh={sh} built_by={} read_by={}",
                    if builder_is_c { "C" } else { "RUST" },
                    if reader_is_c { "C" } else { "RUST" }
                );
                for i in 0..keys.len() {
                    get(t, es, keys.ptr(i), 8, HM_STRING);
                    let temp = (*((t as *mut u8).sub(es) as *mut ArrayHeader).offset(-1)).temp;
                    assert_eq!(temp, i as isize, "{tag}: key {i} not found");
                }
                // stored key strings must round-trip
                let strs = key_strings(t, es, snapshot(t, es).length);
                for i in 0..keys.len() {
                    assert_eq!(&strs[i][..], &keys.bufs[i][..keys.bufs[i].len() - 1], "{tag}: key {i}");
                }
                let mut order: Vec<usize> = (0..keys.len()).collect();
                for i in (1..order.len()).rev() {
                    order.swap(i, rng.below(i + 1));
                }
                for &i in &order {
                    t = del(t, es, keys.ptr(i), 8, 0, HM_STRING);
                    let temp = (*((t as *mut u8).sub(es) as *mut ArrayHeader).offset(-1)).temp;
                    assert_eq!(temp, 1, "{tag}: delete key {i}");
                }
                assert_eq!(snapshot(t, es).length, 1, "{tag}");
                free((t as *mut u8).sub(es) as *mut c_void, es);
            }
        }
    }
}

// --- row 76: the full pipeline over every string.mode x mode -------------

#[test]
fn row76_full_pipeline_matrix() {
    // Meaningful (sh, mode) combinations only. Two families are excluded because
    // the C itself walks off the rails in them, identically for both libraries,
    // and there is no comparable observable:
    //   * SH_NONE + mode >= 1: the key BYTES are memcpy'd into the element, but
    //     `stbds_is_key_equal` then does `strcmp(key, *(char**)elem)` — it reads
    //     those bytes as a pointer. The first lookup of an existing key
    //     segfaults. (Insert-only coverage is CONFIGS row 37.)
    //   * mode == 2 combined with a delete that relocates the final element:
    //     `hmdel_key` gates the string re-find on `mode == 1`, so it re-finds
    //     with `mode == 2`, which hashes the element's pointer BYTES as a
    //     string, fails, and trips `STBDS_ASSERT(slot >= 0)` -> abort().
    //     (mode == 2 is covered by CONFIGS rows 32 and 58.)
    for &(sh, mode) in &[
        (SH_NONE, HM_BINARY),
        (SH_DEFAULT, HM_STRING),
        (SH_STRDUP, HM_STRING),
        (SH_ARENA, HM_STRING),
    ] {
        {
            let _g = serial();
            set_seed(DEFAULT_SEED);
            let mut rng = Rng::new(0x7600 ^ (sh as u64) << 8 ^ (mode as u64));
            let es = 24usize;
            let repr = match sh {
                SH_DEFAULT => KeyRepr::SharedPtr,
                SH_STRDUP | SH_ARENA => KeyRepr::OwnedStr,
                _ => KeyRepr::Bytes,
            };
            let n = 300;
            let mut keys = if mode == HM_BINARY {
                Keys::binary(&mut rng, n, 8)
            } else {
                let mut k = Keys::strings(&mut rng, n * 2, 30, ASCII);
                k.bufs.retain(|b| b.len() >= 9);
                k.bufs.truncate(n);
                k
            };
            let mut d = Dual::with_shmode(es, 8, mode, sh, repr);
            let tag = format!("row76 sh={sh} mode={mode}");

            // 1. insert
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                d.put(k, &format!("{tag} put i={i}"));
            }
            // 2. look up everything (both getters)
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                d.get(k, &format!("{tag} get i={i}"));
                d.get_ts(k, &format!("{tag} get_ts i={i}"));
            }
            // 3. delete half, in random order
            let mut order: Vec<usize> = (0..keys.len()).collect();
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i + 1));
            }
            let half = order.len() / 2;
            for (step, &i) in order[..half].iter().enumerate() {
                let k = keys.ptr(i);
                d.del(k, 0, &format!("{tag} del step={step} i={i}"));
            }
            // 4. look everything up again
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                d.get(k, &format!("{tag} get2 i={i}"));
            }
            // 5. re-insert the deleted half (tombstone reuse)
            for &i in &order[..half] {
                let k = keys.ptr(i);
                d.put(k, &format!("{tag} reput i={i}"));
            }
            // 6. drain
            for &i in &order {
                let k = keys.ptr(i);
                d.del(k, 0, &format!("{tag} drain i={i}"));
            }
            assert_eq!(d.snap_c().length, 1, "{tag}: drained");
            // 7. tear down
            d.free();
        }
    }
}
