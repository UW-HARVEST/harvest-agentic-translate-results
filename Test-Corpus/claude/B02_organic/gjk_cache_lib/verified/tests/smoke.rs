//! Harness self-check: both `.so`s load, layouts agree, symbols resolve.

mod common;
use common::*;

#[test]
fn both_libraries_load_and_layouts_match() {
    assert_layout();
    let p = pair();
    unsafe {
        eq_r("c2RotIdentity", (p.c.c2RotIdentity)(), (p.r.c2RotIdentity)());
        eq_x("c2xIdentity", (p.c.c2xIdentity)(), (p.r.c2xIdentity)());
        eq_v("c2V", (p.c.c2V)(1.5, -2.5), (p.r.c2V)(1.5, -2.5));
    }
}

#[test]
fn gjk_cache_smoke() {
    let p = pair();
    unsafe {
        (p.c.gjk_cache)(0, std::ptr::null_mut(), std::ptr::null_mut(), -10.0, -10.0, 10.0, 10.0,
                        100.0, -25.0, 75.0, 100.0, 10.0);
        (p.r.gjk_cache)(0, std::ptr::null_mut(), std::ptr::null_mut(), -10.0, -10.0, 10.0, 10.0,
                        100.0, -25.0, 75.0, 100.0, 10.0);
    }
}
