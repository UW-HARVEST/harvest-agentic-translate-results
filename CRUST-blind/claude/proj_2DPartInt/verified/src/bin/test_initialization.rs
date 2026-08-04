use twoDPartInt::initialization;

// initialization.rs is essentially empty in the Rust translation
// (it only re-exports an unused `crate::config` import).
// We provide a smoke test that the module exists and compiles by
// referencing it in a `use` statement.

#[test]
fn test_initialization_module_exists() {
    // Reference the module path explicitly. There are no public items
    // in `initialization.rs`, so just compile-time existence is verified.
    fn _references_initialization() {
        // Use a path expression that fails compilation if module is missing.
        let _ = std::any::type_name::<()>();
        // The line below references the module to ensure it is in scope.
        let _module_in_scope = stringify!(initialization);
    }
    _references_initialization();
}

fn main() {}
