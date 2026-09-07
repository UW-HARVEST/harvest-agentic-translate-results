//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH shared objects with `libloading` and compares the
//! results of their exported `tritanopia` symbol byte-for-byte. Inputs are
//! randomized from a fixed seed (`common::SEED`) so runs are reproducible.

mod common;

use common::{
    all_inputs, cast_class, degamma_class, denorm_pre_cast, regamma_class, sample_where,
    scan_all_where, Harness, Rgb, Rng,
};

const N: usize = 4000; // randomized inputs per row
const ATTEMPTS: usize = 4_000_000;

/// Inputs whose Axis-1 de-gamma branch word equals `want` (e.g. `"LPP"`).
fn axis1(rng: &mut Rng, want: &str) -> Vec<Rgb> {
    let pick = |rng: &mut Rng, c: char| -> u8 {
        if c == 'L' {
            rng.range(0, 10)
        } else {
            rng.range(11, 255)
        }
    };
    let w: Vec<char> = want.chars().collect();
    (0..N)
        .map(|_| Rgb::new(pick(rng, w[0]), pick(rng, w[1]), pick(rng, w[2])))
        .collect()
}

fn assert_class(inputs: &[Rgb], f: impl Fn(Rgb) -> String, want: &str, row: &str) {
    for &x in inputs {
        assert_eq!(
            f(x),
            want,
            "{row}: generated input {:?} is not in the intended configuration",
            x
        );
    }
}

// ---------------------------------------------------------------------------
// C1..C8 — Axis 1: the 2^3 de-gamma branch combinations
// ---------------------------------------------------------------------------

fn axis1_row(row: &str, want: &str) {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let inputs = axis1(&mut rng, want);
    assert_class(&inputs, degamma_class, want, row);
    h.check_row(row, inputs);
}

#[test]
fn c1_degamma_lll() {
    axis1_row("C1 (Axis1=LLL)", "LLL");
}
#[test]
fn c2_degamma_llp() {
    axis1_row("C2 (Axis1=LLP)", "LLP");
}
#[test]
fn c3_degamma_lpl() {
    axis1_row("C3 (Axis1=LPL)", "LPL");
}
#[test]
fn c4_degamma_lpp() {
    axis1_row("C4 (Axis1=LPP)", "LPP");
}
#[test]
fn c5_degamma_pll() {
    axis1_row("C5 (Axis1=PLL)", "PLL");
}
#[test]
fn c6_degamma_plp() {
    axis1_row("C6 (Axis1=PLP)", "PLP");
}
#[test]
fn c7_degamma_ppl() {
    axis1_row("C7 (Axis1=PPL)", "PPL");
}
#[test]
fn c8_degamma_ppp() {
    axis1_row("C8 (Axis1=PPP)", "PPP");
}

// ---------------------------------------------------------------------------
// C9 — Axis 1 threshold boundary sweep (the exact 10 -> 11 crossing)
// ---------------------------------------------------------------------------

#[test]
fn c9_degamma_threshold_boundary() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let pins: [u8; 6] = [0, 1, 10, 11, 254, 255];
    let mut inputs = Vec::new();
    for &p in &pins {
        for _ in 0..N / 6 {
            let (a, b) = (rng.byte(), rng.byte());
            inputs.push(Rgb::new(p, a, b));
            inputs.push(Rgb::new(a, p, b));
            inputs.push(Rgb::new(a, b, p));
        }
    }
    // and the full neighbourhood of the threshold, all-channels
    for r in 9..=12u8 {
        for g in 9..=12u8 {
            for b in 9..=12u8 {
                inputs.push(Rgb::new(r, g, b));
            }
        }
    }
    h.check_row("C9 (Axis1 boundary sweep)", inputs);
}

// ---------------------------------------------------------------------------
// C10..C13 — Axis 2: the re-gamma branch combinations
// ---------------------------------------------------------------------------

fn axis2_row(row: &str, want: &str, make: impl Fn(&mut Rng) -> Rgb) {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut inputs = Vec::new();
    let mut attempts = 0usize;
    while inputs.len() < N && attempts < ATTEMPTS {
        attempts += 1;
        let x = make(&mut rng);
        if regamma_class(x) == want {
            inputs.push(x);
        }
    }
    assert!(
        inputs.len() >= N / 4,
        "{row}: only found {} inputs with re-gamma class {want} in {attempts} attempts",
        inputs.len()
    );
    assert_class(&inputs, regamma_class, want, row);
    h.check_row(row, inputs);
}

