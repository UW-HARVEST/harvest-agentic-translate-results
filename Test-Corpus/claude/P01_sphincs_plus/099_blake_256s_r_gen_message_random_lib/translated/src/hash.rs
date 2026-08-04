// Dispatch hash-related operations to the active backend.
use crate::context::SpxCtx;
use crate::params::SPX_N;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let ctx_ref = unsafe { &mut *ctx };
    crate::backend::initialize_hash_function_impl(ctx_ref);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const [u32; 8]) {
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let addr_ref = unsafe { &*addr };
    crate::backend::prf_addr_impl(out_slice, ctx_ref, addr_ref);
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
    let r_slice = unsafe { core::slice::from_raw_parts_mut(r, SPX_N) };
    let sk_prf_slice = unsafe { core::slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand_slice = unsafe { core::slice::from_raw_parts(optrand, SPX_N) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let ctx_ref = unsafe { &*ctx };
    crate::backend::gen_message_random_impl(r_slice, sk_prf_slice, optrand_slice, m_slice, ctx_ref);
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
    let digest_slice =
        unsafe { core::slice::from_raw_parts_mut(digest, crate::params::SPX_FORS_MSG_BYTES) };
    let r_slice = unsafe { core::slice::from_raw_parts(r, SPX_N) };
    let pk_slice = unsafe { core::slice::from_raw_parts(pk, crate::params::SPX_PK_BYTES) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let ctx_ref = unsafe { &*ctx };
    let (t, idx) = crate::backend::hash_message_impl(digest_slice, r_slice, pk_slice, m_slice, ctx_ref);
    unsafe {
        *tree = t;
        *leaf_idx = idx;
    }
}

// Internal callable wrappers for safe Rust use.
pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    crate::backend::initialize_hash_function_impl(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    crate::backend::prf_addr_impl(out, ctx, addr);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) {
    crate::backend::gen_message_random_impl(r, sk_prf, optrand, m, ctx);
}

pub fn hash_message(
    digest: &mut [u8],
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) -> (u64, u32) {
    crate::backend::hash_message_impl(digest, r, pk, m, ctx)
}
