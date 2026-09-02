//! Phase B — CONFIGS.md rows 35-37: the `strkey` / `hm_geti` driver that lives
//! at the bottom of the C translation unit, and the seed cross-check.

mod common;

use common::*;
use std::ffi::c_char;

unsafe fn cstr(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

// ---------------------------------------------------------------------------
// Row 35 — strkey
// ---------------------------------------------------------------------------

#[test]
fn row35_strkey() {
    let (p, _g) = libs();
    let mut fixed: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        11,
        99,
        100,
        101,
        999,
        1000,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX - 1,
        -100,
        -0,
    ];
    let mut rng = Rng::new(0x3500);
    for _ in 0..4000 {
        fixed.push(rng.i32());
    }
    for n in fixed {
        let (a, b) = unsafe { (cstr((p.c.strkey)(n)), cstr((p.rs.strkey)(n))) };
        assert_eq!(
            a,
            b,
            "strkey({n}): C={:?} RS={:?}",
            String::from_utf8_lossy(&a),
            String::from_utf8_lossy(&b)
        );
        // and it really is `sprintf("test_%d", n)`
        assert_eq!(a, format!("test_{n}").into_bytes(), "strkey({n}) content");
    }

    // the buffer is `static`: consecutive calls reuse it, so the previous
    // result is clobbered. Both builds must behave the same way.
    unsafe {
        let p1 = (p.c.strkey)(7);
        let p2 = (p.c.strkey)(8);
        assert_eq!(p1, p2, "C strkey must return the same static buffer");
        let r1 = (p.rs.strkey)(7);
        let r2 = (p.rs.strkey)(8);
        assert_eq!(r1, r2, "RS strkey must return the same static buffer");
        assert_eq!(cstr(p1), b"test_8".to_vec());
        assert_eq!(cstr(r1), b"test_8".to_vec());
    }
}

// ---------------------------------------------------------------------------
// Row 36 — hm_geti end to end
// ---------------------------------------------------------------------------

#[test]
fn row36_hm_geti_end_to_end() {
    let (p, _g) = libs();
    // `hm_geti` is one big pile of live `STBDS_ASSERT`s over the whole
    // put/get/get_ts/del/free pipeline, so "terminated cleanly" is the
    // observable. Run each library in its own child and require the same
    // termination status.
    for &num in &[0i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 33, 64, 100, 257] {
        for &seed in &[DEFAULT_SEED, 0usize, 1, usize::MAX, 0xdeadbeef] {
            let o = assert_same_outcome(p, &format!("hm_geti({num}) seed={seed:#x}"), move |lib| unsafe {
                (lib.rand_seed)(seed);
                (lib.hm_geti)(num);
            });
            assert_eq!(
                o,
                Outcome::Exited(0),
                "hm_geti({num}) seed={seed:#x} should complete cleanly in both builds"
            );
        }
    }
    // also exercise it in-process (an assert failure would abort the harness)
    unsafe {
        reseed(p, DEFAULT_SEED);
        (p.c.hm_geti)(50);
        (p.rs.hm_geti)(50);
    }
    // negative `num`: every loop is skipped, only the leading asserts run
    for &num in &[-1i32, -100, i32::MIN] {
        let o = assert_same_outcome(p, &format!("hm_geti({num})"), move |lib| unsafe {
            (lib.rand_seed)(DEFAULT_SEED);
            (lib.hm_geti)(num);
        });
        assert_eq!(o, Outcome::Exited(0), "hm_geti({num})");
    }
}

// ---------------------------------------------------------------------------
// Row 37 — the seed's effect on slot placement
// ---------------------------------------------------------------------------

#[test]
fn row37a_hash_bytes_is_seed_invariant_in_both() {
    let (p, _g) = libs();
    // The C's `stbds_siphash_bytes` XORs `seed` into each state word twice
    //   v0 = K0 ^ seed;  v0 ^= 0x0706050403020100 ^ seed;
    // so the seed cancels and `stbds_hash_bytes` is seed-independent. That is a
    // property of the ground truth, so the Rust must share it.
    let mut rng = Rng::new(0x3700);
    for len in 0..40usize {
        let mut buf = rng.bytes(len);
        let base_c = unsafe { (p.c.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, len, 0) };
        let base_r = unsafe { (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, len, 0) };
        assert_eq!(base_c, base_r);
        for &seed in &[1usize, 2, DEFAULT_SEED, usize::MAX, 0x1234_5678_9abc_def0] {
            let hc = unsafe {
                (p.c.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, len, seed)
            };
            let hr = unsafe {
                (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut std::ffi::c_void, len, seed)
            };
            assert_eq!(hc, hr, "hash_bytes len={len} seed={seed:#x}");
            assert_eq!(hc, base_c, "C hash_bytes should ignore the seed");
            assert_eq!(hr, base_r, "RS hash_bytes should ignore the seed");
        }
    }
}

