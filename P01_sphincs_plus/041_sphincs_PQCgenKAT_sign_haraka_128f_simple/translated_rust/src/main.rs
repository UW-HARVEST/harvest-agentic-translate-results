#![allow(non_snake_case, non_upper_case_globals, clippy::needless_range_loop)]

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;

// ============================================================
// params (haraka-128f)
// ============================================================
const SPX_N: usize = 16;
const SPX_FULL_HEIGHT: usize = 66;
const SPX_D: usize = 22;
const SPX_FORS_HEIGHT: usize = 6;
const SPX_FORS_TREES: usize = 33;
const SPX_WOTS_W: usize = 16;
const SPX_ADDR_BYTES: usize = 32;
const SPX_WOTS_LOGW: usize = 4;
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

// haraka offsets
const SPX_OFFSET_LAYER: usize = 3;
const SPX_OFFSET_TREE: usize = 8;
const SPX_OFFSET_TYPE: usize = 19;
const SPX_OFFSET_KP_ADDR: usize = 20;
const SPX_OFFSET_CHAIN_ADDR: usize = 27;
const SPX_OFFSET_HASH_ADDR: usize = 31;
const SPX_OFFSET_TREE_HGT: usize = 27;
const SPX_OFFSET_TREE_INDEX: usize = 28;

const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

// address types
const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// KAT constants
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

const HARAKAS_RATE: usize = 32;

// ============================================================
// spx_ctx
// ============================================================
#[derive(Clone)]
struct SpxCtx {
    pub_seed: [u8; SPX_N],
    sk_seed: [u8; SPX_N],
    tweaked512_rc64: [[u64; 8]; 10],
    tweaked256_rc32: [[u32; 8]; 10],
}

impl SpxCtx {
    fn new() -> Self {
        Self {
            pub_seed: [0u8; SPX_N],
            sk_seed: [0u8; SPX_N],
            tweaked512_rc64: [[0u64; 8]; 10],
            tweaked256_rc32: [[0u32; 8]; 10],
        }
    }
}

// ============================================================
// RNG (AES-256-CTR-DRBG)
// ============================================================
struct Aes256CtrDrbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbg = Aes256CtrDrbg {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(key.into());
    let mut block = aes::Block::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

fn randombytes_init(entropy_input: &[u8; 48], personalization_string: Option<&[u8; 48]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy_input);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        DRBG_CTX.key = [0u8; 32];
        DRBG_CTX.v = [0u8; 16];
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

fn randombytes(x: &mut [u8], mut xlen: u64) {
    unsafe {
        let mut block = [0u8; 16];
        let mut i: usize = 0;
        while xlen > 0 {
            // increment V
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff {
                    DRBG_CTX.v[j] = 0x00;
                } else {
                    DRBG_CTX.v[j] += 1;
                    break;
                }
            }
            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }
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

fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// ============================================================
// address functions
// ============================================================
fn addr_bytes(addr: &[u32; 8]) -> [u8; 32] {
    let mut b = [0u8; 32];
    for i in 0..8 {
        let v = addr[i];
        b[4 * i] = v as u8;
        b[4 * i + 1] = (v >> 8) as u8;
        b[4 * i + 2] = (v >> 16) as u8;
        b[4 * i + 3] = (v >> 24) as u8;
    }
    b
}

fn addr_from_bytes(b: &[u8; 32]) -> [u32; 8] {
    let mut addr = [0u32; 8];
    for i in 0..8 {
        addr[i] = b[4 * i] as u32
            | ((b[4 * i + 1] as u32) << 8)
            | ((b[4 * i + 2] as u32) << 16)
            | ((b[4 * i + 3] as u32) << 24);
    }
    addr
}

fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_LAYER] = layer as u8;
    *addr = addr_from_bytes(&b);
}

fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let mut b = addr_bytes(addr);
    ull_to_bytes(&mut b[SPX_OFFSET_TREE..], 8, tree);
    *addr = addr_from_bytes(&b);
}

fn set_type(addr: &mut [u32; 8], type_val: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_TYPE] = type_val as u8;
    *addr = addr_from_bytes(&b);
}

fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let b_in = addr_bytes(inp);
    let mut b_out = addr_bytes(out);
    b_out[..SPX_OFFSET_TREE + 8].copy_from_slice(&b_in[..SPX_OFFSET_TREE + 8]);
    *out = addr_from_bytes(&b_out);
}

fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let mut b = addr_bytes(addr);
    u32_to_bytes(&mut b[SPX_OFFSET_KP_ADDR..], keypair);
    *addr = addr_from_bytes(&b);
}

fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let b_in = addr_bytes(inp);
    let mut b_out = addr_bytes(out);
    b_out[..SPX_OFFSET_TREE + 8].copy_from_slice(&b_in[..SPX_OFFSET_TREE + 8]);
    b_out[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&b_in[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
    *out = addr_from_bytes(&b_out);
}

fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
    *addr = addr_from_bytes(&b);
}

fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_HASH_ADDR] = hash as u8;
    *addr = addr_from_bytes(&b);
}

fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    let mut b = addr_bytes(addr);
    b[SPX_OFFSET_TREE_HGT] = tree_height as u8;
    *addr = addr_from_bytes(&b);
}

fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let mut b = addr_bytes(addr);
    u32_to_bytes(&mut b[SPX_OFFSET_TREE_INDEX..], tree_index);
    *addr = addr_from_bytes(&b);
}

