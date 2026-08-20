//! Phase B — high-volume randomized differential campaign across all three
//! entry points. Scale with `HARVEST_FUZZ_ITERS` (default 50 000 per test).
//!
//! This is the cross-cutting safety net for the `CONFIGS.md` cross-product:
//! rather than one shape per row it hammers the *combinations* of markers,
//! version shapes, arch names, `nmatch` values and `os_data` states.

mod common;
use common::*;

fn iters(default: usize) -> usize {
    std::env::var("HARVEST_FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Alphabet containing every byte that any branch in `lib.c` keys off.
const MARKER_ALPHA: &[u8] = b" [](:|)Ver.0123456789abxi386_64AIXsparcmdv7h";

/// Structured token soup: the exact markers plus arch names and version pieces.
const TOKENS: &[&[u8]] = &[
    b" [Ver: ",
    b" [",
    b": ",
    b" (",
    b")",
    b"]",
    b"|",
    b" ",
    b"",
    b"[",
    b"(",
    b".",
    b"0",
    b"1",
    b"10",
    b"22.04",
    b"6.1.7601",
    b"10.0.19041.1237",
    b"00.00",
    b"rolling",
    b"LTS",
    b"Jammy Jellyfish",
    b"Ubuntu",
    b"Windows",
    b"x86_64",
    b"i386",
    b"i686",
    b"sparc",
    b"amd64",
    b"i86pc",
    b"ia64",
    b"AIX",
    b"armv6",
    b"armv7",
    b"aarch64",
    b"arm64",
    b"Ver: ",
    b" Ver: ",
    b" [Ver:",
    b"[Ver: ",
];

#[test]
fn deep_fuzz_parse_uname_bytes() {
    let n = iters(50_000);
    let mut rng = Rng::new(0xDEEF_0001);
    for i in 0..n {
        let len = rng.range(0, 64);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(MARKER_ALPHA)).collect();
        diff_parse_uname("deep bytes", &s, if i % 2 == 0 { 0x00 } else { 0xAA });
    }
}

#[test]
fn deep_fuzz_parse_uname_tokens() {
    let n = iters(50_000);
    let mut rng = Rng::new(0xDEEF_0002);
    for i in 0..n {
        let k = rng.range(0, 12);
        let mut s = Vec::new();
        for _ in 0..k {
            s.extend_from_slice(*rng.pick(TOKENS));
        }
        diff_parse_uname("deep tokens", &s, if i % 2 == 0 { 0x00 } else { 0xAA });
    }
}

#[test]
fn deep_fuzz_pipeline_tokens() {
    let n = iters(20_000);
    let mut rng = Rng::new(0xDEEF_0003);
    for i in 0..n {
        let k = rng.range(0, 10);
        let mut s = Vec::new();
        for _ in 0..k {
            s.extend_from_slice(*rng.pick(TOKENS));
        }
        diff_pipeline("deep pipeline", &s, if i % 2 == 0 { 0x00 } else { 0xAA });
    }
}

#[test]
fn deep_fuzz_get_os_arch() {
    let n = iters(50_000);
    let mut rng = Rng::new(0xDEEF_0004);
    for _ in 0..n {
        let mode = rng.below(3);
        let s: Vec<u8> = match mode {
            0 => {
                let len = rng.range(0, 64);
                (0..len).map(|_| *rng.pick(MARKER_ALPHA)).collect()
            }
            1 => {
                let k = rng.range(0, 8);
                let mut v = Vec::new();
                for _ in 0..k {
                    v.extend_from_slice(*rng.pick(TOKENS));
                }
                v
            }
            _ => {
                // arbitrary non-NUL bytes
                let len = rng.range(0, 48);
                (0..len).map(|_| 1u8 + rng.below(255) as u8).collect()
            }
        };
        diff_get_os_arch("deep arch", &s);
    }
}

#[test]
fn deep_fuzz_w_regexec() {
    let n = iters(50_000);
    let mut rng = Rng::new(0xDEEF_0005);
    // Patterns kept to a compiling-friendly alphabet plus the production ones,
    // so most iterations reach `regexec` rather than stopping at `regcomp`.
    let base: &[&[u8]] = &[
        br"^([0-9]+)\.*",
        br"^[0-9]+\.([0-9]+)\.*",
        br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*",
        br"([0-9]+)",
        br"([a-z]+)([0-9]*)",
        br"^(a)?(b)?(c)?$",
        br"(x|y|z)+",
        br"^$",
        br"",
    ];
    for _ in 0..n {
        let pat: Vec<u8> = if rng.chance(4) {
            let len = rng.range(0, 8);
            (0..len)
                .map(|_| *rng.pick(b"ab019.*+?[]()^$|\\-{},:".as_slice()))
                .collect()
        } else {
            rng.pick(base).to_vec()
        };
        let slen = rng.range(0, 24);
        let subj: Vec<u8> = (0..slen)
            .map(|_| *rng.pick(b"ab019.xyz ()[]|:".as_slice()))
            .collect();
        let nm = *rng.pick(&[0usize, 1, 2, 3, 5, 9, 17]);
        diff_w_regexec("deep regexec", Some(&pat), Some(&subj), nm, Some(24));
    }
}
