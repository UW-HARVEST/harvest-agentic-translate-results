//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error codes at all
//! (every function is `void`), so its entire rejection surface is implicit:
//!
//! * the `switch` in `colourblind` has no `default:` label, so an out-of-range
//!   `cb_impairment` is a silent no-op that never dereferences the pointers;
//! * the kernels dereference `float *` with no null check, so a null pointer
//!   with a *valid* impairment is an unconditional invalid access.
//!
//! Rows E1–E8 are checked in-process by comparing the (untouched) output
//! buffers bit-for-bit. Rows E9–E12 are checked by re-executing this test
//! binary as a child process and asserting that the C child and the Rust child
//! terminate with the *same* signal.

mod common;

use common::*;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// E1..E6 — out-of-range `cb_impairment` must be a silent no-op
// ---------------------------------------------------------------------------

/// Inputs used for the no-op rows: if either library touched the buffer we
/// would see it, whatever the value class.
fn sentinel_inputs() -> Vec<[f32; 3]> {
    let mut rng = Rng::new(0x0BAD_F00D);
    let mut v = vec![
        [1.0, 2.0, 3.0],
        [0.0, -0.0, 0.0],
        [f32::INFINITY, f32::NEG_INFINITY, f32::MAX],
        [f32::from_bits(0x7FC0_0000), f32::from_bits(0xFFC0_0000), 0.5],
        [f32::MIN_POSITIVE, -f32::MIN_POSITIVE, f32::from_bits(1)],
    ];
    v.extend((0..64).map(|_| [rng.any_bits(), rng.any_bits(), rng.any_bits()]));
    v
}

/// Assert that both libraries leave the buffer *exactly* as it was, and that
/// they agree with each other.
fn assert_noop(row: &str, impairment: i32) {
    for input in sentinel_inputs() {
        let c = c_lib().call(impairment, input);
        let r = rust_lib().call(impairment, input);

        assert!(
            bits_eq(c, r),
            "[{row}] C/Rust divergence for out-of-range Impairment={impairment} \
             input={}\n  C   : {}\n  Rust: {}",
            fmt3(input),
            fmt3(c),
            fmt3(r)
        );
        assert!(
            bits_eq(c, input),
            "[{row}] C modified the buffer for out-of-range Impairment={impairment}: \
             input={} -> {}",
            fmt3(input),
            fmt3(c)
        );
        assert!(
            bits_eq(r, input),
            "[{row}] Rust modified the buffer for out-of-range Impairment={impairment}: \
             input={} -> {}",
            fmt3(input),
            fmt3(r)
        );
    }
    eprintln!("[{row}] OK  Impairment={impairment} is a no-op in both libraries");
}

/// E1: `Impairment == 3`, one step past the last valid enumerator.
#[test]
fn err_e1_impairment_3() {
    assert_noop("E1", 3);
}

/// E2: `Impairment == 4`.
#[test]
fn err_e2_impairment_4() {
    assert_noop("E2", 4);
}

/// E3: `Impairment == -1` (the `switch` compares unsigned, so this is
/// `0xFFFFFFFF` and falls through).
#[test]
fn err_e3_impairment_neg1() {
    assert_noop("E3", -1);
}

/// E4: `Impairment == INT_MIN`.
#[test]
fn err_e4_impairment_int_min() {
    assert_noop("E4", i32::MIN);
}

/// E5: `Impairment == INT_MAX`.
#[test]
fn err_e5_impairment_int_max() {
    assert_noop("E5", i32::MAX);
}

/// E6: every other out-of-range value — exhaustive `3..=4096`, exhaustive
/// `-4096..=-1`, plus randomized 32-bit values. Uses one fixed input triple per
/// impairment value to keep the sweep quick.
#[test]
fn err_e6_impairment_out_of_range_sweep() {
    let row = "E6";
    let input = [0.25f32, -0.75f32, f32::from_bits(0x7FC0_0000)];

    let check = |imp: i32| {
        let c = c_lib().call(imp, input);
        let r = rust_lib().call(imp, input);
        assert!(
            bits_eq(c, r),
            "[{row}] divergence for Impairment={imp}\n  C   : {}\n  Rust: {}",
            fmt3(c),
            fmt3(r)
        );
        assert!(
            bits_eq(c, input),
            "[{row}] buffer modified for out-of-range Impairment={imp}: {}",
            fmt3(c)
        );
    };

    for imp in 3..=4096 {
        check(imp);
    }
    for imp in -4096..=-1 {
        check(imp);
    }
    let mut rng = Rng::new(0xFEED_BEEF);
    let mut random_checked = 0;
    for _ in 0..20_000 {
        let imp = rng.next_u32() as i32;
        if VALID_IMPAIRMENTS.contains(&imp) {
            continue; // in range; covered by Phase B
        }
        check(imp);
        random_checked += 1;
    }
    eprintln!(
        "[{row}] OK  4094 + 4096 exhaustive + {random_checked} random out-of-range values"
    );
}

// ---------------------------------------------------------------------------
// Child-process machinery for the pointer rows (E7..E12)
// ---------------------------------------------------------------------------

