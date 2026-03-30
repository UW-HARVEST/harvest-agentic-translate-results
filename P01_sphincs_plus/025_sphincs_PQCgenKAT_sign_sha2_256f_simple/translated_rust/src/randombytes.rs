use crate::rng::rng_randombytes;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) {
    rng_randombytes(x, xlen);
}
