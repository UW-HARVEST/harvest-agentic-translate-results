use blt::bm;

#[test]
fn test_bm_init_and_report() {
    // bm_init and bm_report should not panic
    bm::bm_init();
    bm::bm_report("test");
}

#[test]
fn test_bm_report_timing() {
    // After init, report should produce output with non-negative time
    bm::bm_init();
    // Just verify it doesn't panic when called multiple times
    bm::bm_report("first");
    bm::bm_report("second");
}

fn main() {}
