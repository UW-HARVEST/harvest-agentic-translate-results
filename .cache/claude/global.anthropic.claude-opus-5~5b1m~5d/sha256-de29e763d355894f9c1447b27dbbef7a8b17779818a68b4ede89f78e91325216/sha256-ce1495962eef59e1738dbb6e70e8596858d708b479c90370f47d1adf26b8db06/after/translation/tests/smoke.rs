mod common;
use common::*;

#[test]
fn both_libs_load_and_agree_on_constants() {
    let l = libs();
    unsafe {
        eq_r("c2RotIdentity", (l.c.c2RotIdentity)(), (l.r.c2RotIdentity)());
        eq_x("c2xIdentity", (l.c.c2xIdentity)(), (l.r.c2xIdentity)());
        eq_v("c2V", (l.c.c2V)(1.5, -2.5), (l.r.c2V)(1.5, -2.5));
        eq_i("capsule", (l.c.capsule)(0.0, 0.0, 1.0, 1.0, 1.0), (l.r.capsule)(0.0, 0.0, 1.0, 1.0, 1.0));
    }
}
