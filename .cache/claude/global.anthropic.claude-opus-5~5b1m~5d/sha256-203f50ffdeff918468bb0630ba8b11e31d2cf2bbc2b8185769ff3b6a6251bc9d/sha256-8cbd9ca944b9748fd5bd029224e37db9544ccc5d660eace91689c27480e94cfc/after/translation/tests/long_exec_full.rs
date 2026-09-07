//! Full end-to-end `long_exec` differential tests (CONFIGS.md rows 17-20,
//! ERRORS.md rows 1-7).
//!
//! A single `long_exec` call is `2000 * 262144 * 100 ~= 5.2e10` inner steps:
//! ~470 s in the C build, ~56 s in the Rust build. So every expensive test is
//! `#[ignore]`d, records its result under `target/long_cache/`, and the cheap
//! always-on tests at the bottom diff those recordings.
//!
//! IMPORTANT — why one `long_exec` per process:
//! `long_exec` calls glibc `srand`/`rand`, whose state is a single
//! process-global. Two `long_exec` calls running concurrently (even in
//! different `.so`s — they share the one libc) interleave their `rand()` draws
//! and produce garbage. Every recorder test therefore performs exactly ONE
//! `long_exec`, guarded by a process-wide mutex, and the C side and the Rust
//! side of a comparison are run in SEPARATE PROCESSES, communicating through
//! the on-disk recordings. Run them with `--test-threads=1`.
//!
//! Driver: `./record_long_exec.sh` (repo root) launches all recorders and then
//! the comparison.

mod common;

use common::*;
use std::sync::Mutex;

/// Serializes every `long_exec` in this process (glibc PRNG is global state).
static PRNG: Mutex<()> = Mutex::new(());

fn cache_dir() -> std::path::PathBuf {
    let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("long_cache");
    std::fs::create_dir_all(&d).expect("create cache dir");
    d
}

/// Everything one recorder writes: the final 1 MiB `array` and the bytes the
/// library actually printed to fd 1.
fn record(which: &'static str, tag: &str, seed: u32, poison: bool, junk_abi: bool) {
    let _g = PRNG.lock().unwrap();
    let imp = match which {
        "c" => Impl::c(),
        "rust" => Impl::rust(),
        _ => unreachable!(),
    };

    if poison {
        // CONFIGS 20 / ERRORS 7: dirty every element first. `long_exec` must
        // overwrite all of them from `rand()`, so the recording has to come out
        // identical to the clean run for the same seed.
        let mut rng = Rng::new(0xDEAD_BEEF_0BAD_F00D);
        let dirt: Vec<i32> = (0..ARRAY_SIZE).map(|_| rng.hard_i32()).collect();
        imp.set_array(&dirt);
    }

    let out = unsafe {
        capture_stdout(|| {
            if junk_abi {
                // ERRORS 6: the ABI parameter is 32-bit. Call through a
                // deliberately mis-shaped pointer so the high half of the seed
                // register and the extra argument registers all carry junk.
                let f: unsafe extern "C" fn(u64, u64, u64) -> i64 =
                    std::mem::transmute(imp.long_exec_addr);
                f(
                    0xDEAD_BEEF_0000_0000u64 | seed as u64,
                    u64::MAX,
                    0xA5A5_A5A5_A5A5_A5A5,
                );
            } else {
                imp.long_exec(seed);
            }
        })
    };

    let d = cache_dir();
    std::fs::write(d.join(format!("arr_{which}_{tag}.bin")), imp.array_bytes()).unwrap();
    std::fs::write(d.join(format!("out_{which}_{tag}.bin")), &out).unwrap();
    eprintln!(
        "recorded {which}/{tag}: stdout = {:?}",
        String::from_utf8_lossy(&out)
    );
}

// ---------------------------------------------------------------------------
// Recorders. Tags mirror CONFIGS.md / ERRORS.md rows.
// ---------------------------------------------------------------------------

