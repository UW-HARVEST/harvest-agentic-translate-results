//! Phase B (deep) — differential tests for the *lowest-level* entry points.
//!
//! `BTAC1C2_PredictSample`, `BTAC1C2_PredictSample_Pfn0..11` and
//! `BTAC1C2_GetPredictFunc` are `static` in the C, so they are not in the
//! dynamic symbol table. They *are* in `.symtab` as local (`t`) symbols in both
//! builds, so this harness resolves them as
//! `runtime_addr(call_predict) - link_addr(call_predict) + link_addr(sym)`
//! and calls them through raw `extern "C"` function pointers — i.e. still
//! across the real FFI/ABI boundary of the two loaded `.so`s, never as Rust
//! functions.
//!
//! This is what actually validates the predictor arithmetic (the bulk of the
//! translated code), which `call_predict` alone cannot observe.
//!
//! Covers CONFIGS.md rows C16..C35.

mod common;

use common::*;
use std::ffi::c_int;

const PFN_NAMES: [&str; 12] = [
    "BTAC1C2_PredictSample_Pfn0",
    "BTAC1C2_PredictSample_Pfn1",
    "BTAC1C2_PredictSample_Pfn2",
    "BTAC1C2_PredictSample_Pfn3",
    "BTAC1C2_PredictSample_Pfn4",
    "BTAC1C2_PredictSample_Pfn5",
    "BTAC1C2_PredictSample_Pfn6",
    "BTAC1C2_PredictSample_Pfn7",
    "BTAC1C2_PredictSample_Pfn8",
    "BTAC1C2_PredictSample_Pfn9",
    "BTAC1C2_PredictSample_Pfn10",
    "BTAC1C2_PredictSample_Pfn11",
];

/// Skip-with-message helper: the deep harness needs the *debug* Rust artifact,
/// because `--release` inlines the statics away.
macro_rules! pair_or_skip {
    ($what:expr) => {
        match open_pair_debug() {
            Some(p) => p,
            None => {
                eprintln!("skipping {}: debug .so not built", $what);
                return;
            }
        }
    };
}

fn resolve(lib: &Lib, name: &str) -> PredictFn {
    lib.predict_fn(name)
        .unwrap_or_else(|| panic!("{}: internal symbol `{name}` not found", lib.name))
}

/// Call the predictor in both libraries with identical inputs and compare the
/// return value *and* that neither library mutated its input buffers.
fn diff_call(
    cname: &str,
    cfn: PredictFn,
    rname: &str,
    rfn: PredictFn,
    psamp: &[c_int; 8],
    idx: c_int,
    pfcn: c_int,
    st: &IdxState,
) {
    let mut cbuf = *psamp;
    let mut rbuf = *psamp;
    let mut cst = *st;
    let mut rst = *st;

    let cv = unsafe { cfn(cbuf.as_mut_ptr(), idx, pfcn, &mut cst) };
    let rv = unsafe { rfn(rbuf.as_mut_ptr(), idx, pfcn, &mut rst) };

    assert_eq!(
        cv, rv,
        "{cname}({psamp:?}, idx={idx}, pfcn={pfcn}, firfx={:?}) = {cv} but {rname} = {rv}",
        st.firfx
    );
    assert_eq!(cbuf, *psamp, "{cname} mutated psamp");
    assert_eq!(rbuf, *psamp, "{rname} mutated psamp (C did not)");
    assert_eq!(cst, *st, "{cname} mutated idxstate");
    assert_eq!(rst, *st, "{rname} mutated idxstate (C did not)");
}

// ---------------------------------------------------------------------------
// C16..C27 — one row per specialised predictor, full idx x psamp matrix
// ---------------------------------------------------------------------------

fn exercise_pfn(row: &str, pfn_index: usize) {
    let (c, r) = pair_or_skip!(row);
    let name = PFN_NAMES[pfn_index];
    let cfn = resolve(&c, name);
    let rfn = resolve(&r, name);

    let st = IdxState::zeroed();
    let idxs = idx_shapes();
    let samples = psamp_shapes();

    for &idx in &idxs {
        for ps in &samples {
            // `pfcn` is ignored by the specialised variants -- pass a range of
            // values (incl. nonsense ones) to prove it really is ignored.
            for pfcn in [pfn_index as c_int, 0, 15, 99, -1, i32::MIN, i32::MAX] {
                diff_call(&c.name, cfn, &r.name, rfn, ps, idx, pfcn, &st);
            }
        }
    }
}

