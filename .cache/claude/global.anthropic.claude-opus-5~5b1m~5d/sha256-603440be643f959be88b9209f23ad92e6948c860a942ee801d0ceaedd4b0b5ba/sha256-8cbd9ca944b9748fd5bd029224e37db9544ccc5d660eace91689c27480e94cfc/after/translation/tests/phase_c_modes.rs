//! Phase C rows M1–M7 and G1–G9: out-of-range `mode` / enum values crossing the
//! FFI boundary, plus the generic keysize / keyoffset / seed boundaries.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Puts `n` distinct string keys with the given `mode` and checks C == Rust.
/// `string.mode` must be one that stores a real `char *` (1/2/3) so lookups are
/// well-defined.
fn string_mode_ops(row: &str, table_mode: c_int, op_mode: c_int, n: usize, seed: u64) {
    let s = session(0x31415926);
    let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), table_mode);
    let mut rng = Rng::new(seed);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..n {
        let k = format!("{row}/{i}/{}", rng.below(1 << 20)).into_bytes();
        if keys.contains(&k) {
            continue;
        }
        let ctx = format!("{row} put#{i}");
        d.shput(&ctx, &k, &rng.bytes(16), op_mode);
        d.check(&ctx);
        keys.push(k);
    }
    for (i, k) in keys.iter().enumerate() {
        let ctx = format!("{row} get#{i}");
        assert!(d.shgeti(&ctx, k, op_mode) >= 0, "{ctx}: must be found");
        d.check(&ctx);
        let ctx = format!("{row} get_ts#{i}");
        assert!(d.shgeti_ts(&ctx, k, op_mode) >= 0, "{ctx}");
        d.check(&ctx);
    }
    for i in 0..30usize {
        let ctx = format!("{row} miss#{i}");
        let k = format!("~~absent~~{i}").into_bytes();
        assert_eq!(d.shgeti(&ctx, &k, op_mode), -1, "{ctx}: must be -1");
        d.check(&ctx);
    }
    // re-put (existing-key hit path) under the same mode
    for (i, k) in keys.iter().enumerate() {
        let ctx = format!("{row} reput#{i}");
        d.shput(&ctx, k, &rng.bytes(16), op_mode);
        d.check(&ctx);
    }
    d.check(&format!("{row} final"));
    d.free();
}

// =========================================================== M1
#[test]
fn m1_mode_two_is_string() {
    // mode == 2 (STBDS_HM_PTR_TO_STRING) satisfies `mode >= STBDS_HM_STRING`.
    for table_mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        string_mode_ops(&format!("M1 t={table_mode}"), table_mode, 2, 120, 0x100u64.wrapping_add(table_mode as i64 as u64));
    }
    // The hash and the compare must be byte-identical to mode == 1.
    let s = session(0x31415926);
    let mut rng = Rng::new(0x1234);
    for _ in 0..200 {
        let body = rng.ascii_range(0, 40);
        // same key, mode 1 vs mode 2 vs 5 vs 1000 -> identical slot in both libs
        for mode in [1i32, 2, 5, 1000] {
            let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), SH_ARENA);
            let ctx = format!("M1 hash-parity mode={mode}");
            d.shput(&ctx, &body, &rng.bytes(16), mode);
            d.check(&ctx);
            d.free();
        }
    }
}

// =========================================================== M2
#[test]
fn m2_mode_large() {
    for mode in [7i32, 100, 65536, c_int::MAX, c_int::MAX - 1] {
        for table_mode in [SH_DEFAULT, SH_ARENA] {
            string_mode_ops(
                &format!("M2 m={mode} t={table_mode}"),
                table_mode,
                mode,
                60,
                0x200u64.wrapping_add(mode as i64 as u64),
            );
        }
    }
    // A brand-new (implicit) table created with a huge mode must still get
    // `string.mode = STBDS_SH_DEFAULT` (because `mode >= STBDS_HM_STRING`).
    let s = session(0x31415926);
    for mode in [7i32, c_int::MAX] {
        let mut d = Driver::lazy(&s, 16, 8, KeyKind::StringAt(0));
        let mut rng = Rng::new(0x2000u64.wrapping_add(mode as i64 as u64));
        for i in 0..40usize {
            let k = format!("big{mode}-{i}").into_bytes();
            let ctx = format!("M2 implicit m={mode} put#{i}");
            d.shput(&ctx, &k, &rng.bytes(16), mode);
            d.check(&ctx);
        }
        unsafe {
            let ct = header(d.c_raw()).hash_table as *mut HashIndex;
            let rt = header(d.r_raw()).hash_table as *mut HashIndex;
            assert_eq!((*ct).string.mode, SH_DEFAULT as u8, "M2 m={mode}: C string.mode");
            assert_eq!((*rt).string.mode, SH_DEFAULT as u8, "M2 m={mode}: Rust string.mode");
        }
        d.free();
    }
}