// ============================================================
// haraka constants and bit-sliced AES
// ============================================================
static HARAKA512_RC64: [[u64; 8]; 10] = [
    [0x24cf0ab9086f628b, 0xbdd6eeecc83b8382, 0xd96fb0306cdad0a7, 0xaace082ac8f95f89, 0x449d8e8870d7041f, 0x49bb2f80b2b3e2f8, 0x0569ae98d93bb258, 0x23dc9691e7d6a4b1],
    [0xd8ba10ede0fe5b6e, 0x7ecf7dbe424c7b8e, 0x6ea9949c6df62a31, 0xbf3f3c97ec9c313e, 0x241d03a196a1861e, 0xead3a51116e5a2ea, 0x77d479fcad9574e3, 0x18657a1af894b7a0],
    [0x10671e1a7f595522, 0xd9a00ff675d28c7b, 0x2f1edf0d2b9ba661, 0xb8ff58b8e3de45f9, 0xee29261da9865c02, 0xd1532aa4b50bdf43, 0x8bf858159b231bb1, 0xdf17439d22d4f599],
    [0xdd4b2f0870b918c0, 0x757a81f3b39b1bb6, 0x7a5c556898952e3f, 0x7dd70a16d915d87a, 0x3ae61971982b8301, 0xc3ab319e030412be, 0x17c0033ac094a8cb, 0x5a0630fc1a8dc4ef],
    [0x17708988c1632f73, 0xf92ddae090b44f4f, 0x11ac0285c43aa314, 0x509059941936b8ba, 0xd03e152fa2ce9b69, 0x3fbcbcb63a32998b, 0x6204696d692254f7, 0x915542ed93ec59b4],
    [0xf4ed94aa8879236e, 0xff6cb41cd38e03c0, 0x069b38602368aeab, 0x669495b820f0ddba, 0xf42013b1b8bf9e3d, 0xcf935efe6439734d, 0xbc1dcf42ca29e3f8, 0x7e6d3ed29f78ad67],
    [0xf3b0f6837ffcddaa, 0x3a76faef934ddf41, 0xcec7ae583a9c8e35, 0xe4dd18c68f0260af, 0x2c0e5df1ad398eaa, 0x478df5236ae22e8c, 0xfb944c46fe865f39, 0xaa48f82f028132ba],
    [0x231b9ae2b76aca77, 0x292a76a712db0b40, 0x5850625dc8134491, 0x73137dd469810fb5, 0x8a12a6a202a474fd, 0xd36fd9daa78bdb80, 0xb34c5e733505706f, 0xbaf1cdca818d9d96],
    [0x2e99781335e8c641, 0xbddfe5cce47d560e, 0xf74e9bf32e5e040c, 0x1d7a709d65996be9, 0x670df36a9cf66cdd, 0xd05ef84a176a2875, 0x0f888e828cb1c44e, 0x1a79e9c9727b052c],
    [0x83497348628d84de, 0x2e9387d51f22a754, 0xb000068da2f852d6, 0x378c9e1190fd6fe5, 0x870027c316de7293, 0xe51a9d4462e047bb, 0x90ecf7f8c6251195, 0x655953bfbed90a9c],
];

fn br_dec32le(src: &[u8]) -> u32 {
    src[0] as u32 | ((src[1] as u32) << 8) | ((src[2] as u32) << 16) | ((src[3] as u32) << 24)
}

fn br_range_dec32le(v: &mut [u32], src: &[u8]) {
    for i in 0..v.len() {
        v[i] = br_dec32le(&src[4 * i..]);
    }
}

fn br_enc32le(dst: &mut [u8], x: u32) {
    dst[0] = x as u8;
    dst[1] = (x >> 8) as u8;
    dst[2] = (x >> 16) as u8;
    dst[3] = (x >> 24) as u8;
}

fn br_range_enc32le(dst: &mut [u8], v: &[u32]) {
    for i in 0..v.len() {
        br_enc32le(&mut dst[4 * i..], v[i]);
    }
}

