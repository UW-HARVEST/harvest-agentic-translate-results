#![allow(
    non_snake_case,
    non_upper_case_globals,
    clippy::identity_op,
    clippy::needless_range_loop,
    clippy::manual_memcpy,
    clippy::too_many_arguments
)]

use std::ptr;

// ============================================================
// params (blake-256s)
// ============================================================
const SPX_N: usize = 32;
const SPX_FULL_HEIGHT: usize = 64;
const SPX_D: usize = 8;
const SPX_FORS_HEIGHT: usize = 14;
const SPX_FORS_TREES: usize = 22;
const SPX_WOTS_W: usize = 16;
const SPX_WOTS_LOGW: usize = 4;
const SPX_ADDR_BYTES: usize = 32;

const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
const SPX_WOTS_LEN2: usize = 3;
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;

const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;

const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

const SPX_OFFSET_LAYER: usize = 3;
const SPX_OFFSET_TREE: usize = 8;
const SPX_OFFSET_TYPE: usize = 19;
const SPX_OFFSET_KP_ADDR: usize = 20;
const SPX_OFFSET_CHAIN_ADDR: usize = 27;
const SPX_OFFSET_HASH_ADDR: usize = 31;
const SPX_OFFSET_TREE_HGT: usize = 27;
const SPX_OFFSET_TREE_INDEX: usize = 28;

const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;

const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// ============================================================
// context
// ============================================================
#[derive(Clone)]
struct SpxCtx {
    pub_seed: [u8; SPX_N],
    sk_seed: [u8; SPX_N],
}

// ============================================================
// address helpers
// ============================================================
fn addr_as_bytes(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}
fn addr_as_bytes_ref(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}

fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_as_bytes(addr)[SPX_OFFSET_LAYER] = layer as u8;
}
fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let b = addr_as_bytes(addr);
    ull_to_bytes(&mut b[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8], 8, tree);
}
fn set_type(addr: &mut [u32; 8], t: u32) {
    addr_as_bytes(addr)[SPX_OFFSET_TYPE] = t as u8;
}
fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let s = addr_as_bytes_ref(inp);
    let d = addr_as_bytes(out);
    d[..SPX_OFFSET_TREE + 8].copy_from_slice(&s[..SPX_OFFSET_TREE + 8]);
}
fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let b = addr_as_bytes(addr);
    u32_to_bytes_slice(&mut b[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4], keypair);
}
fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let s = addr_as_bytes_ref(inp);
    let d = addr_as_bytes(out);
    d[..SPX_OFFSET_TREE + 8].copy_from_slice(&s[..SPX_OFFSET_TREE + 8]);
    d[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&s[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}
fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_as_bytes(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}
fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_as_bytes(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}
fn set_tree_height(addr: &mut [u32; 8], h: u32) {
    addr_as_bytes(addr)[SPX_OFFSET_TREE_HGT] = h as u8;
}
fn set_tree_index(addr: &mut [u32; 8], idx: u32) {
    let b = addr_as_bytes(addr);
    u32_to_bytes_slice(&mut b[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4], idx);
}

// ============================================================
// utils
// ============================================================
fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}
fn u32_to_bytes_slice(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}
fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut r: u64 = 0;
    for i in 0..inlen {
        r |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    r
}

// ============================================================
// BLAKE-256
// ============================================================
static PADDING256: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

static CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

#[derive(Clone)]
struct BlakeState256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

