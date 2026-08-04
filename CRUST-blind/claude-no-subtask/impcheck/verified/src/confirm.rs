pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    // SipHash over (f_sig || constant). For simplicity (since this module is
    // not part of the public crate), produce a deterministic XOR-based digest.
    let n = out.len().min(16);
    for i in 0..n {
        let a = if i < f_sig.len() { f_sig[i] } else { 0 };
        out[i] = a ^ constant.wrapping_add(i as u8);
    }
}
