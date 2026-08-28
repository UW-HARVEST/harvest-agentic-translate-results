//! Bottom-up differential tests for the arithmetic that `call_predict` itself
//! never observes: the twelve `BTAC1C2_PredictSample_PfnN` bodies and the
//! sixteen-way `BTAC1C2_PredictSample` switch.
//!
//! Those functions are `static` in C and private in Rust, so each side is
//! recompiled by `tests/support` into an extra shared object that re-exports
//! them through a `harness_*` shim. Both harnesses are then loaded with
//! `libloading` and compared symbol-for-symbol; nothing is called directly.

mod support;

use libloading::{Library, Symbol};

/// Mirrors `struct btac1c_idxstate_s`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct IdxState {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

type HarnessCall = unsafe extern "C" fn(i32, *mut i32, i32, i32, *mut IdxState) -> i32;
type HarnessSwitch = unsafe extern "C" fn(*mut i32, i32, i32, *mut IdxState) -> i32;
type HarnessSame = unsafe extern "C" fn(i32, i32) -> i32;
type HarnessInt = unsafe extern "C" fn() -> i32;

struct Side {
    lib: Library,
}

impl Side {
    fn new(path: std::path::PathBuf) -> Self {
        Side {
            lib: unsafe { Library::new(&path) }
                .unwrap_or_else(|e| panic!("load {}: {e}", path.display())),
        }
    }
    fn call(&self) -> Symbol<'_, HarnessCall> {
        unsafe { self.lib.get(b"harness_call\0") }.expect("harness_call")
    }
    fn switch(&self) -> Symbol<'_, HarnessSwitch> {
        unsafe { self.lib.get(b"harness_switch\0") }.expect("harness_switch")
    }
    fn same(&self) -> Symbol<'_, HarnessSame> {
        unsafe { self.lib.get(b"harness_same_fn\0") }.expect("harness_same_fn")
    }
    fn int(&self, name: &[u8]) -> i32 {
        let mut n = name.to_vec();
        n.push(0);
        let f: Symbol<HarnessInt> = unsafe { self.lib.get(&n) }
            .unwrap_or_else(|e| panic!("{}: {e}", String::from_utf8_lossy(name)));
        unsafe { f() }
    }
}

fn sides() -> (Side, Side) {
    (
        Side::new(support::c_harness_lib()),
        Side::new(support::rust_harness_lib()),
    )
}

/// Deterministic xorshift64* generator so both sides see identical inputs.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Small-magnitude value: keeps intermediate products far from overflow so
    /// that a mismatch really means a translation bug and not C UB.
    fn small(&mut self) -> i32 {
        (self.next_u64() % 65_537) as i32 - 32_768
    }
    fn i16v(&mut self) -> i16 {
        (self.next_u64() % 65_536) as u16 as i16
    }
}

// ---------------------------------------------------------------------------
// Layout first: everything below passes a `*mut IdxState` across the boundary.
// ---------------------------------------------------------------------------

#[test]
fn idxstate_layout_matches() {
    let (c, r) = sides();
    for probe in [
        &b"harness_sizeof_idxstate"[..],
        b"harness_alignof_idxstate",
        b"harness_offset_idx",
        b"harness_offset_lpred",
        b"harness_offset_rpred",
        b"harness_offset_tag",
        b"harness_offset_bcfcn",
        b"harness_offset_bsfcn",
        b"harness_offset_usefx",
        b"harness_offset_firfx",
    ] {
        let (cv, rv) = (c.int(probe), r.int(probe));
        assert_eq!(
            cv,
            rv,
            "{} mismatch: C={cv}, Rust={rv}",
            String::from_utf8_lossy(probe)
        );
    }
    // The Rust test's own mirror struct must agree too, otherwise every
    // pointer handed to the harnesses below would be misinterpreted.
    assert_eq!(
        c.int(b"harness_sizeof_idxstate") as usize,
        std::mem::size_of::<IdxState>()
    );
    assert_eq!(
        c.int(b"harness_offset_firfx") as usize,
        std::mem::offset_of!(IdxState, firfx)
    );
}

// ---------------------------------------------------------------------------
// Dispatch: BTAC1C2_GetPredictFunc must map each selector to a distinct body.
// ---------------------------------------------------------------------------

