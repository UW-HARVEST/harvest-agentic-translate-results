use libpsbt::cli;

#[test]
fn test_cli_main_returns_zero() {
    assert_eq!(cli::main(), 0);
}

fn main() {}