fn br_aes_ct64_bitslice_Sbox(q: &mut [u64; 8]) {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = (q[7], q[6], q[5], q[4], q[3], q[2], q[1], q[0]);
    let y14 = x3 ^ x5; let y13 = x0 ^ x6; let y9 = x0 ^ x3; let y8 = x0 ^ x5;
    let t0 = x1 ^ x2; let y1 = t0 ^ x7; let y4 = y1 ^ x3; let y12 = y13 ^ y14;
    let y2 = y1 ^ x0; let y5 = y1 ^ x6; let y3 = y5 ^ y8;
    let t1 = x4 ^ y12; let y15 = t1 ^ x5; let y20 = t1 ^ x1;
    let y6 = y15 ^ x7; let y10 = y15 ^ t0; let y11 = y20 ^ y9;
    let y7 = x7 ^ y11; let y17 = y10 ^ y11; let y19 = y10 ^ y8;
    let y16 = t0 ^ y11; let y21 = y13 ^ y16; let y18 = x0 ^ y16;
    let t2 = y12 & y15; let t3 = y3 & y6; let t4 = t3 ^ t2; let t5 = y4 & x7; let t6 = t5 ^ t2;
    let t7 = y13 & y16; let t8 = y5 & y1; let t9 = t8 ^ t7; let t10 = y2 & y7; let t11 = t10 ^ t7;
    let t12 = y9 & y11; let t13 = y14 & y17; let t14 = t13 ^ t12; let t15 = y8 & y10; let t16 = t15 ^ t12;
    let t17 = t4 ^ t14; let t18 = t6 ^ t16; let t19 = t9 ^ t14; let t20 = t11 ^ t16;
    let t21 = t17 ^ y20; let t22 = t18 ^ y19; let t23 = t19 ^ y21; let t24 = t20 ^ y18;
    let t25 = t21 ^ t22; let t26 = t21 & t23; let t27 = t24 ^ t26;
    let t28 = t25 & t27; let t29 = t28 ^ t22; let t30 = t23 ^ t24;
    let t31 = t22 ^ t26; let t32 = t31 & t30; let t33 = t32 ^ t24;
    let t34 = t23 ^ t33; let t35 = t27 ^ t33; let t36 = t24 & t35; let t37 = t36 ^ t34;
    let t38 = t27 ^ t36; let t39 = t29 & t38; let t40 = t25 ^ t39;
    let t41 = t40 ^ t37; let t42 = t29 ^ t33; let t43 = t29 ^ t40;
    let t44 = t33 ^ t37; let t45 = t42 ^ t41;
    let z0 = t44 & y15; let z1 = t37 & y6; let z2 = t33 & x7; let z3 = t43 & y16;
    let z4 = t40 & y1; let z5 = t29 & y7; let z6 = t42 & y11; let z7 = t45 & y17;
    let z8 = t41 & y10; let z9 = t44 & y12; let z10 = t37 & y3; let z11 = t33 & y4;
    let z12 = t43 & y13; let z13 = t40 & y5; let z14 = t29 & y2; let z15 = t42 & y9;
    let z16 = t45 & y14; let z17 = t41 & y8;
    let t46 = z15 ^ z16; let t47 = z10 ^ z11; let t48 = z5 ^ z13; let t49 = z9 ^ z10;
    let t50 = z2 ^ z12; let t51 = z2 ^ z5; let t52 = z7 ^ z8; let t53 = z0 ^ z3;
    let t54 = z6 ^ z7; let t55 = z16 ^ z17; let t56 = z12 ^ t48; let t57 = t50 ^ t53;
    let t58 = z4 ^ t46; let t59 = z3 ^ t54; let t60 = t46 ^ t57; let t61 = z14 ^ t57;
    let t62 = t52 ^ t58; let t63 = t49 ^ t58; let t64 = z4 ^ t59; let t65 = t61 ^ t62;
    let t66 = z1 ^ t63;
    let s0 = t59 ^ t63; let s6 = t56 ^ !t62; let s7 = t48 ^ !t60;
    let t67 = t64 ^ t65;
    let s3 = t53 ^ t66; let s4 = t51 ^ t66; let s5 = t47 ^ t65;
    let s1 = t64 ^ !s3; let s2 = t55 ^ !t67;
    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3; q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

fn br_aes_ct_bitslice_Sbox(q: &mut [u32; 8]) {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = (q[7], q[6], q[5], q[4], q[3], q[2], q[1], q[0]);
    let y14 = x3 ^ x5; let y13 = x0 ^ x6; let y9 = x0 ^ x3; let y8 = x0 ^ x5;
    let t0 = x1 ^ x2; let y1 = t0 ^ x7; let y4 = y1 ^ x3; let y12 = y13 ^ y14;
    let y2 = y1 ^ x0; let y5 = y1 ^ x6; let y3 = y5 ^ y8;
    let t1 = x4 ^ y12; let y15 = t1 ^ x5; let y20 = t1 ^ x1;
    let y6 = y15 ^ x7; let y10 = y15 ^ t0; let y11 = y20 ^ y9;
    let y7 = x7 ^ y11; let y17 = y10 ^ y11; let y19 = y10 ^ y8;
    let y16 = t0 ^ y11; let y21 = y13 ^ y16; let y18 = x0 ^ y16;
    let t2 = y12 & y15; let t3 = y3 & y6; let t4 = t3 ^ t2; let t5 = y4 & x7; let t6 = t5 ^ t2;
    let t7 = y13 & y16; let t8 = y5 & y1; let t9 = t8 ^ t7; let t10 = y2 & y7; let t11 = t10 ^ t7;
    let t12 = y9 & y11; let t13 = y14 & y17; let t14 = t13 ^ t12; let t15 = y8 & y10; let t16 = t15 ^ t12;
    let t17 = t4 ^ t14; let t18 = t6 ^ t16; let t19 = t9 ^ t14; let t20 = t11 ^ t16;
    let t21 = t17 ^ y20; let t22 = t18 ^ y19; let t23 = t19 ^ y21; let t24 = t20 ^ y18;
    let t25 = t21 ^ t22; let t26 = t21 & t23; let t27 = t24 ^ t26;
    let t28 = t25 & t27; let t29 = t28 ^ t22; let t30 = t23 ^ t24;
    let t31 = t22 ^ t26; let t32 = t31 & t30; let t33 = t32 ^ t24;
    let t34 = t23 ^ t33; let t35 = t27 ^ t33; let t36 = t24 & t35; let t37 = t36 ^ t34;
    let t38 = t27 ^ t36; let t39 = t29 & t38; let t40 = t25 ^ t39;
    let t41 = t40 ^ t37; let t42 = t29 ^ t33; let t43 = t29 ^ t40;
    let t44 = t33 ^ t37; let t45 = t42 ^ t41;
    let z0 = t44 & y15; let z1 = t37 & y6; let z2 = t33 & x7; let z3 = t43 & y16;
    let z4 = t40 & y1; let z5 = t29 & y7; let z6 = t42 & y11; let z7 = t45 & y17;
    let z8 = t41 & y10; let z9 = t44 & y12; let z10 = t37 & y3; let z11 = t33 & y4;
    let z12 = t43 & y13; let z13 = t40 & y5; let z14 = t29 & y2; let z15 = t42 & y9;
    let z16 = t45 & y14; let z17 = t41 & y8;
    let t46 = z15 ^ z16; let t47 = z10 ^ z11; let t48 = z5 ^ z13; let t49 = z9 ^ z10;
    let t50 = z2 ^ z12; let t51 = z2 ^ z5; let t52 = z7 ^ z8; let t53 = z0 ^ z3;
    let t54 = z6 ^ z7; let t55 = z16 ^ z17; let t56 = z12 ^ t48; let t57 = t50 ^ t53;
    let t58 = z4 ^ t46; let t59 = z3 ^ t54; let t60 = t46 ^ t57; let t61 = z14 ^ t57;
    let t62 = t52 ^ t58; let t63 = t49 ^ t58; let t64 = z4 ^ t59; let t65 = t61 ^ t62;
    let t66 = z1 ^ t63;
    let s0 = t59 ^ t63; let s6 = t56 ^ !t62; let s7 = t48 ^ !t60;
    let t67 = t64 ^ t65;
    let s3 = t53 ^ t66; let s4 = t51 ^ t66; let s5 = t47 ^ t65;
    let s1 = t64 ^ !s3; let s2 = t55 ^ !t67;
    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3; q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

fn br_aes_ct_ortho(q: &mut [u32; 8]) {
    macro_rules! swapn32 { ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {
        let a = $x; let b = $y;
        $x = (a & $cl) | ((b & $cl) << $s);
        $y = ((a & $ch) >> $s) | (b & $ch);
    }}
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[0], q[1]);
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[2], q[3]);
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[4], q[5]);
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[6], q[7]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[0], q[2]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[1], q[3]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[4], q[6]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[5], q[7]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[0], q[4]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[1], q[5]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[2], q[6]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[3], q[7]);
}

