// Translation of c_src/app/src/fors.c

use core::slice;

use crate::address::{
    copy_keypair_addr_inner, set_tree_height_inner, set_tree_index_inner, set_type_inner,
};
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_TYPE_FORSPK, SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE, SPX_FORS_HEIGHT,
    SPX_FORS_TREES, SPX_N,
};
use crate::thash::thash_inner;
use crate::utils::compute_root_inner;

#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

// Backend prf_addr
#[cfg(feature = "haraka")]
use crate::hash::haraka::hash::prf_addr_inner;
#[cfg(feature = "sha2")]
use crate::hash::sha2::hash::prf_addr_inner;

#[cfg(any(feature = "shake", feature = "blake"))]
fn prf_addr_inner(out: &mut [u8], ctx: &SpxCtx, addr: &[u32]) {
    use crate::params::{SPX_ADDR_BYTES, SPX_N};
    #[cfg(feature = "shake")]
    {
        use crate::hash::shake::fips202::shake256_inner;
        let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
        shake256_inner(&mut out[..SPX_N], &buf);
    }
    #[cfg(feature = "blake")]
    {
        use crate::hash::blake::blake256::blake256_oneshot;
        let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        let mut outbuf = [0u8; 32];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
        blake256_oneshot(&mut outbuf, &buf[..SPX_N + SPX_ADDR_BYTES]);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32]) {
    prf_addr_inner(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32]) {
    thash_inner(leaf, sk, 1, ctx, fors_leaf_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    let leaf = unsafe { slice::from_raw_parts_mut(leaf, SPX_N) };
    let ctx = unsafe { &*ctx };
    let info = unsafe { &mut *info };
    fors_gen_leafx1_inner(leaf, ctx, addr_idx, info);
}

pub fn fors_gen_leafx1_inner(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index_inner(&mut info.leaf_addrx, addr_idx);
    set_type_inner(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);

    set_type_inner(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let leaf_copy = leaf.to_vec();
    fors_sk_to_leaf(leaf, &leaf_copy, ctx, &mut info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            let bit = ((m[offset >> 3] >> (offset & 0x7)) & 1) as u32;
            indices[i] ^= bit << j;
            offset += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let sig_total = SPX_N + SPX_N * SPX_FORS_HEIGHT;
    let sig = unsafe { slice::from_raw_parts_mut(sig, sig_total * SPX_FORS_TREES) };
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_N) };
    let m = unsafe {
        slice::from_raw_parts(m, (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8)
    };
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { slice::from_raw_parts(fors_addr, 8) };
    fors_sign_inner(sig, pk, m, ctx, fors_addr);
}

pub fn fors_sign_inner(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr_inner(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_inner(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr_inner(&mut fors_pk_addr, fors_addr);
    set_type_inner(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height_inner(&mut fors_tree_addr, 0);
        set_tree_index_inner(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type_inner(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..sig_off + SPX_N], ctx, &fors_tree_addr);
        set_type_inner(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        crate::utilsx1::fors_treehashx1_inner(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &mut sig[sig_off..sig_off + SPX_N * SPX_FORS_HEIGHT],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT,
            &mut fors_tree_addr,
            &mut fors_info,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash_inner(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let sig_total = SPX_N + SPX_N * SPX_FORS_HEIGHT;
    let sig = unsafe { slice::from_raw_parts(sig, sig_total * SPX_FORS_TREES) };
    let pk = unsafe { slice::from_raw_parts_mut(pk, SPX_N) };
    let m = unsafe {
        slice::from_raw_parts(m, (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8)
    };
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { slice::from_raw_parts(fors_addr, 8) };
    fors_pk_from_sig_inner(pk, sig, m, ctx, fors_addr);
}

pub fn fors_pk_from_sig_inner(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = vec![0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr_inner(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_inner(&mut fors_pk_addr, fors_addr);

    set_type_inner(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type_inner(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height_inner(&mut fors_tree_addr, 0);
        set_tree_index_inner(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..sig_off + SPX_N], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        compute_root_inner(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sig_off..sig_off + SPX_N * SPX_FORS_HEIGHT],
            SPX_FORS_HEIGHT,
            ctx,
            &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash_inner(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}
