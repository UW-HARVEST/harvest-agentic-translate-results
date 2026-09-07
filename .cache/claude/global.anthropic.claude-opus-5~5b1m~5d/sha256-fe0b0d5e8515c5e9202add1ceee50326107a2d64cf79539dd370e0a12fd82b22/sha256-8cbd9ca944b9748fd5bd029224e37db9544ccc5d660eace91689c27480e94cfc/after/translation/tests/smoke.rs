mod common;
use common::*;

#[test]
fn smoke_both_libraries_load_and_agree_on_a_simple_alert() {
    let b = both();
    let dir = tmp_root();
    unsafe {
        let c = drain(&b.c, &dir, "smoke", &simple_alert(), 0, 4);
        let r = drain(&b.rs, &dir, "smoke", &simple_alert(), 0, 4);
        assert_eq!(c, r, "C vs Rust mismatch on the canonical alert");
        assert!(c[0].is_some(), "C failed to parse the canonical alert: {c:?}");
        println!("{:#?}", c[0]);
    }
}
