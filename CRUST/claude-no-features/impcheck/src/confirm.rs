use crate::siphash::SipHash;
use crate::trusted_utils::SIG_SIZE_BYTES;

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    // Build a SipHash, then update with formula sig and constant byte.
    // The C version uses a global key already initialized via siphash_init.
    // Here we mirror that: we assume the caller has set up the secret key
    // implicitly by initializing top_check; for testability we use the same
    // SECRET_KEY constant as the C source.
    let secret_key: [u8; 16] = [
        86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
    ];
    let mut sh = SipHash::siphash_init(&secret_key);
    sh.siphash_reset();
    sh.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    sh.siphash_update(&[constant], 1);
    let sig = sh.siphash_digest();
    out[..SIG_SIZE_BYTES].copy_from_slice(&sig[..SIG_SIZE_BYTES]);
}
