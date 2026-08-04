// BLAKE hash implementation (PRF, message hash)
#![cfg(feature = "blake")]
#![allow(dead_code)]

use crate::blake::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull;

#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
#[cfg(any(feature = "128f", feature = "128s"))]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    // Note: original C calls blake256(outbuf, buf, SPX_N + SPX_ADDR_BYTES);
    // So it does NOT include sk_seed in the hash even though it copies it!
    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
fn blakex_init(s: &mut BlakeStateAny) {
    blake512_init(&mut s.s512);
}
#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
fn blakex_update(s: &mut BlakeStateAny, data: &[u8], datalen: u64) {
    blake512_update(&mut s.s512, data, datalen);
}
#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
fn blakex_final(s: &mut BlakeStateAny, out: &mut [u8]) {
    blake512_final(&mut s.s512, out);
}
#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
fn blakex_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    blake512_mgf1(out, outlen, input, inlen);
}

#[cfg(any(feature = "128f", feature = "128s"))]
fn blakex_init(s: &mut BlakeStateAny) {
    blake256_init(&mut s.s256);
}
#[cfg(any(feature = "128f", feature = "128s"))]
fn blakex_update(s: &mut BlakeStateAny, data: &[u8], datalen: u64) {
    blake256_update(&mut s.s256, data, datalen);
}
#[cfg(any(feature = "128f", feature = "128s"))]
fn blakex_final(s: &mut BlakeStateAny, out: &mut [u8]) {
    blake256_final(&mut s.s256, out);
}
#[cfg(any(feature = "128f", feature = "128s"))]
fn blakex_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    blake256_mgf1(out, outlen, input, inlen);
}

#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
struct BlakeStateAny {
    s512: BlakeState512,
}
#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
impl BlakeStateAny {
    fn new() -> Self {
        Self {
            s512: BlakeState512::new(),
        }
    }
}

#[cfg(any(feature = "128f", feature = "128s"))]
struct BlakeStateAny {
    s256: BlakeState256,
}
#[cfg(any(feature = "128f", feature = "128s"))]
impl BlakeStateAny {
    fn new() -> Self {
        Self {
            s256: BlakeState256::new(),
        }
    }
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut s = BlakeStateAny::new();
    blakex_init(&mut s);
    // Note: C calls blakeX_update with byte lengths (not bit-lengths) in this
    // function — preserved verbatim for byte-identical output.
    blakex_update(&mut s, sk_prf, SPX_N as u64);
    blakex_update(&mut s, optrand, SPX_N as u64);
    blakex_update(&mut s, m, mlen);
    // blake512_final writes 64 bytes; R buffer is SPX_N. Use a temp buffer.
    let mut tmp = vec![0u8; SPX_BLAKEX_OUTPUT_BYTES];
    blakex_final(&mut s, &mut tmp);
    let n = r.len().min(SPX_BLAKEX_OUTPUT_BYTES);
    r[..n].copy_from_slice(&tmp[..n]);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = vec![0u8; SPX_DGST_BYTES];
    let mut seed = vec![0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeStateAny::new();
    blakex_init(&mut s);

    // Note: C uses byte lengths here (not bit lengths). Preserved verbatim.
    blakex_update(&mut s, r, SPX_N as u64);
    blakex_update(&mut s, pk, SPX_PK_BYTES as u64);
    blakex_update(&mut s, m, mlen);
    blakex_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blakex_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let bufp_off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp_off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp_off = bufp_off + SPX_TREE_BYTES;
    *leaf_idx = bytes_to_ull(&buf[bufp_off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
