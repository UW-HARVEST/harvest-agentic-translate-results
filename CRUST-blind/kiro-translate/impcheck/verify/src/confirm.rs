pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    use crate::siphash::SipHash;
    use crate::trusted_utils::{SIG_SIZE_BYTES, trusted_utils_copy_bytes};

    // This function needs access to the global siphash state.
    // In C, it calls siphash_reset/update/digest on global state.
    // Here we use the SECRET_KEY to create a fresh siphash, reset, compute.
    let secret_key: [u8; 16] = [86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211];
    let mut hasher = SipHash::siphash_init(&secret_key);
    hasher.siphash_reset();
    hasher.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    hasher.siphash_update(&[constant], 1);
    let sig = hasher.siphash_digest();
    trusted_utils_copy_bytes(out, &sig, SIG_SIZE_BYTES as u64);
}
