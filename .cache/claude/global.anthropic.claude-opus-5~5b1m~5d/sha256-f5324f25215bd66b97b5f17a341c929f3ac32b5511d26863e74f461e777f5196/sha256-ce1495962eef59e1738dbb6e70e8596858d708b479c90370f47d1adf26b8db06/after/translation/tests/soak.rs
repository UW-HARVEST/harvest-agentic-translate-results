//! High-volume randomized soak. Runs a small number of iterations by default so
//! the committed suite stays fast; crank it up with
//!
//! ```text
//! SOAK_ITERS=2000000 cargo test --offline --test soak -- --nocapture
//! ```
//!
//! Every input goes through all three entry points and every observable is
//! compared (return values, all 9 `os_data` fields, the mutated caller buffer
//! including its guard bytes, and the full `regmatch_t` array).

mod common;

use common::*;

fn iters(default: usize) -> usize {
    std::env::var("SOAK_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Alphabets that stress different parts of the parser.
fn alphabets() -> Vec<&'static [u8]> {
    vec![
        // dense in every byte the C special-cases
        b"  [[]]:::((()))|||...VVeerr0123456789abz",
        // separators only
        b" []:()|.",
        // digits and dots only (drives the three version regexes)
        b"0123456789..",
        // arch tokens' characters
        b"xX86_64iI3rmvspachdAIX1720nl",
        // full byte range incl. high bytes (no NUL)
        b"\x01\x02\x09\x20\x21\x28\x29\x3a\x5b\x5d\x7c\x7f\x80\xa0\xc3\xa9\xfe\xff0123Vera.",
        // long runs of a single separator
        b"  [[  ::  ((  ||  ",
        SAFE_ALPHA,
        HOSTILE_ALPHA,
    ]
}

#[test]
fn soak_random_bytes_all_entry_points() {
    let n = iters(20_000);
    let rng = Rng::new(SEED ^ 0x50AC);
    let alphas = alphabets();
    let mut done = 0usize;
    with_stderr_silenced(|| {
        for i in 0..n {
            let alpha = alphas[i % alphas.len()];
            let len = rng.below(200);
            let input = rng.bytes_from(alpha, len);

            diff_parse(&input, "soak/parse");
            diff_arch(&input, "soak/arch");
            // the three patterns the parser itself uses, plus the input as a pattern
            for p in PARSER_PATTERNS {
                diff_regexec(Some(p.as_bytes()), Some(&input), 2, 4, "soak/regexec");
            }
            diff_regexec(Some(&input), Some(b"1.2.3"), 4, 8, "soak/regexec-as-pattern");
            done += 1;
        }
    });
    assert_eq!(done, n);
    eprintln!("soak_random_bytes_all_entry_points: {done} iterations OK");
}

#[test]
fn soak_structured_fragments() {
    // Splice real separator tokens so that deeply-nested branch combinations
    // (which uniform random bytes reach only rarely) are hit constantly.
    let n = iters(20_000);
    let rng = Rng::new(SEED ^ 0x57AC);
    let frags: [&[u8]; 32] = [
        b" [Ver: ", b" [", b": ", b" (", b")", b"]", b"|", b" ", b".", b"..", b"0", b"1", b"12",
        b"345", b"0000", b"4294967296", b"99999999999999999999", b"abc", b"Ver", b"Ver: ", b"[",
        b"  ", b"x86_64", b"aarch64", b"arm64", b"i386", b"AIX", b"LTS", b"jammy", b"\xff\xfe",
        b"\t", b"Ubuntu",
    ];
    let mut done = 0usize;
    for _ in 0..n {
        let k = rng.range(1, 16);
        let mut v = Vec::new();
        for _ in 0..k {
            v.extend_from_slice(*rng.pick(&frags));
        }
        diff_parse(&v, "soak/fragments");
        if rng.below(8) == 0 {
            diff_parse_prefilled(&v, "soak/fragments-prefilled");
        }
        diff_arch(&v, "soak/fragments-arch");
        done += 1;
    }
    assert_eq!(done, n);
    eprintln!("soak_structured_fragments: {done} iterations OK");
}

#[test]
fn soak_well_formed_mutated() {
    // Start from a well-formed uname string and apply small random edits, so the
    // inputs stay near the "interesting" region of the input space.
    let n = iters(20_000);
    let rng = Rng::new(SEED ^ 0x5AAC);
    let seeds: [&str; 10] = [
        "Microsoft Windows 10 Pro [Ver: 10.0.19045.3803]",
        "Linux ubuntu 5.15.0 x86_64 [Ubuntu|ubuntu: 22.04.3 LTS (Jammy Jellyfish)]",
        "Linux centos 3.10.0 x86_64 [CentOS Linux|centos: 7.9.2009 (Core)]",
        "Darwin mac 22.6.0 arm64 [Mac OS X|darwin: 13.5.2 (Ventura)]",
        "SunOS s11 5.11 i86pc [SunOS|sunos: 11.4]",
        "AIX p7 1 7 [AIX|aix: 7.2]",
        "Linux nover 5.15.0 i686 [SomeDistro]",
        "Linux x86_64",
        " [Ver: ]",
        " [: ]",
    ];
    let inject: [u8; 14] = [
        b' ', b'[', b']', b':', b'(', b')', b'|', b'.', b'0', b'9', b'V', b'e', b'r', 0xff,
    ];
    let mut done = 0usize;
    for _ in 0..n {
        let mut v = rng.pick(&seeds).as_bytes().to_vec();
        let edits = rng.range(1, 6);
        for _ in 0..edits {
            if v.is_empty() {
                v.push(*rng.pick(&inject));
                continue;
            }
            match rng.below(3) {
                0 => {
                    // substitute
                    let i = rng.below(v.len());
                    v[i] = *rng.pick(&inject);
                }
                1 => {
                    // insert
                    let i = rng.below(v.len() + 1);
                    v.insert(i, *rng.pick(&inject));
                }
                _ => {
                    // delete
                    let i = rng.below(v.len());
                    v.remove(i);
                }
            }
        }
        diff_parse(&v, "soak/mutated");
        diff_parse_prefilled(&v, "soak/mutated-prefilled");
        diff_arch(&v, "soak/mutated-arch");
        done += 1;
    }
    assert_eq!(done, n);
    eprintln!("soak_well_formed_mutated: {done} iterations OK");
}