fn shift_rows32(q: &mut [u32; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000FF)
            | ((x & 0x0000FC00) >> 2) | ((x & 0x00000300) << 6)
            | ((x & 0x00F00000) >> 4) | ((x & 0x000F0000) << 4)
            | ((x & 0xC0000000) >> 6) | ((x & 0x3F000000) << 2);
    }
}

fn mix_columns32(q: &mut [u32; 8]) {
    let (q0, q1, q2, q3, q4, q5, q6, q7) = (q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7]);
    let r0 = q0.rotate_right(8); let r1 = q1.rotate_right(8);
    let r2 = q2.rotate_right(8); let r3 = q3.rotate_right(8);
    let r4 = q4.rotate_right(8); let r5 = q5.rotate_right(8);
    let r6 = q6.rotate_right(8); let r7 = q7.rotate_right(8);
    q[0] = q7 ^ r7 ^ r0 ^ (q0 ^ r0).rotate_right(16);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ (q1 ^ r1).rotate_right(16);
    q[2] = q1 ^ r1 ^ r2 ^ (q2 ^ r2).rotate_right(16);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ (q3 ^ r3).rotate_right(16);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ (q4 ^ r4).rotate_right(16);
    q[5] = q4 ^ r4 ^ r5 ^ (q5 ^ r5).rotate_right(16);
    q[6] = q5 ^ r5 ^ r6 ^ (q6 ^ r6).rotate_right(16);
    q[7] = q6 ^ r6 ^ r7 ^ (q7 ^ r7).rotate_right(16);
}

fn br_aes_ct64_ortho(q: &mut [u64; 8]) {
    macro_rules! swapn { ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {
        let a = $x; let b = $y;
        $x = (a & $cl) | ((b & $cl) << $s);
        $y = ((a & $ch) >> $s) | (b & $ch);
    }}
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[0], q[1]);
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[2], q[3]);
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[4], q[5]);
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[6], q[7]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[0], q[2]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[1], q[3]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[4], q[6]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[5], q[7]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[0], q[4]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[1], q[5]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[2], q[6]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[3], q[7]);
}

fn br_aes_ct64_interleave_in(q0: &mut u64, q1: &mut u64, w: &[u32]) {
    let mut x0 = w[0] as u64; let mut x1 = w[1] as u64;
    let mut x2 = w[2] as u64; let mut x3 = w[3] as u64;
    x0 |= x0 << 16; x1 |= x1 << 16; x2 |= x2 << 16; x3 |= x3 << 16;
    x0 &= 0x0000FFFF0000FFFFu64; x1 &= 0x0000FFFF0000FFFFu64;
    x2 &= 0x0000FFFF0000FFFFu64; x3 &= 0x0000FFFF0000FFFFu64;
    x0 |= x0 << 8; x1 |= x1 << 8; x2 |= x2 << 8; x3 |= x3 << 8;
    x0 &= 0x00FF00FF00FF00FFu64; x1 &= 0x00FF00FF00FF00FFu64;
    x2 &= 0x00FF00FF00FF00FFu64; x3 &= 0x00FF00FF00FF00FFu64;
    *q0 = x0 | (x2 << 8); *q1 = x1 | (x3 << 8);
}

