use merkle_tree_c::blake2b::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn new_state() -> Blake2bState {
    Blake2bState {
        h: [0; 8],
        t: [0; 2],
        f: [0; 2],
        buf: [0; BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    }
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
fn test_blake2b_streaming_api() {
    let mut s = new_state();
    assert_eq!(blake2b_init(&mut s, 32), 0);
    assert_eq!(blake2b_update(&mut s, b"abc"), 0);
    let mut out = [0u8; 32];
    assert_eq!(blake2b_final(&mut s, &mut out), 0);
    assert_eq!(hex(&out), "521c604cc09b814b0a9106305395def35d0211b9996a3e0f326ae4d671bd8fc2");
}

#[test]
fn test_blake2b_keyed() {
    let mut out = [0u8; 32];
    let ret = blake2b(&mut out, b"abc", Some(b"secretkey"));
    assert_eq!(ret, 0);
    assert_eq!(hex(&out), "09825893fbed484d66485fa6be28f98bccd8c932b77501535196e6a121579191");
}

#[test]
fn test_blake2b_64_byte_output() {
    let mut out = [0u8; 64];
    let ret = blake2b(&mut out, b"hello", None);
    assert_eq!(ret, 0);
    assert_eq!(hex(&out), "a1e60e2fbb09f4f071f4e3cc30791fcdd694bfda60223c5b3912ae3d762a6ba59c9e90e9fd185c10eb545a4ca86a9bdc72539d5160576707a43760f4b50013ba");
}

#[test]
fn test_blake2b_zero_outlen() {
    let mut out = [0u8; 0];
    assert_eq!(blake2b(&mut out, &[], None), -1);
}

#[test]
fn test_blake2b_big_outlen() {
    let mut out = [0u8; 65];
    assert_eq!(blake2b(&mut out, &[], None), -1);
}

#[test]
fn test_blake2b_double_final() {
    let mut s = new_state();
    blake2b_init(&mut s, 32);
    blake2b_update(&mut s, b"test");
    let mut out = [0u8; 32];
    blake2b_final(&mut s, &mut out);
    assert_eq!(blake2b_final(&mut s, &mut out), -1);
}

#[test]
fn test_blake2b_init_key() {
    let mut s = new_state();
    let ret = blake2b_init_key(&mut s, 32, b"mykey");
    assert_eq!(ret, 0);
    assert_eq!(s.outlen, 32);
}

#[test]
fn test_blake2b_init_key_empty_key() {
    let mut s = new_state();
    assert_eq!(blake2b_init_key(&mut s, 32, b""), -1);
}

#[test]
fn test_blake2b_init_zero_outlen() {
    let mut s = new_state();
    assert_eq!(blake2b_init(&mut s, 0), -1);
}

#[test]
fn test_blake2b_init_too_large_outlen() {
    let mut s = new_state();
    assert_eq!(blake2b_init(&mut s, BLAKE2B_OUTBYTES + 1), -1);
}

#[test]
fn test_blake2_alias() {
    let mut out = [0u8; 32];
    let ret = blake2(&mut out, b"abc", None);
    assert_eq!(ret, 0);
    assert_eq!(hex(&out), "521c604cc09b814b0a9106305395def35d0211b9996a3e0f326ae4d671bd8fc2");
}

fn main() {}
