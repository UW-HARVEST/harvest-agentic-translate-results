use twoDPartInt::initialization;

// The initialization module only imports config and has no public functions.
// This test verifies the module exists and is accessible.
#[test]
fn test_initialization_module_exists() {
    // Module compiles and is importable - this is the extent of its public API.
    let _ = std::mem::size_of::<u8>();
}

fn main() {}