fn blake256_compress(st: &mut BlakeState256, block: &[u8]) {
    let m: [u32; 16] = core::array::from_fn(|i| u8to32(&block[i * 4..]));

    let mut v = [0u32; 16];
    v[..8].copy_from_slice(&st.h);
    v[8] = st.s[0] ^ 0x243F6A88;
    v[9] = st.s[1] ^ 0x85A308D3;
    v[10] = st.s[2] ^ 0x13198A2E;
    v[11] = st.s[3] ^ 0x03707344;
    v[12] = 0xA4093822;
    v[13] = 0x299F31D0;
    v[14] = 0x082EFA98;
    v[15] = 0xEC4E6C89;
    if st.nullt == 0 {
        v[12] ^= st.t[0];
        v[13] ^= st.t[0];
        v[14] ^= st.t[1];
        v[15] ^= st.t[1];
    }

    // The C ROUND macro takes (m0,c0,m1,c1,...,m15,c15) where each pair
    // is (message_word, constant). The 14 round invocations pass specific
    // permutations. We encode each round as [msg_idx, cst_idx] pairs for
    // positions 0..15 in the ROUND macro parameter list.
    // From the C code, each ROUND(mA,cst[B],...) means msg_idx=A, cst_idx=B.
    // The ROUND macro body uses these as:
    //   Column: G(0,4,8,12, pair0, pair1), G(1,5,9,13, pair2, pair3),
    //           G(2,6,10,14, pair4, pair5), G(3,7,11,15, pair6, pair7)
    //   Diagonal: G(0,5,10,15, pair8, pair9), G(1,6,11,12, pair10, pair11),
    //             G(2,7,8,13, pair12, pair13), G(3,4,9,14, pair14, pair15)
    // Each G(a,b,c,d, (mX,cY), (mZ,cW)) does:
    //   va += mX^cY; va += vb; vd ^= va; vd = ROT(vd,16);
    //   vc += vd; vb ^= vc; vb = ROT(vb,12);
    //   va += mZ^cW; va += vb; vd ^= va; vd = ROT(vd,8);
    //   vc += vd; vb ^= vc; vb = ROT(vb,7);

    // rounds[r] = [(msg_idx, cst_idx); 16] for the 16 ROUND parameters
    static ROUNDS256: [[(usize, usize); 16]; 14] = [
        [(0,1),(1,0),(2,3),(3,2),(4,5),(5,4),(6,7),(7,6),(8,9),(9,8),(10,11),(11,10),(12,13),(13,12),(14,15),(15,14)],
        [(14,10),(10,14),(4,8),(8,4),(9,15),(15,9),(13,6),(6,13),(1,12),(12,1),(0,2),(2,0),(11,7),(7,11),(5,3),(3,5)],
        [(11,8),(8,11),(12,0),(0,12),(5,2),(2,5),(15,13),(13,15),(10,14),(14,10),(3,6),(6,3),(7,1),(1,7),(9,4),(4,9)],
        [(7,9),(9,7),(3,1),(1,3),(13,12),(12,13),(11,14),(14,11),(2,6),(6,2),(5,10),(10,5),(4,0),(0,4),(15,8),(8,15)],
        [(9,0),(0,9),(5,7),(7,5),(2,4),(4,2),(10,15),(15,10),(14,1),(1,14),(11,12),(12,11),(6,8),(8,6),(3,13),(13,3)],
        [(2,12),(12,2),(6,10),(10,6),(0,11),(11,0),(8,3),(3,8),(4,13),(13,4),(7,5),(5,7),(15,14),(14,15),(1,9),(9,1)],
        [(12,5),(5,12),(1,15),(15,1),(14,13),(13,14),(4,10),(10,4),(0,7),(7,0),(6,3),(3,6),(9,2),(2,9),(8,11),(11,8)],
        [(13,11),(11,13),(7,14),(14,7),(12,1),(1,12),(3,9),(9,3),(5,0),(0,5),(15,4),(4,15),(8,6),(6,8),(2,10),(10,2)],
        [(6,15),(15,6),(14,9),(9,14),(11,3),(3,11),(0,8),(8,0),(12,2),(2,12),(13,7),(7,13),(1,4),(4,1),(10,5),(5,10)],
        [(10,2),(2,10),(8,4),(4,8),(7,6),(6,7),(1,5),(5,1),(15,11),(11,15),(9,14),(14,9),(3,12),(12,3),(13,0),(0,13)],
        [(0,1),(1,0),(2,3),(3,2),(4,5),(5,4),(6,7),(7,6),(8,9),(9,8),(10,11),(11,10),(12,13),(13,12),(14,15),(15,14)],
        [(14,10),(10,14),(4,8),(8,4),(9,15),(15,9),(13,6),(6,13),(1,12),(12,1),(0,2),(2,0),(11,7),(7,11),(5,3),(3,5)],
        [(11,8),(8,11),(12,0),(0,12),(5,2),(2,5),(15,13),(13,15),(10,14),(14,10),(3,6),(6,3),(7,1),(1,7),(9,4),(4,9)],
        [(7,9),(9,7),(3,1),(1,3),(13,12),(12,13),(11,14),(14,11),(2,6),(6,2),(5,10),(10,5),(4,0),(0,4),(15,8),(8,15)],
    ];

    for round in &ROUNDS256 {
        macro_rules! g256 {
            ($a:expr, $b:expr, $c:expr, $d:expr, $i1:expr, $i2:expr) => {{
                let (mi1, ci1) = round[$i1];
                let (mi2, ci2) = round[$i2];
                v[$a] = v[$a].wrapping_add(m[mi1] ^ CST256[ci1]);
                v[$a] = v[$a].wrapping_add(v[$b]);
                v[$d] ^= v[$a];
                v[$d] = (v[$d] << 16) | (v[$d] >> 16);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c];
                v[$b] = (v[$b] << 20) | (v[$b] >> 12);
                v[$a] = v[$a].wrapping_add(m[mi2] ^ CST256[ci2]);
                v[$a] = v[$a].wrapping_add(v[$b]);
                v[$d] ^= v[$a];
                v[$d] = (v[$d] << 24) | (v[$d] >> 8);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c];
                v[$b] = (v[$b] << 25) | (v[$b] >> 7);
            }};
        }
        // Column step
        g256!(0, 4, 8, 12, 0, 1);
        g256!(1, 5, 9, 13, 2, 3);
        g256!(2, 6, 10, 14, 4, 5);
        g256!(3, 7, 11, 15, 6, 7);
        // Diagonal step
        g256!(0, 5, 10, 15, 8, 9);
        g256!(1, 6, 11, 12, 10, 11);
        g256!(2, 7, 8, 13, 12, 13);
        g256!(3, 4, 9, 14, 14, 15);
    }

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= st.s[i]; v[i + 4] ^= st.s[i]; }
    for i in 0..8 { st.h[i] ^= v[i]; }
}

