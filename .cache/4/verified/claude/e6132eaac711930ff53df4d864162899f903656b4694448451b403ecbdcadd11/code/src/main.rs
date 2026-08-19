// Thin `main` shim: C's `int main()` returns 0, which libc hands to exit().
//
// All of the translated logic lives in `driver::driver_main` (src/lib.rs) so
// that the very same code can be reached both from this executable and through
// the C ABI symbol `driver_main` exported by the cdylib.

fn main() {
    let status = driver::driver_main();
    // `driver_main` has already flushed stdout, mirroring the libc exit-time
    // fflush that the C program relies on.
    std::process::exit(status);
}
