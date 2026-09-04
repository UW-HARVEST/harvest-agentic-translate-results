//! Smoke test: prove the interpreter observation channel works and that a
//! divergence would actually be detected.

mod common;

use common::*;

#[test]
fn smoke_interpreter_channel() {
    for flags in [0, 1] {
        assert_same_program(flags, "arith", "o(1+2); o(3*4); o(10/4); o(7%3);");
        assert_same_program(flags, "strings", r#"o("a"+"b"); o("abc".length); o("abc".toUpperCase());"#);
        assert_same_program(flags, "json", r#"oj({a:1,b:[1,2,3],c:"x"}); oj([1,[2,[3]]]);"#);
        assert_same_program(flags, "throw", "ok(function(){ return null.x; });");
        assert_same_program(flags, "uncaught", "null.x;");
        assert_same_program(flags, "syntax", "var = ;");
    }
}

#[test]
fn smoke_out_is_actually_populated() {
    let (c, _) = both_apis();
    let r = run_program(c, 0, None, "o(1); o('x');");
    assert_eq!(r.rc, 0, "unexpected rc: {r:?}");
    assert_eq!(r.out.as_deref(), Some("number:1|string:x|"), "out={:?}", r.out);
}

#[test]
fn smoke_uncaught_error_is_reported() {
    let (c, _) = both_apis();
    let r = run_program(c, 0, None, "null.x;");
    assert_eq!(r.rc, 1, "expected failure, got {r:?}");
    assert!(
        r.reports.iter().any(|s| s.contains("TypeError")),
        "reports={:?}",
        r.reports
    );
}