macro_rules! pair {
    ($cname:ident, $rname:ident, $tag:literal, $seed:expr, $poison:expr, $junk:expr) => {
        #[test]
        #[ignore = "~470 s; run one per process with --ignored --test-threads=1"]
        fn $cname() {
            record("c", $tag, $seed, $poison, $junk);
        }
        #[test]
        #[ignore = "~56 s; run with --ignored --test-threads=1"]
        fn $rname() {
            record("rust", $tag, $seed, $poison, $junk);
        }
    };
}

// CONFIGS 17 / ERRORS 1
pair!(rec_c_seed_0, rec_rust_seed_0, "seed_0", 0, false, false);
// ERRORS 2
pair!(rec_c_seed_1, rec_rust_seed_1, "seed_1", 1, false, false);
// CONFIGS 18
pair!(rec_c_seed_42, rec_rust_seed_42, "seed_42", 42, false, false);
// CONFIGS 20 / ERRORS 7
pair!(
    rec_c_seed_42_poisoned,
    rec_rust_seed_42_poisoned,
    "seed_42_poisoned",
    42,
    true,
    false
);
// ERRORS 6 reference
pair!(rec_c_seed_7, rec_rust_seed_7, "seed_7", 7, false, false);
// ERRORS 6
pair!(
    rec_c_seed_7_junk_abi,
    rec_rust_seed_7_junk_abi,
    "seed_7_junk_abi",
    7,
    false,
    true
);
// ERRORS 5
pair!(
    rec_c_seed_int_max,
    rec_rust_seed_int_max,
    "seed_int_max",
    0x7FFF_FFFF,
    false,
    false
);
// CONFIGS 19 / ERRORS 4
pair!(
    rec_c_seed_0x80000000,
    rec_rust_seed_0x80000000,
    "seed_0x80000000",
    0x8000_0000,
    false,
    false
);
// CONFIGS 19 / ERRORS 3
pair!(
    rec_c_seed_uint_max,
    rec_rust_seed_uint_max,
    "seed_uint_max",
    0xFFFF_FFFF,
    false,
    false
);

// ---------------------------------------------------------------------------
// Cheap comparison tests (always run).
// ---------------------------------------------------------------------------

const TAGS: &[&str] = &[
    "seed_0",
    "seed_1",
    "seed_42",
    "seed_42_poisoned",
    "seed_7",
    "seed_7_junk_abi",
    "seed_int_max",
    "seed_0x80000000",
    "seed_uint_max",
];

fn read_if(p: &std::path::Path) -> Option<Vec<u8>> {
    if p.exists() {
        Some(std::fs::read(p).unwrap())
    } else {
        None
    }
}

/// The main end-to-end gate: for every recorded seed, the final `array` and the
/// printed stdout bytes must be identical between C and Rust.
#[test]
fn compare_recorded_long_exec_runs() {
    let d = cache_dir();
    let mut compared = Vec::new();
    let mut missing = Vec::new();

    for tag in TAGS {
        let ca = read_if(&d.join(format!("arr_c_{tag}.bin")));
        let ra = read_if(&d.join(format!("arr_rust_{tag}.bin")));
        let co = read_if(&d.join(format!("out_c_{tag}.bin")));
        let ro = read_if(&d.join(format!("out_rust_{tag}.bin")));
        match (ca, ra, co, ro) {
            (Some(ca), Some(ra), Some(co), Some(ro)) => {
                assert_eq!(ca.len(), 0x10_0000, "{tag}: C array recording size");
                assert_eq!(ra.len(), 0x10_0000, "{tag}: Rust array recording size");
                if ca != ra {
                    let cw: &[i32] =
                        unsafe { std::slice::from_raw_parts(ca.as_ptr() as *const i32, ARRAY_SIZE) };
                    let rw: &[i32] =
                        unsafe { std::slice::from_raw_parts(ra.as_ptr() as *const i32, ARRAY_SIZE) };
                    let i = (0..ARRAY_SIZE).find(|&i| cw[i] != rw[i]).unwrap();
                    let n = (0..ARRAY_SIZE).filter(|&i| cw[i] != rw[i]).count();
                    panic!(
                        "{tag}: final `array` diverged in {n}/{ARRAY_SIZE} elements; \
                         first at {i}: C = {} , Rust = {}",
                        cw[i], rw[i]
                    );
                }
                assert_eq!(
                    co,
                    ro,
                    "{tag}: stdout differs\n  C    = {:?}\n  Rust = {:?}",
                    String::from_utf8_lossy(&co),
                    String::from_utf8_lossy(&ro)
                );
                // Cross-check: the printed value IS the xor fold of the array.
                let cw: &[i32] =
                    unsafe { std::slice::from_raw_parts(ca.as_ptr() as *const i32, ARRAY_SIZE) };
                let xor = cw.iter().fold(0i32, |a, &b| a ^ b);
                assert_eq!(
                    String::from_utf8_lossy(&co).trim(),
                    xor.to_string(),
                    "{tag}: printed line must be the xor fold of the final array"
                );
                compared.push((tag.to_string(), String::from_utf8_lossy(&co).trim().to_string()));
            }
            _ => missing.push(*tag),
        }
    }

    for (tag, line) in &compared {
        eprintln!("MATCH {tag}: printed {line}");
    }
    assert!(
        missing.is_empty(),
        "no recordings for {missing:?} - run ./record_long_exec.sh first"
    );
    assert_eq!(compared.len(), TAGS.len());
}

