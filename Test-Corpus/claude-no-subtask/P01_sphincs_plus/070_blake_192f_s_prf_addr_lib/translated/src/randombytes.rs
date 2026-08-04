// randombytes wrapper - uses the deterministic NIST DRBG (rng.rs)
// This is in lieu of the system's /dev/urandom version since the binary uses
// the deterministic version (sphincs_core_det in CMake), and so does the lib here.

use crate::rng;

pub fn randombytes(x: &mut [u8], xlen: usize) {
    rng::randombytes(x, xlen);
}
