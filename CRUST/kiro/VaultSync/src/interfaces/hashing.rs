use rand::Rng;
use sha2::{Sha256, Digest};

pub const RANDOM_SIZE: usize = 16;

pub fn generate_random_data(buffer: &mut [u8]) {
    let charset = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    for b in buffer.iter_mut() {
        *b = charset[rng.gen::<u8>() as usize % charset.len()];
    }
}

pub fn generate_hash(data: &[u8], hash: &mut [u8]) {
    let result = Sha256::digest(data);
    hash[..result.len()].copy_from_slice(&result);
}