// =========================================================== M3
#[test]
fn m3_mode_negative() {
    // mode < STBDS_HM_STRING => BINARY behaviour (hash_bytes + memcmp).
    let s = session(0x31415926);
    for mode in [-1i32, -2, -1000, c_int::MIN, c_int::MIN + 1] {
        for (elemsize, keysize) in [(16usize, 4usize), (16, 8), (24, 8)] {
            let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
            let mut rng = Rng::new(0x300u64.wrapping_add(mode as i64 as u64));
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..80usize {
                let k = rng.bytes(keysize);
                if keys.contains(&k) {
                    continue;
                }
                let ctx = format!("M3 m={mode} e={elemsize} put#{i}");
                d.hmput(&ctx, &k, &rng.bytes(elemsize), mode);
                d.check(&ctx);
                keys.push(k);
            }
            // A negative mode must produce a BINARY table (string.mode == 0).
            unsafe {
                let ct = header(d.c_raw()).hash_table as *mut HashIndex;
                let rt = header(d.r_raw()).hash_table as *mut HashIndex;
                assert_eq!((*ct).string.mode, 0, "M3 m={mode}: C string.mode must be 0");
                assert_eq!((*rt).string.mode, 0, "M3 m={mode}: Rust string.mode must be 0");
            }
            for (i, k) in keys.iter().enumerate() {
                let ctx = format!("M3 m={mode} get#{i}");
                assert!(d.hmgeti(&ctx, k, mode) >= 0, "{ctx}");
                d.check(&ctx);
            }
            for (i, k) in keys.clone().iter().enumerate() {
                let ctx = format!("M3 m={mode} del#{i}");
                assert_eq!(d.hmdel(&ctx, k, 0, mode), 1, "{ctx}");
                d.check(&ctx);
            }
            d.free();
        }
    }
}

// =========================================================== M4
#[test]
fn m4_hmdel_mode2_binary_relookup() {
    // `hmdel_key` tests `mode == STBDS_HM_STRING` *exactly* (lib.c:836, :842),
    // so mode == 2 finds the slot as a STRING but (a) does NOT free the strdup'd
    // key and (b) would re-look-up the moved element with the BINARY branch.
    //
    // Only the `old_index == final_index` case is exercised: with
    // `old_index != final_index` the C's binary re-lookup hashes the key
    // POINTER BYTES, fails, and trips `STBDS_ASSERT(slot >= 0)` / indexes
    // `storage[-1]`.  That is unrunnable UB in the C, so we delete strictly from
    // the tail, where no re-lookup happens at all.
    let s = session(0x31415926);
    for table_mode in [SH_STRDUP, SH_DEFAULT, SH_ARENA] {
        let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), table_mode);
        let mut rng = Rng::new(0x400u64.wrapping_add(table_mode as i64 as u64));
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..60usize {
            let k = format!("m4-{table_mode}-{i}").into_bytes();
            d.shput(&format!("M4 put#{i}"), &k, &rng.bytes(16), HM_STRING);
            keys.push(k);
        }
        d.check(&format!("M4 t={table_mode} populated"));
        while let Some(k) = keys.pop() {
            let ctx = format!("M4 t={table_mode} del-tail len={}", keys.len());
            // mode = 2: located as a string, but treated as non-STRING for the
            // strdup free and the re-lookup branch.
            assert_eq!(d.shdel(&ctx, &k, 0, 2), 1, "{ctx}: temp must be 1");
            d.check(&ctx);
        }
        d.check(&format!("M4 t={table_mode} emptied"));
        // `hmfree_func` still frees per-element keys when string.mode == STRDUP,
        // and the deleted elements are beyond `length`, so this is safe.
        d.free();
    }

    // mode == 2 deleting an ABSENT key: identical early-out in both.
    let mut d = Driver::shmode(&s, 16, 8, KeyKind::StringAt(0), SH_STRDUP);
    let mut rng = Rng::new(0x4FF);
    for i in 0..30usize {
        d.shput(
            &format!("M4b put#{i}"),
            format!("kk{i}").as_bytes(),
            &rng.bytes(16),
            HM_STRING,
        );
    }
    for mode in [2i32, 7, c_int::MAX, -1, c_int::MIN] {
        let ctx = format!("M4b absent mode={mode}");
        assert_eq!(d.shdel(&ctx, b"~absent~", 0, mode), 0, "{ctx}: temp must be 0");
        d.check(&ctx);
    }
    d.free();
}

