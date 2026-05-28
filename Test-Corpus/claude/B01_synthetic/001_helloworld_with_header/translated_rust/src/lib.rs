pub mod sillymain;

/// FFI wrapper exporting `helloworld` so external callers (and the integration
/// tests) can dlopen the resulting cdylib and invoke it the same way they
/// would the C shared library.
#[no_mangle]
pub extern "C" fn helloworld() -> std::os::raw::c_int {
    sillymain::helloworld() as std::os::raw::c_int
}
