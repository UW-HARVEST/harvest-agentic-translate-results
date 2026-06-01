use merkle_tree_c::blake2b::*;

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn bytes_to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

#[test]
fn test_blake2b_empty_64() {
    // blake2b("", outlen=64) using ckb-default-hash personal
    let mut out = vec![0u8; 64];
    let r = blake2b(&mut out, b"", None);
    assert_eq!(r, 0);
    let expected = hex_to_bytes(
        "8e5e657ab293b4f6146feed495bf87c4c3c5e0cfca6aef78f924311866ea277bf359afae4a763af955e23abdad3f9c941c9e4a0a795c73d8b205679ab68eb294",
    );
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_abc_32() {
    let mut out = vec![0u8; 32];
    let r = blake2b(&mut out, b"abc", None);
    assert_eq!(r, 0);
    let expected = hex_to_bytes("521c604cc09b814b0a9106305395def35d0211b9996a3e0f326ae4d671bd8fc2");
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_hello_world_64() {
    let mut out = vec![0u8; 64];
    let r = blake2b(&mut out, b"hello world", None);
    assert_eq!(r, 0);
    let expected = hex_to_bytes(
        "cdc428c728a9e2354c6db496e490c58a121d8a6e607f84314531356f6800bafc3ca2cd65d0ee8613d9da2cedb72f4e49a1130cfd62951cf2a2d45b0b4fcfa6d6",
    );
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_streaming() {
    let mut state = Blake2bState::default();
    let r = blake2b_init(&mut state, 32);
    assert_eq!(r, 0);
    assert_eq!(state.outlen, 32);
    let r = blake2b_update(&mut state, b"hello ");
    assert_eq!(r, 0);
    let r = blake2b_update(&mut state, b"world");
    assert_eq!(r, 0);
    let mut out = vec![0u8; 32];
    let r = blake2b_final(&mut state, &mut out);
    assert_eq!(r, 0);
    let expected = hex_to_bytes("3376b3e62282513e03d78fc6c5bd555503d0c697bf394d55cd672cc96e6b0a2c");
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_keyed() {
    let mut out = vec![0u8; 32];
    let key = b"1234567890123456";
    let r = blake2b(&mut out, b"abc", Some(key));
    assert_eq!(r, 0);
    let expected = hex_to_bytes("3179ce5369fc8770d4d3cf853ab5df61a467151ed0fc3e64986cb4a3b3fe55ac");
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_long_input() {
    // 200 bytes of 'A'
    let input = vec![b'A'; 200];
    let mut out = vec![0u8; 64];
    let r = blake2b(&mut out, &input, None);
    assert_eq!(r, 0);
    let expected = hex_to_bytes(
        "8eb4fbe592289d1b4b3691c2fcb5ff939439ee0e2efd15b2663a9db18c973185750e46cf0876d8fd77ba94f930ec03f1a0a228e365197e17ac18ac15d38131bc",
    );
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_exactly_128_bytes() {
    // 128 bytes (one block) with values 0..127
    let input: Vec<u8> = (0..128u8).collect();
    let mut out = vec![0u8; 64];
    let r = blake2b(&mut out, &input, None);
    assert_eq!(r, 0);
    let expected = hex_to_bytes(
        "1f23748660a74f1bc769943501048c1cf5843aa953c77847b0d9a313ba0714f0083478a8dc3479192517e8ad1bbf09c1869c7c5450755b868df4eb19442bb4db",
    );
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_129_bytes() {
    let input: Vec<u8> = (0..129u32).map(|i| (i & 0xff) as u8).collect();
    let mut out = vec![0u8; 64];
    let r = blake2b(&mut out, &input, None);
    assert_eq!(r, 0);
    let expected = hex_to_bytes(
        "52ec2b1c2120e7b55e23e6e4ccd6c874bab9828c52a654c816f3adf8e25c207e66c8e56903fe6aecb5affbfc01c65df07d75f8b4422e30db6abb7e5200f0c256",
    );
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_outlen_1() {
    let mut out = vec![0u8; 1];
    let r = blake2b(&mut out, b"abc", None);
    assert_eq!(r, 0);
    assert_eq!(out[0], 0xc0);
}

#[test]
fn test_blake2b_outlen_64_abc() {
    let mut out = vec![0u8; 64];
    let r = blake2b(&mut out, b"abc", None);
    assert_eq!(r, 0);
    let expected = hex_to_bytes(
        "bba98c737f4cbaeda1620bef511666bd9b52dbc230bac323709cbe5bd8957f373e0080ceb828db30a1f6625f90f4a430ff04f1632541d74f9d0234b645ed5d4d",
    );
    assert_eq!(out, expected);
}

#[test]
fn test_blake2b_outlen_zero_error() {
    let mut out = vec![0u8; 0];
    // outlen=0 returns -1
    let r = blake2b(&mut out, b"abc", None);
    assert_eq!(r, -1);
}

#[test]
fn test_blake2b_outlen_too_large_error() {
    let mut out = vec![0u8; 65];
    // out.len() > BLAKE2B_OUTBYTES => -1
    let r = blake2b(&mut out, b"abc", None);
    assert_eq!(r, -1);
}

#[test]
fn test_blake2b_init_invalid_outlen() {
    let mut state = Blake2bState::default();
    assert_eq!(blake2b_init(&mut state, 0), -1);
    assert_eq!(blake2b_init(&mut state, 65), -1);
    // valid
    assert_eq!(blake2b_init(&mut state, 32), 0);
}

#[test]
fn test_blake2b_init_key_invalid() {
    let mut state = Blake2bState::default();
    let key = [1u8; 16];
    assert_eq!(blake2b_init_key(&mut state, 0, &key), -1);
    assert_eq!(blake2b_init_key(&mut state, 65, &key), -1);
    assert_eq!(blake2b_init_key(&mut state, 32, &[]), -1);
    let bigkey = [1u8; 65];
    assert_eq!(blake2b_init_key(&mut state, 32, &bigkey), -1);
    assert_eq!(blake2b_init_key(&mut state, 32, &key), 0);
}

#[test]
fn test_blake2_alias() {
    // blake2 is an alias for blake2b
    let mut out1 = vec![0u8; 32];
    let mut out2 = vec![0u8; 32];
    blake2b(&mut out1, b"abc", None);
    blake2(&mut out2, b"abc", None);
    assert_eq!(out1, out2);
    assert_eq!(
        bytes_to_hex(&out2),
        "521c604cc09b814b0a9106305395def35d0211b9996a3e0f326ae4d671bd8fc2"
    );
}

#[test]
fn test_blake2b_constants() {
    assert_eq!(BLAKE2B_BLOCKBYTES, 128);
    assert_eq!(BLAKE2B_OUTBYTES, 64);
    assert_eq!(BLAKE2B_KEYBYTES, 64);
    assert_eq!(BLAKE2B_SALTBYTES, 16);
    assert_eq!(BLAKE2B_PERSONALBYTES, 16);
}

fn main() {}
