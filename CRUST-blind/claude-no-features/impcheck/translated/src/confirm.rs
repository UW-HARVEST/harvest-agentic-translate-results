use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::SIG_SIZE_BYTES;

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    let mut sh = SipHash::siphash_init(&SECRET_KEY);
    sh.siphash_reset();
    sh.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    sh.siphash_update(&[constant], 1);
    let sig = sh.siphash_digest();
    let n = SIG_SIZE_BYTES.min(out.len()).min(sig.len());
    out[..n].copy_from_slice(&sig[..n]);
}