const ENV_LIB: &str = "CB_CHILD_LIB";
const ENV_IMP: &str = "CB_CHILD_IMP";
const ENV_PTRS: &str = "CB_CHILD_PTRS";
const CHILD_TEST: &str = "zz_child_worker";

/// The pointer configuration a child should use.
#[derive(Copy, Clone, Debug, PartialEq)]
enum Ptrs {
    /// All three pointers valid (control case — must exit 0).
    Valid,
    /// All three null.
    AllNull,
    /// Only `R` null; `G`, `B` valid.
    NullR,
    /// Only `G` null; `R`, `B` valid.
    NullG,
    /// Only `B` null; `R`, `G` valid.
    NullB,
    /// All three set to the non-null but unmapped address `0x1`.
    WildOne,
    /// All three set to `0xdeadbeef`.
    WildDead,
    /// All three set to `usize::MAX` (also maximally misaligned).
    WildMax,
}

impl Ptrs {
    fn tag(self) -> &'static str {
        match self {
            Ptrs::Valid => "valid",
            Ptrs::AllNull => "allnull",
            Ptrs::NullR => "nullr",
            Ptrs::NullG => "nullg",
            Ptrs::NullB => "nullb",
            Ptrs::WildOne => "wildone",
            Ptrs::WildDead => "wilddead",
            Ptrs::WildMax => "wildmax",
        }
    }

    fn from_tag(s: &str) -> Ptrs {
        match s {
            "valid" => Ptrs::Valid,
            "allnull" => Ptrs::AllNull,
            "nullr" => Ptrs::NullR,
            "nullg" => Ptrs::NullG,
            "nullb" => Ptrs::NullB,
            "wildone" => Ptrs::WildOne,
            "wilddead" => Ptrs::WildDead,
            "wildmax" => Ptrs::WildMax,
            other => panic!("unknown pointer tag {other:?}"),
        }
    }
}

/// How a child process finished, in a form that can be compared exactly.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(i32),
    Signalled(i32),
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Exited(c) => write!(f, "exit({c})"),
            Outcome::Signalled(s) => {
                let name = match s {
                    4 => " SIGILL",
                    6 => " SIGABRT",
                    7 => " SIGBUS",
                    8 => " SIGFPE",
                    11 => " SIGSEGV",
                    _ => "",
                };
                write!(f, "signal({s}{name})")
            }
        }
    }
}

/// Re-exec this test binary so it performs one `colourblind` call in isolation.
fn run_child(which: &str, impairment: i32, ptrs: Ptrs) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--exact", CHILD_TEST, "--ignored", "--test-threads", "1"])
        .env(ENV_LIB, which)
        .env(ENV_IMP, impairment.to_string())
        .env(ENV_PTRS, ptrs.tag())
        // Rust's own panic/UB message would only add noise.
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to spawn child test process");

    match status.code() {
        Some(code) => Outcome::Exited(code),
        None => Outcome::Signalled(status.signal().expect("child neither exited nor signalled")),
    }
}

/// The worker that actually performs the (possibly fatal) call. Only runs when
/// the harness has set `CB_CHILD_LIB`; otherwise it is an inert ignored test.
#[test]
#[ignore = "internal child-process worker; driven by the E7..E12 tests"]
fn zz_child_worker() {
    let Ok(which) = std::env::var(ENV_LIB) else {
        return; // invoked directly by a human: do nothing
    };
    let impairment: i32 = std::env::var(ENV_IMP).unwrap().parse().unwrap();
    let ptrs = Ptrs::from_tag(&std::env::var(ENV_PTRS).unwrap());

    let lib = match which.as_str() {
        "c" => c_lib(),
        "rust" => rust_lib(),
        other => panic!("unknown library {other:?}"),
    };

    let mut storage = [1.0f32, 2.0f32, 3.0f32];
    let base = storage.as_mut_ptr();
    let null = std::ptr::null_mut::<f32>();

    let (r, g, b) = unsafe {
        match ptrs {
            Ptrs::Valid => (base, base.add(1), base.add(2)),
            Ptrs::AllNull => (null, null, null),
            Ptrs::NullR => (null, base.add(1), base.add(2)),
            Ptrs::NullG => (base, null, base.add(2)),
            Ptrs::NullB => (base, base.add(1), null),
            Ptrs::WildOne => (1usize as *mut f32, 1usize as *mut f32, 1usize as *mut f32),
            Ptrs::WildDead => (
                0xdead_beefusize as *mut f32,
                0xdead_beefusize as *mut f32,
                0xdead_beefusize as *mut f32,
            ),
            Ptrs::WildMax => (
                usize::MAX as *mut f32,
                usize::MAX as *mut f32,
                usize::MAX as *mut f32,
            ),
        }
    };

    unsafe { lib.call_raw(impairment, r, g, b) };

    // Survived. Make sure the compiler cannot elide the call.
    std::hint::black_box(&storage);
    std::process::exit(0);
}

const SIGSEGV: i32 = 11;
const SIGABRT: i32 = 6;

