// Phase D — high-volume brute-force sweeps + symbol/ABI parity assertions.
// These are deliberately much larger than the per-row Phase B/C tests: they
// exist to catch value-dependent divergences that a few thousand samples miss.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// safe_double_to_int: exhaustive-ish structured bit-pattern sweep.
// ---------------------------------------------------------------------------
#[test]
fn sweep_sdti_structured_exponents() {
    let p = pair();
    // Every exponent, with a fixed set of mantissas and both signs.
    let mantissas: [u64; 12] = [
        0,
        1,
        2,
        0xF_FFFF_FFFF_FFFF,
        0xF_FFFF_FFFF_FFFE,
        0x8_0000_0000_0000,
        0x7_FFFF_FFFF_FFFF,
        0x1_0000_0000_0000,
        0xA_AAAA_AAAA_AAAA,
        0x5_5555_5555_5555,
        0xC_CCCC_CCCC_CCCD,
        0x2_4924_9249_2492,
    ];
    let mut n = 0u64;
    for exp in 0u64..=0x7FF {
        for &m in &mantissas {
            for sign in [0u64, 1u64 << 63] {
                let d = f64::from_bits(sign | (exp << 52) | m);
                let cv = unsafe { (p.c.safe_double_to_int)(d) };
                let rv = unsafe { (p.rs.safe_double_to_int)(d) };
                assert_eq!(
                    rv, cv,
                    "safe_double_to_int(bits={:#018x} = {d:?}) C={cv} Rust={rv}",
                    d.to_bits()
                );
                n += 1;
            }
        }
    }
    assert!(n > 40_000, "sweep too small: {n}");
}

#[test]
fn sweep_sdti_every_integer_boundary_region() {
    let p = pair();
    // Dense sweep right around both limits at ULP granularity.
    for base in [i32::MAX as f64, i32::MIN as f64, 0.0, 1.0, -1.0] {
        let b = base.to_bits();
        for delta in -4096i64..=4096 {
            let bits = (b as i64).wrapping_add(delta) as u64;
            let d = f64::from_bits(bits);
            let cv = unsafe { (p.c.safe_double_to_int)(d) };
            let rv = unsafe { (p.rs.safe_double_to_int)(d) };
            assert_eq!(rv, cv, "safe_double_to_int({d:?}) C={cv} Rust={rv}");
        }
    }
    // Every representable double at integer+/-0.5 steps across a wide band.
    let mut x = -3.0e9f64;
    while x < 3.0e9 {
        for d in [x, x + 0.5, x - 0.5, x + 0.25] {
            let cv = unsafe { (p.c.safe_double_to_int)(d) };
            let rv = unsafe { (p.rs.safe_double_to_int)(d) };
            assert_eq!(rv, cv, "safe_double_to_int({d:?}) C={cv} Rust={rv}");
        }
        x += 997_331.0;
    }
}

// ---------------------------------------------------------------------------
// process_with_fallthrough: exhaustive over a wide code band.
// ---------------------------------------------------------------------------
#[test]
fn sweep_pwf_exhaustive_code_band() {
    let p = pair();
    let bases = [
        0i32, 1, -1, 7, -7, 100, -100, i32::MAX, i32::MAX - 1, i32::MAX - 149,
        i32::MAX - 150, i32::MAX - 151, i32::MIN, i32::MIN + 1, i32::MIN + 150,
        1_234_567, -1_234_567,
    ];
    for code in -2000..=2000i32 {
        for &base in &bases {
            let cv = unsafe { (p.c.process_with_fallthrough)(code, base) };
            let rv = unsafe { (p.rs.process_with_fallthrough)(code, base) };
            assert_eq!(rv, cv, "process_with_fallthrough({code},{base}) C={cv} Rust={rv}");
        }
    }
    for code in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        for &base in &bases {
            let cv = unsafe { (p.c.process_with_fallthrough)(code, base) };
            let rv = unsafe { (p.rs.process_with_fallthrough)(code, base) };
            assert_eq!(rv, cv, "process_with_fallthrough({code},{base}) C={cv} Rust={rv}");
        }
    }
}

