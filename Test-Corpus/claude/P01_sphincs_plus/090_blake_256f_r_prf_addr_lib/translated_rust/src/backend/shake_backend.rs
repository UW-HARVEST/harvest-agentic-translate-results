// SHAKE backend - direct translation from c_src/lib/shake.

#![allow(non_snake_case)]

use crate::address::addr_to_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull_rs;

const NROUNDS: usize = 24;
const SHAKE256_RATE: usize = 136;

#[inline]
fn rol(a: u64, n: u32) -> u64 {
    a.rotate_left(n)
}

const KECCAK_RC: [u64; NROUNDS] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

fn load64(x: &[u8]) -> u64 {
    let mut r: u64 = 0;
    for i in 0..8 {
        r |= (x[i] as u64) << (8 * i);
    }
    r
}
fn store64(x: &mut [u8], v: u64) {
    for i in 0..8 {
        x[i] = (v >> (8 * i)) as u8;
    }
}

fn keccak_f1600(state: &mut [u64; 25]) {
    // straightforward implementation matching the C
    let mut a = *state;
    for round in (0..NROUNDS).step_by(1) {
        let mut bc = [0u64; 5];
        for i in 0..5 {
            bc[i] = a[i] ^ a[i + 5] ^ a[i + 10] ^ a[i + 15] ^ a[i + 20];
        }
        let mut d = [0u64; 5];
        for i in 0..5 {
            d[i] = bc[(i + 4) % 5] ^ rol(bc[(i + 1) % 5], 1);
        }
        for i in 0..5 {
            for j in 0..5 {
                a[i + 5 * j] ^= d[i];
            }
        }

        // Rho + Pi
        let r_consts: [u32; 25] = [
            0, 1, 62, 28, 27,
            36, 44, 6, 55, 20,
            3, 10, 43, 25, 39,
            41, 45, 15, 21, 8,
            18, 2, 61, 56, 14,
        ];
        let pi_dest: [usize; 25] = [
            0, 10, 20, 5, 15,
            16, 1, 11, 21, 6,
            7, 17, 2, 12, 22,
            23, 8, 18, 3, 13,
            14, 24, 9, 19, 4,
        ];
        let mut b = [0u64; 25];
        for i in 0..25 {
            b[pi_dest[i]] = rol(a[i], r_consts[i]);
        }
        // Chi
        for j in 0..5 {
            let row = [b[5 * j], b[5 * j + 1], b[5 * j + 2], b[5 * j + 3], b[5 * j + 4]];
            for i in 0..5 {
                a[5 * j + i] = row[i] ^ ((!row[(i + 1) % 5]) & row[(i + 2) % 5]);
            }
        }
        // Iota
        a[0] ^= KECCAK_RC[round];
    }
    *state = a;
}

