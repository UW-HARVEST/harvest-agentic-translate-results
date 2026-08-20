//! Exploratory search for inputs that drive `c2GJK`'s `while (iter < 20)` loop
//! bound (`ERRORS.md` row 46) and for the highest reachable iteration count.
//!
//! Run with `--ignored --nocapture` to see the histogram:
//!   cargo test --test search_iter_cap -- --ignored --nocapture

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;
use std::os::raw::{c_int, c_void};

fn call_c(
    gjk: FnGJK,
    abuf: &[u8],
    ta: C2_TYPE,
    ax: Option<c2x>,
    bbuf: &[u8],
    tb: C2_TYPE,
    bx: Option<c2x>,
    ur: c_int,
    cache: Option<c2GJKCache>,
) -> (f32, c_int) {
    let mut it: c_int = -1;
    let mut ch = cache.unwrap_or_default();
    let axp = ax.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
    let d = unsafe {
        gjk(
            abuf.as_ptr() as *const c_void,
            ta,
            axp,
            bbuf.as_ptr() as *const c_void,
            tb,
            bxp,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            ur,
            &mut it,
            if cache.is_some() {
                &mut ch
            } else {
                std::ptr::null_mut()
            },
        )
    };
    (d, it)
}

#[test]
#[ignore]
fn search_max_iterations() {
    let (cf, _) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    let mut hist = [0usize; 22];
    let mut best: (c_int, String) = (-1, String::new());

    for _ in 0..400_000 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let (ab, adesc) = match ta {
            C2_TYPE_CIRCLE => {
                let s = rng.circle();
                (raw(&s).to_vec(), format!("{s:?}"))
            }
            C2_TYPE_AABB => {
                let s = rng.aabb();
                (raw(&s).to_vec(), format!("{s:?}"))
            }
            _ => {
                let s = rng.capsule();
                (raw(&s).to_vec(), format!("{s:?}"))
            }
        };
        let (bb, bdesc) = match tb {
            C2_TYPE_CIRCLE => {
                let s = rng.circle();
                (raw(&s).to_vec(), format!("{s:?}"))
            }
            C2_TYPE_AABB => {
                let s = rng.aabb();
                (raw(&s).to_vec(), format!("{s:?}"))
            }
            _ => {
                let s = rng.capsule();
                (raw(&s).to_vec(), format!("{s:?}"))
            }
        };
        let ax = if rng.bool() { Some(rng.x()) } else { None };
        let bx = if rng.bool() { Some(rng.x()) } else { None };
        let ur = if rng.bool() { 1 } else { 0 };
        let cache = if rng.bool() {
            Some(c2GJKCache::default())
        } else {
            None
        };
        let (_, it) = call_c(cf, &ab, ta, ax, &bb, tb, bx, ur, cache);
        if (0..=21).contains(&it) {
            hist[it as usize] += 1;
        }
        if it > best.0 {
            best = (
                it,
                format!("ta={ta} A={adesc} ax={ax:?} tb={tb} B={bdesc} bx={bx:?} ur={ur}"),
            );
        }
    }
    eprintln!("[search] iteration histogram = {hist:?}");
    eprintln!("[search] max iter = {} for {}", best.0, best.1);
}

