//! Exhaustive small-lattice coverage for the simplex routines.
//!
//! `c22` has 3 branches and `c23` has 7, all selected by sign tests on dot
//! products. Enumerating every simplex whose `p` components come from a small
//! integer lattice guarantees each branch is reached, which random sampling
//! only does with high probability.

#![allow(non_snake_case)]

mod harness;
use harness::*;

type FnSimplexF = unsafe extern "C" fn(*mut Simplex) -> f32;
type FnSimplexV = unsafe extern "C" fn(*mut Simplex) -> V;
type FnSimplexVoid = unsafe extern "C" fn(*mut Simplex);
type FnWitness = unsafe extern "C" fn(*mut Simplex, *mut V, *mut V);

const LATTICE: [f32; 5] = [-2.0, -1.0, 0.0, 1.0, 2.0];

/// Enumerate simplices with three lattice `p` points.
fn for_each_lattice_simplex(mut f: impl FnMut(&Simplex)) {
    let mut rng = Rng::new(31);
    for &ax in &LATTICE {
        for &ay in &LATTICE {
            for &bx in &LATTICE {
                for &by in &LATTICE {
                    for &cx in &LATTICE {
                        for &cy in &LATTICE {
                            for count in 1..=3i32 {
                                let mut s = Simplex {
                                    verts: [Sv::default(); 4],
                                    div: if count == 1 { 1.0 } else { rng.unit() * 4.0 },
                                    count,
                                };
                                s.verts[0].p = V { x: ax, y: ay };
                                s.verts[1].p = V { x: bx, y: by };
                                s.verts[2].p = V { x: cx, y: cy };
                                s.verts[3].p = V { x: -ax, y: -by };
                                for (i, v) in s.verts.iter_mut().enumerate() {
                                    v.sA = V {
                                        x: rng.unit() * 4.0,
                                        y: rng.unit() * 4.0,
                                    };
                                    v.sB = V {
                                        x: rng.unit() * 4.0,
                                        y: rng.unit() * 4.0,
                                    };
                                    v.u = rng.unit() * 4.0;
                                    v.iA = i as i32;
                                    v.iB = (3 - i) as i32;
                                }
                                f(&s);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn c22_matches_over_lattice() {
    let (c, r) = pair::<FnSimplexVoid>("c22");
    for_each_lattice_simplex(|base| {
        let mut sc = *base;
        let mut sr = *base;
        unsafe { c(&mut sc) };
        unsafe { r(&mut sr) };
        assert_simplex("c22", base, &sc, &sr);
    });
}

#[test]
fn c23_matches_over_lattice() {
    let (c, r) = pair::<FnSimplexVoid>("c23");
    for_each_lattice_simplex(|base| {
        let mut sc = *base;
        let mut sr = *base;
        unsafe { c(&mut sc) };
        unsafe { r(&mut sr) };
        assert_simplex("c23", base, &sc, &sr);
    });
}

#[test]
fn c2D_c2L_metric_witness_match_over_lattice() {
    let (cd, rd) = pair::<FnSimplexV>("c2D");
    let (cl, rl) = pair::<FnSimplexV>("c2L");
    let (cm, rm) = pair::<FnSimplexF>("c2GJKSimplexMetric");
    let (cw, rw) = pair::<FnWitness>("c2Witness");
    for_each_lattice_simplex(|base| {
        let mut sc = *base;
        let mut sr = *base;
        assert_v("c2D", base, unsafe { cd(&mut sc) }, unsafe { rd(&mut sr) });
        assert_v("c2L", base, unsafe { cl(&mut sc) }, unsafe { rl(&mut sr) });
        assert_f("c2GJKSimplexMetric", base, unsafe { cm(&mut sc) }, unsafe {
            rm(&mut sr)
        });
        let (mut ac, mut bc) = (V::default(), V::default());
        let (mut ar, mut br) = (V::default(), V::default());
        unsafe { cw(&mut sc, &mut ac, &mut bc) };
        unsafe { rw(&mut sr, &mut ar, &mut br) };
        assert_v("c2Witness outA", base, ac, ar);
        assert_v("c2Witness outB", base, bc, br);
        assert_simplex("lattice (structs untouched)", base, &sc, &sr);
    });
}

/// `c2Support` over every vertex multiset drawn from a 3-value lattice, for
/// every count, so ties (where the C loop's strict `>` keeps the first index)
/// are hit exhaustively.
#[test]
fn c2Support_matches_over_lattice() {
    type FnSupport = unsafe extern "C" fn(*const V, i32, V) -> i32;
    let (c, r) = pair::<FnSupport>("c2Support");
    let vals: [f32; 3] = [-1.0, 0.0, 1.0];
    let mut verts = [V::default(); 8];
    // 4 vertices from a 9-point lattice = 6561 sets, times 9 directions.
    for i0 in 0..9usize {
        for i1 in 0..9usize {
            for i2 in 0..9usize {
                for i3 in 0..9usize {
                    for (slot, idx) in [i0, i1, i2, i3].iter().enumerate() {
                        verts[slot] = V {
                            x: vals[idx % 3],
                            y: vals[idx / 3],
                        };
                    }
                    for di in 0..9usize {
                        let d = V {
                            x: vals[di % 3],
                            y: vals[di / 3],
                        };
                        for count in 1..=4i32 {
                            let a = unsafe { c(verts.as_ptr(), count, d) };
                            let b = unsafe { r(verts.as_ptr(), count, d) };
                            assert_eq!(
                                a, b,
                                "c2Support(count={count}, d={d:?}, verts={:?})",
                                &verts[..4]
                            );
                        }
                    }
                }
            }
        }
    }
}
