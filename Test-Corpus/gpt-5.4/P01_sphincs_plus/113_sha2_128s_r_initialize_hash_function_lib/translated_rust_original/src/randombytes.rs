use rand::RngCore;

pub fn randombytes(x: &mut [u8]) {
    rand::thread_rng().fill_bytes(x);
}