/// Mutation / hill-climbing search over the raw input bytes, maximising the
/// reported iteration count.  This is the aggressive attempt at `ERRORS.md`
/// row 46 (`while (iter < 20)` running out).
#[test]
#[ignore]
fn search_max_iterations_mutating() {
    let (cf, _) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(0x0BAD_F00D_1234_5678);
    let mut global_best: (c_int, String) = (-1, String::new());
    let mut hist = [0usize; 22];

    // Value pool the mutator draws from: exact small ints, powers of two,
    // extremes, and specials -- the shapes most likely to create exact ties.
    let mut pool: Vec<u32> = Vec::new();
    for k in -8i32..=8 {
        pool.push((k as f32).to_bits());
        pool.push((k as f32 * 0.5).to_bits());
        pool.push((k as f32 * 0.25).to_bits());
    }
    for &s in SPECIALS.iter() {
        pool.push(s.to_bits());
    }
    for &o in ODDBALLS.iter() {
        pool.push(o);
    }
    for e in [1e-30f32, 1e-8, 1e8, 1e18, 1e30, 3.4e38] {
        pool.push(e.to_bits());
        pool.push((-e).to_bits());
    }

    for _restart in 0..400 {
        // state = (ta, tb, 5 floats each, ax, bx, ur, cache-count)
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let mut av: [u32; 5] = [0; 5];
        let mut bv: [u32; 5] = [0; 5];
        for k in 0..5 {
            av[k] = pool[rng.below(pool.len() as u32) as usize];
            bv[k] = pool[rng.below(pool.len() as u32) as usize];
        }
        let mut axv: [u32; 4] = [
            0f32.to_bits(),
            0f32.to_bits(),
            1f32.to_bits(),
            0f32.to_bits(),
        ];
        let mut bxv = axv;
        let ur: c_int = 1;

        let eval = |av: &[u32; 5], bv: &[u32; 5], axv: &[u32; 4], bxv: &[u32; 4]| -> c_int {
            let mk = |ty: C2_TYPE, v: &[u32; 5]| -> Vec<u8> {
                let f: Vec<f32> = v.iter().map(|&b| f32::from_bits(b)).collect();
                match ty {
                    C2_TYPE_CIRCLE => raw(&c2Circle {
                        p: c2v { x: f[0], y: f[1] },
                        r: f[2],
                    })
                    .to_vec(),
                    C2_TYPE_AABB => raw(&c2AABB {
                        min: c2v { x: f[0], y: f[1] },
                        max: c2v { x: f[2], y: f[3] },
                    })
                    .to_vec(),
                    _ => raw(&c2Capsule {
                        a: c2v { x: f[0], y: f[1] },
                        b: c2v { x: f[2], y: f[3] },
                        r: f[4],
                    })
                    .to_vec(),
                }
            };
            let mkx = |v: &[u32; 4]| c2x {
                p: c2v {
                    x: f32::from_bits(v[0]),
                    y: f32::from_bits(v[1]),
                },
                r: c2r {
                    c: f32::from_bits(v[2]),
                    s: f32::from_bits(v[3]),
                },
            };
            let ab = mk(ta, av);
            let bb = mk(tb, bv);
            let (_, it) = call_c(
                cf,
                &ab,
                ta,
                Some(mkx(axv)),
                &bb,
                tb,
                Some(mkx(bxv)),
                ur,
                None,
            );
            it
        };

        let mut cur = eval(&av, &bv, &axv, &bxv);
        for _step in 0..2_000 {
            let (sav, sbv, saxv, sbxv) = (av, bv, axv, bxv);
            // mutate 1-3 slots
            for _ in 0..(1 + rng.below(3)) {
                let slot = rng.below(18);
                let val = pool[rng.below(pool.len() as u32) as usize];
                match slot {
                    0..=4 => av[slot as usize] = val,
                    5..=9 => bv[(slot - 5) as usize] = val,
                    10..=13 => axv[(slot - 10) as usize] = val,
                    _ => bxv[(slot - 14) as usize] = val,
                }
            }
            let got = eval(&av, &bv, &axv, &bxv);
            if (0..=21).contains(&got) {
                hist[got as usize] += 1;
            }
            if got >= cur {
                cur = got;
            } else {
                av = sav;
                bv = sbv;
                axv = saxv;
                bxv = sbxv;
            }
            if cur > global_best.0 {
                global_best = (
                    cur,
                    format!("ta={ta} av={av:08x?} tb={tb} bv={bv:08x?} ax={axv:08x?} bx={bxv:08x?}"),
                );
            }
        }
    }
    eprintln!("[search] mutating histogram = {hist:?}");
    eprintln!(
        "[search] mutating max iter = {} for {}",
        global_best.0, global_best.1
    );
}

