use crate::siphash::SipHash;
use crate::trusted_utils::{SIG_SIZE_BYTES, trusted_utils_copy_bytes};

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    // This needs access to the global siphash - but in Rust we don't have globals.
    // We'll create a temporary siphash with the secret key, reset, and compute.
    let mut sip = SipHash::siphash_init(&crate::secret::SECRET_KEY);
    sip.siphash_reset();
    sip.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    sip.siphash_update(&[constant], 1);
    let sig = sip.siphash_digest();
    trusted_utils_copy_bytes(out, &sig, SIG_SIZE_BYTES as u64);
}