// =========================================================== M5 / M7
#[test]
fn m5_shmode_out_of_range() {
    let s = session(0x31415926);
    // (unsigned char) casts: 256 -> 0, 257 -> 1, 258 -> 2, 259 -> 3, -1 -> 255,
    // -256 -> 0.  4..=255 all land on the `default:` memcpy arm.
    for raw_mode in [4i32, 5, 100, 254, 255, 256, 257, 258, 259, -1, -2, -256, -255, c_int::MAX, c_int::MIN] {
        let effective = (raw_mode as u32 & 0xFF) as u8;
        let elemsize = 16usize;
        let keysize = 8usize;
        let mut d = Driver::shmode(&s, elemsize, keysize, KeyKind::Binary, raw_mode);
        unsafe {
            let ct = header(d.c_raw()).hash_table as *mut HashIndex;
            let rt = header(d.r_raw()).hash_table as *mut HashIndex;
            assert_eq!(
                (*ct).string.mode, effective,
                "M5 raw={raw_mode}: C string.mode must be (unsigned char) mode"
            );
            assert_eq!(
                (*rt).string.mode, effective,
                "M5 raw={raw_mode}: Rust string.mode must be (unsigned char) mode"
            );
        }
        let stores_pointer = matches!(effective, 1 | 2 | 3);
        let mut rng = Rng::new(0x500u64.wrapping_add(raw_mode as i64 as u64));
        if stores_pointer {
            // Real `char *` in the key slot: full put/get/del cycle is defined.
            let mut d2 = Driver::shmode(&s, elemsize, keysize, KeyKind::StringAt(0), raw_mode);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..60usize {
                let k = format!("m5-{raw_mode}-{i}").into_bytes();
                let ctx = format!("M5 raw={raw_mode} put#{i}");
                d2.shput(&ctx, &k, &rng.bytes(elemsize), HM_STRING);
                d2.check(&ctx);
                keys.push(k);
            }
            for (i, k) in keys.iter().enumerate() {
                let ctx = format!("M5 raw={raw_mode} get#{i}");
                assert!(d2.shgeti(&ctx, k, HM_STRING) >= 0, "{ctx}");
                d2.check(&ctx);
            }
            d2.free();
        } else {
            // `default:` arm -> raw `memcpy` of `keysize` bytes; DISTINCT keys
            // only, because any lookup would dereference those bytes.
            for i in 0..80usize {
                let k = format!("m5none-{raw_mode}-{i:05}").into_bytes();
                let cs = CStrBuf::new(&k);
                let ctx = format!("M5 raw={raw_mode} rawput#{i}");
                unsafe {
                    let kp = cs.ptr() as *mut c_void;
                    d.cm = (s.c.hmput_key)(d.cm, elemsize, kp, keysize, HM_STRING);
                    d.rm = (s.r.hmput_key)(d.rm, elemsize, kp, keysize, HM_STRING);
                    let ci = header(d.c_raw()).temp;
                    let ri = header(d.r_raw()).temp;
                    assert_eq!(ci, ri, "{ctx}: temp index");
                    // define the uninitialised tail (see c24)
                    let tail = rng.bytes(elemsize - keysize);
                    for m in [d.cm, d.rm] {
                        std::ptr::copy_nonoverlapping(
                            tail.as_ptr(),
                            (m as *mut u8).offset(ci * elemsize as isize).add(keysize),
                            tail.len(),
                        );
                    }
                }
                d.check(&ctx);
            }
        }
        // M7: hmfree_func must only run the per-key free loop for string.mode == 2
        d.free();
    }
}