/// ERRORS 6: a 64-bit seed argument with garbage in the high half must behave
/// exactly like its low 32 bits, in both libraries.
#[test]
fn errors06_junk_abi_equals_plain_seed_7() {
    let d = cache_dir();
    for which in ["c", "rust"] {
        let plain = read_if(&d.join(format!("arr_{which}_seed_7.bin")));
        let junk = read_if(&d.join(format!("arr_{which}_seed_7_junk_abi.bin")));
        let (plain, junk) = match (plain, junk) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                eprintln!("SKIP {which}: recordings missing - run ./record_long_exec.sh");
                continue;
            }
        };
        assert_eq!(
            plain, junk,
            "{which}: long_exec(0xDEADBEEF_00000007) must equal long_exec(7) - \
             the high 32 bits of the seed register must be ignored"
        );
        let po = std::fs::read(d.join(format!("out_{which}_seed_7.bin"))).unwrap();
        let jo = std::fs::read(d.join(format!("out_{which}_seed_7_junk_abi.bin"))).unwrap();
        assert_eq!(po, jo, "{which}: junk-ABI stdout differs from plain seed 7");
    }
}

/// CONFIGS 20 / ERRORS 7: a caller-dirtied `array` must not affect the result -
/// `long_exec` overwrites every element from `rand()`. Holds per library, and
/// the poisoned runs are already diffed C-vs-Rust above.
#[test]
fn configs20_errors07_dirty_array_does_not_affect_result() {
    let d = cache_dir();
    for which in ["c", "rust"] {
        let clean = read_if(&d.join(format!("arr_{which}_seed_42.bin")));
        let poisoned = read_if(&d.join(format!("arr_{which}_seed_42_poisoned.bin")));
        let (clean, poisoned) = match (clean, poisoned) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                eprintln!("SKIP {which}: recordings missing - run ./record_long_exec.sh");
                continue;
            }
        };
        assert_eq!(
            clean, poisoned,
            "{which}: long_exec(42) result depends on the prior contents of `array` \
             (the rand() fill must cover all {ARRAY_SIZE} elements)"
        );
    }
}

/// Distinct seeds must give distinct results - proves the tests are not all
/// passing on some degenerate constant output.
#[test]
fn recorded_results_are_seed_dependent() {
    let d = cache_dir();
    let mut seen = std::collections::BTreeMap::new();
    for tag in ["seed_0", "seed_1", "seed_42", "seed_int_max", "seed_uint_max"] {
        if let Some(o) = read_if(&d.join(format!("out_c_{tag}.bin"))) {
            seen.insert(tag, String::from_utf8_lossy(&o).trim().to_string());
        }
    }
    if seen.len() < 2 {
        eprintln!("SKIP: not enough recordings");
        return;
    }
    let vals: std::collections::BTreeSet<_> = seen.values().collect();
    assert!(
        vals.len() > 1,
        "every seed produced the same output {seen:?} - the harness is not really \
         driving the library"
    );
}
