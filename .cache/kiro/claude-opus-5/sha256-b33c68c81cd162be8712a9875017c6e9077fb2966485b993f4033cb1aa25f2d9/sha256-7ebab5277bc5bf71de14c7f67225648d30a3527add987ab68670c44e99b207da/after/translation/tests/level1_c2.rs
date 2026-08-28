//! Level 1: leaf vector helpers from the cute_c2 subset.

mod harness;

use harness::*;

fn v_cases() -> Vec<C2v> {
    let mut out = Vec::new();
    for &x in EDGE_F32 {
        for &y in [0.0f32, 1.0, -1.0, f32::NAN, f32::INFINITY].iter() {
            out.push(C2v { x, y });
        }
    }
    for &y in EDGE_F32 {
        out.push(C2v { x: 0.5, y });
    }
    let mut rng = Rng::new(0xC2_0001);
    for _ in 0..400 {
        out.push(C2v {
            x: rng.next_f32_in(1000.0),
            y: rng.next_f32_in(1000.0),
        });
    }
    for _ in 0..400 {
        out.push(C2v {
            x: rng.next_f32_bits(),
            y: rng.next_f32_bits(),
        });
    }
    out
}

#[test]
fn c2v_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnC2V>("c2V");
    for &v in v_cases().iter() {
        let a = unsafe { c(v.x, v.y) };
        let b = unsafe { r(v.x, v.y) };
        eq_vec2(&format!("c2V({:?},{:?})", v.x, v.y), (a.x, a.y), (b.x, b.y));
    }
}

fn pairwise<F: Fn(C2v, C2v)>(f: F) {
    let cases = v_cases();
    let mut rng = Rng::new(0xC2_0002);
    // Exhaustive over the structured prefix, random pairing for the rest.
    let structured = cases.len().min(200);
    for i in 0..structured {
        for j in 0..structured {
            f(cases[i], cases[j]);
        }
    }
    for _ in 0..400_000 {
        let a = cases[(rng.next_u32() as usize) % cases.len()];
        let b = cases[(rng.next_u32() as usize) % cases.len()];
        f(a, b);
    }
}

#[test]
fn c2maxv_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnV2V>("c2Maxv");
    pairwise(|a, b| {
        let x = unsafe { c(a, b) };
        let y = unsafe { r(a, b) };
        eq_vec2(&format!("c2Maxv({a:?},{b:?})"), (x.x, x.y), (y.x, y.y));
    });
}

#[test]
fn c2minv_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnV2V>("c2Minv");
    pairwise(|a, b| {
        let x = unsafe { c(a, b) };
        let y = unsafe { r(a, b) };
        eq_vec2(&format!("c2Minv({a:?},{b:?})"), (x.x, x.y), (y.x, y.y));
    });
}

#[test]
fn c2sub_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnV2V>("c2Sub");
    pairwise(|a, b| {
        let x = unsafe { c(a, b) };
        let y = unsafe { r(a, b) };
        eq_vec2(&format!("c2Sub({a:?},{b:?})"), (x.x, x.y), (y.x, y.y));
    });
}

#[test]
fn c2dot_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnDot>("c2Dot");
    pairwise(|a, b| {
        let x = unsafe { c(a, b) };
        let y = unsafe { r(a, b) };
        eq_f32(&format!("c2Dot({a:?},{b:?})"), x, y);
    });
}

#[test]
fn c2clampv_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnClampv>("c2Clampv");
    let cases = v_cases();
    let mut rng = Rng::new(0xC2_0003);
    let structured = cases.len().min(40);
    for a in 0..structured {
        for lo in 0..structured {
            for hi in 0..structured {
                let (a, lo, hi) = (cases[a], cases[lo], cases[hi]);
                let x = unsafe { c(a, lo, hi) };
                let y = unsafe { r(a, lo, hi) };
                eq_vec2(
                    &format!("c2Clampv({a:?},{lo:?},{hi:?})"),
                    (x.x, x.y),
                    (y.x, y.y),
                );
            }
        }
    }
    for _ in 0..400_000 {
        let a = cases[(rng.next_u32() as usize) % cases.len()];
        let lo = cases[(rng.next_u32() as usize) % cases.len()];
        let hi = cases[(rng.next_u32() as usize) % cases.len()];
        let x = unsafe { c(a, lo, hi) };
        let y = unsafe { r(a, lo, hi) };
        eq_vec2(
            &format!("c2Clampv({a:?},{lo:?},{hi:?})"),
            (x.x, x.y),
            (y.x, y.y),
        );
    }
}
