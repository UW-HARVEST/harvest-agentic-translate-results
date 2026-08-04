use aes128_SIMD::matrix;

#[test]
fn test_columns_fips_example() {
    // Standard FIPS 197 MixColumns test vector.
    let mut state: [[u8; 4]; 4] = [
        [0xd4, 0xe0, 0xb8, 0x1e],
        [0xbf, 0xb4, 0x41, 0x27],
        [0x5d, 0x52, 0x11, 0x98],
        [0x30, 0xae, 0xf1, 0xe5],
    ];
    matrix::columns(&mut state);
    // Expected output captured by running the C `Columns` function.
    assert_eq!(state[0], [0x04, 0xe0, 0x48, 0x28]);
    assert_eq!(state[1], [0x66, 0xcb, 0xf8, 0x06]);
    assert_eq!(state[2], [0x81, 0x19, 0xd3, 0x26]);
    assert_eq!(state[3], [0xe5, 0x9a, 0x7a, 0x4c]);
}

#[test]
fn test_inv_columns_fips_example() {
    // Inverse of the test_columns_fips_example case.
    let mut state: [[u8; 4]; 4] = [
        [0x04, 0xe0, 0x48, 0x28],
        [0x66, 0xcb, 0xf8, 0x06],
        [0x81, 0x19, 0xd3, 0x26],
        [0xe5, 0x9a, 0x7a, 0x4c],
    ];
    matrix::inv_columns(&mut state);
    assert_eq!(state[0], [0xd4, 0xe0, 0xb8, 0x1e]);
    assert_eq!(state[1], [0xbf, 0xb4, 0x41, 0x27]);
    assert_eq!(state[2], [0x5d, 0x52, 0x11, 0x98]);
    assert_eq!(state[3], [0x30, 0xae, 0xf1, 0xe5]);
}

#[test]
fn test_columns_zero_state_unchanged() {
    let mut state: [[u8; 4]; 4] = [[0u8; 4]; 4];
    matrix::columns(&mut state);
    assert_eq!(state, [[0u8; 4]; 4]);
}

#[test]
fn test_columns_inv_columns_roundtrip() {
    let original: [[u8; 4]; 4] = [
        [0x12, 0x34, 0x56, 0x78],
        [0x9a, 0xbc, 0xde, 0xf0],
        [0x11, 0x22, 0x33, 0x44],
        [0x55, 0x66, 0x77, 0x88],
    ];
    let mut s = original;
    matrix::columns(&mut s);
    matrix::inv_columns(&mut s);
    assert_eq!(s, original);
}

fn main() {}