fn blake256_init(s: &mut BlakeState256) {
    s.h = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
           0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19];
    s.t = [0; 2]; s.buflen = 0; s.nullt = 0; s.s = [0; 4];
    s.buf = [0; 64];
}

fn blake256_update(s: &mut BlakeState256, data: &[u8], datalen: u64) {
    let mut data = data;
    let mut datalen = datalen;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        blake256_compress(s, data);
        data = &data[64..];
        datalen -= 512;
    }

    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        let oo: u8 = 0x81;
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING256, (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING256, (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING256[1..], 440);
            s.nullt = 1;
        }
        let zo: u8 = 0x01;
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[i * 4..], s.h[i]);
    }
}

fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState256 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
}

fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes_slice(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes_slice(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============================================================
// BLAKE-512
// ============================================================
static CST512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING512: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

#[derive(Clone)]
struct BlakeState512 {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 128],
}

fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}
fn u64to8(p: &mut [u8], v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

fn blake512_compress(st: &mut BlakeState512, block: &[u8]) {
    let m: [u64; 16] = core::array::from_fn(|i| u8to64(&block[i * 8..]));

    let mut v = [0u64; 16];
    v[..8].copy_from_slice(&st.h);
    v[8] = st.s[0] ^ 0x243F6A8885A308D3;
    v[9] = st.s[1] ^ 0x13198A2E03707344;
    v[10] = st.s[2] ^ 0xA4093822299F31D0;
    v[11] = st.s[3] ^ 0x082EFA98EC4E6C89;
    v[12] = 0x452821E638D01377;
    v[13] = 0xBE5466CF34E90C6C;
    v[14] = 0xC0AC29B7C97C50DD;
    v[15] = 0x3F84D5B5B5470917;
    if st.nullt == 0 {
        v[12] ^= st.t[0];
        v[13] ^= st.t[0];
        v[14] ^= st.t[1];
        v[15] ^= st.t[1];
    }

    // Same sigma permutations as blake256 (16 rounds for 512, but C code uses 16)
    static ROUNDS512: [[(usize, usize); 16]; 16] = [
        [(0,1),(1,0),(2,3),(3,2),(4,5),(5,4),(6,7),(7,6),(8,9),(9,8),(10,11),(11,10),(12,13),(13,12),(14,15),(15,14)],
        [(14,10),(10,14),(4,8),(8,4),(9,15),(15,9),(13,6),(6,13),(1,12),(12,1),(0,2),(2,0),(11,7),(7,11),(5,3),(3,5)],
        [(11,8),(8,11),(12,0),(0,12),(5,2),(2,5),(15,13),(13,15),(10,14),(14,10),(3,6),(6,3),(7,1),(1,7),(9,4),(4,9)],
        [(7,9),(9,7),(3,1),(1,3),(13,12),(12,13),(11,14),(14,11),(2,6),(6,2),(5,10),(10,5),(4,0),(0,4),(15,8),(8,15)],
        [(9,0),(0,9),(5,7),(7,5),(2,4),(4,2),(10,15),(15,10),(14,1),(1,14),(11,12),(12,11),(6,8),(8,6),(3,13),(13,3)],
        [(2,12),(12,2),(6,10),(10,6),(0,11),(11,0),(8,3),(3,8),(4,13),(13,4),(7,5),(5,7),(15,14),(14,15),(1,9),(9,1)],
        [(12,5),(5,12),(1,15),(15,1),(14,13),(13,14),(4,10),(10,4),(0,7),(7,0),(6,3),(3,6),(9,2),(2,9),(8,11),(11,8)],
        [(13,11),(11,13),(7,14),(14,7),(12,1),(1,12),(3,9),(9,3),(5,0),(0,5),(15,4),(4,15),(8,6),(6,8),(2,10),(10,2)],
        [(6,15),(15,6),(14,9),(9,14),(11,3),(3,11),(0,8),(8,0),(12,2),(2,12),(13,7),(7,13),(1,4),(4,1),(10,5),(5,10)],
        [(10,2),(2,10),(8,4),(4,8),(7,6),(6,7),(1,5),(5,1),(15,11),(11,15),(9,14),(14,9),(3,12),(12,3),(13,0),(0,13)],
        [(0,1),(1,0),(2,3),(3,2),(4,5),(5,4),(6,7),(7,6),(8,9),(9,8),(10,11),(11,10),(12,13),(13,12),(14,15),(15,14)],
        [(14,10),(10,14),(4,8),(8,4),(9,15),(15,9),(13,6),(6,13),(1,12),(12,1),(0,2),(2,0),(11,7),(7,11),(5,3),(3,5)],
        [(11,8),(8,11),(12,0),(0,12),(5,2),(2,5),(15,13),(13,15),(10,14),(14,10),(3,6),(6,3),(7,1),(1,7),(9,4),(4,9)],
        [(7,9),(9,7),(3,1),(1,3),(13,12),(12,13),(11,14),(14,11),(2,6),(6,2),(5,10),(10,5),(4,0),(0,4),(15,8),(8,15)],
        [(9,0),(0,9),(5,7),(7,5),(2,4),(4,2),(10,15),(15,10),(14,1),(1,14),(11,12),(12,11),(6,8),(8,6),(3,13),(13,3)],
        [(2,12),(12,2),(6,10),(10,6),(0,11),(11,0),(8,3),(3,8),(4,13),(13,4),(7,5),(5,7),(15,14),(14,15),(1,9),(9,1)],
    ];

    for round in &ROUNDS512 {
        macro_rules! g512 {
            ($a:expr, $b:expr, $c:expr, $d:expr, $i1:expr, $i2:expr) => {{
                let (mi1, ci1) = round[$i1];
                let (mi2, ci2) = round[$i2];
                v[$a] = v[$a].wrapping_add(m[mi1] ^ CST512[ci1]);
                v[$a] = v[$a].wrapping_add(v[$b]);
                v[$d] ^= v[$a];
                v[$d] = (v[$d] << 32) | (v[$d] >> 32);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c];
                v[$b] = (v[$b] << 39) | (v[$b] >> 25);
                v[$a] = v[$a].wrapping_add(m[mi2] ^ CST512[ci2]);
                v[$a] = v[$a].wrapping_add(v[$b]);
                v[$d] ^= v[$a];
                v[$d] = (v[$d] << 48) | (v[$d] >> 16);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c];
                v[$b] = (v[$b] << 53) | (v[$b] >> 11);
            }};
        }
        g512!(0, 4, 8, 12, 0, 1);
        g512!(1, 5, 9, 13, 2, 3);
        g512!(2, 6, 10, 14, 4, 5);
        g512!(3, 7, 11, 15, 6, 7);
        g512!(0, 5, 10, 15, 8, 9);
        g512!(1, 6, 11, 12, 10, 11);
        g512!(2, 7, 8, 13, 12, 13);
        g512!(3, 4, 9, 14, 14, 15);
    }

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= st.s[i]; v[i + 4] ^= st.s[i]; }
    for i in 0..8 { st.h[i] ^= v[i]; }
}