#[test]
fn row37b_hash_string_is_seed_sensitive_in_both() {
    let (p, _g) = libs();
    // `stbds_hash_string` genuinely mixes the seed in, so it must change.
    let mut buf = b"a_representative_key\0".to_vec();
    let ptr = buf.as_mut_ptr() as *mut c_char;
    let mut c_vals = std::collections::BTreeSet::new();
    let mut r_vals = std::collections::BTreeSet::new();
    for &seed in &[0usize, 1, 2, DEFAULT_SEED, usize::MAX, 0x1234_5678_9abc_def0] {
        let hc = unsafe { (p.c.hash_string)(ptr, seed) };
        let hr = unsafe { (p.rs.hash_string)(ptr, seed) };
        assert_eq!(hc, hr, "hash_string seed={seed:#x}");
        c_vals.insert(hc);
        r_vals.insert(hr);
    }
    assert!(c_vals.len() > 1, "hash_string should depend on the seed");
    assert_eq!(c_vals, r_vals);
}

#[test]
fn row37c_seed_drives_string_map_slot_placement() {
    let (p, _g) = libs();
    const ES: usize = 16;
    const KS: usize = 8;
    let keys: Vec<*mut c_char> = (0..40)
        .map(|i| leak_cstr(&format!("seedkey_{i}")))
        .collect();
    let seeds: Vec<usize> = vec![DEFAULT_SEED, 0, 1, 2, 0x5555_5555, usize::MAX, 0xabcd_1234_5678];
    let mut prints: Vec<Vec<(usize, isize)>> = Vec::new();
    for &seed in &seeds {
        reseed(p, seed);
        let mut m = DualMap::shmode(p, ES, KS, KeyKind::Ptr, SH_STRDUP, &format!("seed{seed:#x}"));
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &(n as i64).to_ne_bytes().to_vec(), STBDS_HM_STRING);
        }
        for (n, &k) in keys.iter().enumerate() {
            if n % 3 == 0 {
                m.del_str(k, 0, STBDS_HM_STRING);
            }
        }
        m.assert_same("seeded string stream");
        let (sc, sr) = m.snaps();
        assert_eq!(sc, sr);
        prints.push(sc.idx.as_ref().unwrap().slots.clone());
        m.free();
    }
    let distinct: std::collections::BTreeSet<_> = prints.iter().collect();
    assert!(
        distinct.len() > 1,
        "string-key seeds should change the bucket layout; the comparison \
         would otherwise be vacuous"
    );
}

#[test]
fn row37d_binary_map_layout_is_seed_invariant() {
    let (p, _g) = libs();
    const ES: usize = 8;
    const KS: usize = 4;
    let mut prints: Vec<Vec<(usize, isize)>> = Vec::new();
    for &seed in &[DEFAULT_SEED, 0usize, 1, usize::MAX, 0xdead_beef] {
        reseed(p, seed);
        let mut m = DualMap::empty(p, ES, KS, KeyKind::Bytes, &format!("bseed{seed:#x}"));
        m.put_default(&(-2i32).to_ne_bytes().to_vec());
        for k in 0..40i32 {
            m.put_bytes(&k.to_ne_bytes(), &(k * 11).to_ne_bytes());
        }
        for k in (0..40i32).step_by(3) {
            m.del_bytes(&k.to_ne_bytes(), 0, STBDS_HM_BINARY);
        }
        m.assert_same("seeded binary stream");
        let (sc, sr) = m.snaps();
        assert_eq!(sc, sr);
        assert_eq!(sc.idx.as_ref().unwrap().seed, seed, "table seed is recorded");
        prints.push(sc.idx.as_ref().unwrap().slots.clone());
        m.free();
    }
    // consequence of row37a: identical layouts despite different table seeds
    for w in prints.windows(2) {
        assert_eq!(
            w[0], w[1],
            "binary-key layout must not depend on the seed (siphash cancels it)"
        );
    }
}