#[test]
fn c10_regamma_lll() {
    // near-black: every post-matrix channel at/below the re-gamma threshold
    axis2_row("C10 (Axis2=lll)", "lll", |rng| {
        Rgb::new(rng.range(0, 3), rng.range(0, 1), rng.range(0, 1))
    });
}

#[test]
fn c11_regamma_l_pp() {
    // R' at/below threshold (often negative) while G',B' are well above it: B > G
    axis2_row("C11 (Axis2=lpp)", "lpp", |rng| {
        Rgb::new(rng.range(0, 8), rng.range(0, 60), rng.range(120, 255))
    });
}

#[test]
fn c12_regamma_p_ll() {
    // R' above threshold, G' and B' at/below it: G and B forced to 0
    axis2_row("C12 (Axis2=pll)", "pll", |rng| {
        Rgb::new(rng.range(1, 255), 0, 0)
    });
}

#[test]
fn c13_regamma_ppp() {
    // ordinary mid/bright colours
    axis2_row("C13 (Axis2=ppp)", "ppp", |rng| {
        Rgb::new(rng.range(40, 255), rng.range(40, 255), rng.range(40, 255))
    });
}

// ---------------------------------------------------------------------------
// C14 — Axis 2 with G' and B' taking DIFFERENT branches.
//
// Reachability is *measured*, not assumed: the whole 2^24 cube is scanned. If
// no input straddles the threshold the row is satisfied by the proof of
// unreachability (and C31 covers it regardless).
// ---------------------------------------------------------------------------

#[test]
fn c14_regamma_g_and_b_branch_differ() {
    let h = Harness::new();
    let straddlers = scan_all_where(2000, |x| {
        let cls = regamma_class(x);
        let bytes = cls.as_bytes();
        bytes[1] != bytes[2]
    });
    if straddlers.is_empty() {
        eprintln!(
            "C14: no input in the 2^24 cube makes G' and B' take different re-gamma \
             branches (their coefficients differ by ~2e-9, far below the 3.13e-3 \
             threshold's quantisation) — configuration proved UNREACHABLE"
        );
        // Still exercise the closest approach so the row is not vacuous.
        let mut best = Rgb::default();
        let mut best_gap = f64::INFINITY;
        for x in all_inputs().step_by(97) {
            let (_, g, b) = common::post_matrix(x);
            let gap = (g as f64 - common::REGAMMA_THRESHOLD)
                .abs()
                .min((b as f64 - common::REGAMMA_THRESHOLD).abs());
            if gap < best_gap {
                best_gap = gap;
                best = x;
            }
        }
        eprintln!("C14: closest approach {best:?} (gap {best_gap:e})");
        h.check_row("C14 (closest approach to the G'/B' straddle)", vec![best]);
    } else {
        h.check_row("C14 (Axis2 G'/B' branches differ)", straddlers);
    }
}

// ---------------------------------------------------------------------------
// C15..C18 — Axis 3: the float -> unsigned char conversion domain
// ---------------------------------------------------------------------------

#[test]
fn c15_cast_negative_wrap() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut inputs = sample_where(&mut rng, N, ATTEMPTS, |x| cast_class(x).starts_with('n'));
    assert!(
        inputs.len() >= N / 4,
        "C15: only {} negative-wrap inputs found",
        inputs.len()
    );
    inputs.push(Rgb::new(0, 0, 255)); // the canonical achiever
    for &x in &inputs {
        assert!(
            denorm_pre_cast(x).0 < 0.0,
            "C15: {x:?} does not drive the red channel negative"
        );
    }
    h.check_row("C15 (Axis3 R=neg, cast wraps)", inputs);
}

