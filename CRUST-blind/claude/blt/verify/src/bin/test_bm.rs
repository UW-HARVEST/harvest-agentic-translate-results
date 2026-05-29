use blt::bm::{bm_init, bm_report};

#[test]
fn test_bm_init_then_report_does_not_panic() {
    bm_init();
    bm_report("test message");
}

#[test]
fn test_bm_report_without_init_does_not_panic() {
    // The C code zeros bm_tp[0] at process start so the duration is huge but
    // doesn't crash. The Rust translation falls back to zero duration when no
    // start is set, which should also be non-panicking.
    bm_report("no init prior");
}

#[test]
fn test_bm_init_resets_timer() {
    bm_init();
    bm_report("first");
    // After bm_report runs, bm_init is implicitly called again (the C code does
    // this), so a subsequent report should not crash.
    bm_report("second");
}

fn main() {}
