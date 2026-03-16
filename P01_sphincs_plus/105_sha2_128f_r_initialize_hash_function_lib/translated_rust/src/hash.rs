use crate::params::*;
use crate::sha2::*;
use crate::utils::{u32_to_bytes, bytes_to_ull_rs};

#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    seed_state_rs(ctx);
}

pub fn initialize_hash_function_rs(ctx: &mut SpxCtx) {
    seed_state_rs(ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let ctx = unsafe { &*ctx };
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr as *const u8, 32) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, SPX_N) };
    prf_addr_rs(out, ctx, addr_bytes);
}

pub fn prf_addr_rs(out: &mut [u8], ctx: &SpxCtx, addr_bytes: &[u8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(outbuf.as_mut_ptr(), sha2_state.as_mut_ptr(), buf.as_ptr(), buf.len());
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let _ = unsafe { &*ctx };
    let sk_prf = unsafe { std::slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { std::slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let r_out = unsafe { std::slice::from_raw_parts_mut(r, SPX_N) };
    gen_message_random_rs(r_out, sk_prf, optrand, m);
}

pub fn gen_message_random_rs(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8]) {
    let mlen = m.len();
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC-SHA256
    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x36; }

    sha256_inc_init(state.as_mut_ptr());
    sha256_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);

    buf[..SPX_N].copy_from_slice(optrand);

    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(m);
        sha256_inc_finalize(
            buf[SPX_SHAX_BLOCK_BYTES..].as_mut_ptr(), state.as_mut_ptr(),
            buf.as_ptr(), mlen + SPX_N,
        );
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha256_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);
        let m_rest = &m[SPX_SHAX_BLOCK_BYTES - SPX_N..];
        sha256_inc_finalize(
            buf[SPX_SHAX_BLOCK_BYTES..].as_mut_ptr(), state.as_mut_ptr(),
            m_rest.as_ptr(), m_rest.len(),
        );
    }

    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x5c; }

    sha256_rs(&mut buf.clone(), &buf[..SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES]);
    // Need to hash into a separate buffer to avoid aliasing
    let mut tmp = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_rs(&mut tmp, &buf[..SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES]);
    r[..SPX_N].copy_from_slice(&tmp[..SPX_N]);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    r_val: *const u8, pk: *const u8, m: *const u8, mlen: u64,
    ctx: *const SpxCtx,
) {
    let _ = unsafe { &*ctx };
    let r_val = unsafe { std::slice::from_raw_parts(r_val, SPX_N) };
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let digest = unsafe { std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let (t, l) = hash_message_rs(digest, r_val, pk, m);
    unsafe { *tree = t; *leaf_idx = l; }
}

pub fn hash_message_rs(digest: &mut [u8], r_val: &[u8], pk: &[u8], m: &[u8]) -> (u64, u32) {
    let mlen = m.len();
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha256_inc_init(state.as_mut_ptr());

    inbuf[..SPX_N].copy_from_slice(r_val);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(pk);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(m);
        sha256_inc_finalize(
            seed[2 * SPX_N..].as_mut_ptr(), state.as_mut_ptr(),
            inbuf.as_ptr(), SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        sha256_inc_blocks(state.as_mut_ptr(), inbuf.as_ptr(), SPX_INBLOCKS);
        let m_rest = &m[fill..];
        sha256_inc_finalize(
            seed[2 * SPX_N..].as_mut_ptr(), state.as_mut_ptr(),
            m_rest.as_ptr(), m_rest.len(),
        );
    }

    seed[..SPX_N].copy_from_slice(r_val);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_256_rs(&mut buf, &seed);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree = if SPX_D == 1 {
        0u64
    } else {
        let t = bytes_to_ull_rs(&buf[bufp..], SPX_TREE_BYTES);
        t & (!0u64 >> (64 - SPX_TREE_BITS))
    };
    bufp += SPX_TREE_BYTES;

    let leaf_idx = (bytes_to_ull_rs(&buf[bufp..], SPX_LEAF_BYTES) as u32)
        & (!0u32 >> (32 - SPX_LEAF_BITS));

    (tree, leaf_idx)
}
