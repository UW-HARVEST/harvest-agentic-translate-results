//! Phase C — `ERRORS.md` rows E18a and E24: the analyzer state *before*
//! `analyzer_init` has ever been called.
//!
//! This file deliberately contains a single test, because `initialized` can
//! never be cleared again once set: the scenario only exists in a virgin
//! process, and every integration-test file gets its own process.

mod common;

use common::*;

#[test]
fn e18a_e24_analyzer_not_initialized() {
    let _g = lock();
    let p = libs();

    // E18a: analyze_text reports on stderr and returns an all-zero result
    let (co, ce) = p.c.captured_both(|| {
        let r = p.c.analyze(b"int a = 1; // c\n");
        assert_eq!(r, CResult::default(), "C result must be all zero");
    });
    let (ro, re) = p.rust.captured_both(|| {
        let r = p.rust.analyze(b"int a = 1; // c\n");
        assert_eq!(r, CResult::default(), "Rust result must be all zero");
    });
    assert_eq!(show(&co), show(&ro), "stdout differs");
    assert_eq!(show(&ce), show(&re), "stderr differs");
    assert_eq!(show(&ce), "Error: Analyzer not initialized\\n");
    assert!(co.is_empty());

    // ... and the tokenizer was not touched at all
    assert_eq!(p.c.stats(), p.rust.stats());
    assert_eq!(p.c.stats().2, 0, "no character may have been consumed");

    // E24: find_patterns returns silently while uninitialized
    for pat in [&b""[..], &b"a"[..]] {
        let (co, ce) = p.c.captured_both(|| {
            let s = cstring(pat);
            (p.c.find_patterns)(s.as_ptr() as *const std::ffi::c_char)
        });
        let (ro, re) = p.rust.captured_both(|| {
            let s = cstring(pat);
            (p.rust.find_patterns)(s.as_ptr() as *const std::ffi::c_char)
        });
        assert_eq!(show(&co), show(&ro));
        assert_eq!(show(&ce), show(&re));
        assert!(co.is_empty() && ce.is_empty(), "{}", show(&co));
    }
    // NULL pattern while uninitialized
    let (co, ce) = p.c.captured_both(|| (p.c.find_patterns)(std::ptr::null()));
    let (ro, re) = p.rust.captured_both(|| (p.rust.find_patterns)(std::ptr::null()));
    assert_eq!(show(&co), show(&ro));
    assert_eq!(show(&ce), show(&re));
    assert!(co.is_empty() && ce.is_empty());

    // the other analyzer entry points are usable while uninitialized
    let c = p.c.captured(|| (p.c.print_token_distribution)());
    let r = p.rust.captured(|| (p.rust.print_token_distribution)());
    assert_eq!(show(&c), show(&r));
    assert_eq!(
        (p.c.calculate_complexity_score)(),
        (p.rust.calculate_complexity_score)()
    );
    assert_eq!((p.c.calculate_complexity_score)(), 0);

    // after analyzer_init the very same call succeeds
    (p.c.analyzer_init)((p.c.get_tokenizer_ops)());
    (p.rust.analyzer_init)((p.rust.get_tokenizer_ops)());
    let (co, ce) = p.c.captured_both(|| {
        let r = p.c.analyze(b"int a = 1; // c\n");
        assert_ne!(r, CResult::default());
    });
    let (ro, re) = p.rust.captured_both(|| {
        let r = p.rust.analyze(b"int a = 1; // c\n");
        assert_ne!(r, CResult::default());
    });
    assert_eq!(show(&co), show(&ro));
    assert_eq!(show(&ce), show(&re));
    assert!(ce.is_empty(), "{}", show(&ce));
    assert_eq!(p.c.analyze(b"x"), p.rust.analyze(b"x"));
}