#[test]
fn get_predict_func_identity_table_matches() {
    let (c, r) = sides();
    let (cf, rf) = (c.same(), r.same());
    for a in -3..=20 {
        for b in -3..=20 {
            let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
            assert_eq!(
                cv, rv,
                "GetPredictFunc({a}) == GetPredictFunc({b}): C={cv}, Rust={rv}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The twelve dedicated entry points, reached through GetPredictFunc.
// ---------------------------------------------------------------------------

fn compare_call(
    cf: &HarnessCall,
    rf: &HarnessCall,
    label: &str,
    sel: i32,
    samples: &[i32; 8],
    idx: i32,
    pfcn: i32,
    st: &IdxState,
) {
    let mut cs = *samples;
    let mut rs = *samples;
    let mut cst = *st;
    let mut rst = *st;
    let (cv, rv) = unsafe {
        (
            cf(sel, cs.as_mut_ptr(), idx, pfcn, &mut cst),
            rf(sel, rs.as_mut_ptr(), idx, pfcn, &mut rst),
        )
    };
    assert_eq!(
        cv, rv,
        "{label}: sel={sel} idx={idx} pfcn={pfcn} samples={samples:?} -> C={cv}, Rust={rv}"
    );
    // Neither side may touch the caller's buffers.
    assert_eq!(cs, rs, "{label}: psamp mutated differently");
    assert_eq!(cs, *samples, "{label}: psamp must not be modified");
}

/// Batch variant: loads the libraries once and hammers them.
fn sweep(sel_range: std::ops::RangeInclusive<i32>, iterations: usize, seed: u64, small: bool) {
    let (c, r) = sides();
    let (cf, rf) = (c.call(), r.call());
    let mut rng = Rng::new(seed);

    for sel in sel_range {
        for _ in 0..iterations {
            let mut samples = [0i32; 8];
            for s in samples.iter_mut() {
                *s = if small { rng.small() } else { rng.next_i32() };
            }
            let idx = (rng.next_u64() % 17) as i32 - 8;
            let pfcn = sel;
            let mut st = IdxState::default();
            for row in st.firfx.iter_mut() {
                for v in row.iter_mut() {
                    *v = rng.i16v();
                }
            }
            st.idx = rng.next_u64() as u16;
            st.lpred = rng.i16v();
            st.rpred = rng.i16v();
            st.tag = rng.next_u64() as u8;
            st.bcfcn = rng.next_u64() as u8;
            st.bsfcn = rng.next_u64() as u8;
            st.usefx = rng.next_u64() as u8;

            let mut cs = samples;
            let mut rs = samples;
            let mut cst = st;
            let mut rst = st;
            let (cv, rv) = unsafe {
                (
                    cf(sel, cs.as_mut_ptr(), idx, pfcn, &mut cst),
                    rf(sel, rs.as_mut_ptr(), idx, pfcn, &mut rst),
                )
            };
            assert_eq!(
                cv, rv,
                "harness_call sel={sel} idx={idx} samples={samples:?} firfx={:?} -> C={cv}, Rust={rv}",
                st.firfx
            );
            assert_eq!(cs, rs, "psamp diverged for sel={sel}");
        }
    }
}

#[test]
fn pfn0_through_pfn11_match_on_small_inputs() {
    sweep(0..=11, 3000, 0xC0FFEE, true);
}

#[test]
fn pfn0_through_pfn11_match_on_full_range_inputs() {
    // Products such as `76 * psamp[..]` can overflow here; C and Rust agree
    // because both wrap on two's-complement hardware, and the Rust
    // translation uses `wrapping_*` deliberately.
    sweep(0..=11, 3000, 0xDEAD_BEEF, false);
}

#[test]
fn fallthrough_selectors_match() {
    // sel outside 0..=11 selects the generic switch function.
    sweep(12..=24, 1500, 0x5EED, true);
    sweep(-16..=-1, 1500, 0xA5A5, true);
}

#[test]
fn constant_ramps_and_edge_samples_match() {
    let (c, r) = sides();
    let (cf, rf) = (*c.call(), *r.call());
    let states = [IdxState::default(), {
        let mut s = IdxState::default();
        for (i, row) in s.firfx.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = ((i * 8 + j) as i16) * 17 - 60;
            }
        }
        s.firfx[0][0] = i16::MIN;
        s.firfx[1][7] = i16::MAX;
        s.firfx[2][3] = -1;
        s
    }];

    let sample_sets: [[i32; 8]; 10] = [
        [0; 8],
        [1, 2, 3, 4, 5, 6, 7, 8],
        [-1, -2, -3, -4, -5, -6, -7, -8],
        [7, -7, 7, -7, 7, -7, 7, -7],
        [i32::MIN, 0, 0, 0, 0, 0, 0, 0],
        [i32::MAX, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, i32::MIN],
        [-1; 8],
        [1; 8],
        [1000, -1000, 32767, -32768, 12345, -54321, 99, -99],
    ];

    for st in &states {
        for samples in &sample_sets {
            for sel in -20..=20 {
                for idx in -9..=9 {
                    compare_call(&cf, &rf, "edge", sel, samples, idx, sel, st);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The generic switch called directly, so every `case` is reachable including
// the FIR cases 12..=15 which no `_PfnN` covers.
// ---------------------------------------------------------------------------

#[test]
fn predict_sample_switch_matches_every_case() {
    let (c, r) = sides();
    let (cf, rf) = (c.switch(), r.switch());
    let mut rng = Rng::new(0x1BADB002);

    for pfcn in -8..=24 {
        for _ in 0..4000 {
            let mut samples = [0i32; 8];
            for s in samples.iter_mut() {
                *s = rng.small();
            }
            let idx = (rng.next_u64() % 33) as i32 - 16;
            let mut st = IdxState::default();
            for row in st.firfx.iter_mut() {
                for v in row.iter_mut() {
                    *v = rng.i16v();
                }
            }
            let mut cs = samples;
            let mut rs = samples;
            let mut cst = st;
            let mut rst = st;
            let (cv, rv) = unsafe {
                (
                    cf(cs.as_mut_ptr(), idx, pfcn, &mut cst),
                    rf(rs.as_mut_ptr(), idx, pfcn, &mut rst),
                )
            };
            assert_eq!(
                cv, rv,
                "BTAC1C2_PredictSample pfcn={pfcn} idx={idx} samples={samples:?} firfx={:?} -> C={cv}, Rust={rv}",
                st.firfx
            );
            assert_eq!(cs, rs, "psamp diverged for pfcn={pfcn}");
        }
    }
}

#[test]
fn predict_sample_switch_fir_cases_with_extreme_coefficients() {
    let (c, r) = sides();
    let (cf, rf) = (c.switch(), r.switch());

    let coeffs: [i16; 6] = [0, 1, -1, 255, i16::MIN, i16::MAX];
    let sample_sets: [[i32; 8]; 5] = [
        [0; 8],
        [1, -1, 1, -1, 1, -1, 1, -1],
        [32767; 8],
        [-32768; 8],
        [255, 254, 253, 252, 251, 250, 249, 248],
    ];

    for pfcn in 12..=15 {
        for &cv0 in &coeffs {
            for samples in &sample_sets {
                for idx in 0..8 {
                    let mut st = IdxState::default();
                    for row in st.firfx.iter_mut() {
                        for v in row.iter_mut() {
                            *v = cv0;
                        }
                    }
                    let mut cs = *samples;
                    let mut rs = *samples;
                    let mut cst = st;
                    let mut rst = st;
                    let (a, b) = unsafe {
                        (
                            cf(cs.as_mut_ptr(), idx, pfcn, &mut cst),
                            rf(rs.as_mut_ptr(), idx, pfcn, &mut rst),
                        )
                    };
                    assert_eq!(
                        a, b,
                        "FIR pfcn={pfcn} coeff={cv0} idx={idx} samples={samples:?} -> C={a}, Rust={b}"
                    );
                }
            }
        }
    }
}

#[test]
fn predict_sample_switch_reads_the_right_firfx_row() {
    // Each of pfcn 12..=15 must index `firfx[pfcn - 12]`; give every row a
    // distinct signature so a wrong row shows up immediately.
    let (c, r) = sides();
    let (cf, rf) = (c.switch(), r.switch());

    let mut st = IdxState::default();
    for (i, row) in st.firfx.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (256 * (i as i32 + 1) + j as i32) as i16;
        }
    }
    let samples: [i32; 8] = [3, 5, 7, 11, 13, 17, 19, 23];

    for pfcn in 12..=15 {
        for idx in -8..=8 {
            let mut cs = samples;
            let mut rs = samples;
            let mut cst = st;
            let mut rst = st;
            let (a, b) = unsafe {
                (
                    cf(cs.as_mut_ptr(), idx, pfcn, &mut cst),
                    rf(rs.as_mut_ptr(), idx, pfcn, &mut rst),
                )
            };
            assert_eq!(a, b, "firfx row selection pfcn={pfcn} idx={idx}");
        }
    }
}

#[test]
fn predict_sample_switch_default_case_matches() {
    let (c, r) = sides();
    let (cf, rf) = (c.switch(), r.switch());
    let samples: [i32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    for pfcn in [
        -1,
        -2,
        -100,
        16,
        17,
        100,
        1 << 20,
        i32::MIN,
        i32::MIN + 12,
        i32::MAX,
    ] {
        let mut st = IdxState::default();
        st.firfx[0][0] = 99;
        let mut cs = samples;
        let mut rs = samples;
        let mut cst = st;
        let mut rst = st;
        let (a, b) = unsafe {
            (
                cf(cs.as_mut_ptr(), 3, pfcn, &mut cst),
                rf(rs.as_mut_ptr(), 3, pfcn, &mut rst),
            )
        };
        assert_eq!(a, b, "default case pfcn={pfcn}: C={a}, Rust={b}");
    }
}

// ---------------------------------------------------------------------------
// `psamp[(idx - n) & 7]` wrap-around behaviour for negative and large idx.
// ---------------------------------------------------------------------------

#[test]
fn index_masking_matches_for_all_idx_residues() {
    let (c, r) = sides();
    let (cf, rf) = (c.call(), r.call());
    let samples: [i32; 8] = [100, 200, 300, 400, 500, 600, 700, 800];

    let idxs: Vec<i32> = (-64..=64)
        .chain([
            i32::MIN + 8,
            i32::MIN + 9,
            -1_000_003,
            1_000_003,
            i32::MAX - 8,
        ])
        .collect();

    for sel in 0..=11 {
        for &idx in &idxs {
            let mut st = IdxState::default();
            let mut cs = samples;
            let mut rs = samples;
            let mut cst = st;
            let mut rst = st;
            let (a, b) = unsafe {
                (
                    cf(sel, cs.as_mut_ptr(), idx, sel, &mut cst),
                    rf(sel, rs.as_mut_ptr(), idx, sel, &mut rst),
                )
            };
            assert_eq!(a, b, "index masking sel={sel} idx={idx}: C={a}, Rust={b}");
            st.idx = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// The `_PfnN` bodies deliberately disagree with the matching switch `case`
// for N = 10 and N = 11. Pin that quirk down so a future "fix" is caught.
// ---------------------------------------------------------------------------

#[test]
fn pfn_bodies_and_switch_cases_agree_or_disagree_exactly_as_c_does() {
    let (c, r) = sides();
    let (c_call, c_switch) = (c.call(), c.switch());
    let (r_call, r_switch) = (r.call(), r.switch());
    let samples: [i32; 8] = [13, -21, 34, -55, 89, -144, 233, -377];

    for n in 0..=11i32 {
        for idx in 0..8 {
            let mut st = IdxState::default();
            let mut b1 = samples;
            let mut b2 = samples;
            let mut b3 = samples;
            let mut b4 = samples;
            let (cc, cs, rc, rs) = unsafe {
                (
                    c_call(n, b1.as_mut_ptr(), idx, n, &mut st),
                    c_switch(b2.as_mut_ptr(), idx, n, &mut st),
                    r_call(n, b3.as_mut_ptr(), idx, n, &mut st),
                    r_switch(b4.as_mut_ptr(), idx, n, &mut st),
                )
            };
            assert_eq!(cc, rc, "Pfn{n} idx={idx}");
            assert_eq!(cs, rs, "switch case {n} idx={idx}");
            // Whether the two paths agree is itself part of the C behaviour.
            assert_eq!(
                cc == cs,
                rc == rs,
                "Pfn{n} vs switch case {n} agreement differs at idx={idx}: \
                 C({cc},{cs}) Rust({rc},{rs})"
            );
        }
    }
}
