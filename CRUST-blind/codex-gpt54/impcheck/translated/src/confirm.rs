pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    let key = [
        86_u8, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
    ];
    let mut hasher = crate::siphash::SipHash::siphash_init(&key);
    hasher.siphash_update(f_sig, crate::trusted_utils::SIG_SIZE_BYTES as u64);
    hasher.siphash_update(&[constant], 1);
    let sig = hasher.siphash_digest();
    crate::trusted_utils::trusted_utils_copy_bytes(
        out,
        &sig,
        crate::trusted_utils::SIG_SIZE_BYTES as u64,
    );
}
