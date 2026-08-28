//! Proofs that the two surviving mutants in `scripts/mutation_check.py` are
//! SEMANTICALLY EQUIVALENT, i.e. they cannot be caught by any test because they
//! do not change observable behaviour. Recording them as "must survive" keeps
//! the mutation gate honest: if one of these ever starts being caught, the
//! equivalence argument below is wrong and must be revisited.

mod common;

use common::*;

/// Mutant `matches_array_not_zeroed`.
///
/// The C initialises its scratch array with `regmatch_t match[2] = {{.rm_so=0}}`
/// (`lib.c:61`) but only ever *reads* it after `w_regexec` returned non-zero
/// (`lib.c:75-79,82-86,89-93,117-121,124-128`). glibc's `regexec` fills all
/// `nmatch` slots whenever it reports a match, so the initial contents are dead.
///
/// This test proves that claim against the real C library: with `pmatch`
/// pre-filled with a sentinel, whenever `w_regexec` returns 1 for any of the
/// three patterns the parser uses, BOTH slots have been overwritten. Hence no
/// initial value is observable, and the Rust may use any.
#[test]
fn initial_pmatch_contents_are_unobservable_for_the_parser_patterns() {
    let b = both();
    let rng = Rng::new(SEED ^ 0xE0);
    let mut matched = 0usize;
    let mut checked = 0usize;

    let mut subjects: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"0".to_vec(),
        b"1".to_vec(),
        b"1.".to_vec(),
        b"1.2".to_vec(),
        b"1.2.".to_vec(),
        b"1.2.3".to_vec(),
        b"1.2.3.4".to_vec(),
        b"1.2.3.4.5.6.7".to_vec(),
        b"abc".to_vec(),
        b"1a".to_vec(),
        b"a1".to_vec(),
        b".1.2".to_vec(),
        b"10.0.19045.3803".to_vec(),
        b"0.0.0.0".to_vec(),
    ];
    for _ in 0..4000 {
        let n = rng.range(1, 6);
        let mut v = Vec::new();
        for i in 0..n {
            if i > 0 {
                v.push(b'.');
            }
            v.extend_from_slice(&rng.number());
        }
        if rng.bool() {
            v.extend_from_slice(&rng.bytes_from(b"abc. ", rng.below(4)));
        }
        subjects.push(v);
    }

    for pat in PARSER_PATTERNS {
        for sub in &subjects {
            for imp in [(b.c.w_regexec, "C"), (b.rs.w_regexec, "Rust")] {
                let (rv, m) = call_regexec(imp.0, Some(pat.as_bytes()), Some(sub), 2, 2);
                checked += 1;
                if rv != 0 {
                    matched += 1;
                    for (i, slot) in m.iter().enumerate() {
                        assert_ne!(
                            *slot,
                            SENTINEL,
                            "{}: {pat:?} matched {:?} but left pmatch[{i}] untouched — the \
                             array's initial value WOULD be observable",
                            imp.1,
                            String::from_utf8_lossy(sub)
                        );
                    }
                }
            }
        }
    }
    assert!(
        matched > 1000,
        "the sweep produced too few matches ({matched} of {checked}) to be conclusive"
    );
}

/// Mutant `dup_match_snprintf_size_plus_two`.
///
/// `lib.c:78,85,92` call `snprintf(dst, match_size + 1, "%.*s", match_size, …)`.
/// The `%.*s` precision caps the conversion at `match_size` bytes and `snprintf`
/// appends exactly one NUL, so at most `match_size + 1` bytes are ever written
/// regardless of the size argument. Any size limit `>= match_size + 1` therefore
/// produces byte-identical output (the `malloc` size is a separate expression and
/// is still `match_size + 1`, so no overflow is introduced either).
///
/// This test pins the consequence that actually matters: the field the C
/// produces is exactly the captured group, never truncated and never padded, for
/// match sizes sweeping across every buffer-size boundary.
#[test]
fn snprintf_size_limit_above_precision_is_unobservable() {
    let b = both();
    for n in 1..=140usize {
        let digits = vec![b'9'; n];
        let mut input = Vec::new();
        input.extend_from_slice(b"w [Ver: ");
        input.extend_from_slice(&digits);
        input.extend_from_slice(b".");
        input.extend_from_slice(&digits);
        input.extend_from_slice(b".");
        input.extend_from_slice(&digits);
        input.extend_from_slice(b"]");

        let c = run_parse_zeroed(b.c.parse_uname_string, &input);
        let r = run_parse_zeroed(b.rs.parse_uname_string, &input);
        for (i, name) in FIELD_NAMES.iter().enumerate() {
            assert_eq!(c.fields[i], r.fields[i], "field {name} differs at n={n}");
        }
        // os_major / os_minor / os_build are exactly the captured group.
        assert_eq!(c.fields[2].as_deref(), Some(&digits[..]), "os_major at n={n}");
        assert_eq!(c.fields[3].as_deref(), Some(&digits[..]), "os_minor at n={n}");
        assert_eq!(c.fields[6].as_deref(), Some(&digits[..]), "os_build at n={n}");
    }
}
