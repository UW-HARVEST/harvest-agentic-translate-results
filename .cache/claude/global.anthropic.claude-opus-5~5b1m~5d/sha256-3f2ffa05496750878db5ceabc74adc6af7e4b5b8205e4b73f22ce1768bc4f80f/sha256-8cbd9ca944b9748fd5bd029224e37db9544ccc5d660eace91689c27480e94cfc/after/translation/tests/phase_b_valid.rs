//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Both libraries are loaded with `libloading` and every call crosses the FFI
//! boundary; no Rust function is ever invoked directly.

mod common;

use common::*;

const ITERS: usize = 400;

// ===========================================================================
// Row 1 / Row 2 — the public entry point and the dispatcher identity.
// ===========================================================================

/// CONFIGS row 1: `get_predict_func` swept over the whole valid domain 0..=11.
#[test]
fn row01_public_valid_domain() {
    let (c, r) = pair_public();
    let cf: libloading::Symbol<GetPredictFunc> = c.sym("get_predict_func");
    let rf: libloading::Symbol<GetPredictFunc> = r.sym("get_predict_func");
    for pfcn in 0..=11 {
        let (a, b) = unsafe { (cf(pfcn), rf(pfcn)) };
        assert_eq!(a, b, "get_predict_func({pfcn}): C={a} Rust={b}");
        assert_eq!(a, 1, "C itself is expected to report 1 for pfcn={pfcn}");
    }
}

/// CONFIGS row 2: the dispatcher `BTAC1C2_GetPredictFunc` returns *exactly* the
/// specialised predictor for each id — compared via the identity tag, over the
/// full `PFCN_ALL` domain, so the mapping (not just a boolean) is verified.
#[test]
fn row02_dispatcher_identity() {
    if !difftest_enabled() {
        return;
    }
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestDispatchTag> = c.sym("__difftest_dispatch_tag");
    let rf: libloading::Symbol<DifftestDispatchTag> = r.sym("__difftest_dispatch_tag");
    for &pfcn in PFCN_ALL {
        let (a, b) = unsafe { (cf(pfcn), rf(pfcn)) };
        assert_eq!(a, b, "dispatch tag for pfcn={pfcn}: C={a} Rust={b}");
        assert_ne!(a, -1, "C dispatcher returned an unrecognised pointer");
    }
    // And the public boolean must agree with the tag on both sides.
    let cp: libloading::Symbol<GetPredictFunc> = c.sym("get_predict_func");
    let rp: libloading::Symbol<GetPredictFunc> = r.sym("get_predict_func");
    for &pfcn in PFCN_ALL {
        let (a, b) = unsafe { (cp(pfcn), rp(pfcn)) };
        assert_eq!(a, b, "get_predict_func({pfcn})");
    }
}

/// CONFIGS row 40 (layout half): `#[repr(C)] btac1c_idxstate` must match the C
/// struct's size, alignment and every field offset.
#[test]
fn row40a_struct_layout() {
    if !difftest_enabled() {
        return;
    }
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestLayout> = c.sym("__difftest_layout");
    let rf: libloading::Symbol<DifftestLayout> = r.sym("__difftest_layout");
    let names = [
        "sizeof", "alignof", "off(idx)", "off(lpred)", "off(rpred)", "off(tag)",
        "off(bcfcn)", "off(bsfcn)", "off(usefx)", "off(firfx)", "sizeof(firfx)",
    ];
    for (what, name) in names.iter().enumerate() {
        let w = what as i32;
        let (a, b) = unsafe { (cf(w), rf(w)) };
        assert_eq!(a, b, "layout {name}: C={a} Rust={b}");
    }
    // Sanity: the Rust mirror used by these tests agrees with the C too.
    assert_eq!(
        unsafe { cf(0) } as usize,
        std::mem::size_of::<IdxState>(),
        "test-side IdxState mirror has the wrong size"
    );
}

// ===========================================================================
// The generic driver shared by rows 3..43.
// ===========================================================================

struct Harness {
    c: Side,
    r: Side,
}

impl Harness {
    fn new() -> Self {
        let (c, r) = pair_internal();
        Harness { c, r }
    }

    /// Call `__difftest_predict` on both sides and assert byte-identical result.
    fn check(
        &self,
        label: &str,
        which: i32,
        psamp: &[i32; 8],
        idx: i32,
        pfcn: i32,
        st: &IdxState,
    ) {
        let cf: libloading::Symbol<DifftestPredict> = self.c.sym("__difftest_predict");
        let rf: libloading::Symbol<DifftestPredict> = self.r.sym("__difftest_predict");

        let mut cs = *psamp;
        let mut rs = *psamp;
        let mut cst = *st;
        let mut rst = *st;

        let a = unsafe { cf(which, cs.as_mut_ptr(), idx, pfcn, &mut cst) };
        let b = unsafe { rf(which, rs.as_mut_ptr(), idx, pfcn, &mut rst) };

        assert_eq!(
            a, b,
            "{label}: which={which} idx={idx} pfcn={pfcn} psamp={psamp:?}\n  C={a} Rust={b}"
        );
        // Neither implementation may mutate its inputs.
        assert_eq!(cs, rs, "{label}: psamp mutated differently");
        assert_eq!(cs, *psamp, "{label}: C mutated psamp (unexpected)");
        assert_eq!(cst, rst, "{label}: ridx mutated differently");
    }