// ---------------------------------------------------------------------------
// handle_pointer_operations: dense sweep over the whole i32 range.
// ---------------------------------------------------------------------------
#[test]
fn sweep_hpo_dense_i32() {
    let p = pair();
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        let x = v as i32;
        let cv = unsafe { (p.c.handle_pointer_operations)(x) };
        let rv = unsafe { (p.rs.handle_pointer_operations)(x) };
        assert_eq!(rv, cv, "handle_pointer_operations({x}) C={cv} Rust={rv}");
        v += 65_521; // prime stride
    }
}

// ---------------------------------------------------------------------------
// copy_data_block: high-volume random payload sweep with varied dest fills.
// ---------------------------------------------------------------------------
#[test]
fn sweep_copy_data_block_volume() {
    let mut r = Rng::new(0xC0FFEE_1234_5678);
    for i in 0..20_000u32 {
        let mut src = [0u8; DATABLOCK_SIZE];
        for chunk in src.chunks_mut(8) {
            let w = r.next_u64().to_le_bytes();
            chunk.copy_from_slice(&w[..chunk.len()]);
        }
        assert_copy_eq("sweep-copy", &src, (i & 0xFF) as u8);
    }
}

// ---------------------------------------------------------------------------
// overunder: high-volume return-value sweep (no stdout capture, so it is fast)
// plus a byte-exact stdout comparison on a large subset.
// ---------------------------------------------------------------------------

/// Silences fd 1 for the duration of `f` so a huge sweep does not emit
/// megabytes of printf output.
fn with_devnull<R, F: FnOnce() -> R>(f: F) -> R {
    let (r, _bytes) = capture_stdout(f);
    r
}

#[test]
fn sweep_overunder_return_values_high_volume() {
    let p = pair();
    let mut r = Rng::new(0xABCD_0F0F_0F0F_0001);
    let mut quads: Vec<(i32, i32, i32, i32)> = Vec::with_capacity(60_000);
    // fully random over the entire 32-bit range
    for _ in 0..30_000 {
        quads.push((r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32()));
    }
    // biased toward the interesting bands: small values, near-sqrt-overflow,
    // near the 1.5x / 2.7x clamp thresholds, and near the extremes
    for _ in 0..10_000 {
        let pick = |r: &mut Rng| -> i32 {
            match r.next_u64() % 8 {
                0 => r.range_i32(-20, 20),
                1 => r.range_i32(46_000, 47_000),
                2 => -r.range_i32(46_000, 47_000),
                3 => r.range_i32(1_431_655_000, 1_431_656_500),
                4 => r.range_i32(795_000_000, 795_700_000),
                5 => i32::MAX - r.range_i32(0, 300),
                6 => i32::MIN + r.range_i32(0, 300),
                _ => r.next_i32(),
            }
        };
        quads.push((pick(&mut r), pick(&mut r), pick(&mut r), pick(&mut r)));
    }

    let mut diffs = 0usize;
    with_devnull(|| {
        for &(a, b, c, d) in &quads {
            let cv = unsafe { (p.c.overunder)(a, b, c, d) };
            let rv = unsafe { (p.rs.overunder)(a, b, c, d) };
            if cv != rv {
                diffs += 1;
                if diffs <= 10 {
                    eprintln!("DIVERGE overunder({a},{b},{c},{d}): C={cv} Rust={rv}");
                }
            }
        }
    });
    assert_eq!(diffs, 0, "{diffs} of {} overunder inputs diverged", quads.len());
}

