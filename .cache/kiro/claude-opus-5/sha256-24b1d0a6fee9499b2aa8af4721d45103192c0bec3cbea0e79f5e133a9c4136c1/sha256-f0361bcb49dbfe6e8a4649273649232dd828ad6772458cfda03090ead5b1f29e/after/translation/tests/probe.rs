mod common;
use common::pair;

/// Probe, not an assertion: prints the C's accept/reject decision so the
/// ERRORS.md table can be derived from observed behaviour rather than guesses.
#[test]
fn probe_negative_size_boundary() {
    let src = b"data\0";
    let mut runs: Vec<(i32, i32, bool)> = Vec::new();
    let mut prev: Option<bool> = None;
    for size in -64i32..0 {
        let is_null = pair().c.call_nullness(size, Some(src));
        if prev != Some(is_null) {
            runs.push((size, size, is_null));
            prev = Some(is_null);
        } else {
            runs.last_mut().unwrap().1 = size;
        }
    }
    println!("negative-size runs (start, end, returns_NULL):");
    for (a, b, n) in &runs {
        println!("  {a}..={b} -> NULL={n}");
    }
    // Also probe the wrap region.
    for size in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        i32::MIN + 3,
        -2_000_000_000,
        -1_610_612_736,
        -1_500_000_000,
        -1_073_741_824,
        -1_000_000_000,
        -600_000_000,
        -536_870_912,
        -536_870_911,
    ] {
        let c = pair().c.call_nullness(size, Some(src));
        let r = pair().rs.call_nullness(size, Some(src));
        println!("  size={size:>12} ({size:#x}) C_NULL={c} RS_NULL={r}");
        assert_eq!(c, r, "divergence at size={size}");
    }
}
