use crate::siphash::SipHash;
use crate::trusted_utils::SIG_SIZE_BYTES;

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    // We use a fresh SipHash with the secret key for this one-shot calculation.
    let mut sh = SipHash::siphash_init(&crate::secret::SECRET_KEY);
    sh.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    let c = [constant];
    sh.siphash_update(&c, 1);
    let sig = sh.siphash_digest();
    out[..SIG_SIZE_BYTES].copy_from_slice(&sig[..SIG_SIZE_BYTES]);
}
