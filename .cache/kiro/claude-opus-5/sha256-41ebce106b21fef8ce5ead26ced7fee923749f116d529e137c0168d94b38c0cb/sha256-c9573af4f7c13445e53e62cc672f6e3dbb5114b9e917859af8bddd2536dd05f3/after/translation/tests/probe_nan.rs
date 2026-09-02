//! Diagnostic probe (not part of the verification suite): feeds NaNs with
//! DISTINCT payloads into the C `.so` so the winning operand of every
//! arithmetic site can be identified by name instead of guessed from the
//! Intel SDM. Run with `cargo test --test probe_nan --release -- --nocapture`.

#![allow(non_snake_case)]

mod common;
use common::*;

/// NaN whose payload encodes an operand id, so the output names its source.
fn tag_nan(id: u32) -> f32 {
    f32::from_bits(0x7fc0_0000 | id)
}

fn name(x: f32) -> String {
    let b = x.to_bits();
    if x.is_nan() {
        let id = b & 0x003f_ffff;
        format!("NaN[id={}{}]", id, if b >> 31 == 1 { ",neg" } else { "" })
    } else {
        format!("{x:e}")
    }
}

#[test]
fn probe() {
    let p = load_pair();
    let n1 = tag_nan(1);
    let n2 = tag_nan(2);
    let n3 = tag_nan(3);
    let n4 = tag_nan(4);
    unsafe {
        let a = c2v { x: n1, y: n2 };
        let b = c2v { x: n3, y: n4 };
        eprintln!("a = (id1, id2), b = (id3, id4)");
        eprintln!("c2Dot   C={}  RS={}", name((p.c.c2Dot)(a, b)), name((p.rs.c2Dot)(a, b)));
        let (c, r) = ((p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
        eprintln!("c2Add   C=({}, {})  RS=({}, {})", name(c.x), name(c.y), name(r.x), name(r.y));
        let (c, r) = ((p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
        eprintln!("c2Sub   C=({}, {})  RS=({}, {})", name(c.x), name(c.y), name(r.x), name(r.y));
        let (c, r) = ((p.c.c2Mulvs)(a, n3), (p.rs.c2Mulvs)(a, n3));
        eprintln!("c2Mulvs C=({}, {})  RS=({}, {})", name(c.x), name(c.y), name(r.x), name(r.y));
        let (c, r) = ((p.c.c2Div)(a, n3), (p.rs.c2Div)(a, n3));
        eprintln!("c2Div   C=({}, {})  RS=({}, {})", name(c.x), name(c.y), name(r.x), name(r.y));

        // c2MulmvT: M = ((id1,id2),(id3,id4)), b = (id5,id6)
        let m = c2m {
            x: c2v { x: n1, y: n2 },
            y: c2v { x: n3, y: n4 },
        };
        let bb = c2v { x: tag_nan(5), y: tag_nan(6) };
        let (c, r) = ((p.c.c2MulmvT)(m, bb), (p.rs.c2MulmvT)(m, bb));
        eprintln!("c2MulmvT M=((1,2),(3,4)) b=(5,6)  C=({}, {})  RS=({}, {})",
            name(c.x), name(c.y), name(r.x), name(r.y));

        // rotations: r = (c=id1, s=id2), b = (id3, id4)
        let rr = c2r { c: n1, s: n2 };
        let (c, r) = ((p.c.c2Mulrv)(rr, b), (p.rs.c2Mulrv)(rr, b));
        eprintln!("c2Mulrv  r=(c1,s2) b=(3,4)  C=({}, {})  RS=({}, {})",
            name(c.x), name(c.y), name(r.x), name(r.y));
        let (c, r) = ((p.c.c2MulrvT)(rr, b), (p.rs.c2MulrvT)(rr, b));
        eprintln!("c2MulrvT r=(c1,s2) b=(3,4)  C=({}, {})  RS=({}, {})",
            name(c.x), name(c.y), name(r.x), name(r.y));

        // Isolate the adds: make exactly one product NaN at a time.
        eprintln!("--- single-NaN sanity (only one operand NaN) ---");
        let a1 = c2v { x: n1, y: 2.0 };
        let b1 = c2v { x: 3.0, y: 5.0 };
        eprintln!("c2Dot(a=(N1,2), b=(3,5)) C={} RS={}",
            name((p.c.c2Dot)(a1, b1)), name((p.rs.c2Dot)(a1, b1)));
        let a2 = c2v { x: 2.0, y: n2 };
        eprintln!("c2Dot(a=(2,N2), b=(3,5)) C={} RS={}",
            name((p.c.c2Dot)(a2, b2_helper()), ), name((p.rs.c2Dot)(a2, b2_helper())));
    }
}

fn b2_helper() -> c2v {
    c2v { x: 3.0, y: 5.0 }
}
