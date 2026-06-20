use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{trusted_utils_copy_bytes, SIG_SIZE_BYTES};

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    let mut siphash = SipHash::siphash_init(&SECRET_KEY);
    siphash.siphash_reset();
    siphash.siphash_update(&f_sig[..SIG_SIZE_BYTES.min(f_sig.len())], SIG_SIZE_BYTES as u64);
    siphash.siphash_update(&[constant], 1);
    let digest = siphash.siphash_digest();
    trusted_utils_copy_bytes(out, &digest, SIG_SIZE_BYTES as u64);
}