#[test]
fn c16_cast_overflow_wrap() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut inputs = sample_where(&mut rng, N, ATTEMPTS, |x| cast_class(x).starts_with('o'));
    assert!(
        inputs.len() >= N / 4,
        "C16: only {} overflow-wrap inputs found",
        inputs.len()
    );
    inputs.push(Rgb::new(255, 255, 0)); // the canonical achiever
    for &x in &inputs {
        assert!(
            denorm_pre_cast(x).0 >= 256.0,
            "C16: {x:?} does not drive the red channel past 255"
        );
    }
    h.check_row("C16 (Axis3 R=ovf, cast wraps)", inputs);
}

#[test]
fn c17_cast_all_in_range() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let inputs = sample_where(&mut rng, N, ATTEMPTS, |x| cast_class(x) == "iii");
    assert!(inputs.len() >= N / 2, "C17: only {} in-range inputs", inputs.len());
    h.check_row("C17 (Axis3 iii, no wrap)", inputs);
}

#[test]
fn c18_cast_domain_boundaries() {
    let h = Harness::new();
    // Extremal achievers of each boundary, found by scanning the whole cube.
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut closest_zero = (f32::INFINITY, Rgb::default());
    let mut closest_256 = (f32::INFINITY, Rgb::default());
    let mut closest_neg1 = (f32::INFINITY, Rgb::default());
    let (mut argmin, mut argmax) = (Rgb::default(), Rgb::default());
    for x in all_inputs().step_by(31) {
        let y = denorm_pre_cast(x).0;
        if y < min_y {
            min_y = y;
            argmin = x;
        }
        if y > max_y {
            max_y = y;
            argmax = x;
        }
        for (target, slot) in [
            (0.0f32, &mut closest_zero),
            (256.0f32, &mut closest_256),
            (-1.0f32, &mut closest_neg1),
        ] {
            let d = (y - target).abs();
            if d < slot.0 {
                *slot = (d, x);
            }
        }
    }
    let inputs = vec![
        argmin,
        argmax,
        closest_zero.1,
        closest_256.1,
        closest_neg1.1,
    ];
    eprintln!(
        "C18: y in [{min_y}, {max_y}]; nearest 0 -> {:?}, nearest 256 -> {:?}, nearest -1 -> {:?}",
        closest_zero.1, closest_256.1, closest_neg1.1
    );
    h.check_row("C18 (Axis3 boundary achievers)", inputs);
}

// ---------------------------------------------------------------------------
// C19, C20 — Axis1 x Axis3 crosses
// ---------------------------------------------------------------------------

#[test]
fn c19_lll_that_still_wraps() {
    let h = Harness::new();
    let inputs = scan_all_where(N, |x| {
        degamma_class(x) == "LLL" && cast_class(x) != "iii"
    });
    assert!(!inputs.is_empty(), "C19: no LLL input wraps the cast");
    h.check_row("C19 (Axis1=LLL and cast wraps)", inputs);
}

#[test]
fn c20_ppp_that_still_wraps() {
    let h = Harness::new();
    let inputs = scan_all_where(N, |x| {
        degamma_class(x) == "PPP" && cast_class(x) != "iii"
    });
    assert!(!inputs.is_empty(), "C20: no PPP input wraps the cast");
    h.check_row("C20 (Axis1=PPP and cast wraps)", inputs);
}

// ---------------------------------------------------------------------------
// C21..C26 — distinct input shapes
// ---------------------------------------------------------------------------

#[test]
fn c21_grays() {
    let h = Harness::new();
    h.check_row("C21 (R==G==B, all 256)", (0..=255u8).map(|v| Rgb::new(v, v, v)));
}

#[test]
fn c22_single_channel_hot() {
    let h = Harness::new();
    let mut inputs = Vec::new();
    for v in 0..=255u8 {
        inputs.push(Rgb::new(v, 0, 0));
        inputs.push(Rgb::new(0, v, 0));
        inputs.push(Rgb::new(0, 0, v));
    }
    h.check_row("C22 (single channel hot)", inputs);
}

#[test]
fn c23_single_channel_cold() {
    let h = Harness::new();
    let mut inputs = Vec::new();
    for v in 0..=255u8 {
        inputs.push(Rgb::new(v, 255, 255));
        inputs.push(Rgb::new(255, v, 255));
        inputs.push(Rgb::new(255, 255, v));
    }
    h.check_row("C23 (single channel swept, others 255)", inputs);
}

