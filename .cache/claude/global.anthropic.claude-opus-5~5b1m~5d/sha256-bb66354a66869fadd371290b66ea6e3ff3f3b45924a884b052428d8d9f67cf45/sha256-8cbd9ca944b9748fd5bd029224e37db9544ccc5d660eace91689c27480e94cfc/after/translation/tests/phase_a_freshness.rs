//! Guard rail: `cargo test` does NOT build a `cdylib` target, so it is very
//! easy to run the whole differential suite against a STALE
//! `libsiphash_lib.so` and get a vacuous pass.  This test fails loudly if the
//! `.so` the other tests are about to dlopen is older than `src/lib.rs`.
//!
//! Always run `cargo build [--release]` before `cargo test [--release]`
//! (see `scripts/check_features.sh`, which does this automatically).

mod common;

use std::path::PathBuf;
use std::time::SystemTime;

fn mtime(p: &std::path::Path) -> SystemTime {
    std::fs::metadata(p)
        .unwrap_or_else(|e| panic!("stat {:?}: {}", p, e))
        .modified()
        .expect("mtime")
}

#[test]
fn rust_so_is_not_stale_relative_to_sources() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let so = common::rust_so();
    let so_t = mtime(&so);

    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = root.join(src);
        let s_t = mtime(&p);
        assert!(
            so_t >= s_t,
            "STALE ARTIFACT: {:?} (mtime {:?}) is older than {:?} (mtime {:?}).\n\
             `cargo test` does not rebuild a cdylib -- run `cargo build{}` first, \
             otherwise this suite silently tests an old library and passes vacuously.",
            so,
            so_t,
            p,
            s_t,
            if so.to_string_lossy().contains("/release/") { " --release" } else { "" }
        );
    }
}

#[test]
fn c_so_is_not_stale_relative_to_c_sources() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_root = root.parent().unwrap().join("c_src");
    let so = common::c_so();
    let so_t = mtime(&so);
    for src in ["src/lib.c", "include/lib.h", "CMakeLists.txt"] {
        let p = c_root.join(src);
        let s_t = mtime(&p);
        assert!(
            so_t >= s_t,
            "STALE C ARTIFACT: {:?} is older than {:?}; rebuild with \
             `cd c_src/build && cmake --build .`",
            so,
            p
        );
    }
}

/// Sanity: the two libraries must be *distinct* files, and both must actually
/// resolve the two entry points.  (Guards against accidentally pointing both
/// handles at the same `.so`, which would make every comparison trivially true.)
#[test]
fn c_and_rust_libraries_are_distinct_and_both_resolve() {
    let c = common::c_so();
    let r = common::rust_so();
    assert_ne!(
        std::fs::canonicalize(&c).unwrap(),
        std::fs::canonicalize(&r).unwrap(),
        "C and Rust .so paths resolve to the same file"
    );
    let l = common::libs();
    let _ = l.c_hash();
    let _ = l.rs_hash();
    let _ = l.c_siphash();
    let _ = l.rs_siphash();

    // The two libraries must not be byte-identical either.
    let cb = std::fs::read(&c).unwrap();
    let rb = std::fs::read(&r).unwrap();
    assert_ne!(cb, rb, "C and Rust .so are byte-identical files?!");
}

/// Negative control for the *value* comparison: a deliberately wrong reference
/// value must NOT match, proving `assert_hash_eq` is comparing real outputs and
/// not, say, two calls into the same library.
#[test]
fn differential_harness_is_not_vacuous() {
    let l = common::libs();
    let ch = l.c_hash();
    let rh = l.rs_hash();
    let mut buf = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let p = buf.as_mut_ptr() as *mut std::ffi::c_void;
    let c_val = unsafe { ch(p, 8, 0) };
    let r_val = unsafe { rh(p, 8, 0) };
    assert_eq!(c_val, r_val, "C/Rust disagree on the canonical probe");
    // Distinct inputs must give distinct outputs (so the functions are really
    // reading the buffer, i.e. the comparison is meaningful).
    let c_other = unsafe { ch(p, 7, 0) };
    assert_ne!(c_val, c_other, "len 7 vs 8 gave the same hash");
    assert_ne!(c_val, 0, "hash of the canonical probe is 0?");
}