// =========================================================== M6
#[test]
fn m6_shmode_none_string_put() {
    // `shmode_func(elemsize, STBDS_SH_NONE)` + `mode == STBDS_HM_STRING`:
    // the `switch` falls to `default:` and `memcpy`s `keysize` bytes of the
    // string; `temp_key` is left at whatever it was.
    let s = session(0x31415926);
    for (elemsize, keysize) in [(16usize, 8usize), (16, 4), (16, 1), (24, 8), (8, 8), (16, 16)] {
        let mut d = Driver::shmode(&s, elemsize, keysize, KeyKind::Binary, SH_NONE);
        let mut rng = Rng::new(0x600 + (elemsize * 41 + keysize) as u64);
        for i in 0..80usize {
            let k = format!("m6-{i:06}-{}", rng.below(1 << 16)).into_bytes();
            let cs = CStrBuf::new(&k);
            let ctx = format!("M6 e={elemsize} k={keysize} put#{i}");
            unsafe {
                let kp = cs.ptr() as *mut c_void;
                d.cm = (s.c.hmput_key)(d.cm, elemsize, kp, keysize, HM_STRING);
                d.rm = (s.r.hmput_key)(d.rm, elemsize, kp, keysize, HM_STRING);
                let ci = header(d.c_raw()).temp;
                let ri = header(d.r_raw()).temp;
                assert_eq!(ci, ri, "{ctx}: temp index");
                // The key slot must hold the first `keysize` BYTES of the string.
                let want = &cs.0[..keysize];
                for (name, m) in [("C", d.cm), ("Rust", d.rm)] {
                    let got = std::slice::from_raw_parts(
                        (m as *const u8).offset(ci * elemsize as isize),
                        keysize,
                    );
                    assert_eq!(got, want, "{ctx}: {name} key slot must be the raw bytes");
                }
                if elemsize > keysize {
                    let tail = rng.bytes(elemsize - keysize);
                    for m in [d.cm, d.rm] {
                        std::ptr::copy_nonoverlapping(
                            tail.as_ptr(),
                            (m as *mut u8).offset(ci * elemsize as isize).add(keysize),
                            tail.len(),
                        );
                    }
                }
                // `temp_key` (table->temp_key) must be identical (both untouched).
                let ctk = *(header(d.c_raw()).hash_table as *mut *mut c_char);
                let rtk = *(header(d.r_raw()).hash_table as *mut *mut c_char);
                assert_eq!(
                    ctk.is_null(),
                    rtk.is_null(),
                    "{ctx}: temp_key null-ness must match (default: leaves it alone)"
                );
            }
            d.check(&ctx);
        }
        d.free();
    }
}

// =========================================================== G3
#[test]
fn g3_hash_string_seed_extremes() {
    let s = session(1);
    let mut rng = Rng::new(0x63);
    let mut bodies: Vec<Vec<u8>> = vec![b"".to_vec(), b"a".to_vec(), b"ab".to_vec()];
    for _ in 0..100 {
        bodies.push(rng.ascii_range(0, 64));
    }
    for body in bodies {
        let cs = CStrBuf::new(&body);
        for seed in [
            0usize,
            1,
            2,
            0x31415926,
            usize::MAX,
            usize::MAX - 1,
            1 << 63,
            (1 << 63) - 1,
            0xFFFF_FFFF,
            0x1_0000_0000,
        ] {
            unsafe {
                assert_eq!(
                    (s.c.hash_string)(cs.ptr(), seed),
                    (s.r.hash_string)(cs.ptr(), seed),
                    "G3 seed={seed:#x} len={}",
                    body.len()
                );
            }
        }
    }
}

// =========================================================== G4
#[test]
fn g4_rand_seed_extremes() {
    let s = session(1);
    let elemsize = 16usize;
    for seed in [0usize, 1, 2, usize::MAX, usize::MAX - 1, 1 << 63, 0x31415926] {
        s.seed(seed);
        unsafe {
            let mut cseeds = Vec::new();
            let mut rseeds = Vec::new();
            for _ in 0..24 {
                let cm = (s.c.shmode_func)(elemsize, SH_ARENA);
                let rm = (s.r.shmode_func)(elemsize, SH_ARENA);
                let ct = header(raw_of(cm, elemsize)).hash_table as *mut HashIndex;
                let rt = header(raw_of(rm, elemsize)).hash_table as *mut HashIndex;
                cseeds.push((*ct).seed);
                rseeds.push((*rt).seed);
                (s.c.hmfree_func)(raw_of(cm, elemsize), elemsize);
                (s.r.hmfree_func)(raw_of(rm, elemsize), elemsize);
            }
            assert_eq!(cseeds, rseeds, "G4 seed={seed:#x}: seed sequence diverged");
            assert_eq!(cseeds[0], seed, "G4 seed={seed:#x}: first table seed");
        }
    }
}

