//! Phase B (continued) — `CONFIGS.md` rows 33..35: the ambient process settings
//! glibc's `%a` and `%.4f` conversions actually branch on.
//!
//! Neither is reachable through `driver.h`, but a caller can change both, and
//! glibc's output changes when they do — so the translation has to follow.
//! These live in their own test binary because the state is process-global.

mod common;

use common::*;

/// Values chosen to make every rounding decision and every radix-character
/// position visible: exact ties of both parities, sub-boundary magnitudes, a
/// value with a long integer part, and both zeros.
fn probe_values(rng: &mut Rng) -> Vec<u64> {
    let mut v = vec![
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1.0f64.to_bits(),
        (-1.0f64).to_bits(),
        0.5f64.to_bits(),
        0.03125f64.to_bits(),
        (-0.03125f64).to_bits(),
        0.09375f64.to_bits(),
        (-0.09375f64).to_bits(),
        0.00012345f64.to_bits(),
        (-0.00012345f64).to_bits(),
        3.14159265358979f64.to_bits(),
        (-3.14159265358979f64).to_bits(),
        1e-300f64.to_bits(),
        (-1e-300f64).to_bits(),
        1.00005f64.to_bits(),
        (-1.00005f64).to_bits(),
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
        1u64,                  // min subnormal
        SIGN | 1,              // negative min subnormal
        MANT_MASK,             // max subnormal
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        0x7ff8_0000_0000_0000, // qNaN
        0xfff8_0000_0000_0000, // -qNaN
    ];
    // Exactly-representable ties k/32 (fifth fraction digit is 5).
    let mut k = 1u64;
    while k < 400 {
        let x = k as f64 / 32.0;
        v.push(x.to_bits());
        v.push((-x).to_bits());
        k += 2;
    }
    // Randomized coverage so a row is not decided by hand-picked values alone.
    for _ in 0..3000 {
        v.push(rng.next_u64());
    }
    for _ in 0..3000 {
        // Values in a range where the 4-digit rounding is actually exercised.
        let x = rng.unit() * 1e6;
        v.push(x.to_bits());
        v.push((-x).to_bits());
    }
    v
}

const ROUNDING_MODES: [(&str, i32); 4] = [
    ("FE_TONEAREST", FE_TONEAREST),
    ("FE_UPWARD", FE_UPWARD),
    ("FE_DOWNWARD", FE_DOWNWARD),
    ("FE_TOWARDZERO", FE_TOWARDZERO),
];

/// Locales worth trying: `C` (`.`), a comma-decimal one, and one whose decimal
/// point is a MULTI-BYTE UTF-8 sequence (`ps_AF` uses U+066B, bytes d9 ab), so
/// the radix character cannot be assumed to be a single `char`.
const LOCALES: [&str; 6] = ["C", "de_DE.utf8", "fr_FR.utf8", "ps_AF.utf8", "es_ES.utf8", "C.utf8"];

// --- row 33 -----------------------------------------------------------------

#[test]
fn cfg_33_default_ambient_state() {
    set_c_locale();
    assert!(set_round(FE_TONEAREST));
    assert_eq!(get_round(), FE_TONEAREST, "default rounding mode is FE_TONEAREST");
    let mut rng = Rng::new(SEED ^ 33);
    check_row("CONFIGS row 33 (C locale, FE_TONEAREST)", &probe_values(&mut rng));
}

// --- row 34 -----------------------------------------------------------------

#[test]
fn cfg_34_locale_decimal_point() {
    let mut tried = Vec::new();
    for name in LOCALES {
        if !try_setlocale(LC_NUMERIC, name) {
            eprintln!("CONFIGS row 34: locale {name} unavailable, skipping");
            continue;
        }
        tried.push(name);
        let mut rng = Rng::new(SEED ^ 34);
        check_row(
            &format!("CONFIGS row 34 (LC_NUMERIC={name})"),
            &probe_values(&mut rng),
        );
    }
    set_c_locale();
    assert!(
        tried.len() >= 3,
        "row 34 needs at least three locales incl. a non-'.' one; only got {tried:?}"
    );
    assert!(
        tried.iter().any(|l| *l == "de_DE.utf8" || *l == "fr_FR.utf8" || *l == "es_ES.utf8"),
        "row 34 must include a comma-decimal locale; got {tried:?}"
    );
    eprintln!("CONFIGS row 34: verified under locales {tried:?}");
}

/// Independently pins down that the comma-decimal case is really being taken —
/// otherwise row 34 could pass vacuously if `setlocale` silently had no effect.
#[test]
fn cfg_34b_comma_locale_actually_changes_output() {
    if !try_setlocale(LC_NUMERIC, "de_DE.utf8") {
        eprintln!("de_DE.utf8 unavailable; cannot pin row 34");
        return;
    }
    let line = check_one("CONFIGS row 34b", 3.14159265358979f64.to_bits());
    set_c_locale();
    assert_eq!(
        line, "400921fb54442d11 0x1,921fb54442d11p+1 3,1416",
        "the comma decimal point must appear in BOTH %a and %.4f"
    );
}