fn br_aes_ct64_interleave_out(w: &mut [u32], q0: u64, q1: u64) {
    let mut x0 = q0 & 0x00FF00FF00FF00FFu64; let mut x1 = q1 & 0x00FF00FF00FF00FFu64;
    let mut x2 = (q0 >> 8) & 0x00FF00FF00FF00FFu64; let mut x3 = (q1 >> 8) & 0x00FF00FF00FF00FFu64;
    x0 |= x0 >> 8; x1 |= x1 >> 8; x2 |= x2 >> 8; x3 |= x3 >> 8;
    x0 &= 0x0000FFFF0000FFFFu64; x1 &= 0x0000FFFF0000FFFFu64;
    x2 &= 0x0000FFFF0000FFFFu64; x3 &= 0x0000FFFF0000FFFFu64;
    w[0] = (x0 as u32) | ((x0 >> 16) as u32);
    w[1] = (x1 as u32) | ((x1 >> 16) as u32);
    w[2] = (x2 as u32) | ((x2 >> 16) as u32);
    w[3] = (x3 as u32) | ((x3 >> 16) as u32);
}

fn shift_rows(q: &mut [u64; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000000000FFFFu64)
            | ((x & 0x00000000FFF00000u64) >> 4) | ((x & 0x00000000000F0000u64) << 12)
            | ((x & 0x0000FF0000000000u64) >> 8) | ((x & 0x000000FF00000000u64) << 8)
            | ((x & 0xF000000000000000u64) >> 12) | ((x & 0x0FFF000000000000u64) << 4);
    }
}

fn mix_columns(q: &mut [u64; 8]) {
    let (q0, q1, q2, q3, q4, q5, q6, q7) = (q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7]);
    let r0 = q0.rotate_right(16); let r1 = q1.rotate_right(16);
    let r2 = q2.rotate_right(16); let r3 = q3.rotate_right(16);
    let r4 = q4.rotate_right(16); let r5 = q5.rotate_right(16);
    let r6 = q6.rotate_right(16); let r7 = q7.rotate_right(16);
    q[0] = q7 ^ r7 ^ r0 ^ (q0 ^ r0).rotate_right(32);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ (q1 ^ r1).rotate_right(32);
    q[2] = q1 ^ r1 ^ r2 ^ (q2 ^ r2).rotate_right(32);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ (q3 ^ r3).rotate_right(32);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ (q4 ^ r4).rotate_right(32);
    q[5] = q4 ^ r4 ^ r5 ^ (q5 ^ r5).rotate_right(32);
    q[6] = q5 ^ r5 ^ r6 ^ (q6 ^ r6).rotate_right(32);
    q[7] = q6 ^ r6 ^ r7 ^ (q7 ^ r7).rotate_right(32);
}

fn interleave_constant(out: &mut [u64; 8], inp: &[u8]) {
    let mut tmp = [0u32; 16];
    br_range_dec32le(&mut tmp, inp);
    for i in 0..4 {
        let mut q0 = 0u64;
        let mut q1 = 0u64;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &tmp[i * 4..]);
        out[i] = q0;
        out[i + 4] = q1;
    }
    br_aes_ct64_ortho(out);
}

fn interleave_constant32(out: &mut [u32; 8], inp: &[u8]) {
    for i in 0..4 {
        out[2 * i] = br_dec32le(&inp[4 * i..]);
        out[2 * i + 1] = br_dec32le(&inp[4 * i + 16..]);
    }
    br_aes_ct_ortho(out);
}

// ============================================================
// haraka512_perm, haraka512, haraka256
// ============================================================
fn haraka512_perm(out: &mut [u8; 64], inp: &[u8], ctx: &SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];
    br_range_dec32le(&mut w, inp);
    for i in 0..4 {
        let mut q0 = 0u64;
        let mut q1 = 0u64;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &w[i * 4..]);
        q[i] = q0;
        q[i + 4] = q1;
    }
    br_aes_ct64_ortho(&mut q);
    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct64_bitslice_Sbox(&mut q);
            shift_rows(&mut q);
            mix_columns(&mut q);
            for k in 0..8 { q[k] ^= ctx.tweaked512_rc64[2 * i + j][k]; }
        }
        for j in 0..8 {
            let tmp_q = q[j];
            q[j] = (tmp_q & 0x0001000100010001) << 5
                | (tmp_q & 0x0002000200020002) << 12
                | (tmp_q & 0x0004000400040004) >> 1
                | (tmp_q & 0x0008000800080008) << 6
                | (tmp_q & 0x0020002000200020) << 9
                | (tmp_q & 0x0040004000400040) >> 4
                | (tmp_q & 0x0080008000800080) << 3
                | (tmp_q & 0x2100210021002100) >> 5
                | (tmp_q & 0x0210021002100210) << 2
                | (tmp_q & 0x0800080008000800) << 4
                | (tmp_q & 0x1000100010001000) >> 12
                | (tmp_q & 0x4000400040004000) >> 10
                | (tmp_q & 0x8400840084008400) >> 3;
        }
    }
    br_aes_ct64_ortho(&mut q);
    for i in 0..4 {
        br_aes_ct64_interleave_out(&mut w[i * 4..], q[i], q[i + 4]);
    }
    br_range_enc32le(out, &w);
}

