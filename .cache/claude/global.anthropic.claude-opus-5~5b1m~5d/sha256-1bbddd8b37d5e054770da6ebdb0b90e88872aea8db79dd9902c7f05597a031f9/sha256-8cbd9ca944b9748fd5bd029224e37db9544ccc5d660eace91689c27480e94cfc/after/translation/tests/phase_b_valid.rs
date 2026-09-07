//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both implementations are reached only
//! through `dlopen`/`dlsym` on their respective `.so` files.

mod common;

use common::{hex, Arena, Md5, Pair, Rng};

/// Run one configuration through both libraries and compare the full output
/// window byte-for-byte.
fn check(p: &Pair, m: &Md5, prefill: u8) -> [u8; 16] {
    let mut oc = [prefill; 16];
    let mut or = [prefill; 16];
    unsafe {
        (p.c)(m as *const Md5, oc.as_mut_ptr());
        (p.rs)(m as *const Md5, or.as_mut_ptr());
    }
    assert_eq!(
        oc,
        or,
        "divergence for {m:?} (prefill {prefill:#04x}): C={} rust={}",
        hex(&oc),
        hex(&or)
    );
    oc
}

// ---------------------------------------------------------------- row 1
#[test]
fn row01_abi_layout_matches_c() {
    // Values below are what the C compiler reports for c_src/include/lib.h;
    // see tests/phase_b_valid.rs::row01 comment and SYMBOLS.md.
    assert_eq!(std::mem::size_of::<Md5>(), 16, "sizeof(tflac_md5)");
    assert_eq!(std::mem::align_of::<Md5>(), 4, "alignof(tflac_md5)");
    let m = Md5 { a: 0, b: 0, c: 0, d: 0 };
    let base = &m as *const Md5 as usize;
    assert_eq!(&m.a as *const u32 as usize - base, 0, "offsetof(a)");
    assert_eq!(&m.b as *const u32 as usize - base, 4, "offsetof(b)");
    assert_eq!(&m.c as *const u32 as usize - base, 8, "offsetof(c)");
    assert_eq!(&m.d as *const u32 as usize - base, 12, "offsetof(d)");

    // The layout claim is also validated behaviourally: writing a distinct
    // byte pattern per field and checking both libraries agree.
    let p = common::pair();
    let out = check(
        &p,
        &Md5 { a: 0x0403_0201, b: 0x0807_0605, c: 0x0C0B_0A09, d: 0x100F_0E0D },
        0x00,
    );
    assert_eq!(out, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
}

// ---------------------------------------------------------------- row 2 / 3
#[test]
fn row02_all_zero_prefill_ff() {
    let p = common::pair();
    let out = check(&p, &Md5 { a: 0, b: 0, c: 0, d: 0 }, 0xFF);
    assert_eq!(out, [0u8; 16], "all 16 bytes must be written");
}

#[test]
fn row03_all_ones_prefill_zero() {
    let p = common::pair();
    let out = check(&p, &Md5 { a: !0, b: !0, c: !0, d: !0 }, 0x00);
    assert_eq!(out, [0xFFu8; 16]);
}

// ---------------------------------------------------------------- row 4
#[test]
fn row04_field_isolation() {
    let p = common::pair();
    for (idx, field) in ["a", "b", "c", "d"].iter().enumerate() {
        let mut m = Md5::default();
        match idx {
            0 => m.a = 0xDEAD_BEEF,
            1 => m.b = 0xDEAD_BEEF,
            2 => m.c = 0xDEAD_BEEF,
            _ => m.d = 0xDEAD_BEEF,
        }
        for prefill in [0x00u8, 0xFF, 0xAA] {
            let out = check(&p, &m, prefill);
            let mut want = [0u8; 16];
            want[idx * 4..idx * 4 + 4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
            assert_eq!(out, want, "field {field} landed in the wrong slot");
        }
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_byte_isolation() {
    let p = common::pair();
    for idx in 0..4usize {
        for (k, v) in [0x0000_00FFu32, 0x0000_FF00, 0x00FF_0000, 0xFF00_0000]
            .iter()
            .enumerate()
        {
            let mut m = Md5::default();
            match idx {
                0 => m.a = *v,
                1 => m.b = *v,
                2 => m.c = *v,
                _ => m.d = *v,
            }
            let out = check(&p, &m, 0x00);
            let mut want = [0u8; 16];
            want[idx * 4 + k] = 0xFF;
            assert_eq!(out, want, "shift/endianness mismatch (field {idx}, byte {k})");
        }
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn row06_single_bit_sweep() {
    let p = common::pair();
    for bit in 0..128usize {
        let field = bit / 32;
        let within = bit % 32;
        let mut m = Md5::default();
        let v = 1u32 << within;
        match field {
            0 => m.a = v,
            1 => m.b = v,
            2 => m.c = v,
            _ => m.d = v,
        }
        let out = check(&p, &m, 0x00);
        let mut want = [0u8; 16];
        want[field * 4 + within / 8] = 1u8 << (within % 8);
        assert_eq!(out, want, "bit {bit} mapped incorrectly");
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_boundary_value_cross_product() {
    let p = common::pair();
    const V: [u32; 6] = [
        0x0000_0000,
        0x0000_0001,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ];
    for &a in &V {
        for &b in &V {
            for &c in &V {
                for &d in &V {
                    check(&p, &Md5 { a, b, c, d }, 0x5A);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn row08_randomized_exact_buffer() {
    let p = common::pair();
    let mut rng = Rng::new();
    for _ in 0..20_000 {
        let m = rng.next_md5_biased();
        check(&p, &m, 0x00);
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn row09_randomized_random_prefill() {
    let p = common::pair();
    let mut rng = Rng::with_seed(0x13198A2E_03707344);
    for _ in 0..20_000 {
        let m = rng.next_md5_biased();
        let mut oc = [0u8; 16];
        rng.fill(&mut oc);
        let mut or = oc;
        unsafe {
            (p.c)(&m as *const Md5, oc.as_mut_ptr());
            (p.rs)(&m as *const Md5, or.as_mut_ptr());
        }
        assert_eq!(oc, or, "divergence for {m:?}: C={} rust={}", hex(&oc), hex(&or));
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn row10_guarded_arena_no_overrun() {
    let p = common::pair();
    let mut rng = Rng::with_seed(0xA4093822_299F31D0);
    const OUT_OFF: usize = 32;
    for _ in 0..10_000 {
        let m = rng.next_md5_biased();
        let mut ac = Arena::new(0x5A);
        let mut ar = Arena::new(0x5A);
        unsafe {
            (p.c)(&m as *const Md5, ac.ptr(OUT_OFF));
            (p.rs)(&m as *const Md5, ar.ptr(OUT_OFF));
        }
        assert_eq!(
            &ac.0[..],
            &ar.0[..],
            "arena divergence for {m:?}\nC   ={}\nrust={}",
            hex(&ac.0),
            hex(&ar.0)
        );
        // Guards untouched in both (checked on the C arena; equality above
        // propagates the property to Rust).
        assert!(ac.0[..OUT_OFF].iter().all(|&b| b == 0x5A), "wrote before out[0]");
        assert!(
            ac.0[OUT_OFF + 16..].iter().all(|&b| b == 0x5A),
            "wrote past out[15]"
        );
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_misaligned_out() {
    let p = common::pair();
    for off in 0..=8usize {
        let mut rng = Rng::with_seed(0x0000_1000 + off as u64);
        for _ in 0..2_000 {
            let m = rng.next_md5_biased();
            let mut ac = Arena::new(0x5A);
            let mut ar = Arena::new(0x5A);
            let base = 32 + off;
            unsafe {
                (p.c)(&m as *const Md5, ac.ptr(base));
                (p.rs)(&m as *const Md5, ar.ptr(base));
            }
            assert_eq!(&ac.0[..], &ar.0[..], "out misaligned by {off}, {m:?}");
        }
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_misaligned_m() {
    let p = common::pair();
    for off in 0..=8usize {
        let mut rng = Rng::with_seed(0x0000_2000 + off as u64);
        for _ in 0..2_000 {
            let m = rng.next_md5_biased();
            let mut src = Arena::new(0x33);
            unsafe {
                // Place the struct bytes at a (possibly) misaligned offset.
                std::ptr::copy_nonoverlapping(
                    &m as *const Md5 as *const u8,
                    src.ptr(16 + off),
                    16,
                );
            }
            let mp = src.ptr(16 + off) as *const Md5;
            let mut oc = [0u8; 16];
            let mut or = [0u8; 16];
            unsafe {
                (p.c)(mp, oc.as_mut_ptr());
                (p.rs)(mp, or.as_mut_ptr());
            }
            assert_eq!(oc, or, "m misaligned by {off}, {m:?}");
        }
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_both_misaligned() {
    let p = common::pair();
    for moff in 0..=4usize {
        for ooff in 0..=4usize {
            let mut rng = Rng::with_seed(0x3000 + (moff * 16 + ooff) as u64);
            for _ in 0..500 {
                let m = rng.next_md5_biased();
                let mut src = Arena::new(0x33);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        &m as *const Md5 as *const u8,
                        src.ptr(8 + moff),
                        16,
                    );
                }
                let mp = src.ptr(8 + moff) as *const Md5;
                let mut ac = Arena::new(0x5A);
                let mut ar = Arena::new(0x5A);
                unsafe {
                    (p.c)(mp, ac.ptr(64 + ooff));
                    (p.rs)(mp, ar.ptr(64 + ooff));
                }
                assert_eq!(&ac.0[..], &ar.0[..], "m+{moff}/out+{ooff}, {m:?}");
            }
        }
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn row14_in_place_out_equals_m() {
    let p = common::pair();
    let mut rng = Rng::with_seed(0x082E_FA98_EC4E_6C89);
    for _ in 0..5_000 {
        let m = rng.next_md5_biased();
        let mut mc = m;
        let mut mr = m;
        unsafe {
            (p.c)(&mc as *const Md5, &mut mc as *mut Md5 as *mut u8);
            (p.rs)(&mr as *const Md5, &mut mr as *mut Md5 as *mut u8);
        }
        assert_eq!(mc, mr, "in-place divergence for {m:?}");
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn row15_overlapping_every_offset() {
    let p = common::pair();
    const M_OFF: usize = 40;
    for delta in -16i32..=16 {
        let mut rng = Rng::with_seed(0x4000 + (delta + 16) as u64);
        for _ in 0..500 {
            let mut ac = Arena::new(0x00);
            let mut ar = Arena::new(0x00);
            let mut seed_bytes = [0u8; 128];
            rng.fill(&mut seed_bytes);
            ac.0 = seed_bytes;
            ar.0 = seed_bytes;

            let out_off = (M_OFF as i32 + delta) as usize;
            unsafe {
                (p.c)(ac.ptr(M_OFF) as *const Md5, ac.ptr(out_off));
                (p.rs)(ar.ptr(M_OFF) as *const Md5, ar.ptr(out_off));
            }
            assert_eq!(
                &ac.0[..],
                &ar.0[..],
                "overlap delta {delta} diverged\nC   ={}\nrust={}",
                hex(&ac.0),
                hex(&ar.0)
            );
        }
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn row16_repeated_invocation_same_buffers() {
    let p = common::pair();
    let mut rng = Rng::with_seed(0x4528_21E6_38D0_1377);
    let mut oc = [0u8; 16];
    let mut or = [0u8; 16];
    let mut mc = Md5::default();
    let mut mr = Md5::default();
    for i in 0..1_000 {
        let m = rng.next_md5_biased();
        mc = m;
        mr = m;
        unsafe {
            (p.c)(&mc as *const Md5, oc.as_mut_ptr());
            (p.rs)(&mr as *const Md5, or.as_mut_ptr());
        }
        assert_eq!(oc, or, "call #{i} diverged for {m:?}");
    }
    assert_eq!(mc, mr);
}

// ---------------------------------------------------------------- row 17
#[test]
fn row17_idempotent_on_fixed_input() {
    let p = common::pair();
    let m = Md5 { a: 0x1234_5678, b: 0x9ABC_DEF0, c: 0x0F0F_F0F0, d: 0xCAFE_BABE };
    let mut first: Option<[u8; 16]> = None;
    for _ in 0..8 {
        let out = check(&p, &m, 0xAA);
        match first {
            None => first = Some(out),
            Some(f) => assert_eq!(f, out, "not idempotent"),
        }
    }
    assert_eq!(
        first.unwrap(),
        [0x78, 0x56, 0x34, 0x12, 0xF0, 0xDE, 0xBC, 0x9A, 0xF0, 0xF0, 0x0F, 0x0F, 0xBE, 0xBA, 0xFE, 0xCA]
    );
}

// ---------------------------------------------------------------- row 18
#[test]
fn row18_multithreaded_reentrancy() {
    let p = common::pair();
    let c = p.c;
    let rs = p.rs;
    let mut handles = Vec::new();
    for t in 0..4u64 {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::with_seed(0x5000 + t);
            for _ in 0..2_000 {
                let m = rng.next_md5_biased();
                let mut oc = [0u8; 16];
                let mut or = [0xFFu8; 16];
                unsafe {
                    c(&m as *const Md5, oc.as_mut_ptr());
                    rs(&m as *const Md5, or.as_mut_ptr());
                }
                assert_eq!(oc, or, "thread {t} diverged for {m:?}");
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn row19_reads_only_sixteen_bytes() {
    let p = common::pair();
    let mut rng = Rng::with_seed(0xBE54_66CF_34E9_0C6C);
    for _ in 0..5_000 {
        // A 64-byte source whose first 16 bytes are the struct and whose
        // remaining bytes are random garbage: output must not depend on them.
        let m = rng.next_md5_biased();
        let mut src = Arena::new(0);
        rng.fill(&mut src.0);
        unsafe {
            std::ptr::copy_nonoverlapping(&m as *const Md5 as *const u8, src.ptr(0), 16);
        }
        let mut oc = [0u8; 16];
        let mut or = [0u8; 16];
        unsafe {
            (p.c)(src.ptr(0) as *const Md5, oc.as_mut_ptr());
            (p.rs)(src.ptr(0) as *const Md5, or.as_mut_ptr());
        }
        assert_eq!(oc, or, "divergence for {m:?}");
        // And equals the plain-struct result: trailing garbage is irrelevant.
        let plain = check(&p, &m, 0x00);
        assert_eq!(oc, plain, "trailing bytes influenced the result");
    }
}
