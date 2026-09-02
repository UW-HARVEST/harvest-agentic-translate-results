mod common;
use common::*;

#[test]
fn smoke_symbols_load() {
    let p = pair();
    assert_eq!(p.c.which, "C");
    assert_eq!(p.r.which, "RUST");
}

#[test]
fn smoke_eval() {
    diff_eval_both_modes("1+1");
    diff_eval_both_modes("'a'+'b'");
    diff_eval_both_modes("(function(){return 42})()");
    diff_eval_both_modes("[1,2,3].join('-')");
    diff_eval_both_modes("this.undefinedThing.x");
    diff_eval_both_modes("var x = ;");
}

#[test]
fn smoke_utf() {
    let p = pair();
    let mut r1: Rune = 0;
    let mut r2: Rune = 0;
    let s = cbuf("héllo".as_bytes());
    let n1 = unsafe { (p.c.jsU_chartorune)(&mut r1, s.as_ptr()) };
    let n2 = unsafe { (p.r.jsU_chartorune)(&mut r2, s.as_ptr()) };
    assert_eq!((n1, r1), (n2, r2));
}