fn haraka512(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut buf = [0u8; 64];
    let mut inp64 = [0u8; 64];
    inp64[..64.min(inp.len())].copy_from_slice(&inp[..64.min(inp.len())]);
    haraka512_perm(&mut buf, &inp64, ctx);
    for i in 0..64 { buf[i] ^= inp64[i]; }
    out[..8].copy_from_slice(&buf[8..16]);
    out[8..16].copy_from_slice(&buf[24..32]);
    out[16..24].copy_from_slice(&buf[32..40]);
    out[24..32].copy_from_slice(&buf[48..56]);
}

fn haraka256(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut q = [0u32; 8];
    for i in 0..4 {
        q[2 * i] = br_dec32le(&inp[4 * i..]);
        q[2 * i + 1] = br_dec32le(&inp[4 * i + 16..]);
    }
    br_aes_ct_ortho(&mut q);
    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct_bitslice_Sbox(&mut q);
            shift_rows32(&mut q);
            mix_columns32(&mut q);
            for k in 0..8 { q[k] ^= ctx.tweaked256_rc32[2 * i + j][k]; }
        }
        for j in 0..8 {
            let tmp_q = q[j];
            q[j] = (tmp_q & 0x81818181)
                | (tmp_q & 0x02020202) << 1
                | (tmp_q & 0x04040404) << 2
                | (tmp_q & 0x08080808) << 3
                | (tmp_q & 0x10101010) >> 3
                | (tmp_q & 0x20202020) >> 2
                | (tmp_q & 0x40404040) >> 1;
        }
    }
    br_aes_ct_ortho(&mut q);
    let mut result = [0u8; 32];
    for i in 0..4 {
        br_enc32le(&mut result[4 * i..], q[2 * i]);
        br_enc32le(&mut result[4 * i + 16..], q[2 * i + 1]);
    }
    for i in 0..32 { result[i] ^= inp[i]; }
    out[..32].copy_from_slice(&result);
}

// ============================================================
// haraka sponge
// ============================================================
fn haraka_S_absorb(s: &mut [u8; 64], m: &[u8], p: u8, ctx: &SpxCtx) {
    let r = HARAKAS_RATE;
    let mut off = 0usize;
    let mlen = m.len();
    while off + r <= mlen {
        for i in 0..r { s[i] ^= m[off + i]; }
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(s);
        haraka512_perm(s, &tmp, ctx);
        off += r;
    }
    let rem = mlen - off;
    let mut t = [0u8; 64]; // >= HARAKAS_RATE
    for i in 0..rem { t[i] = m[off + i]; }
    t[rem] = p;
    t[r - 1] |= 128;
    for i in 0..r { s[i] ^= t[i]; }
}

fn haraka_S(out: &mut [u8], outlen: usize, inp: &[u8], ctx: &SpxCtx) {
    let mut s = [0u8; 64];
    haraka_S_absorb(&mut s, inp, 0x1F, ctx);
    let mut produced = 0usize;
    while produced + 32 <= outlen {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s);
        haraka512_perm(&mut s, &tmp, ctx);
        out[produced..produced + 32].copy_from_slice(&s[..32]);
        produced += 32;
    }
    if produced < outlen {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s);
        haraka512_perm(&mut s, &tmp, ctx);
        let mut d = [0u8; 32];
        d.copy_from_slice(&s[..32]);
        out[produced..outlen].copy_from_slice(&d[..outlen - produced]);
    }
}

fn haraka_S_inc_init(s_inc: &mut [u8; 65]) {
    for i in 0..65 { s_inc[i] = 0; }
}

fn haraka_S_inc_absorb(s_inc: &mut [u8; 65], m: &[u8], mlen: usize, ctx: &SpxCtx) {
    let mut off = 0usize;
    let mut remaining = mlen;
    while remaining + (s_inc[64] as usize) >= HARAKAS_RATE {
        let take = HARAKAS_RATE - s_inc[64] as usize;
        for i in 0..take {
            s_inc[s_inc[64] as usize + i] ^= m[off + i];
        }
        remaining -= take;
        off += take;
        s_inc[64] = 0;
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        let mut out64 = [0u8; 64];
        haraka512_perm(&mut out64, &tmp, ctx);
        s_inc[..64].copy_from_slice(&out64);
    }
    for i in 0..remaining {
        s_inc[s_inc[64] as usize + i] ^= m[off + i];
    }
    s_inc[64] += remaining as u8;
}

fn haraka_S_inc_finalize(s_inc: &mut [u8; 65]) {
    s_inc[s_inc[64] as usize] ^= 0x1F;
    s_inc[HARAKAS_RATE - 1] ^= 128;
    s_inc[64] = 0;
}

