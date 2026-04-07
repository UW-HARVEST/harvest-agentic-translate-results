use crate::context::SpxCtx;
use crate::params::*;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    #[cfg(feature = "shake")]
    { crate::shake_backend::initialize_hash_function(ctx); }
    #[cfg(feature = "sha2")]
    { crate::sha2_backend::initialize_hash_function(ctx); }
    #[cfg(feature = "blake")]
    { crate::blake_backend::initialize_hash_function(ctx); }
    #[cfg(feature = "haraka")]
    { crate::haraka_backend::initialize_hash_function(ctx); }
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    #[cfg(feature = "shake")]
    { crate::shake_backend::prf_addr(out, ctx, addr); }
    #[cfg(feature = "sha2")]
    { crate::sha2_backend::prf_addr(out, ctx, addr); }
    #[cfg(feature = "blake")]
    { crate::blake_backend::prf_addr(out, ctx, addr); }
    #[cfg(feature = "haraka")]
    { crate::haraka_backend::prf_addr(out, ctx, addr); }
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    ctx: &SpxCtx,
) {
    #[cfg(feature = "shake")]
    { crate::shake_backend::gen_message_random(r, sk_prf, optrand, m, mlen, ctx); }
    #[cfg(feature = "sha2")]
    { crate::sha2_backend::gen_message_random(r, sk_prf, optrand, m, mlen, ctx); }
    #[cfg(feature = "blake")]
    { crate::blake_backend::gen_message_random(r, sk_prf, optrand, m, mlen, ctx); }
    #[cfg(feature = "haraka")]
    { crate::haraka_backend::gen_message_random(r, sk_prf, optrand, m, mlen, ctx); }
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: u64,
    ctx: &SpxCtx,
) {
    #[cfg(feature = "shake")]
    { crate::shake_backend::hash_message(digest, tree, leaf_idx, r, pk, m, mlen, ctx); }
    #[cfg(feature = "sha2")]
    { crate::sha2_backend::hash_message(digest, tree, leaf_idx, r, pk, m, mlen, ctx); }
    #[cfg(feature = "blake")]
    { crate::blake_backend::hash_message(digest, tree, leaf_idx, r, pk, m, mlen, ctx); }
    #[cfg(feature = "haraka")]
    { crate::haraka_backend::hash_message(digest, tree, leaf_idx, r, pk, m, mlen, ctx); }
}
