use crate::siphash::SipHash;
use crate::trusted_utils::{trusted_utils_copy_bytes, SIG_SIZE_BYTES};

// secret key constant -- mirrors c_src/src/trusted/secret.c
const SECRET_KEY: [u8; 16] = [
    86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
];

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    let mut sh = SipHash::siphash_init(&SECRET_KEY);
    sh.siphash_reset();
    sh.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    sh.siphash_update(&[constant], 1);
    let sig = sh.siphash_digest();
    trusted_utils_copy_bytes(out, &sig, SIG_SIZE_BYTES as u64);
}
