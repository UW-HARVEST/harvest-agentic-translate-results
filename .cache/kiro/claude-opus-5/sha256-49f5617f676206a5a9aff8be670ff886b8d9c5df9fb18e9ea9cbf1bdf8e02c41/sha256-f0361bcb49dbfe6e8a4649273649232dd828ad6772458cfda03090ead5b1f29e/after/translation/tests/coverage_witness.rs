//! Non-vacuity witness.
//!
//! A differential suite can rot into uselessness if the generators drift so that
//! every case lands on one boring code path. This test replays the Phase B fuzz
//! distribution, classifies each case by the C library's OBSERVABLE behaviour,
//! and asserts every distinct outcome class of `c_src/src/lib.c` was actually
//! reached. It fails loudly if any branch stops being exercised.

mod common;
use common::*;

use std::collections::BTreeMap;

#[test]
fn fuzz_reaches_every_outcome_class() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D); // same seed as configs::row25
    let ignores: [Option<&[u8]>; 6] = [
        None,
        Some(b""),
        Some(b" "),
        Some(b" \t\n:"),
        Some(b"0aF"),
        Some(b"\x80\xFF"),
    ];
    let mut hits: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut bump = |k: &'static str, m: &mut BTreeMap<&'static str, usize>| {
        *m.entry(k).or_insert(0) += 1;
    };

    for _ in 0..200_000usize {
        let hex_len = rng.below(65);
        let biased = rng.below(4) != 0;
        let hex: Vec<u8> = (0..hex_len)
            .map(|_| {
                if biased && rng.below(8) != 0 {
                    *rng.pick(MIXED)
                } else {
                    rng.byte()
                }
            })
            .collect();
        let bin_maxlen = rng.below(41);
        let want_end = rng.below(2) == 1;
        let ig = *rng.pick(&ignores);
        let mut c = Case::new(&hex)
            .bin_cap(bin_maxlen.max(1))
            .bin_maxlen(bin_maxlen)
            .hex_end(want_end);
        c = match ig {
            None => c.no_ignore(),
            Some(s) => c.ignore(s),
        };

        // Both implementations are compared elsewhere; here we only need the C's
        // behaviour to classify, but call both so this test is also differential.
        let out = assert_same("witness", &c);

        let leading_hex_run = hex.iter().take_while(|b| is_c_hex(**b)).count();
        let ignorable_present = ig
            .map(|s| hex.iter().any(|b| s.contains(b) || *b == 0))
            .unwrap_or(false);

        if out.ret >= 0 {
            bump("success", &mut hits);
            if out.ret == 0 {
                bump("success/zero-bytes", &mut hits);
            } else {
                bump("success/nonzero-bytes", &mut hits);
            }
            match out.hex_end {
                Some(e) if e as usize == hex_len => bump("success/full-consume", &mut hits),
                Some(_) => bump("success/early-stop-with-hex-end", &mut hits),
                None => bump("success/null-hex-end-full-consume", &mut hits),
            }
            if ignorable_present && out.ret > 0 {
                bump("success/ignore-path-used", &mut hits);
            }
            if hex_len > 0 && leading_hex_run == 0 {
                bump("success/first-byte-not-hex", &mut hits);
            }
        } else {
            assert_eq!(out.ret, -1, "the only negative return is the -1 sentinel");
            bump("error", &mut hits);
            if bin_maxlen == 0 && leading_hex_run > 0 {
                bump("error/bin-maxlen-zero", &mut hits);
            }
            if bin_maxlen > 0 && leading_hex_run > 2 * bin_maxlen {
                bump("error/buffer-full-midstream", &mut hits);
            }
            if leading_hex_run % 2 == 1 && leading_hex_run <= 2 * bin_maxlen {
                bump("error/odd-nibble", &mut hits);
            }
            match out.hex_end {
                Some(_) => bump("error/with-hex-end", &mut hits),
                None => bump("error/null-hex-end", &mut hits),
            }
            if !want_end && leading_hex_run % 2 == 0 && leading_hex_run < hex_len {
                bump("error/null-hex-end-unconsumed", &mut hits);
            }
            if ignorable_present {
                bump("error/with-ignore-set", &mut hits);
            }
        }
    }

    let required = [
        "success",
        "success/zero-bytes",
        "success/nonzero-bytes",
        "success/full-consume",
        "success/early-stop-with-hex-end",
        "success/null-hex-end-full-consume",
        "success/ignore-path-used",
        "success/first-byte-not-hex",
        "error",
        "error/bin-maxlen-zero",
        "error/buffer-full-midstream",
        "error/odd-nibble",
        "error/with-hex-end",
        "error/null-hex-end",
        "error/null-hex-end-unconsumed",
        "error/with-ignore-set",
    ];
    let mut missing = Vec::new();
    for k in required {
        let n = hits.get(k).copied().unwrap_or(0);
        if n < 10 {
            missing.push(format!("{k} (only {n} hits)"));
        }
    }
    eprintln!("outcome-class histogram:");
    for (k, v) in &hits {
        eprintln!("  {v:>7}  {k}");
    }
    assert!(
        missing.is_empty(),
        "the fuzz distribution no longer reaches these outcome classes: {missing:?}"
    );
}