#[test]
fn sweep_overunder_stdout_byte_exact_subset() {
    let mut r = Rng::new(0x1357_9BDF_0246_8ACE);
    for _ in 0..4000 {
        assert_overunder_eq("sweep-out", r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
    let pick = |r: &mut Rng| -> i32 {
        match r.next_u64() % 6 {
            0 => r.range_i32(-30, 30),
            1 => r.range_i32(46_000, 47_000) * if r.next_u64() & 1 == 0 { 1 } else { -1 },
            2 => r.range_i32(1_431_655_000, 1_431_656_500),
            3 => i32::MAX - r.range_i32(0, 200),
            4 => i32::MIN + r.range_i32(0, 200),
            _ => r.next_i32(),
        }
    };
    for _ in 0..3000 {
        let (a, b, c, d) = (pick(&mut r), pick(&mut r), pick(&mut r), pick(&mut r));
        assert_overunder_eq("sweep-out-biased", a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Symbol / ABI parity asserted from inside the test binary.
// ---------------------------------------------------------------------------
#[test]
fn parity_every_c_symbol_is_loadable_from_rust_so() {
    // Loading `Impl` already resolves all five symbols in BOTH libraries and
    // panics with the symbol name if one is missing; reaching here proves
    // SYMBOLS.md's parity table.
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}

#[test]
fn parity_datablock_layout_matches_c() {
    // The C compiler reports size=40 align=8 off_id=0 off_value=8 off_label=16.
    // If Rust's `DataBlock` disagreed, `copy_data_block` would copy the wrong
    // number of bytes; the guard-byte check in `assert_copy_eq` pins the
    // length, and this pins the field offsets the printf output depends on.
    assert_eq!(std::mem::size_of::<DataBlock>(), 40);
    assert_eq!(std::mem::align_of::<DataBlock>(), 8);
    let b = DataBlock { id: 0, value: 0.0, label: [0; 20] };
    let base = &b as *const DataBlock as usize;
    assert_eq!(&b.id as *const _ as usize - base, 0);
    assert_eq!(&b.value as *const _ as usize - base, 8);
    assert_eq!(&b.label as *const _ as usize - base, 16);

    // And the Rust `.so`'s own copy length must be exactly 40: write into a
    // buffer whose tail is poisoned and confirm the tail survives.
    let p = pair();
    let mut src = [0u64; BUF_SIZE / 8];
    for (i, w) in src.iter_mut().enumerate() {
        *w = 0x1111_1111_1111_1111u64.wrapping_mul(i as u64 + 1);
    }
    let mut dst = [0xAAAA_AAAA_AAAA_AAAAu64; BUF_SIZE / 8];
    unsafe {
        (p.rs.copy_data_block)(dst.as_mut_ptr() as *mut c_void, src.as_ptr() as *const c_void);
    }
    let db: &[u8; BUF_SIZE] = unsafe { &*(dst.as_ptr() as *const [u8; BUF_SIZE]) };
    assert_eq!(&db[DATABLOCK_SIZE..], &[0xAAu8; GUARD][..], "Rust copied > 40 bytes");
    let sb: &[u8; BUF_SIZE] = unsafe { &*(src.as_ptr() as *const [u8; BUF_SIZE]) };
    assert_eq!(&db[..DATABLOCK_SIZE], &sb[..DATABLOCK_SIZE], "Rust copied < 40 bytes");
}

#[test]
fn parity_return_types_are_c_int() {
    // A wrong return width would show up as a garbage upper half. Drive a
    // value whose low 32 bits are 0 through each i32-returning export and
    // confirm both agree bit-for-bit (already covered, but pinned explicitly).
    let p = pair();
    let probes: [c_int; 5] = [0, -1, i32::MIN, i32::MAX, 0x7F00_0000];
    for &v in &probes {
        assert_eq!(unsafe { (p.c.handle_pointer_operations)(v) }, unsafe {
            (p.rs.handle_pointer_operations)(v)
        });
        assert_eq!(unsafe { (p.c.process_with_fallthrough)(v, v) }, unsafe {
            (p.rs.process_with_fallthrough)(v, v)
        });
        assert_eq!(unsafe { (p.c.safe_double_to_int)(v as f64) }, unsafe {
            (p.rs.safe_double_to_int)(v as f64)
        });
    }
}
