#![allow(non_snake_case, non_upper_case_globals, clippy::missing_safety_doc)]

use std::ptr;

// ============================================================================
// SPHINCS+ SHA2-192f simple parameters (from params-sphincs-sha2-192f.h)
// ============================================================================
const SPX_N: usize = 24;
const SPX_FULL_HEIGHT: usize = 66;
const SPX_D: usize = 22;
const SPX_FORS_HEIGHT: usize = 8;
const SPX_FORS_TREES: usize = 33;
const SPX_WOTS_W: usize = 16;
const SPX_WOTS_LOGW: usize = 4;
const SPX_ADDR_BYTES: usize = 32;

const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 48
const SPX_WOTS_LEN2: usize = 3; // precomputed for W=16, N=24
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 51
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 3

const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8; // 33
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
const SPX_FORS_PK_BYTES: usize = SPX_N;

const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// SHA2 offsets (from sha2_offsets.h)
const SPX_OFFSET_LAYER: usize = 0;
const SPX_OFFSET_TREE: usize = 1;
const SPX_OFFSET_TYPE: usize = 9;
const SPX_OFFSET_KP_ADDR: usize = 10;
const SPX_OFFSET_CHAIN_ADDR: usize = 17;
const SPX_OFFSET_HASH_ADDR: usize = 21;
const SPX_OFFSET_TREE_HGT: usize = 17;
const SPX_OFFSET_TREE_INDEX: usize = 18;

// SHA2 constants
const SPX_SHA256_BLOCK_BYTES: usize = 64;
const SPX_SHA256_OUTPUT_BYTES: usize = 32;
const SPX_SHA512_BLOCK_BYTES: usize = 128;
const SPX_SHA512_OUTPUT_BYTES: usize = 64;
const SPX_SHA256_ADDR_BYTES: usize = 22;

// Since SPX_N >= 24, shaX = sha512
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;

// Address type constants
const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// hash_message local constants
const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 63
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 8
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 3
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 1
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES; // 42
const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & !(SPX_SHAX_BLOCK_BYTES - 1)) / SPX_SHAX_BLOCK_BYTES; // 1

// RNG constants
const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

// ============================================================================
// Context struct (from context.h)
// ============================================================================
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
    pub state_seeded_512: [u8; 72],
}

// ============================================================================
// Leaf info structs
// ============================================================================
#[repr(C)]
pub struct LeafInfoX1 {
    pub wots_sig: *mut u8,
    pub wots_sign_leaf: u32,
    pub wots_steps: *mut u32,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

// ============================================================================
// RNG structs
// ============================================================================
#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: libc::c_ulong,
    pub length_remaining: libc::c_ulong,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[repr(C)]
pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: libc::c_int,
}

static mut DRBG_CTX: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

// ============================================================================
// SHA-256 internals
// ============================================================================
fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64) | ((x[6] as u64) << 8) | ((x[5] as u64) << 16) | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32) | ((x[2] as u64) << 40) | ((x[1] as u64) << 48) | ((x[0] as u64) << 56)
}

