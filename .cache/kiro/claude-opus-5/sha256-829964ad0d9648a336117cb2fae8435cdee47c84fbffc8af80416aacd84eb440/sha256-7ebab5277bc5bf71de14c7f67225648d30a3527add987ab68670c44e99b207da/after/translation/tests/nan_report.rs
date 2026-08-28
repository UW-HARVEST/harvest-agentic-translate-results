//! Diagnostic: reports, per exported function, how many NaN-payload
//! (sign/quiet-bit) divergences remain between the C and Rust libraries.
//! Run with `cargo test --test nan_report -- --nocapture`.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::collections::BTreeMap;

const NANMAKERS: &[f32] = &[
    0.0,
    -0.0,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::from_bits(0x7fc0_1234),
    f32::from_bits(0xffc0_abcd),
    f32::from_bits(0x7f80_0001),
    1.0,
    -1.0,
    f32::MAX,
    f32::MIN,
];

struct Tally(BTreeMap<&'static str, (u64, u64, Option<String>)>);

impl Tally {
    fn new() -> Self {
        Tally(BTreeMap::new())
    }
    fn note(&mut self, sym: &'static str, ok: bool, detail: impl FnOnce() -> String) {
        let e = self.0.entry(sym).or_insert((0, 0, None));
        e.0 += 1;
        if !ok {
            e.1 += 1;
            if e.2.is_none() {
                e.2 = Some(detail());
            }
        }
    }
    fn report(&self) -> u64 {
        let mut bad = 0;
        println!("\n{:<22} {:>10} {:>10}", "symbol", "cases", "mismatch");
        for (sym, (n, m, first)) in &self.0 {
            println!("{sym:<22} {n:>10} {m:>10}");
            if let Some(d) = first {
                println!("    first: {d}");
            }
            bad += m;
        }
        bad
    }
    fn f(&mut self, sym: &'static str, c: f32, r: f32, ctx: impl FnOnce() -> String) {
        let ok = c.to_bits() == r.to_bits();
        self.note(sym, ok, || {
            format!("{} -> C=0x{:08x} Rust=0x{:08x}", ctx(), c.to_bits(), r.to_bits())
        });
    }
    fn v(&mut self, sym: &'static str, c: c2v, r: c2v, ctx: impl Fn() -> String) {
        let ok = c.x.to_bits() == r.x.to_bits() && c.y.to_bits() == r.y.to_bits();
        self.note(sym, ok, || {
            format!(
                "{} -> C=(0x{:08x},0x{:08x}) Rust=(0x{:08x},0x{:08x})",
                ctx(),
                c.x.to_bits(),
                c.y.to_bits(),
                r.x.to_bits(),
                r.y.to_bits()
            )
        });
    }
}

#[test]
#[ignore = "diagnostic only: reports NaN-payload codegen differences, see level4_nonfinite.rs"]
fn nan_report() {
    let mut t = Tally::new();

    for sym in ["c2Dot", "c2Det2"] {
        let (c, r) = both::<FnFvv>(sym);
        let s: &'static str = if sym == "c2Dot" { "c2Dot" } else { "c2Det2" };
        for &ax in NANMAKERS {
            for &ay in NANMAKERS {
                for &bx in NANMAKERS {
                    for &by in NANMAKERS {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        unsafe { t.f(s, c(a, b), r(a, b), || format!("({a:?},{b:?})")) };
                    }
                }
            }
        }
    }

    let (c, r) = both::<FnFv>("c2Len");
    for &x in NANMAKERS {
        for &y in NANMAKERS {
            let a = c2v { x, y };
            unsafe { t.f("c2Len", c(a), r(a), || format!("({a:?})")) };
        }
    }

    for sym in ["c2Neg", "c2Skew", "c2CCW90", "c2Norm"] {
        let (c, r) = both::<FnVv>(sym);
        let s: &'static str = match sym {
            "c2Neg" => "c2Neg",
            "c2Skew" => "c2Skew",
            "c2CCW90" => "c2CCW90",
            _ => "c2Norm",
        };
        for &x in NANMAKERS {
            for &y in NANMAKERS {
                let a = c2v { x, y };
                unsafe { t.v(s, c(a), r(a), || format!("({a:?})")) };
            }
        }
    }

    for sym in ["c2Mulvs", "c2Div"] {
        let (c, r) = both::<FnVvf>(sym);
        let s: &'static str = if sym == "c2Mulvs" { "c2Mulvs" } else { "c2Div" };
        for &x in NANMAKERS {
            for &y in NANMAKERS {
                for &k in NANMAKERS {
                    let a = c2v { x, y };
                    unsafe { t.v(s, c(a, k), r(a, k), || format!("({a:?},{k:?})")) };
                }
            }
        }
    }

    for sym in ["c2Add", "c2Sub", "c2Maxv", "c2Minv"] {
        let (c, r) = both::<FnVvv>(sym);
        let s: &'static str = match sym {
            "c2Add" => "c2Add",
            "c2Sub" => "c2Sub",
            "c2Maxv" => "c2Maxv",
            _ => "c2Minv",
        };
        for &ax in NANMAKERS {
            for &ay in NANMAKERS {
                for &bx in NANMAKERS {
                    for &by in NANMAKERS {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        unsafe { t.v(s, c(a, b), r(a, b), || format!("({a:?},{b:?})")) };
                    }
                }
            }
        }
    }

    let (c, r) = both::<FnVvvv>("c2Clampv");
    for &ax in NANMAKERS {
        for &ay in NANMAKERS {
            for &lo in NANMAKERS {
                for &hi in NANMAKERS {
                    let a = c2v { x: ax, y: ay };
                    let l = c2v { x: lo, y: hi };
                    let h = c2v { x: hi, y: lo };
                    unsafe {
                        t.v("c2Clampv", c(a, l, h), r(a, l, h), || {
                            format!("({a:?},{l:?},{h:?})")
                        })
                    };
                }
            }
        }
    }

    for sym in ["c2Mulrv", "c2MulrvT"] {
        let (c, r) = both::<FnVrv>(sym);
        let s: &'static str = if sym == "c2Mulrv" { "c2Mulrv" } else { "c2MulrvT" };
        for &rc in NANMAKERS {
            for &rs in NANMAKERS {
                for &bx in NANMAKERS {
                    for &by in NANMAKERS {
                        let rot = c2r { c: rc, s: rs };
                        let b = c2v { x: bx, y: by };
                        unsafe { t.v(s, c(rot, b), r(rot, b), || format!("({rot:?},{b:?})")) };
                    }
                }
            }
        }
    }

    let (c, r) = both::<FnVxv>("c2Mulxv");
    for &f in NANMAKERS {
        for &g in NANMAKERS {
            for &h in NANMAKERS {
                for &k in NANMAKERS {
                    let x = c2x {
                        p: c2v { x: f, y: g },
                        r: c2r { c: h, s: k },
                    };
                    let b = c2v { x: k, y: h };
                    unsafe { t.v("c2Mulxv", c(x, b), r(x, b), || format!("({x:?},{b:?})")) };
                }
            }
        }
    }

    // Simplex-level functions.
    let mut g = Rng::new(4242);
    let pick = |g: &mut Rng| NANMAKERS[g.below(NANMAKERS.len() as u32) as usize];
    let metric = both::<FnSimplexF>("c2GJKSimplexMetric");
    let dfun = both::<FnSimplexV>("c2D");
    let lfun = both::<FnSimplexV>("c2L");
    let wit = both::<FnWitness>("c2Witness");
    let s2 = both::<FnSimplexVoid>("c22");
    let s3 = both::<FnSimplexVoid>("c23");
    for it in 0..60_000u32 {
        let mut s = c2Simplex::default();
        for v in s.verts.iter_mut() {
            v.sA = c2v { x: pick(&mut g), y: pick(&mut g) };
            v.sB = c2v { x: pick(&mut g), y: pick(&mut g) };
            v.p = c2v { x: pick(&mut g), y: pick(&mut g) };
            v.u = pick(&mut g);
            v.iA = g.below(4) as i32;
            v.iB = g.below(4) as i32;
        }
        s.div = pick(&mut g);
        s.count = (it % 5) as i32;

        {
            let (mut a, mut b) = (s, s);
            unsafe {
                t.f("c2GJKSimplexMetric", metric.0(&mut a), metric.1(&mut b), || {
                    format!("count={}", s.count)
                })
            };
        }
        {
            let (mut a, mut b) = (s, s);
            unsafe { t.v("c2D", dfun.0(&mut a), dfun.1(&mut b), || format!("count={}", s.count)) };
        }
        {
            let (mut a, mut b) = (s, s);
            unsafe { t.v("c2L", lfun.0(&mut a), lfun.1(&mut b), || format!("count={}", s.count)) };
        }
        {
            let (mut cs, mut rs) = (s, s);
            let (mut ca, mut cb) = (c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 4.0 });
            let (mut ra, mut rb) = (ca, cb);
            unsafe {
                wit.0(&mut cs, &mut ca, &mut cb);
                wit.1(&mut rs, &mut ra, &mut rb);
            }
            t.v("c2Witness.a", ca, ra, || format!("count={}", s.count));
            t.v("c2Witness.b", cb, rb, || format!("count={}", s.count));
        }
        for (name, f, need) in [
            ("c22", s2, 2i32),
            ("c23", s3, 3),
        ] {
            let mut base = s;
            base.count = need;
            let (mut cs, mut rs) = (base, base);
            unsafe {
                f.0(&mut cs);
                f.1(&mut rs);
            }
            let n = std::mem::size_of::<c2Simplex>();
            let cb = unsafe { std::slice::from_raw_parts(&cs as *const _ as *const u8, n) };
            let rb = unsafe { std::slice::from_raw_parts(&rs as *const _ as *const u8, n) };
            let ok = cb == rb;
            let off = (0..n).step_by(4).find(|&i| cb[i..i + 4] != rb[i..i + 4]);
            t.note(name, ok, move || format!("first differing word at +{off:?}"));
        }
    }

    let bad = t.report();
    println!("\ntotal strict mismatches: {bad}");
}