#[test]
fn cfg_c16_pfn0() {
    exercise_pfn("C16", 0);
}
#[test]
fn cfg_c17_pfn1() {
    exercise_pfn("C17", 1);
}
#[test]
fn cfg_c18_pfn2() {
    exercise_pfn("C18", 2);
}
#[test]
fn cfg_c19_pfn3() {
    exercise_pfn("C19", 3);
}
#[test]
fn cfg_c20_pfn4() {
    exercise_pfn("C20", 4);
}
#[test]
fn cfg_c21_pfn5() {
    exercise_pfn("C21", 5);
}
#[test]
fn cfg_c22_pfn6() {
    exercise_pfn("C22", 6);
}
#[test]
fn cfg_c23_pfn7_div16() {
    exercise_pfn("C23", 7);
}
#[test]
fn cfg_c24_pfn8_div64() {
    exercise_pfn("C24", 8);
}
#[test]
fn cfg_c25_pfn9_div64() {
    exercise_pfn("C25", 9);
}
#[test]
fn cfg_c26_pfn10_shift3() {
    exercise_pfn("C26", 10);
}
#[test]
fn cfg_c27_pfn11_shift1() {
    exercise_pfn("C27", 11);
}

// ---------------------------------------------------------------------------
// C28 — generic `BTAC1C2_PredictSample`, switch cases 0..11
// ---------------------------------------------------------------------------

#[test]
fn cfg_c28_generic_cases_0_to_11() {
    let (c, r) = pair_or_skip!("C28");
    let cfn = resolve(&c, "BTAC1C2_PredictSample");
    let rfn = resolve(&r, "BTAC1C2_PredictSample");

    let st = IdxState::zeroed();
    for pfcn in 0..=11 {
        for &idx in &idx_shapes() {
            for ps in &psamp_shapes() {
                diff_call(&c.name, cfn, &r.name, rfn, ps, idx, pfcn, &st);
            }
        }
    }
}

/// The C deliberately makes `case 10`/`case 11` of the generic switch differ
/// from `Pfn10`/`Pfn11` (`>>4` vs `>>3`, `>>3` vs `>>1`). Guard that the Rust
/// reproduced the discrepancy instead of "fixing" it, by asserting the two
/// disagree in exactly the same way in both libraries.
#[test]
fn cfg_c28b_generic_vs_specialised_discrepancy_preserved() {
    let (c, r) = pair_or_skip!("C28b");
    let cg = resolve(&c, "BTAC1C2_PredictSample");
    let rg = resolve(&r, "BTAC1C2_PredictSample");

    let st = IdxState::zeroed();
    let mut divergences_10 = 0usize;
    let mut divergences_11 = 0usize;

    for pfcn in 0..=11usize {
        let cs = resolve(&c, PFN_NAMES[pfcn]);
        let rs = resolve(&r, PFN_NAMES[pfcn]);
        for &idx in &idx_shapes() {
            for ps in &psamp_shapes() {
                let mut b = *ps;
                let mut s1 = st;
                let mut s2 = st;
                let mut s3 = st;
                let mut s4 = st;
                let c_generic = unsafe { cg(b.as_mut_ptr(), idx, pfcn as c_int, &mut s1) };
                let c_special = unsafe { cs(b.as_mut_ptr(), idx, pfcn as c_int, &mut s2) };
                let r_generic = unsafe { rg(b.as_mut_ptr(), idx, pfcn as c_int, &mut s3) };
                let r_special = unsafe { rs(b.as_mut_ptr(), idx, pfcn as c_int, &mut s4) };

                assert_eq!(
                    c_generic == c_special,
                    r_generic == r_special,
                    "generic-vs-specialised agreement differs for pfcn={pfcn} idx={idx} \
                     psamp={ps:?}: C({c_generic},{c_special}) Rust({r_generic},{r_special})"
                );
                if c_generic != c_special {
                    match pfcn {
                        10 => divergences_10 += 1,
                        11 => divergences_11 += 1,
                        other => panic!(
                            "unexpected C divergence at pfcn={other} idx={idx} psamp={ps:?}: \
                             generic={c_generic} specialised={c_special}"
                        ),
                    }
                }
            }
        }
    }
    assert!(
        divergences_10 > 0 && divergences_11 > 0,
        "the deliberate >>4/>>3 and >>3/>>1 discrepancies were never observed \
         (10: {divergences_10}, 11: {divergences_11}) -- test is not exercising them"
    );
}