/// Longer, finer-grained search: bit-level mutations in addition to pool
/// draws, warm caches, and many more restarts.
#[test]
#[ignore]
fn search_max_iterations_deep() {
    let (cf, _) = fnpair!("c2GJK", FnGJK);
    let mut rng = Rng::new(0x5EED_5EED_0000_0001);
    let mut hist = [0usize; 22];
    let mut global_best: (c_int, String) = (-1, String::new());

    let mut pool: Vec<u32> = Vec::new();
    for k in -16i32..=16 {
        for m in [1.0f32, 0.5, 0.25, 0.125, 2.0, 4.0] {
            pool.push((k as f32 * m).to_bits());
        }
    }
    for &s in SPECIALS.iter() {
        pool.push(s.to_bits());
    }
    for &o in ODDBALLS.iter() {
        pool.push(o);
    }
    for e in [1e-38f32, 1e-30, 1e-8, 1e-3, 1e3, 1e8, 1e18, 1e30, 3.4e38] {
        pool.push(e.to_bits());
        pool.push((-e).to_bits());
    }

    #[derive(Clone, Copy)]
    struct St {
        ta: C2_TYPE,
        tb: C2_TYPE,
        av: [u32; 5],
        bv: [u32; 5],
        axv: [u32; 4],
        bxv: [u32; 4],
        use_ax: bool,
        use_bx: bool,
        cc: c_int,
        cia: [c_int; 3],
        cib: [c_int; 3],
        cmet: u32,
        cdiv: u32,
    }

    let eval = |s: &St| -> c_int {
        let mk = |ty: C2_TYPE, v: &[u32; 5]| -> Vec<u8> {
            let f: Vec<f32> = v.iter().map(|&b| f32::from_bits(b)).collect();
            match ty {
                C2_TYPE_CIRCLE => raw(&c2Circle {
                    p: c2v { x: f[0], y: f[1] },
                    r: f[2],
                })
                .to_vec(),
                C2_TYPE_AABB => raw(&c2AABB {
                    min: c2v { x: f[0], y: f[1] },
                    max: c2v { x: f[2], y: f[3] },
                })
                .to_vec(),
                _ => raw(&c2Capsule {
                    a: c2v { x: f[0], y: f[1] },
                    b: c2v { x: f[2], y: f[3] },
                    r: f[4],
                })
                .to_vec(),
            }
        };
        let mkx = |v: &[u32; 4]| c2x {
            p: c2v {
                x: f32::from_bits(v[0]),
                y: f32::from_bits(v[1]),
            },
            r: c2r {
                c: f32::from_bits(v[2]),
                s: f32::from_bits(v[3]),
            },
        };
        let ab = mk(s.ta, &s.av);
        let bb = mk(s.tb, &s.bv);
        // keep cache indices inside the initialised proxy range (avoid UB)
        let na = match s.ta {
            C2_TYPE_CIRCLE => 1,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let nb = match s.tb {
            C2_TYPE_CIRCLE => 1,
            C2_TYPE_CAPSULE => 2,
            _ => 4,
        };
        let cache = if s.cc == 0 {
            None
        } else {
            Some(c2GJKCache {
                metric: f32::from_bits(s.cmet),
                count: s.cc,
                iA: [
                    s.cia[0].rem_euclid(na),
                    s.cia[1].rem_euclid(na),
                    s.cia[2].rem_euclid(na),
                ],
                iB: [
                    s.cib[0].rem_euclid(nb),
                    s.cib[1].rem_euclid(nb),
                    s.cib[2].rem_euclid(nb),
                ],
                div: f32::from_bits(s.cdiv),
            })
        };
        let (_, it) = call_c(
            cf,
            &ab,
            s.ta,
            if s.use_ax { Some(mkx(&s.axv)) } else { None },
            &bb,
            s.tb,
            if s.use_bx { Some(mkx(&s.bxv)) } else { None },
            1,
            cache,
        );
        it
    };

    for _restart in 0..2_000 {
        let mut st = St {
            ta: ALL_TYPES[rng.below(3) as usize],
            tb: ALL_TYPES[rng.below(3) as usize],
            av: [0; 5],
            bv: [0; 5],
            axv: [
                0f32.to_bits(),
                0f32.to_bits(),
                1f32.to_bits(),
                0f32.to_bits(),
            ],
            bxv: [
                0f32.to_bits(),
                0f32.to_bits(),
                1f32.to_bits(),
                0f32.to_bits(),
            ],
            use_ax: rng.bool(),
            use_bx: rng.bool(),
            cc: rng.below(4) as c_int,
            cia: [0; 3],
            cib: [0; 3],
            cmet: rng.u32(),
            cdiv: 1f32.to_bits(),
        };
        for k in 0..5 {
            st.av[k] = pool[rng.below(pool.len() as u32) as usize];
            st.bv[k] = pool[rng.below(pool.len() as u32) as usize];
        }
        for k in 0..3 {
            st.cia[k] = rng.below(4) as c_int;
            st.cib[k] = rng.below(4) as c_int;
        }
        let mut cur = eval(&st);
        for _step in 0..3_000 {
            let save = st;
            for _ in 0..(1 + rng.below(3)) {
                match rng.below(26) {
                    v @ 0..=4 => {
                        if rng.bool() {
                            st.av[v as usize] = pool[rng.below(pool.len() as u32) as usize];
                        } else {
                            st.av[v as usize] ^= 1 << rng.below(32);
                        }
                    }
                    v @ 5..=9 => {
                        if rng.bool() {
                            st.bv[(v - 5) as usize] = pool[rng.below(pool.len() as u32) as usize];
                        } else {
                            st.bv[(v - 5) as usize] ^= 1 << rng.below(32);
                        }
                    }
                    v @ 10..=13 => {
                        if rng.bool() {
                            st.axv[(v - 10) as usize] = pool[rng.below(pool.len() as u32) as usize];
                        } else {
                            st.axv[(v - 10) as usize] ^= 1 << rng.below(32);
                        }
                    }
                    v @ 14..=17 => {
                        if rng.bool() {
                            st.bxv[(v - 14) as usize] = pool[rng.below(pool.len() as u32) as usize];
                        } else {
                            st.bxv[(v - 14) as usize] ^= 1 << rng.below(32);
                        }
                    }
                    18 => st.use_ax = !st.use_ax,
                    19 => st.use_bx = !st.use_bx,
                    20 => st.cc = rng.below(4) as c_int,
                    21 => st.cia[rng.below(3) as usize] = rng.below(4) as c_int,
                    22 => st.cib[rng.below(3) as usize] = rng.below(4) as c_int,
                    23 => st.cmet = pool[rng.below(pool.len() as u32) as usize],
                    24 => st.cdiv = pool[rng.below(pool.len() as u32) as usize],
                    _ => st.ta = ALL_TYPES[rng.below(3) as usize],
                }
            }
            let got = eval(&st);
            if (0..=21).contains(&got) {
                hist[got as usize] += 1;
            }
            if got >= cur {
                cur = got;
                if cur > global_best.0 {
                    global_best = (cur, format!(
                        "ta={} av={:08x?} tb={} bv={:08x?} use_ax={} ax={:08x?} use_bx={} bx={:08x?} cc={} cia={:?} cib={:?} cmet={:08x} cdiv={:08x}",
                        st.ta, st.av, st.tb, st.bv, st.use_ax, st.axv, st.use_bx, st.bxv, st.cc, st.cia, st.cib, st.cmet, st.cdiv));
                }
            } else {
                st = save;
            }
            if cur >= 20 {
                break;
            }
        }
        if global_best.0 >= 20 {
            break;
        }
    }
    eprintln!("[search] deep histogram = {hist:?}");
    eprintln!("[search] deep max iter = {} for {}", global_best.0, global_best.1);
}
