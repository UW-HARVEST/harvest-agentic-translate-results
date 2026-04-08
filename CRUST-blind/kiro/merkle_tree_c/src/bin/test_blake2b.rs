use merkle_tree_c::blake2b::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[test]
fn test_blake2b_empty_input() {
    let mut out = [0u8; 32];
    let ret = blake2b(&mut out, &[], None);
    assert_eq!(ret, 0);
    assert_eq!(hex(&out), "44f4c69744d5f8c55d642062949dcae49bc4e7ef43d388c5a12f42b5633d163e");
}

#[test]
fn test_blake2b_abc() {
    let mut out = [0u8; 32];
    let ret = blake2b(&mut out, b"abc", None);
    assert_eq!(ret, 0);
    assert_eq!(hex(&out), "521c604cc09b814b0a9106305395def35d0211b9996a3e0f326ae4d671bd8fc2");
}

#[test]
fn test_blake2b_with_key() {
    let mut out = [0u8; 32];
    let ret = blake2b(&mut out, b"hello", Some(b"secretkey"));
    assert_eq!(ret, 0);
    assert_eq!(hex(&out), "e3a5e7bff6aad98ef045663228ad8d9d223c2cfa1dc11cf3f01f22a0296e9205");
}

#[test]
fn test_blake2b_streaming_matches_oneshot() {
    // Streaming
    let mut state = Blake2bState {
        h: [0; 8], t: [0; 2], f: [0; 2],
        buf: [0; BLAKE2B_BLOCKBYTES], buflen: 0, outlen: 0, last_node: 0,
    };
    blake2b_init(&mut state, 32);
    blake2b_update(&mut state, b"hello ");
    blake2b_update(&mut state, b"world");
    let mut out_stream = [0u8; 32];
    blake2b_final(&mut state, &mut out_stream);

    // One-shot
    let mut out_oneshot = [0u8; 32];
    blake2b(&mut out_oneshot, b"hello world", None);

    assert_eq!(out_stream, out_oneshot);
    assert_eq!(hex(&out_stream), "3376b3e62282513e03d78fc6c5bd555503d0c697bf394d55cd672cc96e6b0a2c");
}

#[test]
fn test_blake2b_outlen_zero() {
    let mut out = [0u8; 0];
    let ret = blake2b(&mut out, b"x", None);
    assert_eq!(ret, -1);
}

#[test]
fn test_blake2b_outlen_too_large() {
    let mut out = [0u8; 65];
    let ret = blake2b(&mut out, b"x", None);
    assert_eq!(ret, -1);
}

#[test]
fn test_blake2_alias() {
    let mut out1 = [0u8; 32];
    let mut out2 = [0u8; 32];
    blake2b(&mut out1, b"test", None);
    blake2(&mut out2, b"test", None);
    assert_eq!(out1, out2);
}

#[test]
fn test_blake2b_init_key() {
    let mut state = Blake2bState {
        h: [0; 8], t: [0; 2], f: [0; 2],
        buf: [0; BLAKE2B_BLOCKBYTES], buflen: 0, outlen: 0, last_node: 0,
    };
    let ret = blake2b_init_key(&mut state, 32, b"mykey");
    assert_eq!(ret, 0);

    // Empty key should fail
    let mut state2 = state.clone();
    let ret2 = blake2b_init_key(&mut state2, 32, b"");
    assert_eq!(ret2, -1);
}

fn main() {}