    /// Randomized sweep for one (which, pfcn) configuration over one value shape.
    fn sweep(&self, label: &str, which: i32, pfcn: i32, shape: Shape, seed: u64) {
        let mut rng = Rng::new(seed);
        for _ in 0..ITERS {
            let psamp = shape.gen(&mut rng);
            let idx = rng.next_i32();
            let st = random_state(&mut rng);
            self.check(label, which, &psamp, idx, pfcn, &st);
        }
        // ... and the same shape against every boundary idx.
        for &idx in IDX_BOUNDARIES {
            let psamp = shape.gen(&mut rng);
            let st = random_state(&mut rng);
            self.check(label, which, &psamp, idx, pfcn, &st);
        }
    }
}

fn random_state(rng: &mut Rng) -> IdxState {
    let mut st = IdxState {
        idx: rng.next_u64() as u16,
        lpred: rng.next_u64() as i16,
        rpred: rng.next_u64() as i16,
        tag: rng.next_u64() as u8,
        bcfcn: rng.next_u64() as u8,
        bsfcn: rng.next_u64() as u8,
        usefx: rng.next_u64() as u8,
        firfx: [[0i16; 8]; 4],
    };
    for row in st.firfx.iter_mut() {
        for cell in row.iter_mut() {
            *cell = match rng.below(10) {
                0 => i16::MIN,
                1 => i16::MAX,
                2 => 0,
                _ => rng.next_u64() as i16,
            };
        }
    }
    st
}

// ===========================================================================
// Rows 3..21 — the twelve specialised predictors (lowest-level entry points).
// ===========================================================================

macro_rules! spec_rows {
    ($( $name:ident : which = $which:expr , seed = $seed:expr ; )*) => { $(
        #[test]
        fn $name() {
            if !difftest_enabled() { return; }
            let h = Harness::new();
            for (i, shape) in Shape::ALL.iter().enumerate() {
                h.sweep(
                    concat!(stringify!($name)),
                    $which,
                    0,
                    *shape,
                    $seed + i as u64 * 7919,
                );
            }
            // Row 4 / 6 / …: exhaustive small idx crossed with extreme values.
            let mut rng = Rng::new($seed ^ 0xDEAD_BEEF);
            for idx in -9i32..=9 {
                for _ in 0..40 {
                    let psamp = Shape::Extreme.gen(&mut rng);
                    let st = random_state(&mut rng);
                    h.check("exhaustive-idx", $which, &psamp, idx, 0, &st);
                }
            }
        }
    )* };
}

spec_rows! {
    row03_04_spec0  : which =  0, seed = 0x0001;
    row05_06_spec1  : which =  1, seed = 0x0002;
    row07_spec2     : which =  2, seed = 0x0003;
    row08_spec3     : which =  3, seed = 0x0004;
    row09_spec4     : which =  4, seed = 0x0005;
    row10_spec5     : which =  5, seed = 0x0006;
    row11_spec6     : which =  6, seed = 0x0007;
    row12_13_spec7  : which =  7, seed = 0x0008;
    row14_15_spec8  : which =  8, seed = 0x0009;
    row16_17_spec9  : which =  9, seed = 0x000A;
    row18_19_spec10 : which = 10, seed = 0x000B;
    row20_21_spec11 : which = 11, seed = 0x000C;
}

// ===========================================================================
// Rows 22..37 — every arm of the generic BTAC1C2_PredictSample.
// `which = 99` is outside 0..=11 so the dispatcher hands back the generic fn.
// ===========================================================================

const GENERIC: i32 = 99;

macro_rules! generic_rows {
    ($( $name:ident : pfcn = $pfcn:expr , seed = $seed:expr ; )*) => { $(
        #[test]
        fn $name() {
            if !difftest_enabled() { return; }
            let h = Harness::new();
            for (i, shape) in Shape::ALL.iter().enumerate() {
                h.sweep(
                    concat!(stringify!($name)),
                    GENERIC,
                    $pfcn,
                    *shape,
                    $seed + i as u64 * 6271,
                );
            }
            let mut rng = Rng::new($seed ^ 0xFEED_FACE);
            for idx in -9i32..=9 {
                for _ in 0..40 {
                    let psamp = Shape::Extreme.gen(&mut rng);
                    let st = random_state(&mut rng);
                    h.check("generic-exhaustive-idx", GENERIC, &psamp, idx, $pfcn, &st);
                }
            }
        }
    )* };
}

