//! Level 4: float-heavy functions (`f9` barycentric, `f11`/`f12`/`f13` colour).

mod harness;

use harness::*;

fn hue_pool() -> Vec<f32> {
    let mut v: Vec<f32> = EDGE_F32.to_vec();
    // Every branch boundary in f11/f12 plus values just inside/outside.
    for b in [0.0f32, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0, -60.0] {
        v.push(b);
        v.push(b - 1e-5);
        v.push(b + 1e-5);
        v.push(b - 0.5);
        v.push(b + 0.5);
    }
    let mut rng = Rng::new(0x40E);
    for _ in 0..120 {
        v.push(rng.next_f32_in(400.0));
    }
    v
}

fn unit_pool() -> Vec<f32> {
    let mut v: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        0.25,
        0.75,
        1e-7,
        -1e-7,
        1.0 - f32::EPSILON,
        1.0 + f32::EPSILON,
        2.0,
        -2.0,
        f32::MIN_POSITIVE,
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
    ];
    let mut rng = Rng::new(0x1111);
    for _ in 0..80 {
        v.push(rng.next_f32_in(1.0));
    }
    for _ in 0..40 {
        v.push(rng.next_f32_bits());
    }
    v
}

#[test]
fn f9_matches() {
    let i = impls();
    let (c, r) = i.sym::<FnF9>("f9");

    let mut pts: Vec<LmVec2> = Vec::new();
    for &x in EDGE_F32 {
        pts.push(LmVec2 { x, y: 0.0 });
        pts.push(LmVec2 { x: 0.0, y: x });
        pts.push(LmVec2 { x, y: x });
    }
    let mut rng = Rng::new(0xF9);
    for _ in 0..400 {
        pts.push(LmVec2 {
            x: rng.next_f32_in(10.0),
            y: rng.next_f32_in(10.0),
        });
    }
    for _ in 0..200 {
        pts.push(LmVec2 {
            x: rng.next_f32_bits(),
            y: rng.next_f32_bits(),
        });
    }
    // Degenerate triangles (zero denominator) are important: invDenom = inf.
    pts.push(LmVec2 { x: 0.0, y: 0.0 });
    pts.push(LmVec2 { x: 1.0, y: 1.0 });
    pts.push(LmVec2 { x: 2.0, y: 2.0 });

    let n = pts.len();
    for _ in 0..400_000 {
        let p1 = pts[(rng.next_u32() as usize) % n];
        let p2 = pts[(rng.next_u32() as usize) % n];
        let p3 = pts[(rng.next_u32() as usize) % n];
        let p = pts[(rng.next_u32() as usize) % n];
        let x = unsafe { c(p1, p2, p3, p) };
        let y = unsafe { r(p1, p2, p3, p) };
        eq_vec2(
            &format!("f9({p1:?},{p2:?},{p3:?},{p:?})"),
            (x.x, x.y),
            (y.x, y.y),
        );
    }

    // Collinear / duplicated-vertex cases hit exactly.
    let degenerate = [
        (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32),
        (0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 0.5, 0.5),
        (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 3.0, 4.0),
        (0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 1.0, 1.0),
    ];
    for &(a, b, cc, d, e, f, g, h) in degenerate.iter() {
        let (p1, p2, p3, p) = (
            LmVec2 { x: a, y: b },
            LmVec2 { x: cc, y: d },
            LmVec2 { x: e, y: f },
            LmVec2 { x: g, y: h },
        );
        let x = unsafe { c(p1, p2, p3, p) };
        let y = unsafe { r(p1, p2, p3, p) };
        eq_vec2(
            &format!("f9 degenerate({p1:?},{p2:?},{p3:?},{p:?})"),
            (x.x, x.y),
            (y.x, y.y),
        );
    }
}

fn run_triple(name: &str, cases: &[[f32; 3]]) {
    let i = impls();
    let (c, r) = i.sym::<FnTriple>(name);
    for src in cases {
        let mut cd = [0f32; 3];
        let mut rd = [0f32; 3];
        unsafe { c(cd.as_mut_ptr(), src.as_ptr()) };
        unsafe { r(rd.as_mut_ptr(), src.as_ptr()) };
        for k in 0..3 {
            eq_f32(&format!("{name}({src:?})[{k}]"), cd[k], rd[k]);
        }
    }
}

fn hsl_hsv_cases(seed: u64) -> Vec<[f32; 3]> {
    let hues = hue_pool();
    let units = unit_pool();
    let mut out = Vec::new();
    // Structured sweeps: vary one axis at a time.
    for &h in hues.iter() {
        out.push([h, 0.5, 0.5]);
        out.push([h, 1.0, 0.5]);
        out.push([h, 0.0, 0.5]);
        out.push([h, 0.5, 0.0]);
        out.push([h, 0.5, 1.0]);
    }
    for &s in units.iter() {
        for &l in units.iter() {
            out.push([30.0, s, l]);
            out.push([200.0, s, l]);
        }
    }
    let mut rng = Rng::new(seed);
    for _ in 0..120_000 {
        out.push([
            hues[(rng.next_u32() as usize) % hues.len()],
            units[(rng.next_u32() as usize) % units.len()],
            units[(rng.next_u32() as usize) % units.len()],
        ]);
    }
    for _ in 0..60_000 {
        out.push([
            rng.next_f32_bits(),
            rng.next_f32_bits(),
            rng.next_f32_bits(),
        ]);
    }
    out
}

#[test]
fn f11_matches() {
    run_triple("f11", &hsl_hsv_cases(0x11));
}

#[test]
fn f12_matches() {
    run_triple("f12", &hsl_hsv_cases(0x12));
}

#[test]
fn f13_matches() {
    let units = unit_pool();
    let mut out: Vec<[f32; 3]> = Vec::new();
    // f13 (rgb -> hsv) branches on which channel is max and on delta == 0.
    for &a in units.iter() {
        for &b in units.iter() {
            out.push([a, b, 0.5]);
            out.push([a, 0.5, b]);
            out.push([0.5, a, b]);
            out.push([a, a, b]);
            out.push([a, b, b]);
            out.push([a, b, a]);
            out.push([a, a, a]);
        }
    }
    for &x in EDGE_F32 {
        out.push([x, x, x]);
        out.push([x, 0.0, 0.0]);
        out.push([0.0, x, 0.0]);
        out.push([0.0, 0.0, x]);
    }
    let mut rng = Rng::new(0x13);
    for _ in 0..120_000 {
        out.push([
            units[(rng.next_u32() as usize) % units.len()],
            units[(rng.next_u32() as usize) % units.len()],
            units[(rng.next_u32() as usize) % units.len()],
        ]);
    }
    for _ in 0..60_000 {
        out.push([
            rng.next_f32_bits(),
            rng.next_f32_bits(),
            rng.next_f32_bits(),
        ]);
    }
    run_triple("f13", &out);
}
