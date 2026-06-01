#[test]
#[cfg(feature = "blake")]
fn blake256_known_vectors() {
    use sphincs_plus::hash::blake::blake256::blake256_oneshot;
    let mut out = [0u8; 32];
    blake256_oneshot(&mut out, b"abc");
    let hex: String = out.iter().map(|b| format!("{:02x}", b)).collect();
    assert_eq!(hex, "1833a9fa7cf4086bd5fda73da32e5a1d75b4c3f89d5c436369f9d78bb2da5c28", "abc");

    let mut out = [0u8; 32];
    blake256_oneshot(&mut out, b"");
    let hex: String = out.iter().map(|b| format!("{:02x}", b)).collect();
    assert_eq!(hex, "716f6e863f744b9ac22c97ec7b76ea5f5908bc5b2f67c61510bfc4751384ea7a", "empty");
}

#[test]
#[cfg(feature = "blake")]
fn blake256_mgf1() {
    use sphincs_plus::hash::blake::blake256::blake256_mgf1_inner;
    let mut out = [0u8; 64];
    blake256_mgf1_inner(&mut out, b"hello world");
    let hex: String = out.iter().map(|b| format!("{:02x}", b)).collect();
    assert_eq!(hex, "e66a08cb6ae92daa06393c99a6d5be5818b96ce74b2387c62cc7ecba4f0ba74c7ce64ea72f8ffbb03802d7050d33e58396f3af92444fd1f89081783901f4ccd7");
}