generic_rows! {
    row22_gen0  : pfcn =  0, seed = 0x1001;
    row23_gen1  : pfcn =  1, seed = 0x1002;
    row24_gen2  : pfcn =  2, seed = 0x1003;
    row25_gen3  : pfcn =  3, seed = 0x1004;
    row26_gen4  : pfcn =  4, seed = 0x1005;
    row27_gen5  : pfcn =  5, seed = 0x1006;
    row28_gen6  : pfcn =  6, seed = 0x1007;
    row29_gen7  : pfcn =  7, seed = 0x1008;
    row30_gen8  : pfcn =  8, seed = 0x1009;
    row31_gen9  : pfcn =  9, seed = 0x100A;
    row32_gen10 : pfcn = 10, seed = 0x100B;
    row33_gen11 : pfcn = 11, seed = 0x100C;
    row34_gen12 : pfcn = 12, seed = 0x100D;
    row35_gen13 : pfcn = 13, seed = 0x100E;
    row36_gen14 : pfcn = 14, seed = 0x100F;
    row37_gen15 : pfcn = 15, seed = 0x1010;
}

// ===========================================================================
// Rows 32/33 — the deliberate C inconsistency: generic arm 10/11 must NOT
// agree with the specialised Pfn10/Pfn11 (>>4 vs >>3, >>3 vs >>1).  If the
// Rust "fixed" either one, this test catches it on both sides at once.
// ===========================================================================

#[test]
fn row32_33_generic_vs_specialised_shift_quirk() {
    if !difftest_enabled() {
        return;
    }
    let (c, r) = pair_internal();
    let cf: libloading::Symbol<DifftestPredict> = c.sym("__difftest_predict");
    let rf: libloading::Symbol<DifftestPredict> = r.sym("__difftest_predict");
    let mut rng = Rng::new(0x5111);
    let mut saw_difference_10 = false;
    let mut saw_difference_11 = false;
    for _ in 0..ITERS {
        let psamp = Shape::Small.gen(&mut rng);
        let idx = rng.next_i32();
        let mut st = random_state(&mut rng);
        for (pfcn, seen) in [(10, &mut saw_difference_10), (11, &mut saw_difference_11)] {
            let mut a = psamp;
            let mut b = psamp;
            let gen_c = unsafe { cf(GENERIC, a.as_mut_ptr(), idx, pfcn, &mut st) };
            let spec_c = unsafe { cf(pfcn, b.as_mut_ptr(), idx, pfcn, &mut st) };
            let gen_r = unsafe { rf(GENERIC, a.as_mut_ptr(), idx, pfcn, &mut st) };
            let spec_r = unsafe { rf(pfcn, b.as_mut_ptr(), idx, pfcn, &mut st) };
            assert_eq!(gen_c, gen_r, "generic pfcn={pfcn} idx={idx} {psamp:?}");
            assert_eq!(spec_c, spec_r, "specialised {pfcn} idx={idx} {psamp:?}");
            // The C's generic and specialised results differ (different shifts);
            // whatever the C does, the Rust must mirror it exactly.
            assert_eq!(
                gen_c == spec_c,
                gen_r == spec_r,
                "generic-vs-specialised agreement diverges for pfcn={pfcn}"
            );
            if gen_c != spec_c {
                *seen = true;
            }
        }
    }
    assert!(
        saw_difference_10 && saw_difference_11,
        "expected the C's >>4/>>3 and >>3/>>1 quirk to be observable \
         (saw10={saw_difference_10} saw11={saw_difference_11})"
    );
}

// ===========================================================================
// Row 38 — FIR arm with all-zero taps.
// ===========================================================================

#[test]
fn row38_fir_zero_taps() {
    if !difftest_enabled() {
        return;
    }
    let h = Harness::new();
    let st = IdxState::default();
    let mut rng = Rng::new(0x2001);
    for pfcn in 12..=15 {
        for shape in Shape::ALL {
            for _ in 0..60 {
                let psamp = shape.gen(&mut rng);
                let idx = rng.next_i32();
                h.check("fir-zero-taps", GENERIC, &psamp, idx, pfcn, &st);
            }
        }
    }
}

// ===========================================================================
// Row 39 — FIR arm with saturated i16 taps and extreme psamp (overflow).
// ===========================================================================

