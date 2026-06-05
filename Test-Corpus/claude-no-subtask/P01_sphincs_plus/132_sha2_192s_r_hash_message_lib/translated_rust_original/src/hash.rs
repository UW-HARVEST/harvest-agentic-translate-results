// Hash dispatcher
use crate::context::SpxCtx;

#[cfg(feature = "sha2")]
pub use crate::sha2_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};

#[cfg(feature = "shake")]
pub use crate::shake_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};

#[cfg(feature = "haraka")]
pub use crate::haraka_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};

#[cfg(feature = "blake")]
pub use crate::blake_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};

// C-ABI exports
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    initialize_hash_function(unsafe { &mut *ctx });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let o = unsafe { std::slice::from_raw_parts_mut(out, crate::params::SPX_N) };
    let c = unsafe { &*ctx };
    let a = unsafe { &*(addr as *const [u32; 8]) };
    prf_addr(o, c, a);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let o = unsafe { std::slice::from_raw_parts_mut(r, crate::params::SPX_N) };
    let s = unsafe { std::slice::from_raw_parts(sk_prf, crate::params::SPX_N) };
    let opt = unsafe { std::slice::from_raw_parts(optrand, crate::params::SPX_N) };
    let mm = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let c = unsafe { &*ctx };
    gen_message_random(o, s, opt, mm, mlen, c);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    use crate::params::*;
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BYTES: usize = (SPX_TREE_HEIGHT + 7) / 8;
    let _ = SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let d = unsafe { std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let t = unsafe { &mut *tree };
    let l = unsafe { &mut *leaf_idx };
    let r_s = unsafe { std::slice::from_raw_parts(r, SPX_N) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let c = unsafe { &*ctx };
    hash_message(d, t, l, r_s, pk_s, m_s, mlen, c);
}
