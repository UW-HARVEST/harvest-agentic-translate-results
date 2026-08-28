//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH implementations through their `.so` exports and
//! compares the returned `float` **bit-for-bit** (`to_bits()`), so a difference
//! of one ULP — or a differing NaN payload — fails.

mod common;

use common::{Checker, Impl, Rgb, Rgb4, Rng, LIN_MAX, POW_MIN};
use std::collections::HashMap;

/// Randomized inputs per (A-pattern, B-pattern) row of C1..C64.
const PER_ROW: usize = 3000;

/// Build a color whose per-channel sRGB branch matches `pattern`
/// (bit2 = R, bit1 = G, bit0 = B; 1 = `pow` branch, 0 = linear branch).
fn color_for_pattern(rng: &mut Rng, pattern: u8) -> Rgb {
    let chan = |rng: &mut Rng, is_pow: bool| {
        if is_pow {
            rng.range_u8(POW_MIN, 255)
        } else {
            rng.range_u8(0, LIN_MAX)
        }
    };
    Rgb::new(
        chan(rng, pattern & 0b100 != 0),
        chan(rng, pattern & 0b010 != 0),
        chan(rng, pattern & 0b001 != 0),
    )
}

fn pattern_name(p: u8) -> String {
    let n = |b: u8| if b != 0 { "pow" } else { "lin" };
    format!("({},{},{})", n(p & 0b100), n(p & 0b010), n(p & 0b001))
}

/// Rows C1..C64 — the full cross product of the six sRGB branch decisions
/// (8 branch patterns for operand A x 8 for operand B), each driven with
/// `PER_ROW` randomized values drawn from the correct sub-range per channel.
#[test]
fn c1_c64_srgb_branch_cross_product() {
    let (c, r) = common::load_pair();
    let mut row = 0usize;
    let mut total = 0u64;
    let mut rows_gt_one = 0usize;
    let mut rows_swap_and_noswap = 0usize;

    for pa in 0u8..8 {
        for pb in 0u8..8 {
            row += 1;
            // Distinct, reproducible seed per row.
            let mut rng = Rng::new(0x5EED_1234 ^ ((row as u64) << 32));
            let mut ck = Checker::new(&c, &r);

            // Track both sides of the X7 swap branch within this row. Each
            // iteration checks the pair in BOTH argument orders, so whenever the
            // two luminances differ, one order takes the swap branch and the
            // other does not -- the row covers both sides of `High < Low`.
            let (mut saw_swap, mut saw_noswap, mut saw_tie) = (false, false, false);
            for _ in 0..PER_ROW {
                let a = color_for_pattern(&mut rng, pa);
                let b = color_for_pattern(&mut rng, pb);

                // The generated colors must really select this row's branches.
                for (v, is_pow) in [
                    (a.r, pa & 0b100 != 0), (a.g, pa & 0b010 != 0), (a.b, pa & 0b001 != 0),
                    (b.r, pb & 0b100 != 0), (b.g, pb & 0b010 != 0), (b.b, pb & 0b001 != 0),
                ] {
                    if is_pow {
                        assert!(v >= POW_MIN, "row C{row}: {v} should take the pow branch");
                    } else {
                        assert!(v <= LIN_MAX, "row C{row}: {v} should take the linear branch");
                    }
                }

                // Which side of `High < Low` an ordering lands on is decided by
                // the C implementation, our ground truth. The ratio against
                // white is monotone-decreasing in luminance.
                let ca = c.call(a, Rgb::WHITE);
                let cb = c.call(b, Rgb::WHITE);
                if ca.to_bits() == cb.to_bits() {
                    saw_tie = true; // equal luminance: `High < Low` false, no swap
                    saw_noswap = true;
                } else {
                    // (a,b) takes one side, (b,a) takes the other; both are checked.
                    saw_swap = true;
                    saw_noswap = true;
                }

                ck.check(a, b);
                ck.check(b, a);
            }
            let _ = saw_tie;
            total += ck.checked;
            if ck.saw_ratio_gt_one {
                rows_gt_one += 1;
            }
            if saw_swap && saw_noswap {
                rows_swap_and_noswap += 1;
            }
            ck.finish(&format!(
                "C{row} [A={} B={}]",
                pattern_name(pa),
                pattern_name(pb)
            ));
        }
    }

    assert_eq!(row, 64, "expected 64 branch-pattern rows");
    println!("C1..C64: {total} bit-exact comparisons across {row} rows");
    println!("  rows reaching ratio>1: {rows_gt_one}/64");
    println!("  rows exercising BOTH sides of the swap branch: {rows_swap_and_noswap}/64");
    // Because every pair is checked in both argument orders, EVERY row must have
    // exercised both sides of the swap branch.
    assert_eq!(
        rows_swap_and_noswap, 64,
        "swap branch under-covered: only {rows_swap_and_noswap}/64 rows hit both sides"
    );
    assert_eq!(rows_gt_one, 64, "every row should reach a ratio > 1");
}

