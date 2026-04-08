use blt::bm::*;

#[test]
fn test_bm_init_report() {
    bm_init();
    // Should not panic; just verify it runs
    bm_report("test");
}

#[test]
fn test_bm_init_twice() {
    bm_init();
    bm_init();
    bm_report("after reinit");
}

fn main() {}