fn store_bigendian_32(x: &mut [u8], mut u: u64) {
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn store_bigendian_64(x: &mut [u8], mut u: u64) {
    x[7] = u as u8; u >>= 8;
    x[6] = u as u8; u >>= 8;
    x[5] = u as u8; u >>= 8;
    x[4] = u as u8; u >>= 8;
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn crypto_hashblocks_sha256(statebytes: &mut [u8], inp: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[4*i..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut offset = 0usize;
    while inlen >= 64 {
        let block = &inp[offset..];
        let mut w = [0u32; 16];
        for i in 0..16 { w[i] = load_bigendian_32(&block[4*i..]); }

        macro_rules! ch { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ (!$x & $z) } }
        macro_rules! maj { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ ($x & $z) ^ ($y & $z) } }
        macro_rules! sigma0_32 { ($x:expr) => { $x.rotate_right(2) ^ $x.rotate_right(13) ^ $x.rotate_right(22) } }
        macro_rules! sigma1_32 { ($x:expr) => { $x.rotate_right(6) ^ $x.rotate_right(11) ^ $x.rotate_right(25) } }
        macro_rules! lsigma0_32 { ($x:expr) => { $x.rotate_right(7) ^ $x.rotate_right(18) ^ ($x >> 3) } }
        macro_rules! lsigma1_32 { ($x:expr) => { $x.rotate_right(17) ^ $x.rotate_right(19) ^ ($x >> 10) } }

        macro_rules! f32_round {
            ($w:expr, $k:expr) => {
                let t1 = h.wrapping_add(sigma1_32!(e)).wrapping_add(ch!(e,f,g)).wrapping_add($k).wrapping_add($w);
                let t2 = sigma0_32!(a).wrapping_add(maj!(a,b,c));
                h = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
            }
        }

        macro_rules! expand_32 {
            () => {
                w[0] = lsigma1_32!(w[14]).wrapping_add(w[9]).wrapping_add(lsigma0_32!(w[1])).wrapping_add(w[0]);
                w[1] = lsigma1_32!(w[15]).wrapping_add(w[10]).wrapping_add(lsigma0_32!(w[2])).wrapping_add(w[1]);
                w[2] = lsigma1_32!(w[0]).wrapping_add(w[11]).wrapping_add(lsigma0_32!(w[3])).wrapping_add(w[2]);
                w[3] = lsigma1_32!(w[1]).wrapping_add(w[12]).wrapping_add(lsigma0_32!(w[4])).wrapping_add(w[3]);
                w[4] = lsigma1_32!(w[2]).wrapping_add(w[13]).wrapping_add(lsigma0_32!(w[5])).wrapping_add(w[4]);
                w[5] = lsigma1_32!(w[3]).wrapping_add(w[14]).wrapping_add(lsigma0_32!(w[6])).wrapping_add(w[5]);
                w[6] = lsigma1_32!(w[4]).wrapping_add(w[15]).wrapping_add(lsigma0_32!(w[7])).wrapping_add(w[6]);
                w[7] = lsigma1_32!(w[5]).wrapping_add(w[0]).wrapping_add(lsigma0_32!(w[8])).wrapping_add(w[7]);
                w[8] = lsigma1_32!(w[6]).wrapping_add(w[1]).wrapping_add(lsigma0_32!(w[9])).wrapping_add(w[8]);
                w[9] = lsigma1_32!(w[7]).wrapping_add(w[2]).wrapping_add(lsigma0_32!(w[10])).wrapping_add(w[9]);
                w[10] = lsigma1_32!(w[8]).wrapping_add(w[3]).wrapping_add(lsigma0_32!(w[11])).wrapping_add(w[10]);
                w[11] = lsigma1_32!(w[9]).wrapping_add(w[4]).wrapping_add(lsigma0_32!(w[12])).wrapping_add(w[11]);
                w[12] = lsigma1_32!(w[10]).wrapping_add(w[5]).wrapping_add(lsigma0_32!(w[13])).wrapping_add(w[12]);
                w[13] = lsigma1_32!(w[11]).wrapping_add(w[6]).wrapping_add(lsigma0_32!(w[14])).wrapping_add(w[13]);
                w[14] = lsigma1_32!(w[12]).wrapping_add(w[7]).wrapping_add(lsigma0_32!(w[15])).wrapping_add(w[14]);
                w[15] = lsigma1_32!(w[13]).wrapping_add(w[8]).wrapping_add(lsigma0_32!(w[0])).wrapping_add(w[15]);
            }
        }

        static K256: [u32; 64] = [
            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
        ];

        for i in 0..16 { f32_round!(w[i], K256[i]); }
        expand_32!();
        for i in 0..16 { f32_round!(w[i], K256[16+i]); }
        expand_32!();
        for i in 0..16 { f32_round!(w[i], K256[32+i]); }
        expand_32!();
        for i in 0..16 { f32_round!(w[i], K256[48+i]); }

        a = a.wrapping_add(state[0]); b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]); d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]); f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]); h = h.wrapping_add(state[7]);
        state = [a, b, c, d, e, f, g, h];

        offset += 64;
        inlen -= 64;
    }
    for i in 0..8 { store_bigendian_32(&mut statebytes[4*i..], state[i] as u64); }
    inlen
}