fn haraka_S_inc_squeeze(out: &mut [u8], mut outlen: usize, s_inc: &mut [u8; 65], ctx: &SpxCtx) {
    let mut out_off = 0usize;
    // consume leftover
    let avail = s_inc[64] as usize;
    let take = outlen.min(avail);
    for i in 0..take {
        out[out_off + i] = s_inc[HARAKAS_RATE - avail + i];
    }
    out_off += take;
    outlen -= take;
    s_inc[64] -= take as u8;

    while outlen > 0 {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        let mut out64 = [0u8; 64];
        haraka512_perm(&mut out64, &tmp, ctx);
        s_inc[..64].copy_from_slice(&out64);
        let take = outlen.min(HARAKAS_RATE);
        out[out_off..out_off + take].copy_from_slice(&s_inc[..take]);
        out_off += take;
        outlen -= take;
        s_inc[64] = (HARAKAS_RATE - take) as u8;
    }
}

fn tweak_constants(ctx: &mut SpxCtx) {
    // Copy standard constants first
    ctx.tweaked512_rc64 = HARAKA512_RC64;
    // Generate tweaked constants using haraka_S with pub_seed
    let mut buf = vec![0u8; 40 * 16];
    haraka_S(&mut buf, 40 * 16, &ctx.pub_seed, ctx);
    for i in 0..10 {
        interleave_constant32(&mut ctx.tweaked256_rc32[i], &buf[32 * i..]);
        interleave_constant(&mut ctx.tweaked512_rc64[i], &buf[64 * i..]);
    }
}

// ============================================================
// hash_haraka (initialize_hash_function, prf_addr, gen_message_random, hash_message)
// ============================================================
fn initialize_hash_function(ctx: &mut SpxCtx) {
    tweak_constants(ctx);
}

fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];
    let ab = addr_bytes(addr);
    buf[..SPX_ADDR_BYTES].copy_from_slice(&ab);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);
    haraka512(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn gen_message_random(r_out: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, ctx: &SpxCtx) {
    let mut s_inc = [0u8; 65];
    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, sk_prf, SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, optrand, SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    haraka_S_inc_squeeze(r_out, SPX_N, &mut s_inc, ctx);
}

fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                r: &[u8], pk: &[u8], m: &[u8], mlen: u64, ctx: &SpxCtx) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];
    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, r, SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, &pk[SPX_N..], SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    haraka_S_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc, ctx);

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
// thash_haraka_simple
// ============================================================
fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let ab = addr_bytes(addr);
    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        buf_tmp[..32].copy_from_slice(&ab);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&inp[..SPX_N]);
        let mut outbuf = [0u8; 32];
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let total = SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; total];
        buf[..32].copy_from_slice(&ab);
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + inblocks * SPX_N]
            .copy_from_slice(&inp[..inblocks * SPX_N]);
        haraka_S(out, SPX_N, &buf, ctx);
    }
}

// ============================================================
// wots
// ============================================================
fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    let mut i = start;
    while i < start + steps && i < SPX_WOTS_W as u32 {
        set_hash_addr(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash(out, &tmp, 1, ctx, addr);
        i += 1;
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
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let tmp: Vec<u32> = lengths[..SPX_WOTS_LEN1].to_vec();
    wots_checksum(&mut lengths[SPX_WOTS_LEN1..], &tmp);
}

fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..],
            &sig[i * SPX_N..],
            lengths[i],
            SPX_WOTS_W as u32 - 1 - lengths[i],
            ctx,
            addr,
        );
    }
}

// ============================================================
// fors
// ============================================================
fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, fors_leaf_addr: &mut [u32; 8]) {
    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
    fors_sk_to_leaf(leaf, &tmp, ctx, fors_leaf_addr);
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
    let mut fors_leaf_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_leaf_addr, fors_addr);
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

        fors_treehashx1(
            &mut roots[i * SPX_N..],
            &mut sig[sig_off..],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_leaf_addr,
            true, // is_fors
        );
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

        compute_root(
            &mut roots[i * SPX_N..],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sig_off..],
            SPX_FORS_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// ============================================================
// compute_root
// ============================================================
fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
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

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);
        let tmp_buf = buffer;
        if leaf_idx & 1 != 0 {
            thash(&mut buffer[SPX_N..], &tmp_buf, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            thash(&mut buffer[..SPX_N], &tmp_buf, 2, ctx, addr);
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
// treehash x1 (unified for wots and fors)
// ============================================================
#[allow(clippy::too_many_arguments)]
fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], leaf_addr: &mut [u32; 8],
    _is_fors: bool,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, leaf_addr);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp_cur = current.clone();
            thash(&mut current[SPX_N..], &tmp_cur, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        // save left child
        let h_val = {
            let mut h = 0u32;
            let mut ti = idx;
            let mut tl = leaf_idx;
            loop {
                if h == tree_height { break; }
                if (ti ^ tl) == 0x01 { }
                if (ti & 1) == 0 && idx < max_idx { break; }
                ti >>= 1; tl >>= 1; h += 1;
            }
            h
        };
        stack[h_val as usize * SPX_N..(h_val as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..SPX_N * 2]);
    }
}

// ============================================================
// wots_gen_leafx1
// ============================================================
struct LeafInfoX1 {
    wots_sig: Vec<u8>,
    wots_sign_leaf: u32,
    wots_steps: [u32; SPX_WOTS_LEN],
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[i * SPX_N..], ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N]
                    .copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            thash(&mut pk_buffer[i * SPX_N..], &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

// ============================================================
// wots_treehashx1
// ============================================================
fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp_cur = current.clone();
            thash(&mut current[SPX_N..], &tmp_cur, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        // Recompute h to know where to save
        let h_val = {
            let mut h = 0u32;
            let mut ti = idx;
            let mut tl = leaf_idx;
            loop {
                if h == tree_height { break; }
                if (ti & 1) == 0 && idx < max_idx { break; }
                ti >>= 1; tl >>= 1; h += 1;
            }
            h
        };
        stack[h_val as usize * SPX_N..(h_val as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..SPX_N * 2]);
    }
}

