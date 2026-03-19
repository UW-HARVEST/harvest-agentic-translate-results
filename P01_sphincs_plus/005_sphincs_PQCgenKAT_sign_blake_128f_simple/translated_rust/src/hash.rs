// Unified hash interface. Dispatches to the correct backend via features.
// Also re-exports thash.

use crate::params::*;
use crate::context::SpxCtx;

// --- Blake backend ---
#[cfg(feature = "blake")]
mod blake_impl {
    use super::*;
    use crate::blake::*;

    pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

    pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
        let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8] = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
        buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);
        blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
        if SPX_N >= 24 {
            let mut s = BlakeState512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
            blake512_init(&mut s);
            blake512_update(&mut s, &sk_prf[..SPX_N], (SPX_N as u64) * 8);
            blake512_update(&mut s, &optrand[..SPX_N], (SPX_N as u64) * 8);
            blake512_update(&mut s, m, mlen * 8);
            let mut tmp = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
            blake512_final(&mut s, &mut tmp);
            r[..SPX_N].copy_from_slice(&tmp[..SPX_N]);
        } else {
            let mut s = BlakeState256 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;64] };
            blake256_init(&mut s);
            blake256_update(&mut s, &sk_prf[..SPX_N], (SPX_N as u64) * 8);
            blake256_update(&mut s, &optrand[..SPX_N], (SPX_N as u64) * 8);
            blake256_update(&mut s, m, mlen * 8);
            let mut tmp = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
            blake256_final(&mut s, &mut tmp);
            r[..SPX_N].copy_from_slice(&tmp[..SPX_N]);
        }
    }

    pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                        r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
        let mut buf = vec![0u8; SPX_DGST_BYTES];

        if SPX_N >= 24 {
            let blakex_output = SPX_BLAKE512_OUTPUT_BYTES;
            let mut seed = vec![0u8; 2 * SPX_N + blakex_output];
            let mut s = BlakeState512 { h:[0;8], s:[0;4], t:[0;2], buflen:0, nullt:0, buf:[0;128] };
            blake512_init(&mut s);
            blake512_update(&mut s, &r[..SPX_N], (SPX_N as u64)*8);
            blake512_update(&mut s, &pk[..SPX_PK_BYTES], (SPX_PK_BYTES as u64)*8);
            blake512_update(&mut s, m, mlen*8);
            blake512_final(&mut s, &mut seed[2*SPX_N..]);
            seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
            seed[SPX_N..2*SPX_N].copy_from_slice(&pk[..SPX_N]);
            blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2*SPX_N + blakex_output);
        } else {
            let blakex_output = SPX_BLAKE256_OUTPUT_BYTES;
            let mut seed = vec![0u8; 2 * SPX_N + blakex_output];
            let mut s = BlakeState256 { h:[0;8], s:[0;4], t:[0;2], buflen:0, nullt:0, buf:[0;64] };
            blake256_init(&mut s);
            blake256_update(&mut s, &r[..SPX_N], (SPX_N as u64)*8);
            blake256_update(&mut s, &pk[..SPX_PK_BYTES], (SPX_PK_BYTES as u64)*8);
            blake256_update(&mut s, m, mlen*8);
            blake256_final(&mut s, &mut seed[2*SPX_N..]);
            seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
            seed[SPX_N..2*SPX_N].copy_from_slice(&pk[..SPX_N]);
            blake256_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2*SPX_N + blakex_output);
        }

        digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
        let mut off = SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            *tree = crate::utils::bytes_to_ull(&buf[off..], SPX_TREE_BYTES);
            *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
        }
        off += SPX_TREE_BYTES;

        *leaf_idx = crate::utils::bytes_to_ull(&buf[off..], SPX_LEAF_BYTES) as u32;
        *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
    }

    // thash - simple variant
    #[cfg(feature = "simple")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        if SPX_BLAKE512 && inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; buflen];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8] = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
        buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&input[..inblocks * SPX_N]);
        blake256(&mut outbuf, &buf, buflen as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(feature = "simple")]
    fn thash_512(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; buflen];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8] = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
        buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&input[..inblocks * SPX_N]);
        blake512(&mut outbuf, &buf, buflen as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    // thash - robust variant
    #[cfg(feature = "robust")]
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        if SPX_BLAKE512 && inblocks > 1 {
            thash_512_robust(out, input, inblocks, ctx, addr);
            return;
        }
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; buflen];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8] = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
        blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);
        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }
        blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    #[cfg(feature = "robust")]
    fn thash_512_robust(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; buflen];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8] = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
        blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);
        for i in 0..inblocks * SPX_N {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }
        blake512(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

// --- Stub backends for sha2/shake/haraka (minimal to compile) ---
#[cfg(feature = "sha2")]
mod sha2_impl {
    use super::*;
    pub fn initialize_hash_function(_ctx: &mut SpxCtx) { unimplemented!("sha2 backend") }
    pub fn prf_addr(_out: &mut [u8], _ctx: &SpxCtx, _addr: &[u32; 8]) { unimplemented!("sha2 backend") }
    pub fn gen_message_random(_r: &mut [u8], _sk_prf: &[u8], _optrand: &[u8], _m: &[u8], _mlen: u64, _ctx: &SpxCtx) { unimplemented!("sha2 backend") }
    pub fn hash_message(_digest: &mut [u8], _tree: &mut u64, _leaf_idx: &mut u32, _r: &[u8], _pk: &[u8], _m: &[u8], _mlen: u64, _ctx: &SpxCtx) { unimplemented!("sha2 backend") }
    pub fn thash(_out: &mut [u8], _input: &[u8], _inblocks: usize, _ctx: &SpxCtx, _addr: &mut [u32; 8]) { unimplemented!("sha2 backend") }
}

#[cfg(feature = "shake")]
mod shake_impl {
    use super::*;
    pub fn initialize_hash_function(_ctx: &mut SpxCtx) { unimplemented!("shake backend") }
    pub fn prf_addr(_out: &mut [u8], _ctx: &SpxCtx, _addr: &[u32; 8]) { unimplemented!("shake backend") }
    pub fn gen_message_random(_r: &mut [u8], _sk_prf: &[u8], _optrand: &[u8], _m: &[u8], _mlen: u64, _ctx: &SpxCtx) { unimplemented!("shake backend") }
    pub fn hash_message(_digest: &mut [u8], _tree: &mut u64, _leaf_idx: &mut u32, _r: &[u8], _pk: &[u8], _m: &[u8], _mlen: u64, _ctx: &SpxCtx) { unimplemented!("shake backend") }
    pub fn thash(_out: &mut [u8], _input: &[u8], _inblocks: usize, _ctx: &SpxCtx, _addr: &mut [u32; 8]) { unimplemented!("shake backend") }
}

#[cfg(feature = "haraka")]
mod haraka_impl {
    use super::*;
    pub fn initialize_hash_function(_ctx: &mut SpxCtx) { unimplemented!("haraka backend") }
    pub fn prf_addr(_out: &mut [u8], _ctx: &SpxCtx, _addr: &[u32; 8]) { unimplemented!("haraka backend") }
    pub fn gen_message_random(_r: &mut [u8], _sk_prf: &[u8], _optrand: &[u8], _m: &[u8], _mlen: u64, _ctx: &SpxCtx) { unimplemented!("haraka backend") }
    pub fn hash_message(_digest: &mut [u8], _tree: &mut u64, _leaf_idx: &mut u32, _r: &[u8], _pk: &[u8], _m: &[u8], _mlen: u64, _ctx: &SpxCtx) { unimplemented!("haraka backend") }
    pub fn thash(_out: &mut [u8], _input: &[u8], _inblocks: usize, _ctx: &SpxCtx, _addr: &mut [u32; 8]) { unimplemented!("haraka backend") }
}

// --- Public dispatch ---

#[cfg(feature = "blake")]
pub use blake_impl::*;
#[cfg(all(feature = "sha2", not(feature = "blake")))]
pub use sha2_impl::*;
#[cfg(all(feature = "shake", not(feature = "blake"), not(feature = "sha2")))]
pub use shake_impl::*;
#[cfg(all(feature = "haraka", not(feature = "blake"), not(feature = "sha2"), not(feature = "shake")))]
pub use haraka_impl::*;