fn crypto_hashblocks_sha512(statebytes: &mut [u8], inp: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u64; 8];
    for i in 0..8 { state[i] = load_bigendian_64(&statebytes[8*i..]); }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut offset = 0usize;
    while inlen >= 128 {
        let block = &inp[offset..];
        let mut w = [0u64; 16];
        for i in 0..16 { w[i] = load_bigendian_64(&block[8*i..]); }

        macro_rules! ch { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ (!$x & $z) } }
        macro_rules! maj { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ ($x & $z) ^ ($y & $z) } }
        macro_rules! sigma0_64 { ($x:expr) => { $x.rotate_right(28) ^ $x.rotate_right(34) ^ $x.rotate_right(39) } }
        macro_rules! sigma1_64 { ($x:expr) => { $x.rotate_right(14) ^ $x.rotate_right(18) ^ $x.rotate_right(41) } }
        macro_rules! lsigma0_64 { ($x:expr) => { $x.rotate_right(1) ^ $x.rotate_right(8) ^ ($x >> 7) } }
        macro_rules! lsigma1_64 { ($x:expr) => { $x.rotate_right(19) ^ $x.rotate_right(61) ^ ($x >> 6) } }

        macro_rules! f64_round {
            ($w:expr, $k:expr) => {
                let t1 = h.wrapping_add(sigma1_64!(e)).wrapping_add(ch!(e,f,g)).wrapping_add($k).wrapping_add($w);
                let t2 = sigma0_64!(a).wrapping_add(maj!(a,b,c));
                h = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
            }
        }

        macro_rules! expand_64 {
            () => {
                w[0] = lsigma1_64!(w[14]).wrapping_add(w[9]).wrapping_add(lsigma0_64!(w[1])).wrapping_add(w[0]);
                w[1] = lsigma1_64!(w[15]).wrapping_add(w[10]).wrapping_add(lsigma0_64!(w[2])).wrapping_add(w[1]);
                w[2] = lsigma1_64!(w[0]).wrapping_add(w[11]).wrapping_add(lsigma0_64!(w[3])).wrapping_add(w[2]);
                w[3] = lsigma1_64!(w[1]).wrapping_add(w[12]).wrapping_add(lsigma0_64!(w[4])).wrapping_add(w[3]);
                w[4] = lsigma1_64!(w[2]).wrapping_add(w[13]).wrapping_add(lsigma0_64!(w[5])).wrapping_add(w[4]);
                w[5] = lsigma1_64!(w[3]).wrapping_add(w[14]).wrapping_add(lsigma0_64!(w[6])).wrapping_add(w[5]);
                w[6] = lsigma1_64!(w[4]).wrapping_add(w[15]).wrapping_add(lsigma0_64!(w[7])).wrapping_add(w[6]);
                w[7] = lsigma1_64!(w[5]).wrapping_add(w[0]).wrapping_add(lsigma0_64!(w[8])).wrapping_add(w[7]);
                w[8] = lsigma1_64!(w[6]).wrapping_add(w[1]).wrapping_add(lsigma0_64!(w[9])).wrapping_add(w[8]);
                w[9] = lsigma1_64!(w[7]).wrapping_add(w[2]).wrapping_add(lsigma0_64!(w[10])).wrapping_add(w[9]);
                w[10] = lsigma1_64!(w[8]).wrapping_add(w[3]).wrapping_add(lsigma0_64!(w[11])).wrapping_add(w[10]);
                w[11] = lsigma1_64!(w[9]).wrapping_add(w[4]).wrapping_add(lsigma0_64!(w[12])).wrapping_add(w[11]);
                w[12] = lsigma1_64!(w[10]).wrapping_add(w[5]).wrapping_add(lsigma0_64!(w[13])).wrapping_add(w[12]);
                w[13] = lsigma1_64!(w[11]).wrapping_add(w[6]).wrapping_add(lsigma0_64!(w[14])).wrapping_add(w[13]);
                w[14] = lsigma1_64!(w[12]).wrapping_add(w[7]).wrapping_add(lsigma0_64!(w[15])).wrapping_add(w[14]);
                w[15] = lsigma1_64!(w[13]).wrapping_add(w[8]).wrapping_add(lsigma0_64!(w[0])).wrapping_add(w[15]);
            }
        }

        static K512: [u64; 80] = [
            0x428a2f98d728ae22,0x7137449123ef65cd,0xb5c0fbcfec4d3b2f,0xe9b5dba58189dbbc,
            0x3956c25bf348b538,0x59f111f1b605d019,0x923f82a4af194f9b,0xab1c5ed5da6d8118,
            0xd807aa98a3030242,0x12835b0145706fbe,0x243185be4ee4b28c,0x550c7dc3d5ffb4e2,
            0x72be5d74f27b896f,0x80deb1fe3b1696b1,0x9bdc06a725c71235,0xc19bf174cf692694,
            0xe49b69c19ef14ad2,0xefbe4786384f25e3,0x0fc19dc68b8cd5b5,0x240ca1cc77ac9c65,
            0x2de92c6f592b0275,0x4a7484aa6ea6e483,0x5cb0a9dcbd41fbd4,0x76f988da831153b5,
            0x983e5152ee66dfab,0xa831c66d2db43210,0xb00327c898fb213f,0xbf597fc7beef0ee4,
            0xc6e00bf33da88fc2,0xd5a79147930aa725,0x06ca6351e003826f,0x142929670a0e6e70,
            0x27b70a8546d22ffc,0x2e1b21385c26c926,0x4d2c6dfc5ac42aed,0x53380d139d95b3df,
            0x650a73548baf63de,0x766a0abb3c77b2a8,0x81c2c92e47edaee6,0x92722c851482353b,
            0xa2bfe8a14cf10364,0xa81a664bbc423001,0xc24b8b70d0f89791,0xc76c51a30654be30,
            0xd192e819d6ef5218,0xd69906245565a910,0xf40e35855771202a,0x106aa07032bbd1b8,
            0x19a4c116b8d2d0c8,0x1e376c085141ab53,0x2748774cdf8eeb99,0x34b0bcb5e19b48a8,
            0x391c0cb3c5c95a63,0x4ed8aa4ae3418acb,0x5b9cca4f7763e373,0x682e6ff3d6b2b8a3,
            0x748f82ee5defb2fc,0x78a5636f43172f60,0x84c87814a1f0ab72,0x8cc702081a6439ec,
            0x90befffa23631e28,0xa4506cebde82bde9,0xbef9a3f7b2c67915,0xc67178f2e372532b,
            0xca273eceea26619c,0xd186b8c721c0c207,0xeada7dd6cde0eb1e,0xf57d4f7fee6ed178,
            0x06f067aa72176fba,0x0a637dc5a2c898a6,0x113f9804bef90dae,0x1b710b35131c471b,
            0x28db77f523047d84,0x32caab7b40c72493,0x3c9ebe0a15c9bebc,0x431d67c49c100d4c,
            0x4cc5d4becb3e42b6,0x597f299cfc657e2a,0x5fcb6fab3ad6faec,0x6c44198c4a475817,
        ];

        for i in 0..16 { f64_round!(w[i], K512[i]); }
        expand_64!();
        for i in 0..16 { f64_round!(w[i], K512[16+i]); }
        expand_64!();
        for i in 0..16 { f64_round!(w[i], K512[32+i]); }
        expand_64!();
        for i in 0..16 { f64_round!(w[i], K512[48+i]); }
        expand_64!();
        for i in 0..16 { f64_round!(w[i], K512[64+i]); }

        a = a.wrapping_add(state[0]); b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]); d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]); f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]); h = h.wrapping_add(state[7]);
        state = [a, b, c, d, e, f, g, h];

        offset += 128;
        inlen -= 128;
    }
    for i in 0..8 { store_bigendian_64(&mut statebytes[8*i..], state[i]); }
    inlen
}

