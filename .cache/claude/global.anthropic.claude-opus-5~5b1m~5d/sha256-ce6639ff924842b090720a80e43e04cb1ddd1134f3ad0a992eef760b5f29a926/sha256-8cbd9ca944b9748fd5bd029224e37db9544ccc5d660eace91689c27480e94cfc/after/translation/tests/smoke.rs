mod common;
use common::*;
use std::ffi::c_int;
use std::ptr::{null, null_mut};

#[test]
fn symbols_load() {
    let l = libs();
    assert_eq!(l.c.which, "C");
    assert_eq!(l.r.which, "RUST");
}

#[test]
fn smoke_dostring() {
    diff("smoke_dostring", |api| unsafe {
        let J = newstate(api, 0);
        let src = cs("var a = 1 + 2; print(a); print(typeof a);");
        let rc = (api.js_dostring)(J, src.as_ptr());
        p_int("rc", rc);
        p_int("top", (api.js_gettop)(J));
        (api.js_freestate)(J);
    });
}

#[test]
fn smoke_lowlevel() {
    diff("smoke_lowlevel", |api| unsafe {
        let mut k: c_int = 0;
        let mut buf = [0i8; 64];
        let n = (api.js_grisu2)(1.5, buf.as_mut_ptr(), &mut k);
        p_int("n", n);
        p_int("k", k);
        p_str("digits", buf.as_ptr());
        p_int("runelen", (api.jsU_runelen)(0x2028));
        p_str("token", (api.jsY_tokenstring)(100));
    });
}