fn keccak_inc_init(s_inc: &mut [u64; 26]) {
    for v in s_inc.iter_mut() {
        *v = 0;
    }
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: u32, m: &[u8]) {
    let mut mlen = m.len();
    let mut idx = 0;
    let r_us = r as u64;
    while mlen as u64 + s_inc[25] >= r_us {
        let chunk = (r_us - s_inc[25]) as usize;
        for i in 0..chunk {
            let pos = (s_inc[25] + i as u64) as usize;
            s_inc[pos >> 3] ^= (m[idx + i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= chunk;
        idx += chunk;
        s_inc[25] = 0;
        let st: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(st);
    }
    for i in 0..mlen {
        let pos = (s_inc[25] + i as u64) as usize;
        s_inc[pos >> 3] ^= (m[idx + i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64; 26], r: u32, p: u8) {
    let pos = s_inc[25] as usize;
    s_inc[pos >> 3] ^= (p as u64) << (8 * (pos & 0x07));
    let r_us = r as usize;
    s_inc[(r_us - 1) >> 3] ^= 128u64 << (8 * ((r_us - 1) & 0x07));
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(out: &mut [u8], s_inc: &mut [u64; 26], r: u32) {
    let mut outlen = out.len();
    let r_us = r as u64;
    let mut written = 0;
    let mut i: usize = 0;
    while i < outlen && (i as u64) < s_inc[25] {
        let pos = ((r_us - s_inc[25]) + i as u64) as usize;
        out[i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    written += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        let st: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(st);
        let mut k: usize = 0;
        while k < outlen && (k as u64) < r_us {
            out[written + k] = (s_inc[k >> 3] >> (8 * (k & 0x07))) as u8;
            k += 1;
        }
        written += k;
        outlen -= k;
        s_inc[25] = r_us - (k as u64);
    }
}

fn keccak_absorb(s: &mut [u64; 25], r: u32, m: &[u8], p: u8) {
    let r_us = r as usize;
    let mut t = vec![0u8; 200];
    for v in s.iter_mut() {
        *v = 0;
    }
    let mut mlen = m.len();
    let mut idx = 0;
    while mlen >= r_us {
        for i in 0..r_us / 8 {
            s[i] ^= load64(&m[idx + 8 * i..]);
        }
        keccak_f1600(s);
        mlen -= r_us;
        idx += r_us;
    }
    for i in 0..r_us {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = m[idx + i];
    }
    t[mlen] = p;
    t[r_us - 1] |= 128;
    for i in 0..r_us / 8 {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u64; 25], r: u32) {
    let r_us = r as usize;
    let mut written = 0;
    for _ in 0..nblocks {
        keccak_f1600(s);
        for i in 0..(r_us >> 3) {
            store64(&mut h[written + 8 * i..], s[i]);
        }
        written += r_us;
    }
}

pub fn shake256(out: &mut [u8], inp: &[u8]) {
    let outlen = out.len();
    let nblocks = outlen / SHAKE256_RATE;
    let mut s = [0u64; 25];
    keccak_absorb(&mut s, SHAKE256_RATE as u32, inp, 0x1F);
    keccak_squeezeblocks(out, nblocks, &mut s, SHAKE256_RATE as u32);
    let mut written = nblocks * SHAKE256_RATE;
    let leftover = outlen - written;
    if leftover > 0 {
        let mut t = vec![0u8; SHAKE256_RATE];
        keccak_squeezeblocks(&mut t, 1, &mut s, SHAKE256_RATE as u32);
        for i in 0..leftover {
            out[written + i] = t[i];
        }
        written += leftover;
        let _ = written;
    }
}

pub fn shake256_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}
pub fn shake256_inc_absorb(s_inc: &mut [u64; 26], inp: &[u8]) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE as u32, inp);
}
pub fn shake256_inc_finalize(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE as u32, 0x1F);
}
pub fn shake256_inc_squeeze(out: &mut [u8], s_inc: &mut [u64; 26]) {
    keccak_inc_squeeze(out, s_inc, SHAKE256_RATE as u32);
}

// Backend impl

pub fn initialize_hash_function_impl(_ctx: &mut SpxCtx) {}

pub fn prf_addr_impl(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
    let mut tmp = vec![0u8; SPX_N];
    shake256(&mut tmp, &buf);
    out[..SPX_N].copy_from_slice(&tmp);
}

pub fn gen_message_random_impl(
    R: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut s_inc = [0u64; 26];
    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, sk_prf);
    shake256_inc_absorb(&mut s_inc, optrand);
    shake256_inc_absorb(&mut s_inc, m);
    shake256_inc_finalize(&mut s_inc);
    let mut tmp = vec![0u8; SPX_N];
    shake256_inc_squeeze(&mut tmp, &mut s_inc);
    R.copy_from_slice(&tmp);
}

pub fn hash_message_impl(
    digest: &mut [u8],
    R: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) -> (u64, u32) {
    let spx_tree_bits = SPX_TREE_HEIGHT * (SPX_D - 1);
    let spx_tree_bytes = (spx_tree_bits + 7) / 8;
    let spx_leaf_bits = SPX_TREE_HEIGHT;
    let spx_leaf_bytes = (spx_leaf_bits + 7) / 8;
    let spx_dgst_bytes = SPX_FORS_MSG_BYTES + spx_tree_bytes + spx_leaf_bytes;

    let mut buf = vec![0u8; spx_dgst_bytes];
    let mut s_inc = [0u64; 26];
    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, R);
    shake256_inc_absorb(&mut s_inc, pk);
    shake256_inc_absorb(&mut s_inc, m);
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(&mut buf, &mut s_inc);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree = if SPX_D == 1 {
        0u64
    } else {
        let mut t = bytes_to_ull_rs(&buf[bufp..bufp + spx_tree_bytes]);
        t &= !0u64 >> (64 - spx_tree_bits);
        t
    };
    bufp += spx_tree_bytes;

    let mut leaf_idx = bytes_to_ull_rs(&buf[bufp..bufp + spx_leaf_bytes]) as u32;
    leaf_idx &= !0u32 >> (32 - spx_leaf_bits);

    (tree, leaf_idx)
}

pub fn thash_impl(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(feature = "robust")]
    {
        thash_robust(out, inp, inblocks, ctx, addr);
    }
    #[cfg(feature = "simple")]
    {
        thash_simple(out, inp, inblocks, ctx, addr);
    }
}

#[cfg(feature = "robust")]
fn thash_robust(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N];
    let mut bitmask = vec![0u8; inblocks_us * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    let buf_clone: Vec<u8> = buf[..SPX_N + SPX_ADDR_BYTES].to_vec();
    shake256(&mut bitmask, &buf_clone);
    for i in 0..inblocks_us * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }
    let mut tmp = vec![0u8; SPX_N];
    shake256(&mut tmp, &buf);
    out[..SPX_N].copy_from_slice(&tmp);
}

#[cfg(feature = "simple")]
fn thash_simple(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&inp[..inblocks_us * SPX_N]);
    let mut tmp = vec![0u8; SPX_N];
    shake256(&mut tmp, &buf);
    out[..SPX_N].copy_from_slice(&tmp);
}

// FFI exports for fips202 functions used by the driver.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init_c(s: *mut u64) {
    let s = unsafe { &mut *(s as *mut [u64; 26]) };
    shake256_inc_init(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb_c(s: *mut u64, inp: *const u8, inlen: usize) {
    let s = unsafe { &mut *(s as *mut [u64; 26]) };
    let in_slice = unsafe { core::slice::from_raw_parts(inp, inlen) };
    shake256_inc_absorb(s, in_slice);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize_c(s: *mut u64) {
    let s = unsafe { &mut *(s as *mut [u64; 26]) };
    shake256_inc_finalize(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze_c(out: *mut u8, outlen: usize, s: *mut u64) {
    let s = unsafe { &mut *(s as *mut [u64; 26]) };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, outlen) };
    shake256_inc_squeeze(out_slice, s);
}
