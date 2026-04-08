use ulidgen::ulidgen;

#[test]
fn test_default_generates_one_ulid() {
    // Default (no args) should return 0 and produce 1 ULID
    let rc = ulidgen::main(1, &["ulidgen"]);
    assert_eq!(rc, 0);
}

#[test]
fn test_n_zero_no_output() {
    let rc = ulidgen::main(3, &["ulidgen", "-n", "0"]);
    assert_eq!(rc, 0);
}

#[test]
fn test_n_negative_no_output() {
    let rc = ulidgen::main(3, &["ulidgen", "-n", "-1"]);
    assert_eq!(rc, 0);
}

#[test]
fn test_n_three() {
    let rc = ulidgen::main(3, &["ulidgen", "-n", "3"]);
    assert_eq!(rc, 0);
}

#[test]
fn test_unknown_flag_ignored() {
    let rc = ulidgen::main(2, &["ulidgen", "-z"]);
    assert_eq!(rc, 0);
}

fn main() {}
