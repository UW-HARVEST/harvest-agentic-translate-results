// The cli module has no public API (main is private).
// We verify the module exists and is accessible.
use libpsbt::cli;

#[test]
fn test_cli_module_exists() {
    // cli module is declared as pub mod in lib.rs
    // It contains no public functions, so we just verify it compiles.
    let _ = std::mem::size_of::<()>();
}

fn main() {}