static IV_256: [u8; 32] = [
    0x6a,0x09,0xe6,0x67,0xbb,0x67,0xae,0x85,0x3c,0x6e,0xf3,0x72,0xa5,0x4f,0xf5,0x3a,
    0x51,0x0e,0x52,0x7f,0x9b,0x05,0x68,0x8c,0x1f,0x83,0xd9,0xab,0x5b,0xe0,0xcd,0x19,
];

static IV_512: [u8; 64] = [
    0x6a,0x09,0xe6,0x67,0xf3,0xbc,0xc9,0x08,0xbb,0x67,0xae,0x85,0x84,0xca,0xa7,0x3b,
    0x3c,0x6e,0xf3,0x72,0xfe,0x94,0xf8,0x2b,0xa5,0x4f,0xf5,0x3a,0x5f,0x1d,0x36,0xf1,
    0x51,0x0e,0x52,0x7f,0xad,0xe6,0x82,0xd1,0x9b,0x05,0x68,0x8c,0x2b,0x3e,0x6c,0x1f,
    0x1f,0x83,0xd9,0xab,0xfb,0x41,0xbd,0x6b,0x5b,0xe0,0xcd,0x19,0x13,0x7e,0x21,0x79,
];

fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 { state[i] = 0; }
}

fn sha512_inc_init(state: &mut [u8]) {
    state[..64].copy_from_slice(&IV_512);
    for i in 64..72 { state[i] = 0; }
}

fn sha256_inc_blocks(state: &mut [u8], inp: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, inp, 64 * inblocks);
    bytes += (64 * inblocks) as u64;
    store_bigendian_64(&mut state[32..], bytes);
}

fn sha512_inc_blocks(state: &mut [u8], inp: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[64..]);
    crypto_hashblocks_sha512(state, inp, 128 * inblocks);
    bytes += (128 * inblocks) as u64;
    store_bigendian_64(&mut state[64..], bytes);
}

fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]).wrapping_add(inlen as u64);
    crypto_hashblocks_sha256(state, inp, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;
    padded[..remaining].copy_from_slice(&inp[start..start + remaining]);
    padded[remaining] = 0x80;
    if remaining < 56 {
        for i in (remaining + 1)..56 { padded[i] = 0; }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 64);
    } else {
        for i in (remaining + 1)..120 { padded[i] = 0; }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 128);
    }
    out[..32].copy_from_slice(&state[..32]);
}

fn sha512_inc_finalize(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 256];
    let bytes = load_bigendian_64(&state[64..]).wrapping_add(inlen as u64);
    crypto_hashblocks_sha512(state, inp, inlen);
    let remaining = inlen & 127;
    let start = inlen - remaining;
    padded[..remaining].copy_from_slice(&inp[start..start + remaining]);
    padded[remaining] = 0x80;
    if remaining < 112 {
        for i in (remaining + 1)..119 { padded[i] = 0; }
        padded[119] = (bytes >> 61) as u8;
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(state, &padded, 128);
    } else {
        for i in (remaining + 1)..247 { padded[i] = 0; }
        padded[247] = (bytes >> 61) as u8;
        padded[248] = (bytes >> 53) as u8;
        padded[249] = (bytes >> 45) as u8;
        padded[250] = (bytes >> 37) as u8;
        padded[251] = (bytes >> 29) as u8;
        padded[252] = (bytes >> 21) as u8;
        padded[253] = (bytes >> 13) as u8;
        padded[254] = (bytes >> 5) as u8;
        padded[255] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(state, &padded, 256);
    }
    out[..64].copy_from_slice(&state[..64]);
}

fn sha256(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, inp, inlen);
}

fn sha512(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut state = [0u8; 72];
    sha512_inc_init(&mut state);
    sha512_inc_finalize(out, &mut state, inp, inlen);
}

// ============================================================================
// MGF1, seed_state, utils, address functions
// ============================================================================
fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        out[off..off + (outlen - i * SPX_SHA256_OUTPUT_BYTES)]
            .copy_from_slice(&outbuf[..outlen - i * SPX_SHA256_OUTPUT_BYTES]);
    }
}

fn mgf1_512(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        sha512(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA512_OUTPUT_BYTES {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        sha512(&mut outbuf, &inbuf, inlen + 4);
        out[off..off + (outlen - i * SPX_SHA512_OUTPUT_BYTES)]
            .copy_from_slice(&outbuf[..outlen - i * SPX_SHA512_OUTPUT_BYTES]);
    }
}

fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest already zero
    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
    sha512_inc_init(&mut ctx.state_seeded_512);
    sha512_inc_blocks(&mut ctx.state_seeded_512, &block, 1);
}

fn ull_to_bytes_internal(out: &mut [u8], outlen: usize, mut inp: u64) {
    for i in (0..outlen).rev() {
        out[i] = (inp & 0xff) as u8;
        inp >>= 8;
    }
}