// =========================================================== G5
#[test]
fn g5_keysize_zero() {
    // `keysize == 0`: `hash_bytes(key, 0, seed)` is the same for every key and
    // `memcmp(..., 0) == 0` always compares equal, so the FIRST key inserted
    // swallows every later one.  Both libraries must agree exactly.
    let s = session(0x31415926);
    for elemsize in [16usize, 24, 8, 1] {
        let mut d = Driver::lazy(&s, elemsize, 0, KeyKind::Binary);
        let mut rng = Rng::new(0x700 + elemsize as u64);
        for i in 0..50usize {
            let ctx = format!("G5 e={elemsize} put#{i}");
            let k: Vec<u8> = Vec::new();
            d.hmput(&ctx, &k, &rng.bytes(elemsize), HM_BINARY);
            d.check(&ctx);
            let snap = d.snap_c();
            assert_eq!(
                snap.length, 2,
                "{ctx}: keysize 0 must collapse to a single element"
            );
        }
        for i in 0..10usize {
            let ctx = format!("G5 e={elemsize} get#{i}");
            let k: Vec<u8> = Vec::new();
            assert_eq!(d.hmgeti(&ctx, &k, HM_BINARY), 0, "{ctx}");
            d.check(&ctx);
        }
        let k: Vec<u8> = Vec::new();
        assert_eq!(d.hmdel("G5 del", &k, 0, HM_BINARY), 1, "G5 del");
        d.check("G5 del");
        assert_eq!(d.hmdel("G5 del-again", &k, 0, HM_BINARY), 0, "G5 del-again");
        d.check("G5 del-again");
        d.free();
    }
}

// =========================================================== G6
#[test]
fn g6_keysize_variants() {
    // `keysize == elemsize` (the key fills the element) and `keysize < elemsize`.
    // `keysize > elemsize` is NOT tested: `memcpy(elem, key, keysize)` would run
    // past the element (and possibly past the array) in the C -- unrunnable UB.
    let s = session(0x31415926);
    for (elemsize, keysize) in [
        (16usize, 16usize),
        (16, 15),
        (16, 9),
        (16, 8),
        (16, 1),
        (8, 8),
        (8, 7),
        (4, 4),
        (4, 3),
        (1, 1),
        (24, 24),
        (24, 17),
    ] {
        let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
        let mut rng = Rng::new(0x800 + (elemsize * 97 + keysize) as u64);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..120usize {
            let k = rng.bytes(keysize);
            let ctx = format!("G6 e={elemsize} k={keysize} put#{i}");
            d.hmput(&ctx, &k, &rng.bytes(elemsize), HM_BINARY);
            d.check(&ctx);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let ctx = format!("G6 e={elemsize} k={keysize} get#{i}");
            assert!(d.hmgeti(&ctx, k, HM_BINARY) >= 0, "{ctx}");
            d.check(&ctx);
        }
        for (i, k) in keys.clone().iter().enumerate() {
            let ctx = format!("G6 e={elemsize} k={keysize} del#{i}");
            assert_eq!(d.hmdel(&ctx, k, 0, HM_BINARY), 1, "{ctx}");
            d.check(&ctx);
        }
        d.free();
    }
}

// =========================================================== G7
#[test]
fn g7_keyoffset_nonzero() {
    // `hmdel_key` is the only entry point with a `keyoffset`; put/get hard-code 0.
    let s = session(0x31415926);
    for (elemsize, keysize) in [(24usize, 8usize), (16, 4), (32, 8)] {
        for keyoffset in [1usize, 2, 4, 8, 16, elemsize - keysize] {
            if keyoffset + keysize > elemsize {
                continue;
            }
            let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
            let mut rng = Rng::new(0x900 + (elemsize * 13 + keyoffset) as u64);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for i in 0..50usize {
                let k = rng.bytes(keysize);
                if keys.contains(&k) {
                    continue;
                }
                // Mirror the key at `keyoffset` so the offset lookup can succeed.
                let mut v = rng.bytes(elemsize);
                v[keyoffset - keysize.min(keyoffset)..].fill(0);
                d.hmput(&format!("G7 put#{i}"), &k, &v, HM_BINARY);
                let idx = d.snap_c().temp;
                unsafe {
                    for m in [d.cm, d.rm] {
                        std::ptr::copy_nonoverlapping(
                            k.as_ptr(),
                            (m as *mut u8).offset(idx * elemsize as isize).add(keyoffset),
                            keysize,
                        );
                    }
                }
                keys.push(k);
            }
            d.check(&format!("G7 e={elemsize} off={keyoffset} populated"));
            for (i, k) in keys.iter().enumerate() {
                let ctx = format!("G7 e={elemsize} off={keyoffset} del#{i}");
                d.hmdel(&ctx, k, keyoffset, HM_BINARY);
                d.check(&ctx);
            }
            d.free();
        }
    }
}
