use crate::siphash::SipHash;
use crate::trusted_utils::SIG_SIZE_BYTES;

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    // Note: in the C code, siphash uses a global state previously initialized
    // with SECRET_KEY. Here we mirror that by initializing a SipHash with the
    // SECRET_KEY constant from secret.rs.
    let key: [u8; 16] = [
        86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
    ];
    let mut sh = SipHash::siphash_init(&key);
    sh.siphash_reset();
    sh.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    sh.siphash_update(&[constant], 1);
    let sig = sh.siphash_digest();
    for i in 0..SIG_SIZE_BYTES {
        out[i] = sig[i];
    }
}