#[test]
fn row39_fir_saturated_taps() {
    if !difftest_enabled() {
        return;
    }
    let h = Harness::new();
    let mut rng = Rng::new(0x2002);
    let patterns: [i16; 4] = [i16::MIN, i16::MAX, -1, 1];
    for pfcn in 12..=15 {
        for &tap in patterns.iter() {
            let mut st = IdxState::default();
            for row in st.firfx.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = tap;
                }
            }
            for shape in [Shape::Extreme, Shape::Medium, Shape::Positive, Shape::Negative] {
                for _ in 0..60 {
                    let psamp = shape.gen(&mut rng);
                    let idx = rng.next_i32();
                    h.check("fir-saturated", GENERIC, &psamp, idx, pfcn, &st);
                }
            }
        }
    }
}

// ===========================================================================
// Row 40 — every firfx cell distinct + all header fields non-zero, so any
// #[repr(C)] offset mistake changes the result.
// ===========================================================================

#[test]
fn row40b_fir_distinct_cells() {
    if !difftest_enabled() {
        return;
    }
    let h = Harness::new();
    let mut st = IdxState {
        idx: 0xBEEF,
        lpred: -12345,
        rpred: 23456,
        tag: 0xA1,
        bcfcn: 0xB2,
        bsfcn: 0xC3,
        usefx: 0xD4,
        firfx: [[0i16; 8]; 4],
    };
    let mut n: i16 = 1;
    for row in st.firfx.iter_mut() {
        for cell in row.iter_mut() {
            *cell = n.wrapping_mul(101).wrapping_sub(37);
            n += 1;
        }
    }
    let mut rng = Rng::new(0x2003);
    for pfcn in 12..=15 {
        for shape in Shape::ALL {
            for _ in 0..80 {
                let psamp = shape.gen(&mut rng);
                let idx = rng.next_i32();
                h.check("fir-distinct-cells", GENERIC, &psamp, idx, pfcn, &st);
            }
        }
        for &idx in IDX_BOUNDARIES {
            let psamp = Shape::Small.gen(&mut rng);
            h.check("fir-distinct-cells-idx", GENERIC, &psamp, idx, pfcn, &st);
        }
    }
}

// ===========================================================================
// Row 41 — idx boundary sweep across every entry point.
// ===========================================================================

#[test]
fn row41_idx_boundary_sweep_all_entry_points() {
    if !difftest_enabled() {
        return;
    }
    let h = Harness::new();
    let mut rng = Rng::new(0x3001);
    for which in 0..=11 {
        for &idx in IDX_BOUNDARIES {
            for shape in Shape::ALL {
                let psamp = shape.gen(&mut rng);
                let st = random_state(&mut rng);
                h.check("idx-sweep-spec", which, &psamp, idx, 0, &st);
            }
        }
    }
    for pfcn in 0..=15 {
        for &idx in IDX_BOUNDARIES {
            for shape in Shape::ALL {
                let psamp = shape.gen(&mut rng);
                let st = random_state(&mut rng);
                h.check("idx-sweep-gen", GENERIC, &psamp, idx, pfcn, &st);
            }
        }
    }
}

// ===========================================================================
// Row 42 — the PfnN ignore their `pfcn` argument; varying it must not matter,
// and must not matter identically on both sides.
// ===========================================================================

#[test]
fn row42_specialised_ignore_pfcn_argument() {
    if !difftest_enabled() {
        return;
    }
    let h = Harness::new();
    let mut rng = Rng::new(0x4001);
    for which in 0..=11 {
        for &pfcn in &[0i32, 5, 11, 12, 15, 16, 99, -1, i32::MIN, i32::MAX] {
            for _ in 0..40 {
                let psamp = Shape::Small.gen(&mut rng);
                let idx = rng.next_i32();
                let st = random_state(&mut rng);
                h.check("pfcn-ignored", which, &psamp, idx, pfcn, &st);
            }
        }
    }
}

// ===========================================================================
// Row 43 — arms 0..=11 of the generic function never read `ridx`; garbage
// firfx must not perturb the result on either side.
// ===========================================================================

#[test]
fn row43_generic_low_arms_ignore_ridx() {
    if !difftest_enabled() {
        return;
    }
    let h = Harness::new();
    let mut rng = Rng::new(0x4002);
    for pfcn in 0..=11 {
        for _ in 0..80 {
            let psamp = Shape::Medium.gen(&mut rng);
            let idx = rng.next_i32();
            let st = random_state(&mut rng);
            h.check("ridx-ignored", GENERIC, &psamp, idx, pfcn, &st);
        }
    }
}

// ===========================================================================
// Row 46 — there is no binary/driver to compare stdout for.
// ===========================================================================

#[test]
fn row46_no_binary_target_exists() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !root.join("src").join("main.rs").exists(),
        "a src/main.rs appeared — the stdout comparison must be added"
    );
    assert!(
        !root.join("src").join("bin").exists(),
        "a src/bin/ appeared — the stdout comparison must be added"
    );
    let cmake = std::fs::read_to_string(root.parent().unwrap().join("c_src/CMakeLists.txt"))
        .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src grew an executable target — the stdout comparison must be added"
    );
}