/// Rows C67 / C68 — X7 forced to each side using strictly ordered luminances.
#[test]
fn c67_c68_swap_branch_both_directions() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC067_C068);
    let mut ck = Checker::new(&c, &r);
    let (mut noswap, mut swap) = (0u32, 0u32);

    for _ in 0..20_000 {
        let a = rng.rgb();
        let b = rng.rgb();
        if a == b {
            continue;
        }
        // Order them by luminance using the C library as ground truth.
        let la = c.call(a, Rgb::WHITE);
        let lb = c.call(b, Rgb::WHITE);
        if la.to_bits() == lb.to_bits() {
            continue;
        }
        // brighter first => High=LumA, no swap ; dimmer first => swap
        let (bright, dim) = if la < lb { (a, b) } else { (b, a) };
        ck.check(bright, dim); // C67: LumA > LumB -> `High < Low` false
        noswap += 1;
        ck.check(dim, bright); // C68: LumA < LumB -> swap taken
        swap += 1;
    }

    assert!(noswap > 5_000 && swap > 5_000, "insufficient coverage: {noswap}/{swap}");
    println!("C67 no-swap: {noswap} cases; C68 swap: {swap} cases");
    ck.finish("C67/C68 swap branch");
}

/// Row C69 — the `LumA == LumB` tie via identical colors: `High < Low` is false
/// and the result must be exactly 1.0 on both sides.
#[test]
fn c69_identical_colors_tie() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC069);
    let mut ck = Checker::new(&c, &r);
    let mut exact_ones = 0u32;

    // All 256 grayscales plus randomized colors.
    for v in 0u16..=255 {
        let a = Rgb::new(v as u8, v as u8, v as u8);
        ck.check(a, a);
    }
    for _ in 0..20_000 {
        let a = rng.rgb();
        let got = c.call(a, a);
        if got.to_bits() == 1.0f32.to_bits() {
            exact_ones += 1;
        }
        ck.check(a, a);
    }
    assert!(exact_ones > 19_000, "expected x/x == 1.0 for non-black: {exact_ones}");
    println!("C69: {} identical-color comparisons, {exact_ones} exactly 1.0", ck.checked);
    ck.finish("C69 identical colors");
}

/// Row C70 — the tie path with *different* colors: distinct RGB triples whose
/// `float` luminances collide. Found by search, then compared differentially.
#[test]
fn c70_equal_luminance_distinct_colors() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC070);
    let mut ck = Checker::new(&c, &r);

    // Key colors by their C-computed ratio against white (a proxy for
    // luminance), then keep collisions that are *true* ties, i.e. where the C
    // library returns exactly 1.0 for the pair.
    let mut seen: HashMap<u32, Rgb> = HashMap::with_capacity(1 << 19);
    let mut true_ties = 0u32;
    let mut candidates = 0u32;

    for _ in 0..400_000 {
        let col = rng.rgb();
        let key = c.call(col, Rgb::WHITE).to_bits();
        match seen.get(&key) {
            Some(&other) if other != col => {
                candidates += 1;
                ck.check(other, col);
                ck.check(col, other);
                if c.call(other, col).to_bits() == 1.0f32.to_bits() {
                    true_ties += 1;
                }
            }
            Some(_) => {}
            None => {
                seen.insert(key, col);
            }
        }
    }

    println!("C70: {candidates} near-tie candidate pairs, {true_ties} exact ties (ratio == 1.0)");
    assert!(candidates > 0, "search found no distinct colors with colliding luminance");
    assert!(
        true_ties > 0,
        "search found no EXACT luminance ties between distinct colors ({candidates} candidates)"
    );
    ck.finish("C70 equal-luminance distinct colors");
}