// ---------------------------------------------------------------------------
// C29..C32 — generic FIR cases 12..15, one row per `firfx` row selected
// ---------------------------------------------------------------------------

fn exercise_fir(row: &str, pfcn: c_int) {
    let (c, r) = pair_or_skip!(row);
    let cfn = resolve(&c, "BTAC1C2_PredictSample");
    let rfn = resolve(&r, "BTAC1C2_PredictSample");

    let idxs = idx_shapes();
    let samples = psamp_shapes();
    for fir in &firfx_shapes() {
        let mut st = IdxState::zeroed();
        st.firfx = *fir;
        // fill the other fields too: they must not influence the result
        st.idx = 0xBEEF;
        st.lpred = -12345;
        st.rpred = 23456;
        st.tag = 0xAB;
        st.bcfcn = 0xCD;
        st.bsfcn = 0xEF;
        st.usefx = 0x7F;
        for &idx in &idxs {
            for ps in &samples {
                diff_call(&c.name, cfn, &r.name, rfn, ps, idx, pfcn, &st);
            }
        }
    }
}

#[test]
fn cfg_c29_fir_row0_pfcn12() {
    exercise_fir("C29", 12);
}
#[test]
fn cfg_c30_fir_row1_pfcn13() {
    exercise_fir("C30", 13);
}
#[test]
fn cfg_c31_fir_row2_pfcn14() {
    exercise_fir("C31", 14);
}
#[test]
fn cfg_c32_fir_row3_pfcn15() {
    exercise_fir("C32", 15);
}

