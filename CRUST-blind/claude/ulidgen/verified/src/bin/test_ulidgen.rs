// The crate is named `ulidgen` and contains a module also named `ulidgen`.
// Use an alias to avoid the name collision.
use ulidgen::ulidgen as ulidgen_mod;

// The only public symbol in the `ulidgen` module is `main(argc, argv) -> i32`.
// It mirrors the C `main` in c_src/src/ulidgen.c which:
//   - Parses `-n N` and `-t` via getopt(argc, argv, "n:t")
//   - When `-t` is absent, prints `n` (default 1) ULIDs, one per line
//   - Returns 0 on success and !!ferror(stdout) otherwise

#[test]
fn test_main_no_args_returns_zero() {
    // C: `./test_prog` (n defaults to 1) prints one ULID and exits 0.
    let argv = ["ulidgen"];
    let rc = ulidgen_mod::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_n_zero_returns_zero() {
    // C: `./test_prog -n 0` prints nothing and exits 0.
    let argv = ["ulidgen", "-n", "0"];
    let rc = ulidgen_mod::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_n_three_returns_zero() {
    // C: `./test_prog -n 3` prints three ULIDs and exits 0.
    let argv = ["ulidgen", "-n", "3"];
    let rc = ulidgen_mod::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_n_combined_arg_returns_zero() {
    // getopt allows the option arg to be glued: `-n5` is the same as `-n 5`.
    let argv = ["ulidgen", "-n5"];
    let rc = ulidgen_mod::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_n_negative_returns_zero() {
    // C: atol("-1") = -1, so the for-loop body runs 0 times. Exit is 0.
    let argv = ["ulidgen", "-n", "-1"];
    let rc = ulidgen_mod::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_n_invalid_string_returns_zero() {
    // C: atol("abc") = 0, so nothing is printed. Exit is 0.
    let argv = ["ulidgen", "-n", "abc"];
    let rc = ulidgen_mod::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

#[test]
fn test_main_unknown_option_returns_zero() {
    // C getopt continues past unknown options (printing to stderr).
    let argv = ["ulidgen", "-x"];
    let rc = ulidgen_mod::main(argv.len() as i32, &argv);
    assert_eq!(rc, 0);
}

fn main() {}
