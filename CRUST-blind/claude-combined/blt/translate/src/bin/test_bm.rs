use blt::bm::{bm_init, bm_report};
use std::thread;
use std::time::Duration;

#[test]
fn test_bm_init_does_not_panic() {
    bm_init();
    bm_init();
}

#[test]
fn test_bm_report_does_not_panic() {
    bm_init();
    thread::sleep(Duration::from_millis(2));
    // Should print something like: "label: 0.00xxxxxxs"
    bm_report("label");
    bm_report("label2");
}

#[test]
fn test_bm_init_then_report_zero_duration_ok() {
    bm_init();
    bm_report("immediate");
}

fn main() {}