fn blake512_init(s: &mut BlakeState512) {
    s.h = [0x6A09E667F3BCC908, 0xBB67AE8584CAA73B,
           0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
           0x510E527FADE682D1, 0x9B05688C2B3E6C1F,
           0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179];
    s.t = [0; 2]; s.buflen = 0; s.nullt = 0; s.s = [0; 4];
    s.buf = [0; 128];
}

fn blake512_update(s: &mut BlakeState512, data: &[u8], datalen: u64) {
    let mut data = data;
    let mut datalen = datalen;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, data);
        data = &data[128..];
        datalen -= 1024;
    }

    if datalen > 0 {
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

fn blake512_final(s: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 { hi = hi.wrapping_add(1); }
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        let oo: u8 = 0x81;
        blake512_update(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, &PADDING512, (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, &PADDING512, (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING512[1..], 888);
            s.nullt = 1;
        }
        let zo: u8 = 0x01;
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[i * 8..], s.h[i]);
    }
}

fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState512 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
}

fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes_slice(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes_slice(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============================================================
// hash_blake.c: initialize_hash_function, prf_addr, gen_message_random, hash_message
// For N>=24, blakeX = blake512
// ============================================================
fn initialize_hash_function(_ctx: &mut SpxCtx) {}

fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes_ref(addr));
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    // C code: blake256(outbuf, buf, SPX_N + SPX_ADDR_BYTES) — note: does NOT include sk_seed in length
    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// gen_message_random: NOTE the C code passes byte counts to blakeX_update
// which expects BIT counts. This is a bug in the C code that we reproduce exactly.
fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
                      m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut s = BlakeState512 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, sk_prf, SPX_N as u64);
    blake512_update(&mut s, optrand, SPX_N as u64);
    blake512_update(&mut s, m, mlen);
    blake512_final(&mut s, r);
}

fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                r_val: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeState512 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    // C code passes byte counts to update (bug reproduced)
    blake512_update(&mut s, r_val, SPX_N as u64);
    blake512_update(&mut s, pk, SPX_PK_BYTES as u64);
    blake512_update(&mut s, m, mlen);
    blake512_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    off += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ============================================================
// thash_blake_simple.c
// ============================================================
fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes_ref(addr));
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake256(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes_ref(addr));
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake512(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================
// randombytes.c
// ============================================================
fn randombytes(x: &mut [u8], mut xlen: u64) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("open /dev/urandom");
    let mut off = 0usize;
    while xlen > 0 {
        let chunk = if xlen < 1048576 { xlen as usize } else { 1048576 };
        match f.read(&mut x[off..off + chunk]) {
            Ok(n) if n > 0 => { off += n; xlen -= n as u64; }
            _ => { std::thread::sleep(std::time::Duration::from_secs(1)); }
        }
    }
}

// ============================================================
// wots.c
// ============================================================
fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32,
             ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    let mut i = start;
    while i < start.wrapping_add(steps) && i < SPX_WOTS_W as u32 {
        set_hash_addr(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash(out, &tmp, 1, ctx, addr);
        i += 1;
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut inp_idx = 0usize;
    let mut out_idx = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;
    for _ in 0..out_len {
        if bits == 0 {
            total = input[inp_idx];
            inp_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) as u32) & (SPX_WOTS_W as u32 - 1);
        out_idx += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; 4]; // max needed
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(&mut lengths[..SPX_WOTS_LEN1], SPX_WOTS_LEN1, msg);
    let (first, second) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(second, first);
}

fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8],
                    ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(&mut pk[i * SPX_N..], &sig[i * SPX_N..],
                  lengths[i], SPX_WOTS_W as u32 - 1 - lengths[i], ctx, addr);
    }
}

// ============================================================
// wotsx1.c
// ============================================================
struct LeafInfoX1 {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *const u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(buffer, ctx, &info.leaf_addr);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                unsafe {
                    ptr::copy_nonoverlapping(buffer.as_ptr(),
                        info.wots_sig.add(i * SPX_N), SPX_N);
                }
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash(buffer, &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

// ============================================================
// fors.c
// ============================================================
struct ForsGenLeafInfo {
    leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    prf_addr(sk, ctx, addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, addr);
}

fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp = leaf[..SPX_N].to_vec();
    fors_sk_to_leaf(leaf, &tmp, ctx, &mut info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32; SPX_FORS_TREES], m: &[u8]) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        // Use a LeafInfoX1-like approach for fors_treehashx1
        fors_treehashx1(&mut roots[i * SPX_N..], &mut sig[sig_off..], ctx,
                        indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
                        &mut fors_tree_addr, &mut fors_info);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        compute_root(&mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
                     &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// ============================================================
// utils.c: compute_root
// ============================================================
fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
                auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
    }
    ap_off += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let tmp = buffer;
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        }
        ap_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