/// Rows C71 / C72 / C73 — the X8 degenerate divisor (`Low == 0`), i.e. a pure
/// black operand in either position, and both operands black.
#[test]
fn c71_c73_zero_luminance_divisor() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC071);
    let mut ck = Checker::new(&c, &r);

    for _ in 0..20_000 {
        let mut col = rng.rgb();
        if col == Rgb::BLACK {
            col = Rgb::new(1, 0, 0);
        }
        // C71: A black, B non-black -> swap -> Low = 0 -> +inf
        let v1 = c.call(Rgb::BLACK, col);
        assert!(v1.is_infinite() && v1 > 0.0, "C71 expected +inf, got {v1:?}");
        ck.check(Rgb::BLACK, col);
        // C72: A non-black, B black -> no swap -> Low = 0 -> +inf
        let v2 = c.call(col, Rgb::BLACK);
        assert!(v2.is_infinite() && v2 > 0.0, "C72 expected +inf, got {v2:?}");
        ck.check(col, Rgb::BLACK);
    }
    // C73: both black -> 0.0/0.0 -> NaN, bit pattern must match
    ck.check(Rgb::BLACK, Rgb::BLACK);
    assert!(c.call(Rgb::BLACK, Rgb::BLACK).is_nan(), "C73 expected NaN from C");
    assert!(ck.saw_non_finite, "C71..C73 should have produced non-finite results");

    println!("C71/C72/C73: {} comparisons with a zero-luminance divisor", ck.checked);
    ck.finish("C71/C72/C73 zero divisor");
}

/// Row C74 — grayscale x grayscale: all 256 x 256 = 65,536 pairs.
#[test]
fn c74_grayscale_full_cross_product() {
    let (c, r) = common::load_pair();
    let mut ck = Checker::new(&c, &r);
    for a in 0u16..=255 {
        for b in 0u16..=255 {
            ck.check(
                Rgb::new(a as u8, a as u8, a as u8),
                Rgb::new(b as u8, b as u8, b as u8),
            );
        }
    }
    assert_eq!(ck.checked, 65_536);
    println!("C74: {} grayscale pairs", ck.checked);
    ck.finish("C74 grayscale cross product");
}

/// Row C75 — single-channel colors over all intensities, isolating each of the
/// three luminance weights, paired in both operand positions.
#[test]
fn c75_single_channel_colors() {
    let (c, r) = common::load_pair();
    let mut ck = Checker::new(&c, &r);

    let mk = |chan: usize, v: u8| match chan {
        0 => Rgb::new(v, 0, 0),
        1 => Rgb::new(0, v, 0),
        _ => Rgb::new(0, 0, v),
    };

    // Every (channel, intensity) against every (channel, intensity):
    // 3*256 = 768 colors -> 768*768 = 589,824 pairs.
    let colors: Vec<Rgb> = (0..3)
        .flat_map(|ch| (0u16..=255).map(move |v| mk(ch, v as u8)))
        .collect();
    assert_eq!(colors.len(), 768);
    for &a in &colors {
        for &b in &colors {
            ck.check(a, b);
        }
    }
    println!("C75: {} single-channel pairs", ck.checked);
    ck.finish("C75 single-channel colors");
}

/// Row C76 — the branch-boundary lattice: every channel from the set straddling
/// the `> 0.04045` threshold and the domain endpoints, full pair cross product
/// (8^3 x 8^3 = 262,144).
#[test]
fn c76_branch_boundary_lattice() {
    let (c, r) = common::load_pair();
    let mut ck = Checker::new(&c, &r);

    const VALS: [u8; 8] = [0, 1, 9, 10, 11, 12, 254, 255];
    let colors: Vec<Rgb> = VALS
        .iter()
        .flat_map(|&r0| {
            VALS.iter()
                .flat_map(move |&g0| VALS.iter().map(move |&b0| Rgb::new(r0, g0, b0)))
        })
        .collect();
    assert_eq!(colors.len(), 512);

    for &a in &colors {
        for &b in &colors {
            ck.check(a, b);
        }
    }
    assert_eq!(ck.checked, 262_144);
    println!("C76: {} boundary-lattice pairs", ck.checked);
    ck.finish("C76 branch boundary lattice");
}