/// The C code XORs the seed into `v0..v3` **twice**, so it cancels:
///
/// ```c
/// v0 = ((0x736f6d65<<32) + 0x70736575) ^  seed;   // lib.c:10
/// v1 = ((0x646f7261<<32) + 0x6e646f6d) ^ ~seed;   // lib.c:11
/// v2 = ((0x6c796765<<32) + 0x6e657261) ^  seed;   // lib.c:12
/// v3 = ((0x74656462<<32) + 0x79746573) ^ ~seed;   // lib.c:13
/// v0 ^= 0x0706050403020100ull ^  seed;            // lib.c:14  <- seed cancels
/// v1 ^= 0x0f0e0d0c0b0a0908ull ^ ~seed;            // lib.c:15  <- ~seed cancels
/// v2 ^= 0x0706050403020100ull ^  seed;            // lib.c:16  <- seed cancels
/// v3 ^= 0x0f0e0d0c0b0a0908ull ^ ~seed;            // lib.c:17  <- ~seed cancels
/// ```
///
/// `x ^ s ^ k ^ s == x ^ k`, therefore **`stbds_hash_bytes` ignores `seed`
/// entirely**.  Verified directly against the C `.so`.  This is deliberately
/// asserted (rather than "fixed") because the C is the ground truth, and it is
/// a strong structural property that a mistranslation of lines 10-17 would
/// break.
#[test]
fn c_hash_is_seed_invariant_and_rust_matches() {
    let l = common::libs();
    let ch = l.c_hash();
    let rh = l.rs_hash();
    let seeds = [
        0usize,
        1,
        2,
        usize::MAX,
        usize::MAX / 2,
        1usize << 63,
        0x5555_5555_5555_5555,
        0xaaaa_aaaa_aaaa_aaaa,
        12345,
    ];
    let mut rng = common::Rng::new(0xA11_5EED);
    let mut buf = vec![0u8; 96];
    for len in 0..=80usize {
        rng.fill(&mut buf);
        let p = buf.as_mut_ptr() as *mut std::ffi::c_void;
        let c_ref = unsafe { ch(p, len, 0) };
        let r_ref = unsafe { rh(p, len, 0) };
        assert_eq!(c_ref, r_ref, "C/Rust differ at len={}", len);
        for &s in &seeds {
            let c_s = unsafe { ch(p, len, s) };
            let r_s = unsafe { rh(p, len, s) };
            assert_eq!(
                c_s, c_ref,
                "C is NOT seed-invariant at len={} seed={:#x} (double-XOR assumption broken)",
                len, s
            );
            assert_eq!(
                r_s, c_s,
                "Rust does not match C at len={} seed={:#x}",
                len, s
            );
        }
        // Random seeds too.
        for _ in 0..16 {
            let s = rng.next_u64() as usize;
            let c_s = unsafe { ch(p, len, s) };
            let r_s = unsafe { rh(p, len, s) };
            assert_eq!(c_s, c_ref, "C not seed-invariant at len={} seed={:#x}", len, s);
            assert_eq!(r_s, c_s, "Rust != C at len={} seed={:#x}", len, s);
        }
    }
}

/// The known-answer vector for the empty input, taken from the C `.so` itself:
/// `stbds_hash_bytes(p, 0, any_seed) == 0x726fdb47dd0e0e31`.  Pins the
/// initialisation constants (lib.c:10-17) plus the 2+4 finalisation rounds.
#[test]
fn known_answer_empty_input() {
    let l = common::libs();
    let ch = l.c_hash();
    let rh = l.rs_hash();
    let expect: usize = 0x726f_db47_dd0e_0e31;
    for seed in [0usize, 1, usize::MAX, 999] {
        let c = unsafe { ch(std::ptr::null_mut(), 0, seed) };
        let r = unsafe { rh(std::ptr::null_mut(), 0, seed) };
        assert_eq!(c, expect, "C known-answer changed (seed={})", seed);
        assert_eq!(r, expect, "Rust known-answer mismatch (seed={})", seed);
    }
}
