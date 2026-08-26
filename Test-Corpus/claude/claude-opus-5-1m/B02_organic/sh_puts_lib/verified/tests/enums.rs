//! Phase C — ERRORS.md rows 37..39 and 41: out-of-range "enum" values crossing
//! the FFI boundary.  C enums accept any `int`, so these are real inputs.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const SIGABRT: c_int = 6;

// ============================================================ row 37
#[test]
fn mode_out_of_range_put_get() {
    let _g = lock();

    // `mode < STBDS_HM_STRING` => binary hashing + memcmp.  -1 and INT_MIN must
    // behave exactly like 0.
    for mode in [0i32, -1, -2, i32::MIN, -12345] {
        sync_seed(0x3141_5926);
        let mut rng = Rng::new(0x3700 ^ (mode as u32 as u64));
        let mut m = Dual::new(16, false);
        let mut keys: Vec<i64> = Vec::new();
        for i in 0..120usize {
            let k = (rng.next_u64() % 60) as i64;
            let (a, b) = m.put_bin(&le64(k), 8, &le64(i as i64), mode);
            assert_eq!(a, b, "binary-ish mode {mode} put #{i} diverged");
            m.check(&format!("mode={mode} put #{i}"));
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for k in 0..80i64 {
            let (a, b) = m.get(&le64(k), 8, mode, false);
            assert_eq!(a, b, "mode={mode} get diverged for {k}");
            assert_eq!(a >= 0, keys.contains(&k), "mode={mode} presence wrong for {k}");
            let (a, b) = m.get_ts(&le64(k), 8, mode, false);
            assert_eq!(a, b, "mode={mode} get_ts diverged for {k}");
        }
        // hmdel_key: `mode == STBDS_HM_STRING` is false, so the binary
        // memmove-fix-up path runs — identical to mode 0.
        for k in keys.clone() {
            let (a, b) = m.del(&le64(k), 8, 0, mode, false);
            assert_eq!((a, b), (1, 1), "mode={mode} delete of {k}");
            m.check(&format!("mode={mode} del {k}"));
        }
        assert_eq!(m.len(), (0, 0));
        m.free();
    }

    // `mode >= STBDS_HM_STRING` => string hashing + strcmp, and the table is
    // created with string.mode = STBDS_SH_DEFAULT.  1, 2, 3, 7 and INT_MAX all
    // take that path.
    for mode in [1i32, 2, 3, 7, 1000, i32::MAX] {
        sync_seed(0x3141_5926);
        let mut rng = Rng::new(0x3800 ^ (mode as u32 as u64));
        let mut m = Dual::new(16, true);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..120usize {
            let k = rng.cbytes_len(1, 8, b'a', b'e');
            let (a, b) = m.put_str(&k, &le64(i as i64), mode);
            assert_eq!(a, b, "string-ish mode {mode} put #{i} diverged");
            m.check(&format!("mode={mode} str put #{i}"));
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        unsafe {
            assert_eq!(
                (*map_table(m.c, 16)).string.mode,
                SH_DEFAULT as u8,
                "mode={mode} must create an SH_DEFAULT table"
            );
        }
        for k in &keys {
            let (a, b) = m.get(k, 8, mode, true);
            assert_eq!(a, b, "mode={mode} str get diverged");
            assert!(a >= 0);
        }
        for k in [b"zzz".to_vec(), b"".to_vec()] {
            let (a, b) = m.get(&k, 8, mode, true);
            assert_eq!(a, b);
        }
        m.check(&format!("mode={mode} after gets"));
        m.free();
    }
}

// ============================================================ row 38
#[test]
fn hmdel_mode_two() {
    let _g = lock();
    // `stbds_hmdel_key` tests `mode == STBDS_HM_STRING` *exactly*, while
    // `find_slot` tests `mode >= STBDS_HM_STRING`.  With mode == 2 the initial
    // lookup is a string lookup but the post-`memmove` re-find takes the
    // *binary* branch, which hands `&elem.key` (the address of the pointer
    // field) to `find_slot` as if it were the key text.
    //
    // (a) deleting the LAST element takes no memmove, so it is fully
    //     deterministic and must match.
    for mode in [2i32, 3, 7, i32::MAX] {
        sync_seed(0x2222);
        let mut m = Dual::new(16, true);
        for i in 0..6usize {
            let key: Vec<u8> = format!("k{i}").into_bytes();
            m.put_str(&key, &le64(i as i64), mode);
        }
        m.check(&format!("mode={mode} setup"));
        for i in (0..6usize).rev() {
            let key: Vec<u8> = format!("k{i}").into_bytes();
            let (a, b) = m.del(&key, 8, 0, mode, true);
            assert_eq!((a, b), (1, 1), "mode={mode} delete-last of k{i}");
            m.check(&format!("mode={mode} del-last k{i}"));
        }
        assert_eq!(m.len(), (0, 0));
        m.free();
    }

    // (b) deleting a NON-last element makes the re-find hash the raw pointer
    //     bytes; the key pointers are the caller's buffers (identical addresses
    //     for both libraries), so the two must fail identically — with the
    //     `slot >= 0` assertion.
    let (c, r) = pair();
    sync_seed(0x2223);
    let es = 16usize;
    let mut m = Dual::new(es, true);
    let mut first_key = Vec::new();
    for i in 0..6usize {
        let key: Vec<u8> = format!("key-{i}").into_bytes();
        if i == 0 {
            first_key = key.clone();
        }
        // payload begins with a NUL so the "pointer as string" is exactly the
        // pointer bytes
        m.put_str(&key, &le64(0), 2);
    }
    m.check("mode=2 memmove setup");
    let kb = CBuf::cstr(&first_key);
    let (mc, mr, kp) = (m.c, m.r, kb.as_void());
    let (oc, ec) = in_child(|| unsafe {
        (c.hmdel_key)(mc, es, kp, 8, 0, 2);
    });
    let (or_, er) = in_child(|| unsafe {
        (r.hmdel_key)(mr, es, kp, 8, 0, 2);
    });
    assert_eq!(oc, or_, "mode=2 memmove outcome diverged: C={oc:?} RUST={or_:?}");
    let first = |v: &[u8]| {
        String::from_utf8_lossy(v.split(|&b| b == b'\n').next().unwrap_or(&[])).into_owned()
    };
    assert_eq!(first(&ec), first(&er), "mode=2 memmove stderr diverged");
    assert_eq!(oc, Outcome::Signalled(SIGABRT), "expected the `slot >= 0` abort");
    assert!(
        first(&ec).contains("Assertion `slot >= 0' failed."),
        "unexpected message: {}",
        first(&ec)
    );
    m.free();
}

// ============================================================ rows 39 & 41
#[test]
fn shmode_out_of_range() {
    let _g = lock();
    let (c, r) = pair();
    // `stbds_shmode_func` stores `(unsigned char) mode`, and the `switch` in
    // `stbds_hmput_key` has a `default:` arm, so *every* int is accepted.
    let modes: [i32; 16] = [
        0, 1, 2, 3, 4, 5, 6, 255, 256, 257, 258, 259, -1, -256, i32::MIN, i32::MAX,
    ];
    for &mode in &modes {
        let eff = (mode as u32 & 0xff) as u8;
        // string.mode must equal the truncated value in both libraries
        for es in [8usize, 16, 24] {
            sync_seed(0x3939);
            unsafe {
                let tc = (c.shmode_func)(es, mode);
                let tr = (r.shmode_func)(es, mode);
                assert_eq!((*map_table(tc, es)).string.mode, eff, "C mode={mode} es={es}");
                assert_eq!((*map_table(tr, es)).string.mode, eff, "RUST mode={mode} es={es}");
                assert_eq!(
                    snap(tc, es, false),
                    snap(tr, es, false),
                    "fresh shmode({mode}) es={es} diverged"
                );
                (c.hmfree_func)(raw_of(tc, es), es);
                (r.hmfree_func)(raw_of(tr, es), es);
            }
        }

        // now actually insert through the mode's `switch` arm
        let ptr_keys = matches!(eff, 1 | 2 | 3);
        let es = 16usize;
        sync_seed(0x3939);
        let mut m = Dual::new(es, ptr_keys);
        m.shmode(mode);
        // keys are >= 8 bytes so the `default:` `memcpy(key, keysize=8)` arm
        // stays inside the caller's buffer and copies deterministic bytes
        let keys: [&[u8]; 4] = [b"aaaaaaaa", b"bbbbbbbb", b"cccccccc", b"dddddddd"];
        for (i, k) in keys.iter().enumerate() {
            let (a, b) = m.put_str(k, &le64(i as i64), HM_STRING);
            assert_eq!(a, b, "shmode({mode}) put #{i} diverged");
            m.check(&format!("shmode({mode}) put #{i}"));
        }
        unsafe {
            assert_eq!(
                (*map_table(m.c, es)).used_count,
                4,
                "unexpected bucket collision for mode={mode}"
            );
        }
        if ptr_keys {
            // lookups only make sense when the key field really is a pointer
            for k in keys.iter() {
                let (a, b) = m.get(k, 8, HM_STRING, true);
                assert_eq!(a, b, "shmode({mode}) get diverged");
                assert!(a >= 0);
            }
            m.check(&format!("shmode({mode}) after gets"));
        }
        m.free();
    }

    // row 41: elemsize 8 is the smallest element that can hold a `char *` key
    for &mode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        sync_seed(0x4141);
        let mut m = Dual::new(8, true);
        m.shmode(mode);
        for i in 0..30usize {
            let key: Vec<u8> = format!("k{i:04}").into_bytes();
            let (a, b) = m.put_str(&key, &[], HM_STRING);
            assert_eq!(a, b, "es=8 mode={mode} put #{i}");
            m.check(&format!("es=8 mode={mode} put #{i}"));
        }
        for i in 0..30usize {
            let key: Vec<u8> = format!("k{i:04}").into_bytes();
            let (a, b) = m.get(&key, 8, HM_STRING, true);
            assert_eq!(a, b);
            assert!(a >= 0);
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// mixed mode / table-mode combinations (the cross product the C distinguishes)
// ---------------------------------------------------------------------------

#[test]
fn mixed_table_mode_and_call_mode() {
    let _g = lock();
    // table built as *binary* (string.mode == 0) but written with mode = STRING:
    // hashed/compared as a string, stored with the `default:` memcpy arm.
    sync_seed(0x5151);
    let es = 16usize;
    let mut m = Dual::new(es, false);
    for (i, k) in [b"11111111", b"22222222", b"33333333"].iter().enumerate() {
        // first make the table exist in binary mode
        if i == 0 {
            m.put_bin(&vec![0xEEu8; 8], 8, &le64(-1), HM_BINARY);
        }
        let (a, b) = m.put_str(&k[..], &le64(i as i64), HM_STRING);
        assert_eq!(a, b, "mixed put #{i} diverged");
        m.check(&format!("mixed put #{i}"));
    }
    unsafe {
        assert_eq!((*map_table(m.c, es)).string.mode, 0);
    }
    m.free();

    // table built as *string* (SH_DEFAULT) but written with mode = BINARY:
    // hashed/compared with memcmp while the store goes through the SH_DEFAULT
    // arm (which writes the caller's pointer).  Both libraries see the same
    // caller pointer, so the behaviour is deterministic.
    sync_seed(0x5152);
    let mut m = Dual::new(es, false);
    m.shmode(SH_DEFAULT);
    for i in 0..8usize {
        let key = le64(i as i64);
        let (a, b) = m.put_bin(&key, 8, &le64(i as i64), HM_BINARY);
        assert_eq!(a, b, "sh_default+binary put #{i} diverged");
        m.check(&format!("sh_default+binary put #{i}"));
    }
    for i in 0..8i64 {
        let (a, b) = m.get(&le64(i), 8, HM_BINARY, false);
        assert_eq!(a, b, "sh_default+binary get diverged for {i}");
    }
    m.check("sh_default+binary after gets");
    m.free();
}

// ---------------------------------------------------------------------------
// `stbds_shmode_func` with degenerate element sizes
// ---------------------------------------------------------------------------

#[test]
fn shmode_elemsize_matrix() {
    let _g = lock();
    let (c, r) = pair();
    for es in [8usize, 16, 24, 32, 64, 128] {
        for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            sync_seed(0x6161);
            unsafe {
                let tc = (c.shmode_func)(es, mode);
                let tr = (r.shmode_func)(es, mode);
                assert_eq!(
                    snap(tc, es, false),
                    snap(tr, es, false),
                    "shmode_func(es={es}, mode={mode}) diverged"
                );
                let h = *map_header(tc, es);
                assert_eq!(h.length, 1);
                assert!(!h.hash_table.is_null());
                (c.hmfree_func)(raw_of(tc, es), es);
                (r.hmfree_func)(raw_of(tr, es), es);
            }
        }
    }
}

/// keep `c_void` referenced so the import is not flagged
const _: Option<*mut c_void> = None;

/// Diagnostic: proves the `mode == 2` memmove path really does reach the
/// `slot >= 0` assertion (so `hmdel_mode_two` case (b) is not vacuous).
#[test]
fn diag_mode_two_aborts() {
    let _g = lock();
    let (c, r) = pair();
    sync_seed(0x2223);
    let es = 16usize;
    let mut m = Dual::new(es, true);
    for i in 0..6usize {
        let key: Vec<u8> = format!("key-{i}").into_bytes();
        m.put_str(&key, &le64(0), 2);
    }
    let kb = CBuf::cstr(b"key-0");
    let (mc, mr, kp) = (m.c, m.r, kb.as_void());
    let (oc, ec) = in_child(|| unsafe { (c.hmdel_key)(mc, es, kp, 8, 0, 2); });
    let (or_, er) = in_child(|| unsafe { (r.hmdel_key)(mr, es, kp, 8, 0, 2); });
    eprintln!("C  outcome={oc:?} stderr={:?}", String::from_utf8_lossy(&ec));
    eprintln!("R  outcome={or_:?} stderr={:?}", String::from_utf8_lossy(&er));
    assert_eq!(oc, Outcome::Signalled(SIGABRT), "expected SIGABRT from C");
    assert_eq!(or_, Outcome::Signalled(SIGABRT), "expected SIGABRT from RUST");
    m.free();
}