// ============================================================
// merkle
// ============================================================
fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
               wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let auth_path_off = SPX_WOTS_BYTES;
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    let mut info = LeafInfoX1 {
        wots_sig: vec![0u8; SPX_WOTS_BYTES],
        wots_sign_leaf: idx_leaf,
        wots_steps: steps,
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    wots_treehashx1(root, &mut sig[auth_path_off..], ctx,
                    idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);

    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
}

fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];
    set_layer_addr(&mut top_tree_addr, SPX_D as u32 - 1);
    set_layer_addr(&mut wots_addr, SPX_D as u32 - 1);
    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}

// ============================================================
// sign
// ============================================================
fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) {
    let mut ctx = SpxCtx::new();
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    initialize_hash_function(&mut ctx);
    merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
}

fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

fn crypto_sign_signature(sig: &mut [u8], m: &[u8], mlen: usize, sk: &[u8]) -> usize {
    let mut ctx = SpxCtx::new();
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand, SPX_N as u64);
    gen_message_random(sig, sk_prf, &optrand, m, mlen as u64, &ctx);

    // Build pk_full for hash_message: [PUB_SEED || root]
    let mut pk_full = [0u8; SPX_PK_BYTES];
    pk_full[..SPX_N].copy_from_slice(&pk[..SPX_N]);
    pk_full[SPX_N..2 * SPX_N].copy_from_slice(&pk[SPX_N..2 * SPX_N]);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, &pk_full, m, mlen as u64, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }
    SPX_BYTES
}

fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    if siglen != SPX_BYTES { return -1; }
    let mut ctx = SpxCtx::new();
    let pub_root = &pk[SPX_N..2 * SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);
    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }
    if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
    0
}

fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let siglen = crypto_sign_signature(sm, m, mlen as usize, sk);
    // memmove sm + SPX_BYTES, m, mlen
    let ml = mlen as usize;
    for i in (0..ml).rev() {
        sm[SPX_BYTES + i] = m[i];
    }
    *smlen = (siglen + ml) as u64;
    0
}

fn crypto_sign_open(m_out: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    let sml = smlen as usize;
    if sml < SPX_BYTES {
        for i in 0..sml { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }
    *mlen = (sml - SPX_BYTES) as u64;
    if crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], *mlen as usize, pk) != 0 {
        for i in 0..sml { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }
    let ml = *mlen as usize;
    for i in 0..ml {
        m_out[i] = sm[SPX_BYTES + i];
    }
    0
}

// ============================================================
// KAT transcript (HARAKA_TR variant)
// ============================================================
struct KatTrCtx {
    inner: SpxCtx,
    s: [u8; 65],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    for i in 0..SPX_N {
        ctx.inner.pub_seed[i] = 0;
        ctx.inner.sk_seed[i] = 0;
    }
    tweak_constants(&mut ctx.inner);
    haraka_S_inc_init(&mut ctx.s);
    let tag = b"KAT-TRANSCRIPT-v1-HARAKA";
    haraka_S_inc_absorb(&mut ctx.s, tag, tag.len(), &ctx.inner);
    let sep = [0u8; 1];
    haraka_S_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    haraka_S_inc_absorb(&mut ctx.s, label, label.len(), &ctx.inner);
    let sep = [0u8; 1];
    haraka_S_inc_absorb(&mut ctx.s, &sep, 1, &ctx.inner);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    haraka_S_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
    haraka_S_inc_absorb(&mut ctx.s, &le, 8, &ctx.inner);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    haraka_S_inc_absorb(&mut ctx.s, &lenle, 8, &ctx.inner);
    if len > 0 {
        haraka_S_inc_absorb(&mut ctx.s, buf, len, &ctx.inner);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    haraka_S_inc_finalize(&mut ctx.s);
    haraka_S_inc_squeeze(out32, 32, &mut ctx.s, &ctx.inner);
}

// ============================================================
// main
// ============================================================
fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx {
        inner: SpxCtx::new(),
        s: [0u8; 65],
    };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME, CRYPTO_ALGNAME.len());
    kat_tr_absorb_label(&mut tctx, b"SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);

        kat_tr_absorb_label(&mut tctx, b"count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, b"seed");
        kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen: u64 = (BASE_MLEN * (i + 1)) as u64;
        if mlen > (BASE_MLEN * LOOP_COUNT) as u64 {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen);

        randombytes(&mut msg, mlen);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, mlen as usize);

        let ml = mlen as usize;
        for j in 0..ml { m[j] = 0; }
        for j in 0..ml + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..ml + CRYPTO_BYTES { sm[j] = 0; }
        m[..ml].copy_from_slice(&msg[..ml]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen: u64 = 0;
        let ret = crypto_sign(&mut sm, &mut smlen, &m[..ml], mlen, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(-2);
        }
        if mlen1 != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(-2);
        }
        if m[..ml] != m1[..ml] {
            eprintln!("m mismatch");
            std::process::exit(-2);
        }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 {
        print!("{:02X}", digest[i]);
    }
    println!();
}