#[test]
fn c24_cube_corners() {
    let h = Harness::new();
    let mut inputs = Vec::new();
    for r in [0u8, 255] {
        for g in [0u8, 255] {
            for b in [0u8, 255] {
                inputs.push(Rgb::new(r, g, b));
            }
        }
    }
    assert_eq!(inputs.len(), 8);
    h.check_row("C24 (8 cube corners)", inputs);
}

#[test]
fn c25_g_equals_b_cancellation() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut inputs = Vec::new();
    for v in 0..=255u8 {
        inputs.push(Rgb::new(rng.byte(), v, v));
        inputs.push(Rgb::new(v, v, v));
    }
    for _ in 0..N {
        let v = rng.byte();
        inputs.push(Rgb::new(rng.byte(), v, v));
    }
    h.check_row("C25 (G==B, 0.1274 terms cancel)", inputs);
}

#[test]
fn c26_g_and_b_differ_by_one() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut inputs = Vec::new();
    for v in 0..=254u8 {
        inputs.push(Rgb::new(rng.byte(), v, v + 1));
        inputs.push(Rgb::new(rng.byte(), v + 1, v));
    }
    h.check_row("C26 (|G-B| == 1)", inputs);
}

// ---------------------------------------------------------------------------
// C27, C28 — Axis 4: argument / return register padding
// ---------------------------------------------------------------------------

#[test]
fn c27_argument_padding_ignored() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut n = 0usize;
    for _ in 0..N {
        let x = rng.rgb();
        let base = x.r as u32 | (x.g as u32) << 8 | (x.b as u32) << 16;
        let expect_c = h.c_raw(base) & 0x00FF_FFFF;
        for pad in [0x00u32, 0xFFu32, rng.byte() as u32] {
            let word = base | (pad << 24);
            let c = h.c_raw(word) & 0x00FF_FFFF;
            let r = h.rust_raw(word) & 0x00FF_FFFF;
            assert_eq!(
                c, expect_c,
                "C27: C result changed when the argument padding byte changed to {pad:#04x} \
                 for {x:?} — the harness assumption is wrong, not the translation"
            );
            assert_eq!(
                c, r,
                "C27 DIVERGENCE for {x:?} with padding {pad:#04x}: C {c:#08x} vs Rust {r:#08x}"
            );
            n += 1;
        }
    }
    eprintln!("C27 (argument padding ignored): {n} calls matched");
}

#[test]
fn c28_return_padding_not_compared() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let x = rng.rgb();
        let word = x.r as u32 | (x.g as u32) << 8 | (x.b as u32) << 16;
        let c = h.c_raw(word);
        let r = h.rust_raw(word);
        // Only the low three bytes are ABI-significant for a 3-byte struct.
        assert_eq!(
            c & 0x00FF_FFFF,
            r & 0x00FF_FFFF,
            "C28 DIVERGENCE for {x:?}: C {c:#010x} vs Rust {r:#010x} (low 3 bytes)"
        );
        // Cross-check against the struct-typed view.
        let s = h.c(x);
        assert_eq!(
            c & 0x00FF_FFFF,
            s.r as u32 | (s.g as u32) << 8 | (s.b as u32) << 16,
            "C28: raw and struct views of the C result disagree for {x:?}"
        );
        h.check(x);
    }
    eprintln!("C28 (return padding): {N} inputs matched on the 3 significant bytes");
}

// ---------------------------------------------------------------------------
// C29 — repeated invocation / no hidden state
// ---------------------------------------------------------------------------

#[test]
fn c29_iterated_feedback() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    let mut n = 0usize;
    for _ in 0..N {
        let mut cx = rng.rgb();
        let mut rx = cx;
        for _round in 0..8 {
            cx = h.c(cx);
            rx = h.rust(rx);
            assert_eq!(cx, rx, "C29 DIVERGENCE after feedback round {_round}");
            n += 1;
        }
    }
    eprintln!("C29 (8-round feedback): {n} calls matched");
}

// ---------------------------------------------------------------------------
// C30 — uniform random over the whole cube
// ---------------------------------------------------------------------------

#[test]
fn c30_uniform_random() {
    let h = Harness::new();
    let mut rng = Rng::seeded();
    h.check_row("C30 (200k uniform random)", (0..200_000).map(|_| rng.rgb()));
}
