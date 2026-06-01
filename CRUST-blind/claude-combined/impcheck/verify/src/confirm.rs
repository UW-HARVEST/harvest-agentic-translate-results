pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    // Mirrors C: compute SipHash-2-4 over f_sig (16 bytes) || [constant], using
    // the secret key. Since secret.rs is empty/independent here, we hardcode
    // the same SECRET_KEY and use a private siphash impl.
    const SECRET_KEY: [u8; 16] = [
        86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
    ];
    let mut s = crate::siphash::SipHash::siphash_init(&SECRET_KEY);
    s.siphash_reset();
    s.siphash_update(f_sig, 16);
    s.siphash_update(&[constant], 1);
    let sig = s.siphash_digest();
    for i in 0..16 {
        out[i] = sig[i];
    }
}