// ============================================================
// utilsx1.c: wots_treehashx1, fors_treehashx1
// ============================================================
fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let ap_off = h as usize * SPX_N;
                auth_path[ap_off..ap_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let st_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[st_off..st_off + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        // Save left child
        let h_val = {
            // Find h where we broke out
            let mut ii = idx;
            let mut h = 0u32;
            loop {
                if (ii & 1) == 0 && idx < max_idx { break; }
                if h == tree_height { break; }
                ii >>= 1;
                h += 1;
            }
            h
        };
        let st_off = h_val as usize * SPX_N;
        stack[st_off..st_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let ap_off = h as usize * SPX_N;
                auth_path[ap_off..ap_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let st_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[st_off..st_off + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        let h_val = {
            let mut ii = idx;
            let mut h = 0u32;
            loop {
                if (ii & 1) == 0 && idx < max_idx { break; }
                if h == tree_height { break; }
                ii >>= 1;
                h += 1;
            }
            h
        };
        let st_off = h_val as usize * SPX_N;
        stack[st_off..st_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

// ============================================================
// merkle.c
// ============================================================
fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
               wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let auth_path = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: sig.as_mut_ptr(),
        wots_sign_leaf: idx_leaf,
        wots_steps: ptr::null(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);
    info.wots_steps = steps.as_ptr();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    wots_treehashx1(root, &mut sig[auth_path..], ctx,
                    idx_leaf, 0, SPX_TREE_HEIGHT as u32,
                    tree_addr, &mut info);
}

fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(&mut auth_path, root, ctx,
                &mut wots_addr, &mut top_tree_addr, !0u32);
}

// ============================================================
// sign.c — public API
// ============================================================
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    unsafe {
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);

        sk_s[..CRYPTO_SEEDBYTES].copy_from_slice(seed_s);
        pk_s[..SPX_N].copy_from_slice(&sk_s[2 * SPX_N..3 * SPX_N]);

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
        initialize_hash_function(&mut ctx);

        merkle_gen_root(&mut sk_s[3 * SPX_N..], &ctx);
        pk_s[SPX_N..2 * SPX_N].copy_from_slice(&sk_s[3 * SPX_N..4 * SPX_N]);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    unsafe {
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);

        let sk_prf = &sk_s[SPX_N..2 * SPX_N];
        let pk_part = &sk_s[2 * SPX_N..];

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk_part[..SPX_N]);
        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

        let mut optrand = [0u8; SPX_N];
        randombytes(&mut optrand, SPX_N as u64);

        gen_message_random(sig_s, sk_prf, &optrand, m_s, mlen as u64, &ctx);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk_part, m_s, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_sign(&mut sig_s[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);

            merkle_sign(&mut sig_s[sig_off..], &mut root, &ctx,
                        &mut wots_addr, &mut tree_addr, idx_leaf);
            sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        *siglen = SPX_BYTES;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    unsafe {
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);

        if siglen != SPX_BYTES { return -1; }
        let sig_s = std::slice::from_raw_parts(sig, SPX_BYTES);

        let pub_root = &pk_s[SPX_N..];
        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        let mut wots_pk_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
        set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk_s, m_s, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_pk_from_sig(&mut root, &sig_s[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        let mut leaf = [0u8; SPX_N];

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);
            copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

            wots_pk_from_sig(&mut wots_pk, &sig_s[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;

            thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);
            compute_root(&mut root, &leaf, idx_leaf, 0, &sig_s[sig_off..],
                         SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
            sig_off += SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        if root != pub_root[..SPX_N] { return -1; }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    unsafe {
        let mut siglen: usize = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
        ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    unsafe {
        if smlen < SPX_BYTES as u64 {
            ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }
        *mlen = smlen - SPX_BYTES as u64;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }
        ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);
    }
    0
}