/// And that a multi-byte decimal point survives verbatim in both conversions.
#[test]
fn cfg_34c_multibyte_decimal_point() {
    if !try_setlocale(LC_NUMERIC, "ps_AF.utf8") {
        eprintln!("ps_AF.utf8 unavailable; cannot pin the multi-byte radix case");
        return;
    }
    let line = check_one("CONFIGS row 34c", 3.14159265358979f64.to_bits());
    set_c_locale();
    // U+066B ARABIC DECIMAL SEPARATOR = d9 ab.
    assert_eq!(
        line, "400921fb54442d11 0x1\u{66b}921fb54442d11p+1 3\u{66b}1416",
        "multi-byte radix bytes must be emitted verbatim"
    );
}

// --- row 35 -----------------------------------------------------------------

#[test]
fn cfg_35_rounding_modes() {
    set_c_locale();
    for (name, mode) in ROUNDING_MODES {
        assert!(set_round(mode), "fesetround({name}) failed");
        assert_eq!(get_round(), mode);
        let mut rng = Rng::new(SEED ^ 35);
        check_row(&format!("CONFIGS row 35 ({name})"), &probe_values(&mut rng));
    }
    assert!(set_round(FE_TONEAREST));
}

/// Pins that the directional modes really do change the C output, so row 35
/// cannot pass vacuously.
#[test]
fn cfg_35b_rounding_modes_actually_change_output() {
    set_c_locale();
    let cases: [(f64, [&str; 4]); 4] = [
        // value, [TONEAREST, UPWARD, DOWNWARD, TOWARDZERO]
        (0.00012345, ["0.0001", "0.0002", "0.0001", "0.0001"]),
        (-0.00012345, ["-0.0001", "-0.0001", "-0.0002", "-0.0001"]),
        (0.09375, ["0.0938", "0.0938", "0.0937", "0.0937"]),
        (-0.09375, ["-0.0938", "-0.0937", "-0.0938", "-0.0937"]),
    ];
    for (value, expected) in cases {
        for (i, (name, mode)) in ROUNDING_MODES.iter().enumerate() {
            assert!(set_round(*mode));
            let line = check_one(&format!("CONFIGS row 35b ({name}, {value})"), value.to_bits());
            let fixed = line.split(' ').nth(2).expect("fixed field");
            assert_eq!(
                fixed, expected[i],
                "glibc reference for {value} under {name} changed"
            );
        }
    }
    assert!(set_round(FE_TONEAREST));
}

/// The interaction of the two ambient axes: locale x rounding mode. Bugs hide
/// in combinations, not in either axis alone.
#[test]
fn cfg_34_35_cross_product() {
    let mut combos = 0usize;
    for name in LOCALES {
        if !try_setlocale(LC_NUMERIC, name) {
            continue;
        }
        for (mode_name, mode) in ROUNDING_MODES {
            assert!(set_round(mode));
            let mut rng = Rng::new(SEED ^ 0x3435);
            check_row(
                &format!("CONFIGS rows 34x35 (LC_NUMERIC={name}, {mode_name})"),
                &probe_values(&mut rng),
            );
            combos += 1;
        }
    }
    set_c_locale();
    assert!(set_round(FE_TONEAREST));
    assert!(combos >= 12, "expected at least 12 locale x rounding combinations, ran {combos}");
    eprintln!("CONFIGS rows 34x35: {combos} combinations verified");
}

/// Row 34, done mechanically instead of from a hand-picked list: sweep EVERY
/// locale installed on the system. This is what catches a radix byte string no
/// hand-written list would think of (empty, multi-byte, or non-UTF-8).
#[test]
fn cfg_34d_every_installed_locale() {
    let out = match std::process::Command::new("locale").arg("-a").output() {
        Ok(o) if o.status.success() => o.stdout,
        _ => {
            eprintln!("`locale -a` unavailable; skipping the exhaustive locale sweep");
            return;
        }
    };
    let names: Vec<String> = String::from_utf8_lossy(&out)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    assert!(names.len() > 10, "expected many installed locales, found {}", names.len());

    // A compact probe: enough to exercise both radix positions (%a and %.4f),
    // the no-radix cases, and the specials.
    let probe: Vec<u64> = vec![
        3.14159265358979f64.to_bits(),
        (-0.5f64).to_bits(),
        1.0f64.to_bits(),
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1u64,
        MANT_MASK,
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
        0.09375f64.to_bits(),
        (-0.09375f64).to_bits(),
        0x7ff8_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x3ff0_0000_0000_0001,
        0x3ff1_2300_0000_0000,
    ];

    let mut checked = 0usize;
    let mut skipped = 0usize;
    let mut divergent: Vec<String> = Vec::new();
    for name in &names {
        if !try_setlocale(LC_NUMERIC, name) {
            skipped += 1;
            continue;
        }
        let divs = compare_batch(&probe);
        if !divs.is_empty() {
            divergent.push(format!("{name}: {}", divs[0]));
        }
        checked += 1;
    }
    set_c_locale();

    assert!(
        divergent.is_empty(),
        "CONFIGS row 34d: {} of {checked} locales diverge (first 10):\n{}",
        divergent.len(),
        divergent.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
    );
    eprintln!(
        "CONFIGS row 34d: {checked} locales verified ({skipped} unavailable), \
         {} values each",
        probe.len()
    );
    assert!(checked > 100, "only {checked} locales were actually loadable");
}
