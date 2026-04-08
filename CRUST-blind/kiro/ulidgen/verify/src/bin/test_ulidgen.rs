use ulidgen::ulidgen;

#[test]
fn test_main_returns_zero_on_success() {
    // Default: generate 1 ULID, should succeed
    let result = ulidgen::main(1, &["ulidgen"]);
    assert_eq!(result, 0);
}

#[test]
fn test_main_with_n_flag() {
    // -n 3 should generate 3 ULIDs and return 0
    let result = ulidgen::main(3, &["ulidgen", "-n", "3"]);
    assert_eq!(result, 0);
}

#[test]
fn test_main_with_n_zero() {
    // -n 0 should generate 0 ULIDs and return 0
    let result = ulidgen::main(2, &["ulidgen", "-n", "0"]);
    assert_eq!(result, 0);
}

#[test]
fn test_main_unknown_flag_ignored() {
    // Unknown flags should be ignored
    let result = ulidgen::main(2, &["ulidgen", "-x"]);
    assert_eq!(result, 0);
}

fn main() {}