/// Assert the C child and the Rust child finish identically, and (optionally)
/// that the shared outcome is a specific one. Used for the rows that must
/// *survive*.
fn assert_same_outcome(row: &str, impairment: i32, ptrs: Ptrs, expect: Option<Outcome>) {
    let c = run_child("c", impairment, ptrs);
    let r = run_child("rust", impairment, ptrs);
    assert_eq!(
        c, r,
        "[{row}] outcome mismatch for Impairment={impairment} ptrs={ptrs:?}: \
         C {c}, Rust {r}"
    );
    if let Some(expected) = expect {
        assert_eq!(
            c, expected,
            "[{row}] unexpected shared outcome for Impairment={impairment} ptrs={ptrs:?}: \
             got {c}, expected {expected}"
        );
    }
    eprintln!("[{row}] OK  Impairment={impairment} ptrs={ptrs:?} -> both {c}");
}

/// Assert that a null dereference kills both children the same way.
///
/// The C library always dies with `SIGSEGV` (it just executes `movss (%rax)`
/// with `%rax == 0`), and so does the Rust library **as shipped** — verified by
/// running this same suite under `--release`, where the comparison is exact
/// `SIGSEGV == SIGSEGV`.
///
/// When the crate is built with `debug_assertions` (the `dev` profile), the
/// precondition check inside `core::ptr::read_unaligned` — "requires that the
/// pointer argument is dereferenceable and non-null" — fires *before* the
/// faulting access and aborts with `SIGABRT` instead. That is a `rustc`
/// diagnostic for undefined behaviour, not a behavioural difference in the
/// library, so it is accepted only in that configuration, only for `SIGABRT`,
/// and only after confirming the C side really did segfault.
fn assert_same_fatal_outcome(row: &str, impairment: i32, ptrs: Ptrs) {
    let c = run_child("c", impairment, ptrs);
    let r = run_child("rust", impairment, ptrs);

    assert_eq!(
        c,
        Outcome::Signalled(SIGSEGV),
        "[{row}] expected the C library to segfault for Impairment={impairment} \
         ptrs={ptrs:?}, got {c}"
    );

    if c == r {
        eprintln!("[{row}] OK  Impairment={impairment} ptrs={ptrs:?} -> both {c}");
        return;
    }

    if cfg!(debug_assertions) && r == Outcome::Signalled(SIGABRT) {
        eprintln!(
            "[{row}] OK  Impairment={impairment} ptrs={ptrs:?} -> C {c}, Rust {r} \
             (dev profile: rustc's `read_unaligned` non-null precondition traps the \
             UB before the fault; exact SIGSEGV==SIGSEGV holds under --release)"
        );
        return;
    }

    panic!(
        "[{row}] fatal-outcome mismatch for Impairment={impairment} ptrs={ptrs:?}: \
         C {c}, Rust {r}"
    );
}

/// Control case: the child mechanism itself works and a valid call exits 0.
#[test]
fn err_e0_child_mechanism_control() {
    for &imp in &VALID_IMPAIRMENTS {
        assert_same_outcome("E0-control", imp, Ptrs::Valid, Some(Outcome::Exited(0)));
    }
}

/// E7: out-of-range enum **and** all three pointers null. The `switch` falls
/// through before any dereference, so this must NOT crash.
#[test]
fn err_e7_null_ptrs_with_invalid_impairment() {
    for imp in [3, 4, -1, i32::MIN, i32::MAX, 12345] {
        assert_same_outcome("E7", imp, Ptrs::AllNull, Some(Outcome::Exited(0)));
    }
}

/// E8: out-of-range enum with wild non-null pointers — again never dereferenced.
#[test]
fn err_e8_wild_ptrs_with_invalid_impairment() {
    for ptrs in [Ptrs::WildOne, Ptrs::WildDead, Ptrs::WildMax] {
        for imp in [3, -1, i32::MAX] {
            assert_same_outcome("E8", imp, ptrs, Some(Outcome::Exited(0)));
        }
    }
}

/// E9: `Impairment == cbProtanopia` (valid) with `R == NULL` ⇒ fatal.
#[test]
fn err_e9_null_r_segv_matches() {
    assert_same_fatal_outcome("E9", CB_PROTANOPIA, Ptrs::NullR);
}

/// E10: `Impairment == cbDeuteranopia` (valid) with `G == NULL` ⇒ fatal.
#[test]
fn err_e10_null_g_segv_matches() {
    assert_same_fatal_outcome("E10", CB_DEUTERANOPIA, Ptrs::NullG);
}

/// E11: `Impairment == cbTritanopia` (valid) with `B == NULL` ⇒ fatal.
#[test]
fn err_e11_null_b_segv_matches() {
    assert_same_fatal_outcome("E11", CB_TRITANOPIA, Ptrs::NullB);
}

/// E12: valid enum with all three pointers null ⇒ fatal, for every impairment.
#[test]
fn err_e12_all_null_segv_matches() {
    for &imp in &VALID_IMPAIRMENTS {
        assert_same_fatal_outcome("E12", imp, Ptrs::AllNull);
    }
}
