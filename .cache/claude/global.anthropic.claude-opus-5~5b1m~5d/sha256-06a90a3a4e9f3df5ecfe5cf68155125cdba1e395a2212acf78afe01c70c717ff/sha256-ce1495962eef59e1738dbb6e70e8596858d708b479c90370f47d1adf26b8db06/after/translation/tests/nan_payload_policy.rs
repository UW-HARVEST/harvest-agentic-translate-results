//! Justification for the ONE tolerance in the differential harness.
//!
//! When two NaN operands reach a single `mulss`/`addss`, IEEE-754 leaves the
//! *payload* of the result unspecified; x86 propagates the payload of whichever
//! operand the compiler placed in the destination register.  That is a property
//! of the compiler's instruction selection, not of the source program, so the C
//! library does not even agree with itself across optimisation levels.
//!
//! This test measures that, using the very same corpus for
//!   * C `-O0` (the reference build) vs C `-O2`, and
//!   * C `-O0` vs the Rust `cdylib`,
//! and asserts that the Rust translation is at LEAST as close to the reference
//! as a second, perfectly legitimate build of the C code itself.  Everything
//! that is *not* a NaN payload must be bit-identical.
//!
//! Build the second C reference with:
//! ```text
//! cmake -S ../c_src -B target/cref-O2 -DCMAKE_BUILD_TYPE=Release \
//!       -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build target/cref-O2
//! ```

mod common;
use common::*;
use std::path::PathBuf;

fn find_so(dir: PathBuf) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Result of one library-vs-library comparison over the NaN corpus.
struct Score {
    compared: usize,
    hard: usize,
    payload: usize,
}

fn score(base: &Api, other: &Api) -> Score {
    let mut s = Score { compared: 0, hard: 0, payload: 0 };
    let mut acc = |a: f32, b: f32| {
        s.compared += 1;
        if a.to_bits() == b.to_bits() {
            return;
        }
        if a.is_nan() && b.is_nan() {
            s.payload += 1;
        } else {
            s.hard += 1;
        }
    };
    // every pair of specials, so every "two NaN operands" combination appears
    for x in SPECIALS {
        for y in SPECIALS {
            let (va, vb) = (v(x, y), v(y, x));
            unsafe {
                acc((base.c2Dot)(va, vb), (other.c2Dot)(va, vb));
                acc((base.c2Len)(va), (other.c2Len)(va));
                let (a1, b1) = ((base.c2Add)(va, vb), (other.c2Add)(va, vb));
                acc(a1.x, b1.x);
                acc(a1.y, b1.y);
                let (a2, b2) = ((base.c2Sub)(va, vb), (other.c2Sub)(va, vb));
                acc(a2.x, b2.x);
                acc(a2.y, b2.y);
                let (a3, b3) = ((base.c2Mulvs)(va, y), (other.c2Mulvs)(va, y));
                acc(a3.x, b3.x);
                acc(a3.y, b3.y);
                let (a4, b4) = ((base.c2Div)(va, y), (other.c2Div)(va, y));
                acc(a4.x, b4.x);
                acc(a4.y, b4.y);
                let (a5, b5) = ((base.c2Norm)(va), (other.c2Norm)(va));
                acc(a5.x, b5.x);
                acc(a5.y, b5.y);
                let m = C2m { x: va, y: vb };
                let (a6, b6) = ((base.c2MulmvT)(m, va), (other.c2MulmvT)(m, va));
                acc(a6.x, b6.x);
                acc(a6.y, b6.y);
                let (a7, b7) = ((base.c2Minv)(va, vb), (other.c2Minv)(va, vb));
                acc(a7.x, b7.x);
                acc(a7.y, b7.y);
                let (a8, b8) = ((base.c2Maxv)(va, vb), (other.c2Maxv)(va, vb));
                acc(a8.x, b8.x);
                acc(a8.y, b8.y);
                let (a9, b9) = ((base.c2Absv)(va), (other.c2Absv)(va));
                acc(a9.x, b9.x);
                acc(a9.y, b9.y);
            }
        }
    }
    // plus randomized NaN-heavy vectors
    let mut rng = Rng::new(0x4E_41_4E_50);
    for _ in 0..20_000 {
        let va = v(rng.wild(), rng.wild());
        let vb = v(rng.wild(), rng.wild());
        unsafe {
            acc((base.c2Dot)(va, vb), (other.c2Dot)(va, vb));
            acc((base.c2Len)(va), (other.c2Len)(va));
            let (a, b) = ((base.c2Add)(va, vb), (other.c2Add)(va, vb));
            acc(a.x, b.x);
            acc(a.y, b.y);
            let (a, b) = ((base.c2Norm)(va), (other.c2Norm)(va));
            acc(a.x, b.x);
            acc(a.y, b.y);
        }
    }
    s
}

#[test]
fn nan_payload_is_compiler_defined_not_behaviour() {
    let p = apis();
    let alt = find_so(manifest_dir().join("target/cref-O2"));
    let rust = score(&p.c, &p.r);
    eprintln!(
        "[nan_policy] {} vs rust : {} comparisons, {} hard mismatches, {} NaN-payload diffs",
        p.c.path.file_name().unwrap().to_string_lossy(),
        rust.compared,
        rust.hard,
        rust.payload
    );
    assert_eq!(
        rust.hard, 0,
        "the Rust build differs from the C reference on {} NON-NaN results",
        rust.hard
    );
    let Some(alt) = alt else {
        eprintln!(
            "[nan_policy] target/cref-O2 not built — skipping the C-vs-C comparison. \
             Build it with `cmake -S ../c_src -B target/cref-O2 -DCMAKE_BUILD_TYPE=Release \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build target/cref-O2`."
        );
        return;
    };
    let canon = |x: &std::path::Path| std::fs::canonicalize(x).unwrap_or_else(|_| x.to_path_buf());
    if canon(&alt) == canon(&p.c.path) {
        eprintln!(
            "[nan_policy] the harness reference IS target/cref-O2 ({}) — skipping the \
             C-vs-C comparison (it would compare a build with itself).",
            alt.display()
        );
        return;
    }
    let alt_api = Api::open("C-O2", alt);
    let c2c = score(&p.c, &alt_api);
    eprintln!(
        "[nan_policy] reference vs the -O2 build of the same C source: {} comparisons, \
         {} hard mismatches, {} NaN-payload diffs",
        c2c.compared, c2c.hard, c2c.payload
    );
    assert_eq!(
        c2c.hard, 0,
        "the two C builds differ on {} NON-NaN results — the corpus is unsound",
        c2c.hard
    );
    assert!(
        c2c.payload > 0,
        "the two C builds agree on every NaN payload, so the harness should NOT \
         tolerate payload differences — re-tighten the policy in Checker::finish"
    );
    eprintln!(
        "[nan_policy] => the C library disagrees with ITSELF on {} NaN payloads; the Rust \
         translation disagrees with the reference on {}.  NaN payloads are therefore \
         unspecified, and are compared as \"both NaN\" (everything else is bit-exact).",
        c2c.payload, rust.payload
    );
    assert!(
        rust.payload <= c2c.payload,
        "the Rust build ({} payload diffs) is further from the C reference than another \
         build of the C code itself ({})",
        rust.payload,
        c2c.payload
    );
}
