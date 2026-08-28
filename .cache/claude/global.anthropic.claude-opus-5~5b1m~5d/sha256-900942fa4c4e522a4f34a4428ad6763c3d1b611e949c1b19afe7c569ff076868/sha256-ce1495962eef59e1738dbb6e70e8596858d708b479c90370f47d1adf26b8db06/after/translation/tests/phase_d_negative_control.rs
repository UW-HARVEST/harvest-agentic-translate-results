//! Phase D — negative control for the differential harness (mutation testing).
//!
//! "All tests pass" only means something if the comparator can actually FAIL.
//! This test compiles deliberately-broken copies of the C source into mutant
//! `.so`s (in `$TMPDIR`, never touching `c_src/`) and asserts that the very
//! same comparison logic used by Phases B and C flags every one of them, while
//! the real Rust `.so` is flagged on none of them.

mod common;

use common::{Libs, Rng, SEED, comparable_len, effective_size};
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

unsafe extern "C" {
    fn free(p: *mut c_void);
}

type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

/// One differential input: `(size, buffer)`.
fn input_corpus() -> Vec<(i32, Vec<u8>)> {
    let mut rng = Rng::new(SEED ^ 0xD00D);
    let mut v: Vec<(i32, Vec<u8>)> = Vec::new();

    // every padding class, every encode() branch, plus the NULL-ness boundaries
    for len in 1usize..=64 {
        v.push((len as i32, rng.bytes(len)));
    }
    v.push((3, vec![0x00, 0x00, 0x00]));
    v.push((3, vec![0xFF, 0xFF, 0xFF]));
    for s in 0u8..64 {
        v.push((3, vec![s << 2, 0x00, 0x00]));
        v.push((3, vec![0x00, 0x00, s]));
    }
    for len in [1usize, 2, 3, 4, 5, 6, 7, 8, 9] {
        v.push((len as i32, rng.bytes_in(len, 0x80, 0xFF)));
    }
    // strlen mode
    for len in 0usize..=20 {
        let mut b = rng.bytes_in(len, 0x01, 0xFF);
        b.push(0);
        v.push((0, b));
    }
    // negative-size / calloc-boundary cases (NULL-ness sensitive)
    for size in [-1i32, -2, -3, -4, -5, -6, -100, i32::MIN, -(1 << 30)] {
        v.push((size, rng.bytes(8)));
    }
    v
}

/// Compare two `encode_base64` implementations over the corpus.
/// Returns the number of inputs on which they disagree.
fn count_mismatches(a: &Symbol<'_, EncodeBase64>, b: &Symbol<'_, EncodeBase64>) -> usize {
    let mut bad = 0usize;
    for (size, buf) in input_corpus() {
        let eff = effective_size(size, &buf);
        let p = buf.as_ptr() as *const c_char;
        let ap = unsafe { a(size, p) };
        let bp = unsafe { b(size, p) };

        if ap.is_null() != bp.is_null() {
            bad += 1;
        } else if !ap.is_null() {
            let len = comparable_len(eff);
            let as_ = unsafe { std::slice::from_raw_parts(ap as *const u8, len) };
            let bs = unsafe { std::slice::from_raw_parts(bp as *const u8, len) };
            if as_ != bs {
                bad += 1;
            }
        }
        unsafe {
            if !ap.is_null() {
                free(ap as *mut c_void);
            }
            if !bp.is_null() {
                free(bp as *mut c_void);
            }
        }
    }
    bad
}

fn c_compiler() -> Option<String> {
    for cc in ["cc", "gcc", "clang"] {
        if Command::new(cc)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cc.to_string());
        }
    }
    None
}

/// Build a mutant `.so` from a copy of the C source with `from` -> `to` applied.
fn build_mutant(cc: &str, dir: &Path, name: &str, from: &str, to: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let src = std::fs::read_to_string(root.join("c_src/src/lib.c")).expect("read C source");
    assert_eq!(
        src.matches(from).count(),
        1,
        "mutation pattern {from:?} must occur exactly once in lib.c"
    );
    let mutated = src.replace(from, to);
    assert_ne!(mutated, src);

    let inc = dir.join("include");
    std::fs::create_dir_all(&inc).unwrap();
    std::fs::copy(root.join("c_src/include/lib.h"), inc.join("lib.h")).unwrap();
    let cfile = dir.join(format!("{name}.c"));
    std::fs::write(&cfile, mutated).unwrap();

    let so = dir.join(format!("lib{name}.so"));
    let out = Command::new(cc)
        .args(["-shared", "-fPIC", "-O0"])
        .arg("-I")
        .arg(&inc)
        .arg("-o")
        .arg(&so)
        .arg(&cfile)
        .output()
        .expect("spawn C compiler");
    assert!(
        out.status.success(),
        "compiling mutant {name} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    so
}

#[test]
fn harness_detects_injected_bugs() {
    let libs = Libs::load();
    let c = libs.c_encode();
    let rust = libs.rust_encode();

    // Baseline: the real Rust .so must agree with the real C .so everywhere.
    let baseline = count_mismatches(&c, &rust);
    assert_eq!(
        baseline, 0,
        "Rust .so disagrees with C .so on {baseline} corpus inputs"
    );
    let corpus = input_corpus().len();
    assert!(corpus > 200, "corpus is too small: {corpus}");
    println!("baseline: 0 mismatches over {corpus} inputs");

    let Some(cc) = c_compiler() else {
        panic!("no C compiler found (cc/gcc/clang) - cannot run the negative control");
    };

    let dir = std::env::temp_dir().join(format!("driver-mutants-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    // Each mutation is a single-token change that a correct comparator MUST see.
    let mutations: [(&str, &str, &str); 6] = [
        // encode(): the >= 63 catch-all
        ("m_slash", "return '/';", "return '_';"),
        // encode(): the == 62 branch
        ("m_plus", "return '+';", "return '-';"),
        // encode(): the digit branch
        ("m_digit", "return '0' + (u - 52);", "return '0' + (u - 51);"),
        // bit plumbing
        ("m_shift", "(b2 >> 4)", "(b2 >> 3)"),
        // padding branch condition
        ("m_pad", "if (i + 1 < size) {\n            *p++ = encode(b6);", "if (i + 1 <= size) {\n            *p++ = encode(b6);"),
        // the calloc size expression -> flips the NULL/non-NULL boundary at size=-4
        ("m_alloc", "size * 4 / 3 + 4", "size * 4 / 3 + 5"),
    ];

    let mut detected = 0;
    for (name, from, to) in mutations {
        let so = build_mutant(&cc, &dir, name, from, to);
        let lib = unsafe { Library::new(&so) }.expect("load mutant .so");
        let m: Symbol<'_, EncodeBase64> =
            unsafe { lib.get(b"encode_base64\0") }.expect("mutant symbol");

        let n = count_mismatches(&c, &m);
        println!("mutant {name:8}: {n} mismatches vs C");
        assert!(
            n > 0,
            "NEGATIVE CONTROL FAILED: mutation {name} ({from:?} -> {to:?}) was \
             NOT detected by the comparator - the differential tests are vacuous"
        );

        // and the real Rust .so must disagree with the mutant too
        let n_rust = count_mismatches(&rust, &m);
        assert!(
            n_rust > 0,
            "Rust .so did not disagree with mutant {name} either"
        );
        detected += 1;
    }

    assert_eq!(detected, 6, "all six mutants must be detected");
    let _ = std::fs::remove_dir_all(&dir);
}
