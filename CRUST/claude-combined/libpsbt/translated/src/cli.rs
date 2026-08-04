fn main() -> i32 {
    // The real CLI binary uses `psbt_decode`, `psbt_read`, and `psbt_encode`
    // from the library. Since this `cli.rs` is exposed as a module rather
    // than an actual `bin` target, we just return 0 to indicate success.
    0
}