/// Row C77 — ABI: the 3-byte struct travels packed in one INTEGER register.
/// Call the same exported symbol through a 4-byte-struct signature so the byte
/// above the struct is garbage; neither implementation may let it change the
/// result.
#[test]
fn c77_struct_register_padding_garbage() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC077);
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0u64;

    const PADS: [u8; 5] = [0x00, 0x01, 0x7F, 0xAA, 0xFF];

    for _ in 0..20_000 {
        let a = rng.rgb();
        let b = rng.rgb();
        let baseline_c = c.call(a, b);
        let baseline_r = r.call(a, b);

        for &pa in &PADS {
            for &pb in &PADS {
                let a4 = Rgb4 { r: a.r, g: a.g, b: a.b, pad: pa };
                let b4 = Rgb4 { r: b.r, g: b.g, b: b.b, pad: pb };
                let cv = c.call_padded(a4, b4);
                let rv = r.call_padded(a4, b4);
                checked += 1;
                if cv.to_bits() != rv.to_bits()
                    || cv.to_bits() != baseline_c.to_bits()
                    || rv.to_bits() != baseline_r.to_bits()
                {
                    if failures.len() < 20 {
                        failures.push(format!(
                            "padding leak ({},{},{})/pad=0x{pa:02X} vs ({},{},{})/pad=0x{pb:02X}: \
                             C=0x{:08X} Rust=0x{:08X} baselineC=0x{:08X} baselineR=0x{:08X}",
                            a.r, a.g, a.b, b.r, b.g, b.b,
                            cv.to_bits(), rv.to_bits(),
                            baseline_c.to_bits(), baseline_r.to_bits()
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "C77: {} failures:\n{}", failures.len(), failures.join("\n"));
    println!("C77: {checked} padded-register comparisons, no leakage");
}

/// Row C78 — ABI: both structs copied out of unaligned offsets in a byte buffer.
#[test]
fn c78_unaligned_struct_source() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC078);
    let mut ck = Checker::new(&c, &r);

    let mut buf = [0u8; 32];
    for _ in 0..20_000 {
        for byte in buf.iter_mut() {
            *byte = rng.next_u8();
        }
        for off in [1usize, 3, 5, 7, 11, 13] {
            let a: Rgb = unsafe { std::ptr::read_unaligned(buf.as_ptr().add(off) as *const Rgb) };
            let b: Rgb =
                unsafe { std::ptr::read_unaligned(buf.as_ptr().add(off + 8) as *const Rgb) };
            ck.check(a, b);
        }
    }
    println!("C78: {} unaligned-source comparisons", ck.checked);
    ck.finish("C78 unaligned struct source");
}

/// Row C79 — large-scale unconstrained randomized fuzz.
#[test]
fn c79_random_fuzz_unconstrained() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC079_F0FF);
    let mut ck = Checker::new(&c, &r);

    // Kept at 1.5M here (the 5M-scale sweep is the exhaustive test file, which
    // covers strictly more ground: every one of the 2^24 colors).
    for _ in 0..1_500_000 {
        let a = rng.rgb();
        let b = rng.rgb();
        ck.check(a, b);
    }
    assert!(ck.saw_ratio_gt_one, "fuzz never produced a ratio > 1");
    println!("C79: {} randomized bit-exact comparisons", ck.checked);
    ck.finish("C79 random fuzz");
}

/// Row C80 — the argument-order invariant, verified as a differential property
/// on both implementations independently.
#[test]
fn c80_argument_order_invariant() {
    let (c, r) = common::load_pair();
    let mut rng = Rng::new(0xC080);
    let mut ck = Checker::new(&c, &r);
    let mut asym = Vec::new();

    for _ in 0..100_000 {
        let a = rng.rgb();
        let b = rng.rgb();
        ck.check(a, b);
        ck.check(b, a);

        let (cab, cba) = (c.call(a, b), c.call(b, a));
        let (rab, rba) = (r.call(a, b), r.call(b, a));
        // Whatever symmetry the C has, the Rust must have identically.
        let c_sym = cab.to_bits() == cba.to_bits();
        let r_sym = rab.to_bits() == rba.to_bits();
        if c_sym != r_sym && asym.len() < 20 {
            asym.push(format!(
                "order-symmetry differs for ({},{},{})/({},{},{}): C {} vs Rust {}",
                a.r, a.g, a.b, b.r, b.g, b.b, c_sym, r_sym
            ));
        }
    }
    assert!(asym.is_empty(), "C80 failures:\n{}", asym.join("\n"));
    println!("C80: {} comparisons, argument-order behaviour identical", ck.checked);
    ck.finish("C80 argument order");
}

/// Sanity: the harness really is calling two different shared objects.
#[test]
fn harness_loads_two_distinct_shared_objects() {
    let (c, r) = common::load_pair();
    println!("C   .so: {}", c.path.display());
    println!("Rust.so: {}", r.path.display());
    assert_ne!(c.path.canonicalize().unwrap(), r.path.canonicalize().unwrap());
    assert!(c.path.to_string_lossy().contains("c_src"));
    assert!(r.path.to_string_lossy().contains("target"));
    // And both actually compute something non-trivial.
    let v = c.call(Rgb::WHITE, Rgb::new(0x77, 0x77, 0x77));
    assert!(v.is_finite() && v > 1.0, "unexpected baseline value {v:?}");
    assert_eq!(
        v.to_bits(),
        r.call(Rgb::WHITE, Rgb::new(0x77, 0x77, 0x77)).to_bits()
    );
    let _ = Impl::call; // keep the import meaningful
}
