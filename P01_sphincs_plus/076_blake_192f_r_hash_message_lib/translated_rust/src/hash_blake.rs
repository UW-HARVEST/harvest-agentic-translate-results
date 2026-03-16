use crate::blake256;
use crate::blake512;
use crate::context::spx_ctx;
use crate::params::*;
use crate::utils::bytes_to_ull;

// For blake 192f (N>=24): blakeX = blake512

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(_ctx: *mut spx_ctx) {}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const spx_ctx, addr: *const u32) {
    unsafe {
        let ctx_ref = &*ctx;
        let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        buf[..SPX_N].copy_from_slice(&ctx_ref.pub_seed);
        std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx_ref.sk_seed);
        blake256::blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
        std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, _ctx: *const spx_ctx,
) {
    unsafe {
        let mut s = blake512::Blakestate512 {
            h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
        };
        blake512::blake512_init(&mut s);
        blake512::blake512_update(&mut s, std::slice::from_raw_parts(sk_prf, SPX_N), SPX_N as u64);
        blake512::blake512_update(&mut s, std::slice::from_raw_parts(optrand, SPX_N), SPX_N as u64);
        blake512::blake512_update(&mut s, std::slice::from_raw_parts(m, mlen as usize), mlen);
        blake512::blake512_final(&mut s, std::slice::from_raw_parts_mut(r, SPX_BLAKE512_OUTPUT_BYTES));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    r: *const u8, pk: *const u8,
    m: *const u8, mlen: u64, _ctx: *const spx_ctx,
) {
    unsafe {
        let mut buf = [0u8; SPX_DGST_BYTES];
        let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

        let mut s = blake512::Blakestate512 {
            h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
        };
        blake512::blake512_init(&mut s);
        blake512::blake512_update(&mut s, std::slice::from_raw_parts(r, SPX_N), SPX_N as u64);
        blake512::blake512_update(&mut s, std::slice::from_raw_parts(pk, SPX_PK_BYTES), SPX_PK_BYTES as u64);
        blake512::blake512_update(&mut s, std::slice::from_raw_parts(m, mlen as usize), mlen);
        blake512::blake512_final(&mut s, &mut seed[2 * SPX_N..]);

        std::ptr::copy_nonoverlapping(r, seed.as_mut_ptr(), SPX_N);
        std::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

        blake512::blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

        std::ptr::copy_nonoverlapping(buf.as_ptr(), digest, SPX_FORS_MSG_BYTES);
        let mut bufp = SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
            *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
        }
        bufp += SPX_TREE_BYTES;

        *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
        *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
    }
}
