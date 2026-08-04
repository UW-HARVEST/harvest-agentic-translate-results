use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{trusted_utils_copy_bytes, SIG_SIZE_BYTES};

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    let mut sh = SipHash::siphash_init(&SECRET_KEY);
    sh.siphash_reset();
    sh.siphash_update(f_sig, SIG_SIZE_BYTES as u64);
    let c = [constant];
    sh.siphash_update(&c, 1);
    let sig = sh.siphash_digest();
    trusted_utils_copy_bytes(out, &sig, SIG_SIZE_BYTES as u64);
}
