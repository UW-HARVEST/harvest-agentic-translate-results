/// CLI entry-point counterpart for the C `cli.c` executable.
///
/// In this Rust port the binary is exposed via the test harness, so this
/// function is intentionally a stub that simply returns a successful exit
/// code. It is preserved as part of the public module layout for parity
/// with the original C project structure.
fn main() -> i32 {
    0
}
