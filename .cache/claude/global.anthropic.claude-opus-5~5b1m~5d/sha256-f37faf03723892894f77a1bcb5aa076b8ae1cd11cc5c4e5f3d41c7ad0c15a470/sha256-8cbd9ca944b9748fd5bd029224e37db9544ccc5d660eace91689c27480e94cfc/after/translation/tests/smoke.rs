mod common;
use common::*;

#[test]
fn smoke_both_libs_load_and_agree() {
    let l = libs();
    eprintln!("C   : {}", l.c.path.display());
    eprintln!("RUST: {}", l.rs.path.display());

    cmp_sdti("smoke", 3.9);
    cmp_pwf("smoke", 5, 1);
    cmp_hpo("smoke", 7);
    cmp_cdb("smoke", &[0x11; DATABLOCK_SIZE], 0xAA);
    cmp_overunder("smoke", 7, 11, 13, 17);
}

#[test]
fn smoke_stdout_is_actually_captured() {
    let l = libs();
    let (_, out) = capture_stdout(|| unsafe { (l.c.overunder)(7, 11, 13, 17) });
    assert!(!out.is_empty(), "captured no stdout at all");
    assert!(
        out.starts_with(b"result_1 = 7\n"),
        "unexpected capture: {}",
        show(&out)
    );
}
