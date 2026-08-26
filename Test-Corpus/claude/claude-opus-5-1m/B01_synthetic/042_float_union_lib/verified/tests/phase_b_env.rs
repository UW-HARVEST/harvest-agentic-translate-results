// Phase B (continued) — environment axes that glibc's `printf` branches on.
//
// CONFIGS.md rows C26 and C27.
//
// `driver` takes no options, but the two conversions it delegates to `printf`
// (`%a` and `%.4f`) are sensitive to process state a caller can change:
//
//   * the FPU rounding direction, which glibc honours when converting a double
//     to decimal digits — verified empirically: the C library prints `0.0001`
//     for 5e-5 under FE_TONEAREST but `0.0000` under FE_DOWNWARD;
//   * the LC_NUMERIC locale, which supplies the radix character.
//
// These rows matter because a translation that formatted the number in Rust
// instead of delegating to `printf` would agree with C under the default
// environment and silently diverge under a non-default one.

mod common;

use common::{assert_same, Rng, SEED};
use std::ffi::{c_char, c_int};

extern "C" {
    fn fesetround(mode: c_int) -> c_int;
    fn fegetround() -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

// glibc / x86-64 values.
const FE_TONEAREST: c_int = 0x000;
const FE_DOWNWARD: c_int = 0x400;
const FE_UPWARD: c_int = 0x800;
const FE_TOWARDZERO: c_int = 0xc00;
const LC_ALL: c_int = 6;

/// Restores the FPU rounding direction even if the test body panics.
struct RoundingGuard(c_int);
impl RoundingGuard {
    fn set(mode: c_int) -> Self {
        let prev = unsafe { fegetround() };
        assert_eq!(unsafe { fesetround(mode) }, 0, "fesetround({mode:#x}) failed");
        RoundingGuard(prev)
    }
}
impl Drop for RoundingGuard {
    fn drop(&mut self) {
        unsafe { fesetround(self.0) };
    }
}

/// Restores the locale even if the test body panics.
struct LocaleGuard(std::ffi::CString);
impl LocaleGuard {
    /// Returns `None` when the requested locale is not installed.
    fn set(name: &str) -> Option<Self> {
        unsafe {
            // Query the current setting first so it can be restored verbatim.
            let cur = setlocale(LC_ALL, std::ptr::null());
            assert!(!cur.is_null(), "setlocale query failed");
            let saved = std::ffi::CStr::from_ptr(cur).to_owned();

            let want = std::ffi::CString::new(name).unwrap();
            if setlocale(LC_ALL, want.as_ptr()).is_null() {
                return None;
            }
            Some(LocaleGuard(saved))
        }
    }
}
impl Drop for LocaleGuard {
    fn drop(&mut self) {
        unsafe { setlocale(LC_ALL, self.0.as_ptr()) };
    }
}

/// Inputs that make the decimal conversion rounding-sensitive, plus a
/// randomized bulk set.
fn rounding_sensitive_inputs(seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut inputs = vec![
        0.00005f64,
        -0.00005,
        0.00015,
        -0.00015,
        0.99995,
        -0.99995,
        0.1,
        -0.1,
        1.0 / 3.0,
        2.0 / 3.0,
        0.00004999999,
        1e-300,
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    for _ in 0..1500 {
        inputs.push(rng.next_signed_unit());
    }
    for _ in 0..500 {
        inputs.push(rng.next_bit_pattern());
    }
    for k in -30..=30 {
        inputs.push(rng.next_signed_unit() * 10f64.powi(k));
    }
    inputs
}

// ---------------------------------------------------------------------------
// C26 — every FPU rounding direction
// ---------------------------------------------------------------------------

fn check_rounding_mode(name: &str, mode: c_int) {
    let _guard = RoundingGuard::set(mode);
    assert_eq!(
        unsafe { fegetround() },
        mode,
        "rounding mode {name} did not take effect"
    );
    let inputs = rounding_sensitive_inputs(SEED ^ (mode as u64) ^ 0xC26);
    assert_same(&format!("C26 rounding={name}"), &inputs);
}

#[test]
fn config_c26_rounding_to_nearest() {
    check_rounding_mode("FE_TONEAREST", FE_TONEAREST);
}

#[test]
fn config_c26_rounding_downward() {
    check_rounding_mode("FE_DOWNWARD", FE_DOWNWARD);
}

#[test]
fn config_c26_rounding_upward() {
    check_rounding_mode("FE_UPWARD", FE_UPWARD);
}

#[test]
fn config_c26_rounding_toward_zero() {
    check_rounding_mode("FE_TOWARDZERO", FE_TOWARDZERO);
}

/// Guard for the row itself: prove the rounding direction really does change
/// what the C library prints, otherwise the four tests above are vacuous.
#[test]
fn config_c26_rounding_actually_changes_c_output() {
    let c = common::libs().c;
    let nearest = {
        let _g = RoundingGuard::set(FE_TONEAREST);
        common::capture(c, &[0.00005])
    };
    let downward = {
        let _g = RoundingGuard::set(FE_DOWNWARD);
        common::capture(c, &[0.00005])
    };
    assert_ne!(
        nearest, downward,
        "the rounding direction no longer affects the C output, so CONFIGS.md \
         row C26 is no longer meaningful"
    );
    assert_eq!(nearest, b"3f0a36e2eb1c432d 0x1.a36e2eb1c432dp-15 0.0001\n");
    assert_eq!(downward, b"3f0a36e2eb1c432d 0x1.a36e2eb1c432dp-15 0.0000\n");
}

// ---------------------------------------------------------------------------
// C27 — LC_NUMERIC radix character
// ---------------------------------------------------------------------------

#[test]
fn config_c27_locale_with_comma_radix() {
    // de_DE uses ',' as the decimal separator. Try a few spellings; skip the
    // row if none of them is installed on this machine.
    let mut guard = None;
    for name in ["de_DE.utf8", "de_DE.UTF-8", "de_DE", "fr_FR.utf8", "ru_RU.utf8"] {
        if let Some(g) = LocaleGuard::set(name) {
            eprintln!("C27: using locale {name}");
            guard = Some(g);
            break;
        }
    }
    let Some(_guard) = guard else {
        eprintln!("C27: no comma-radix locale installed; skipping");
        return;
    };

    let inputs = rounding_sensitive_inputs(SEED ^ 0xC27);
    assert_same("C27 comma-radix locale", &inputs);
}

#[test]
fn config_c27_locale_actually_changes_c_output() {
    let c = common::libs().c;
    let default = common::capture(c, &[1.5]);
    assert_eq!(default, b"3ff8000000000000 0x1.8p+0 1.5000\n");

    let mut guard = None;
    for name in ["de_DE.utf8", "de_DE.UTF-8", "de_DE", "fr_FR.utf8", "ru_RU.utf8"] {
        if let Some(g) = LocaleGuard::set(name) {
            guard = Some(g);
            break;
        }
    }
    let Some(_guard) = guard else {
        eprintln!("C27 guard: no comma-radix locale installed; skipping");
        return;
    };

    let localized = common::capture(c, &[1.5]);
    assert_ne!(
        default, localized,
        "LC_NUMERIC no longer affects the C output, so CONFIGS.md row C27 is no \
         longer meaningful"
    );
    // Both conversions pick up the locale radix character.
    assert!(
        localized.contains(&b','),
        "expected a comma radix in {:?}",
        String::from_utf8_lossy(&localized)
    );
    // And the Rust translation must agree byte-for-byte.
    assert_same("C27 localized 1.5", &[1.5]);
}

// ---------------------------------------------------------------------------
// C26 x C27 — both axes at once
// ---------------------------------------------------------------------------

#[test]
fn config_c26_c27_rounding_and_locale_combined() {
    let mut guard = None;
    for name in ["de_DE.utf8", "de_DE.UTF-8", "de_DE", "fr_FR.utf8", "ru_RU.utf8"] {
        if let Some(g) = LocaleGuard::set(name) {
            guard = Some(g);
            break;
        }
    }
    let Some(_locale) = guard else {
        eprintln!("C26xC27: no comma-radix locale installed; skipping");
        return;
    };

    for (name, mode) in [
        ("FE_TONEAREST", FE_TONEAREST),
        ("FE_DOWNWARD", FE_DOWNWARD),
        ("FE_UPWARD", FE_UPWARD),
        ("FE_TOWARDZERO", FE_TOWARDZERO),
    ] {
        let _rounding = RoundingGuard::set(mode);
        let inputs = rounding_sensitive_inputs(SEED ^ (mode as u64) ^ 0xC2627);
        assert_same(&format!("C26xC27 locale+rounding={name}"), &inputs);
    }
}