// ---------------------------------------------------------------------------
// C33 — `idx` outside 0..7 for every pfcn 0..15 (the `& 7` masking)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c33_idx_masking_all_pfcn() {
    let (c, r) = pair_or_skip!("C33");
    let cfn = resolve(&c, "BTAC1C2_PredictSample");
    let rfn = resolve(&r, "BTAC1C2_PredictSample");

    let mut st = IdxState::zeroed();
    let mut rng = Rng::new(0x5EED_0033);
    for row in st.firfx.iter_mut() {
        for c2 in row.iter_mut() {
            *c2 = rng.next_i16();
        }
    }

    let odd_idxs: Vec<c_int> = idx_shapes().into_iter().filter(|v| !(0..8).contains(v)).collect();
    assert!(!odd_idxs.is_empty());

    for pfcn in 0..=15 {
        for &idx in &odd_idxs {
            for ps in &psamp_shapes() {
                diff_call(&c.name, cfn, &r.name, rfn, ps, idx, pfcn, &st);
                // masking property: result must equal the one for `idx & 7`
                let masked = idx & 7;
                let mut b1 = *ps;
                let mut b2 = *ps;
                let mut s1 = st;
                let mut s2 = st;
                let a = unsafe { cfn(b1.as_mut_ptr(), idx, pfcn, &mut s1) };
                let b = unsafe { cfn(b2.as_mut_ptr(), masked, pfcn, &mut s2) };
                assert_eq!(a, b, "C masking property broken at idx={idx} pfcn={pfcn}");
                let mut b3 = *ps;
                let mut b4 = *ps;
                let mut s3 = st;
                let mut s4 = st;
                let a2 = unsafe { rfn(b3.as_mut_ptr(), idx, pfcn, &mut s3) };
                let b2v = unsafe { rfn(b4.as_mut_ptr(), masked, pfcn, &mut s4) };
                assert_eq!(a2, b2v, "Rust masking property broken at idx={idx} pfcn={pfcn}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C34 — `BTAC1C2_GetPredictFunc` dispatch table
// ---------------------------------------------------------------------------

#[test]
fn cfg_c34_getpredictfunc_dispatch_table() {
    let (c, r) = pair_or_skip!("C34");

    // Label a returned pointer by which internal function it is.
    let label = |lib: &Lib, p: usize| -> String {
        for (i, n) in PFN_NAMES.iter().enumerate() {
            if lib.internal_addr(n) == Some(p) {
                return format!("Pfn{i}");
            }
        }
        if lib.internal_addr("BTAC1C2_PredictSample") == Some(p) {
            return "generic".to_string();
        }
        format!("<unknown:{p:#x}>")
    };

    let cg = c
        .get_predict_func_fn()
        .expect("C BTAC1C2_GetPredictFunc not found");
    let rg = r
        .get_predict_func_fn()
        .expect("Rust BTAC1C2_GetPredictFunc not found");

    let mut selectors: Vec<c_int> = (-16..=32).collect();
    selectors.extend([i32::MIN, i32::MAX, 1 << 20, -(1 << 20)]);
    let mut rng = Rng::new(0x5EED_0034);
    for _ in 0..200 {
        selectors.push(rng.next_i32());
    }

    let mut seen_c: Vec<(c_int, String)> = Vec::new();
    for pfcn in selectors {
        let cp = unsafe { cg(pfcn) } as usize;
        let rp = unsafe { rg(pfcn) } as usize;
        let cl = label(&c, cp);
        let rl = label(&r, rp);
        assert_eq!(
            cl, rl,
            "GetPredictFunc({pfcn}): C resolved to {cl}, Rust resolved to {rl}"
        );
        assert!(!cl.starts_with("<unknown"), "C returned unlabelled ptr for {pfcn}");
        if (0..=11).contains(&pfcn) || pfcn == 12 || pfcn == -1 {
            seen_c.push((pfcn, cl));
        }
    }

    // 0..11 must be 12 pairwise-distinct pointers in *both* libraries
    let mut cptrs = Vec::new();
    let mut rptrs = Vec::new();
    for pfcn in 0..=11 {
        cptrs.push(unsafe { cg(pfcn) } as usize);
        rptrs.push(unsafe { rg(pfcn) } as usize);
    }
    for i in 0..12 {
        for j in (i + 1)..12 {
            assert_ne!(cptrs[i], cptrs[j], "C: Pfn{i} and Pfn{j} share an address");
            assert_ne!(rptrs[i], rptrs[j], "Rust: Pfn{i} and Pfn{j} share an address");
        }
    }
    assert_eq!(
        seen_c
            .iter()
            .filter(|(p, l)| (0..=11).contains(p) && l.starts_with("Pfn"))
            .count(),
        12
    );
}

// ---------------------------------------------------------------------------
// C35 — full randomized property sweep over every axis at once
// ---------------------------------------------------------------------------

#[test]
fn cfg_c35_full_random_property_sweep() {
    let (c, r) = pair_or_skip!("C35");
    let cgen = resolve(&c, "BTAC1C2_PredictSample");
    let rgen = resolve(&r, "BTAC1C2_PredictSample");
    let cspec: Vec<PredictFn> = PFN_NAMES.iter().map(|n| resolve(&c, n)).collect();
    let rspec: Vec<PredictFn> = PFN_NAMES.iter().map(|n| resolve(&r, n)).collect();

    let mut rng = Rng::new(0x5EED_0035);
    for iter in 0..4000 {
        let pfcn = rng.range_i32(-4, 20);
        let idx = match iter % 3 {
            0 => rng.range_i32(0, 7),
            1 => rng.range_i32(-32, 32),
            _ => rng.next_i32(),
        };
        let mut ps = [0i32; 8];
        for v in ps.iter_mut() {
            *v = match iter % 4 {
                0 => rng.range_i32(-8, 8),
                1 => rng.range_i32(-32768, 32767),
                2 => rng.range_i32(-(1 << 28), 1 << 28),
                _ => rng.next_i32(),
            };
        }
        let mut st = IdxState::zeroed();
        st.idx = rng.next_u64() as u16;
        st.lpred = rng.next_i16();
        st.rpred = rng.next_i16();
        st.tag = rng.next_u64() as u8;
        st.bcfcn = rng.next_u64() as u8;
        st.bsfcn = rng.next_u64() as u8;
        st.usefx = rng.next_u64() as u8;
        for row in st.firfx.iter_mut() {
            for cf in row.iter_mut() {
                *cf = match iter % 3 {
                    0 => rng.range_i32(-256, 256) as i16,
                    1 => rng.next_i16(),
                    _ => rng.range_i32(-4, 4) as i16,
                };
            }
        }

        diff_call(&c.name, cgen, &r.name, rgen, &ps, idx, pfcn, &st);
        if (0..12).contains(&pfcn) {
            let k = pfcn as usize;
            diff_call(&c.name, cspec[k], &r.name, rspec[k], &ps, idx, pfcn, &st);
        }
    }
}
