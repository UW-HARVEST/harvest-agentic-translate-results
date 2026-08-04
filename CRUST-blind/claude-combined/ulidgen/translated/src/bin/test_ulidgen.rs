use ulidgen::ulidgen;

#[test]
fn test_main_returns_zero_on_default_args() {
    // No flags: should default to n=1 and print one ULID, returning 0.
    // We can't easily capture stdout from the binary here, but we can verify
    // it returns 0.
    let argv = ["ulidgen"];
    let rc = ulidgen::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_returns_zero_for_n_arg() {
    let argv = ["ulidgen", "-n", "3"];
    let rc = ulidgen::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_returns_zero_for_n_zero() {
    // -n 0 means print zero ULIDs; this should still succeed
    let argv = ["ulidgen", "-n", "0"];
    let rc = ulidgen::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_handles_unknown_arg_gracefully() {
    // Unknown options are simply skipped (matches getopt switch fallthrough)
    let argv = ["ulidgen", "-x"];
    let rc = ulidgen::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_negative_n_returns_zero() {
    // Negative n: the for loop runs zero times, exit success
    let argv = ["ulidgen", "-n", "-5"];
    let rc = ulidgen::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

fn main() {}
