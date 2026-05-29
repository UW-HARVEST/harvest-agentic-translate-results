use merkle_tree_c::blake2b;

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn test_constants() {
    assert_eq!(blake2b::BLAKE2B_BLOCKBYTES, 128);
    assert_eq!(blake2b::BLAKE2B_OUTBYTES, 64);
    assert_eq!(blake2b::BLAKE2B_KEYBYTES, 64);
    assert_eq!(blake2b::BLAKE2B_SALTBYTES, 16);
    assert_eq!(blake2b::BLAKE2B_PERSONALBYTES, 16);
}

#[test]
fn test_blake2b_empty_32() {
    let mut out = [0u8; 32];
    let ret = blake2b::blake2b(&mut out, b"", None);
    assert_eq!(ret, 0);
    let expected = hex_to_bytes("44f4c69744d5f8c55d642062949dcae49bc4e7ef43d388c5a12f42b5633d163e");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_empty_64() {
    let mut out = [0u8; 64];
    let ret = blake2b::blake2b(&mut out, b"", None);
    assert_eq!(ret, 0);
    let expected = hex_to_bytes(
        "8e5e657ab293b4f6146feed495bf87c4c3c5e0cfca6aef78f924311866ea277bf359afae4a763af955e23abdad3f9c941c9e4a0a795c73d8b205679ab68eb294",
    );
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_abc_32() {
    let mut out = [0u8; 32];
    let ret = blake2b::blake2b(&mut out, b"abc", None);
    assert_eq!(ret, 0);
    let expected = hex_to_bytes("521c604cc09b814b0a9106305395def35d0211b9996a3e0f326ae4d671bd8fc2");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_hello_world_32() {
    let mut out = [0u8; 32];
    let ret = blake2b::blake2b(&mut out, b"hello world", None);
    assert_eq!(ret, 0);
    let expected = hex_to_bytes("3376b3e62282513e03d78fc6c5bd555503d0c697bf394d55cd672cc96e6b0a2c");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_with_key_abc_32() {
    let mut out = [0u8; 32];
    let key: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let ret = blake2b::blake2b(&mut out, b"abc", Some(&key));
    assert_eq!(ret, 0);
    let expected = hex_to_bytes("3fd8fd31501cdbe942607b204368e3dc4ebce5744de1a955a6025f5c9848bb20");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_long_input_200_32() {
    let mut out = [0u8; 32];
    let big: Vec<u8> = (0..200u32).map(|i| (i & 0xFF) as u8).collect();
    let ret = blake2b::blake2b(&mut out, &big, None);
    assert_eq!(ret, 0);
    let expected = hex_to_bytes("0134cea72ef7e497466d0b9355b1655f699f2856516752e21332d7fc7698d647");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_streaming_abc_32() {
    let mut state = blake2b::Blake2bState {
        h: [0u64; 8],
        t: [0u64; 2],
        f: [0u64; 2],
        buf: [0u8; blake2b::BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    };
    let r = blake2b::blake2b_init(&mut state, 32);
    assert_eq!(r, 0);
    assert_eq!(state.outlen, 32);

    let r = blake2b::blake2b_update(&mut state, b"ab");
    assert_eq!(r, 0);
    let r = blake2b::blake2b_update(&mut state, b"c");
    assert_eq!(r, 0);

    let mut out = [0u8; 32];
    let r = blake2b::blake2b_final(&mut state, &mut out);
    assert_eq!(r, 0);

    // Same as one-shot
    let expected = hex_to_bytes("521c604cc09b814b0a9106305395def35d0211b9996a3e0f326ae4d671bd8fc2");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_outlen_20() {
    let mut out = [0u8; 20];
    let ret = blake2b::blake2b(&mut out, b"test", None);
    assert_eq!(ret, 0);
    let expected = hex_to_bytes("4f2c4d73b40a2a3f36e2ca2ce741aace1df08d41");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_invalid_outlen_zero() {
    let mut out = [0u8; 0];
    let ret = blake2b::blake2b(&mut out, b"abc", None);
    assert_eq!(ret, -1);
}

#[test]
fn test_blake2b_invalid_outlen_too_large() {
    let mut out = [0u8; 65];
    let ret = blake2b::blake2b(&mut out, b"abc", None);
    assert_eq!(ret, -1);
}

#[test]
fn test_blake2b_invalid_key_too_long() {
    let mut out = [0u8; 32];
    let key = [0u8; 65];
    let ret = blake2b::blake2b(&mut out, b"abc", Some(&key));
    assert_eq!(ret, -1);
}

#[test]
fn test_blake2b_init_bad_outlen() {
    let mut state = blake2b::Blake2bState {
        h: [0u64; 8],
        t: [0u64; 2],
        f: [0u64; 2],
        buf: [0u8; blake2b::BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    };
    let r = blake2b::blake2b_init(&mut state, 0);
    assert_eq!(r, -1);
    let r = blake2b::blake2b_init(&mut state, 65);
    assert_eq!(r, -1);
}

#[test]
fn test_blake2b_init_key_bad_args() {
    let mut state = blake2b::Blake2bState {
        h: [0u64; 8],
        t: [0u64; 2],
        f: [0u64; 2],
        buf: [0u8; blake2b::BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    };
    // outlen=0
    let r = blake2b::blake2b_init_key(&mut state, 0, b"key");
    assert_eq!(r, -1);
    // outlen too large
    let r = blake2b::blake2b_init_key(&mut state, 65, b"key");
    assert_eq!(r, -1);
    // empty key
    let r = blake2b::blake2b_init_key(&mut state, 32, b"");
    assert_eq!(r, -1);
    // key too long
    let key = [0u8; 65];
    let r = blake2b::blake2b_init_key(&mut state, 32, &key);
    assert_eq!(r, -1);
}

#[test]
fn test_blake2_alias() {
    let mut out_a = [0u8; 32];
    let mut out_b = [0u8; 32];
    blake2b::blake2b(&mut out_a, b"abc", None);
    blake2b::blake2(&mut out_b, b"abc", None);
    assert_eq!(out_a, out_b);
}

#[test]
fn test_blake2b_streaming_long_input() {
    // Streaming should match one-shot for >block-size input
    let big: Vec<u8> = (0..200u32).map(|i| (i & 0xFF) as u8).collect();

    let mut state = blake2b::Blake2bState {
        h: [0u64; 8],
        t: [0u64; 2],
        f: [0u64; 2],
        buf: [0u8; blake2b::BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    };
    blake2b::blake2b_init(&mut state, 32);
    blake2b::blake2b_update(&mut state, &big[..50]);
    blake2b::blake2b_update(&mut state, &big[50..]);
    let mut out = [0u8; 32];
    blake2b::blake2b_final(&mut state, &mut out);

    let expected = hex_to_bytes("0134cea72ef7e497466d0b9355b1655f699f2856516752e21332d7fc7698d647");
    assert_eq!(out.to_vec(), expected);
}

#[test]
fn test_blake2b_final_too_small_buf() {
    let mut state = blake2b::Blake2bState {
        h: [0u64; 8],
        t: [0u64; 2],
        f: [0u64; 2],
        buf: [0u8; blake2b::BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    };
    blake2b::blake2b_init(&mut state, 32);
    blake2b::blake2b_update(&mut state, b"abc");
    let mut out = [0u8; 16]; // too small (< outlen 32)
    let r = blake2b::blake2b_final(&mut state, &mut out);
    assert_eq!(r, -1);
}

fn main() {}