fn u32_to_bytes_internal(out: &mut [u8], inp: u32) {
    out[0] = (inp >> 24) as u8;
    out[1] = (inp >> 16) as u8;
    out[2] = (inp >> 8) as u8;
    out[3] = inp as u8;
}

fn bytes_to_ull_internal(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

fn addr_bytes(addr: &[u32; 8]) -> &[u8; SPX_ADDR_BYTES] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; SPX_ADDR_BYTES]) }
}

fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; SPX_ADDR_BYTES] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; SPX_ADDR_BYTES]) }
}

fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    ull_to_bytes_internal(&mut addr_bytes_mut(addr)[SPX_OFFSET_TREE..], 8, tree);
}

fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    u32_to_bytes_internal(&mut addr_bytes_mut(addr)[SPX_OFFSET_KP_ADDR..], keypair);
}

fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    u32_to_bytes_internal(&mut addr_bytes_mut(addr)[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// ============================================================================
// Hash functions: initialize_hash_function, prf_addr, gen_message_random, hash_message
// ============================================================================
fn initialize_hash_function_internal(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

fn prf_addr_internal(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn gen_message_random_internal(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: usize, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x36; }

    sha512_inc_init(&mut state);
    sha512_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let input_copy: Vec<u8> = buf[..SPX_N + mlen].to_vec();
        sha512_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &input_copy, mlen + SPX_N);
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha512_inc_blocks(&mut state, &buf, 1);
        let m_off = SPX_SHAX_BLOCK_BYTES - SPX_N;
        let mlen2 = mlen - m_off;
        sha512_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &m[m_off..], mlen2);
    }

    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x5c; }

    let mut tmp = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    tmp.copy_from_slice(&buf);
    sha512(&mut buf, &tmp, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

fn hash_message_internal(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                         r_val: &[u8], pk: &[u8], m: &[u8], mlen: usize, _ctx: &SpxCtx) {
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha512_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        sha512_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
        let m_off = fill;
        let mlen2 = mlen - fill;
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[m_off..], mlen2);
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_512(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull_internal(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull_internal(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ============================================================================
// thash (simple variant)
// ============================================================================
fn thash_internal(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    // For inblocks > 1, use SHA-512 (SPX_SHA512 = 1)
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    let buflen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buflen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    sha2_state.copy_from_slice(&ctx.state_seeded_512);
    let buflen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buflen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================================
// WOTS
// ============================================================================
fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    for i in start..std::cmp::min(start + steps, SPX_WOTS_W as u32) {
        set_hash_addr(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash_internal(out, &tmp, 1, ctx, addr);
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0usize;
    let mut bits = 0i32;
    let mut total = 0u8;
    for consumed in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[consumed] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
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
    ull_to_bytes_internal(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

fn chain_lengths_internal(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(&mut lengths[..SPX_WOTS_LEN1], SPX_WOTS_LEN1, msg);
    let mut csum = [0u32; SPX_WOTS_LEN2];
    wots_checksum(&mut csum, &lengths[..SPX_WOTS_LEN1]);
    lengths[SPX_WOTS_LEN1..SPX_WOTS_LEN].copy_from_slice(&csum);
}

fn wots_pk_from_sig_internal(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths_internal(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(&mut pk[i * SPX_N..], &sig[i * SPX_N..], lengths[i], SPX_WOTS_W as u32 - 1 - lengths[i], ctx, addr);
    }
}

// ============================================================================
// wotsx1: wots_gen_leafx1
// ============================================================================
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
        prf_addr_internal(buffer, ctx, &info.leaf_addr);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                unsafe {
                    let sig_ptr = info.wots_sig.add(i * SPX_N);
                    ptr::copy_nonoverlapping(buffer.as_ptr(), sig_ptr, SPX_N);
                }
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash_internal(buffer, &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }
    thash_internal(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

// ============================================================================
// FORS
// ============================================================================
fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr_internal(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash_internal(leaf, sk, 1, ctx, fors_leaf_addr);
}

fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
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

fn compute_root_internal(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
                         auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut buffer = [0u8; 2 * SPX_N];
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut ap_off = SPX_N;
    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);
        if leaf_idx & 1 != 0 {
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&buffer);
            thash_internal(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let mut tmp = [0u8; 2 * SPX_N];
            tmp.copy_from_slice(&buffer);
            thash_internal(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        }
        ap_off += SPX_N;
    }
    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash_internal(root, &buffer, 2, ctx, addr);
}

// ============================================================================
// utilsx1: treehash functions (wots_treehashx1, fors_treehashx1)
// ============================================================================
fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx { break; }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash_internal(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        let off = h as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
        idx += 1;
    }
}

fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx { break; }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash_internal(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        let off = h as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
        idx += 1;
    }
}

// ============================================================================
// FORS sign / pk_from_sig
// ============================================================================
fn fors_sign_internal(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
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

        fors_treehashx1(&mut roots[i * SPX_N..], &mut sig[sig_off..], ctx,
                         indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
                         &mut fors_tree_addr, &mut fors_info);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    thash_internal(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

fn fors_pk_from_sig_internal(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
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
        compute_root_internal(&mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
                              &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    thash_internal(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// ============================================================================
// Merkle
// ============================================================================
fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
               wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let auth_path = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: sig.as_mut_ptr(),
        wots_sign_leaf: idx_leaf,
        wots_steps: ptr::null_mut(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths_internal(&mut steps, root);
    info.wots_steps = steps.as_mut_ptr();

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
    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}

// ============================================================================
// RNG (rng.c - deterministic DRBG using OpenSSL AES-256-ECB)
// ============================================================================
fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
    let cipher = aes::Aes256::new(GenericArray::from_slice(&key[..32]));
    let mut block = GenericArray::clone_from_slice(&ctr[..16]);
    cipher.encrypt_block(&mut block);
    buffer[..16].copy_from_slice(&block);
}

fn aes256_ctr_drbg_update(provided_data: *const u8, key: &mut [u8], v: &mut [u8]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
        for j in (0..=15).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }
    if !provided_data.is_null() {
        for i in 0..48 {
            temp[i] ^= unsafe { *provided_data.add(i) };
        }
    }
    key[..32].copy_from_slice(&temp[..32]);
    v[..16].copy_from_slice(&temp[32..48]);
}

// ============================================================================
// Public extern "C" API
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe { initialize_hash_function_internal(&mut *ctx) };
}

#[unsafe(no_mangle)]
pub extern "C" fn prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        let addr_ref = &*(addr as *const [u32; 8]);
        prf_addr_internal(std::slice::from_raw_parts_mut(out, SPX_N), &*ctx, addr_ref);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gen_message_random(r: *mut u8, sk_prf: *const u8, optrand: *const u8,
                                     m: *const u8, mlen: libc::c_ulonglong, ctx: *const SpxCtx) {
    unsafe {
        gen_message_random_internal(
            std::slice::from_raw_parts_mut(r, SPX_N),
            std::slice::from_raw_parts(sk_prf, SPX_N),
            std::slice::from_raw_parts(optrand, SPX_N),
            std::slice::from_raw_parts(m, mlen as usize),
            mlen as usize, &*ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_message(digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
                               r: *const u8, pk: *const u8, m: *const u8,
                               mlen: libc::c_ulonglong, ctx: *const SpxCtx) {
    unsafe {
        hash_message_internal(
            std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES),
            &mut *tree, &mut *leaf_idx,
            std::slice::from_raw_parts(r, SPX_N),
            std::slice::from_raw_parts(pk, SPX_PK_BYTES),
            std::slice::from_raw_parts(m, mlen as usize),
            mlen as usize, &*ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn thash(out: *mut u8, inp: *const u8, inblocks: libc::c_uint,
                        ctx: *const SpxCtx, addr: *mut u32) {
    unsafe {
        let addr_ref = &mut *(addr as *mut [u32; 8]);
        thash_internal(
            std::slice::from_raw_parts_mut(out, SPX_N),
            std::slice::from_raw_parts(inp, inblocks as usize * SPX_N),
            inblocks as usize, &*ctx, addr_ref);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ull_to_bytes(out: *mut u8, outlen: libc::c_uint, inp: libc::c_ulonglong) {
    unsafe {
        ull_to_bytes_internal(std::slice::from_raw_parts_mut(out, outlen as usize), outlen as usize, inp);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn u32_to_bytes(out: *mut u8, inp: u32) {
    unsafe {
        u32_to_bytes_internal(std::slice::from_raw_parts_mut(out, 4), inp);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bytes_to_ull(inp: *const u8, inlen: libc::c_uint) -> libc::c_ulonglong {
    unsafe { bytes_to_ull_internal(std::slice::from_raw_parts(inp, inlen as usize), inlen as usize) }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_layer_addr_c(addr: *mut u32, layer: u32) {
    unsafe { set_layer_addr(&mut *(addr as *mut [u32; 8]), layer); }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_tree_addr_c(addr: *mut u32, tree: u64) {
    unsafe { set_tree_addr(&mut *(addr as *mut [u32; 8]), tree); }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_type_c(addr: *mut u32, type_val: u32) {
    unsafe { set_type(&mut *(addr as *mut [u32; 8]), type_val); }
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_subtree_addr_c(out: *mut u32, inp: *const u32) {
    unsafe {
        copy_subtree_addr(&mut *(out as *mut [u32; 8]), &*(inp as *const [u32; 8]));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_keypair_addr_c(addr: *mut u32, keypair: u32) {
    unsafe { set_keypair_addr(&mut *(addr as *mut [u32; 8]), keypair); }
}

#[unsafe(no_mangle)]
pub extern "C" fn copy_keypair_addr_c(out: *mut u32, inp: *const u32) {
    unsafe {
        copy_keypair_addr(&mut *(out as *mut [u32; 8]), &*(inp as *const [u32; 8]));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_chain_addr_c(addr: *mut u32, chain: u32) {
    unsafe { set_chain_addr(&mut *(addr as *mut [u32; 8]), chain); }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_hash_addr_c(addr: *mut u32, hash: u32) {
    unsafe { set_hash_addr(&mut *(addr as *mut [u32; 8]), hash); }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_tree_height_c(addr: *mut u32, tree_height: u32) {
    unsafe { set_tree_height(&mut *(addr as *mut [u32; 8]), tree_height); }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_tree_index_c(addr: *mut u32, tree_index: u32) {
    unsafe { set_tree_index(&mut *(addr as *mut [u32; 8]), tree_index); }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> libc::c_ulonglong {
    CRYPTO_SECRETKEYBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> libc::c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> libc::c_ulonglong {
    CRYPTO_BYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> libc::c_ulonglong {
    CRYPTO_SEEDBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> libc::c_int {
    unsafe {
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);

        sk_s[..CRYPTO_SEEDBYTES].copy_from_slice(seed_s);
        pk_s[..SPX_N].copy_from_slice(&sk_s[2 * SPX_N..3 * SPX_N]);

        let mut ctx = std::mem::zeroed::<SpxCtx>();
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);

        initialize_hash_function_internal(&mut ctx);
        merkle_gen_root(&mut sk_s[3 * SPX_N..], &ctx);
        pk_s[SPX_N..2 * SPX_N].copy_from_slice(&sk_s[3 * SPX_N..4 * SPX_N]);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> libc::c_int {
    unsafe {
        let mut seed = [0u8; CRYPTO_SEEDBYTES];
        randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as libc::c_ulonglong);
        crypto_sign_seed_keypair(pk, sk, seed.as_ptr())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(sig: *mut u8, siglen: *mut libc::size_t,
                                        m: *const u8, mlen: libc::size_t,
                                        sk: *const u8) -> libc::c_int {
    unsafe {
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);

        let sk_prf = &sk_s[SPX_N..2 * SPX_N];
        let pk = &sk_s[2 * SPX_N..];

        let mut ctx = std::mem::zeroed::<SpxCtx>();
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
        initialize_hash_function_internal(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

        let mut optrand = [0u8; SPX_N];
        randombytes(optrand.as_mut_ptr(), SPX_N as libc::c_ulonglong);

        gen_message_random_internal(&mut sig_s[..SPX_N], sk_prf, &optrand, m_s, mlen, &ctx);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message_internal(&mut mhash, &mut tree, &mut idx_leaf, &sig_s[..SPX_N], pk, m_s, mlen, &ctx);

        let mut sig_off = SPX_N;
        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_sign_internal(&mut sig_s[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);
            merkle_sign(&mut sig_s[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
            sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }
        *siglen = SPX_BYTES;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(sig: *const u8, siglen: libc::size_t,
                                     m: *const u8, mlen: libc::size_t,
                                     pk: *const u8) -> libc::c_int {
    unsafe {
        if siglen != SPX_BYTES { return -1; }
        let sig_s = std::slice::from_raw_parts(sig, SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let pub_root = &pk_s[SPX_N..];

        let mut ctx = std::mem::zeroed::<SpxCtx>();
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        initialize_hash_function_internal(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        let mut wots_pk_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
        set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message_internal(&mut mhash, &mut tree, &mut idx_leaf, &sig_s[..SPX_N], pk_s, m_s, mlen, &ctx);

        let mut sig_off = SPX_N;
        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_pk_from_sig_internal(&mut root, &sig_s[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);
            copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

            let mut wots_pk = [0u8; SPX_WOTS_BYTES];
            wots_pk_from_sig_internal(&mut wots_pk, &sig_s[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;

            let mut leaf = [0u8; SPX_N];
            thash_internal(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);
            compute_root_internal(&mut root, &leaf, idx_leaf, 0, &sig_s[sig_off..],
                                  SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
            sig_off += SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(sm: *mut u8, smlen: *mut libc::c_ulonglong,
                              m: *const u8, mlen: libc::c_ulonglong,
                              sk: *const u8) -> libc::c_int {
    unsafe {
        let mut siglen: libc::size_t = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as libc::size_t, sk);
        ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as libc::c_ulonglong + mlen;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(m_out: *mut u8, mlen: *mut libc::c_ulonglong,
                                   sm: *const u8, smlen: libc::c_ulonglong,
                                   pk: *const u8) -> libc::c_int {
    unsafe {
        if (smlen as usize) < SPX_BYTES {
            ptr::write_bytes(m_out, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }
        *mlen = smlen - SPX_BYTES as libc::c_ulonglong;
        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as libc::size_t, pk) != 0 {
            ptr::write_bytes(m_out, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }
        ptr::copy(sm.add(SPX_BYTES), m_out, *mlen as usize);
        0
    }
}

// ============================================================================
// RNG public API (rng.c)
// ============================================================================
#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(ctx: *mut AesXofStruct, seed: *mut u8,
                                    diversifier: *mut u8, maxlen: libc::c_ulong) -> libc::c_int {
    unsafe {
        if maxlen >= 0x100000000 { return RNG_BAD_MAXLEN; }
        (*ctx).length_remaining = maxlen;
        (*ctx).key.copy_from_slice(std::slice::from_raw_parts(seed, 32));
        (*ctx).ctr[..8].copy_from_slice(std::slice::from_raw_parts(diversifier, 8));
        let mut ml = maxlen;
        (*ctx).ctr[11] = (ml % 256) as u8; ml >>= 8;
        (*ctx).ctr[10] = (ml % 256) as u8; ml >>= 8;
        (*ctx).ctr[9] = (ml % 256) as u8; ml >>= 8;
        (*ctx).ctr[8] = (ml % 256) as u8;
        (*ctx).ctr[12..16].fill(0);
        (*ctx).buffer_pos = 16;
        (*ctx).buffer.fill(0);
        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: libc::c_ulong) -> libc::c_int {
    unsafe {
        if x.is_null() { return RNG_BAD_OUTBUF; }
        if xlen >= (*ctx).length_remaining { return RNG_BAD_REQ_LEN; }
        (*ctx).length_remaining -= xlen;
        let mut offset: usize = 0;
        while xlen > 0 {
            let bp = (*ctx).buffer_pos as usize;
            if xlen <= (16 - bp) as libc::c_ulong {
                ptr::copy_nonoverlapping((*ctx).buffer.as_ptr().add(bp), x.add(offset), xlen as usize);
                (*ctx).buffer_pos += xlen;
                return RNG_SUCCESS;
            }
            let take = 16 - bp;
            ptr::copy_nonoverlapping((*ctx).buffer.as_ptr().add(bp), x.add(offset), take);
            xlen -= take as libc::c_ulong;
            offset += take;
            aes256_ecb(&(*ctx).key, &(*ctx).ctr, &mut (*ctx).buffer);
            (*ctx).buffer_pos = 0;
            for i in (12..=15).rev() {
                if (*ctx).ctr[i] == 0xff { (*ctx).ctr[i] = 0x00; }
                else { (*ctx).ctr[i] += 1; break; }
            }
        }
        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(entropy_input: *mut u8, personalization_string: *mut u8) {
    unsafe {
        let mut seed_material = [0u8; 48];
        seed_material.copy_from_slice(std::slice::from_raw_parts(entropy_input, 48));
        if !personalization_string.is_null() {
            let ps = std::slice::from_raw_parts(personalization_string, 48);
            for i in 0..48 { seed_material[i] ^= ps[i]; }
        }
        DRBG_CTX.key.fill(0);
        DRBG_CTX.v.fill(0);
        aes256_ctr_drbg_update(seed_material.as_ptr(), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, mut xlen: libc::c_ulonglong) -> libc::c_int {
    unsafe {
        let mut block = [0u8; 16];
        let mut i: usize = 0;
        while xlen > 0 {
            for j in (0..=15).rev() {
                if DRBG_CTX.v[j] == 0xff { DRBG_CTX.v[j] = 0x00; }
                else { DRBG_CTX.v[j] += 1; break; }
            }
            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
            if xlen > 15 {
                ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
                i += 16;
                xlen -= 16;
            } else {
                ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update(ptr::null(), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(provided_data: *mut u8, key: *mut u8, v: *mut u8) {
    unsafe {
        aes256_ctr_drbg_update(
            provided_data as *const u8,
            std::slice::from_raw_parts_mut(key, 32),
            std::slice::from_raw_parts_mut(v, 16),
        );
    }
}

// wots public API
#[unsafe(no_mangle)]
pub extern "C" fn wots_pk_from_sig(pk: *mut u8, sig: *const u8, msg: *const u8,
                                   ctx: *const SpxCtx, addr: *mut u32) {
    unsafe {
        wots_pk_from_sig_internal(
            std::slice::from_raw_parts_mut(pk, SPX_WOTS_BYTES),
            std::slice::from_raw_parts(sig, SPX_WOTS_BYTES),
            std::slice::from_raw_parts(msg, SPX_N),
            &*ctx, &mut *(addr as *mut [u32; 8]));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn chain_lengths(lengths: *mut libc::c_uint, msg: *const u8) {
    unsafe {
        let mut lens = [0u32; SPX_WOTS_LEN];
        chain_lengths_internal(&mut lens, std::slice::from_raw_parts(msg, SPX_N));
        ptr::copy_nonoverlapping(lens.as_ptr(), lengths, SPX_WOTS_LEN);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_root(root: *mut u8, leaf: *const u8, leaf_idx: u32, idx_offset: u32,
                               auth_path: *const u8, tree_height: u32,
                               ctx: *const SpxCtx, addr: *mut u32) {
    unsafe {
        compute_root_internal(
            std::slice::from_raw_parts_mut(root, SPX_N),
            std::slice::from_raw_parts(leaf, SPX_N),
            leaf_idx, idx_offset,
            std::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N),
            tree_height, &*ctx, &mut *(addr as *mut [u32; 8]));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fors_sign(sig: *mut u8, pk: *mut u8, m: *const u8,
                            ctx: *const SpxCtx, fors_addr: *const u32) {
    unsafe {
        fors_sign_internal(
            std::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES),
            std::slice::from_raw_parts_mut(pk, SPX_FORS_PK_BYTES),
            std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES),
            &*ctx, &*(fors_addr as *const [u32; 8]));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fors_pk_from_sig(pk: *mut u8, sig: *const u8, m: *const u8,
                                   ctx: *const SpxCtx, fors_addr: *const u32) {
    unsafe {
        fors_pk_from_sig_internal(
            std::slice::from_raw_parts_mut(pk, SPX_FORS_PK_BYTES),
            std::slice::from_raw_parts(sig, SPX_FORS_BYTES),
            std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES),
            &*ctx, &*(fors_addr as *const [u32; 8]));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seed_state_c(ctx: *mut SpxCtx) {
    unsafe { seed_state(&mut *ctx); }
}
