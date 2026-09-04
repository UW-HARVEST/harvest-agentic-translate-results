//! Harness smoke test: both .so files load, every symbol resolves, basic eval matches.
mod common;
use common::*;

#[test]
fn both_libraries_load_all_symbols() {
    let p = libs();
    assert_eq!(p.c.tag, "C");
    assert_eq!(p.r.tag, "RUST");
}

#[test]
fn trivial_eval_matches() {
    diff_eval("1+1", "1+1", 0);
    diff_eval("str", "'a'+'b'", 0);
    diff_eval("obj", "var o={a:1,b:[1,2,3]}; JSON.stringify(o)", 0);
    diff_eval("throw", "null.x", 0);
    diff_eval("syntax", "var 1", 0);
}

#[test]
fn native_action_matches() {
    fn act(a: &Api, J: JS) {
        unsafe {
            (a.js_pushnumber)(J, 42.5);
        }
    }
    diff_native("push", act, 0);
}
