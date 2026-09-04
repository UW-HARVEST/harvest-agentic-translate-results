//! Translation of `app/src/fors.c` and the `fors_gen_leaf_info` struct.

use crate::address::{copy_keypair_addr, set_tree_height, set_tree_index, set_type};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::utilsx1::fors_treehashx1;

/// Mirrors `struct fors_gen_leaf_info`.
#[derive(Clone)]
#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

impl ForsGenLeafInfo {
    pub fn new() -> Self {
        ForsGenLeafInfo {
            leaf_addrx: [0u32; 8],
        }
    }
}

impl Default for ForsGenLeafInfo {
    fn default() -> Self {
        Self::new()
    }
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    let fors_leaf_addr = &mut info.leaf_addrx;

    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);

    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    // fors_sk_to_leaf(leaf, leaf, ...) — in-place, so copy first.
    let tmp: [u8; SPX_N] = leaf[..SPX_N].try_into().unwrap();
    fors_sk_to_leaf(leaf, &tmp, ctx, fors_leaf_addr);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1u8) as u32) << j;
            offset += 1;
        }
    }
}

/// Signs a message m, deriving the secret key from sk_seed and the FTS address.
pub fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo::new();
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);

    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i * (1 << SPX_FORS_HEIGHT)) as u32;

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..sig_off + SPX_N], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        let root_off = i * SPX_N;
        fors_treehashx1(
            &mut roots[root_off..root_off + SPX_N],
            &mut sig[sig_off..],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );

        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES as u32, ctx, &fors_pk_addr);
}

/// Derives the FORS public key from a signature.
pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
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
        let idx_offset = (i * (1 << SPX_FORS_HEIGHT)) as u32;

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..sig_off + SPX_N], ctx, &fors_tree_addr);
        sig_off += SPX_N;

        let root_off = i * SPX_N;
        compute_root(
            &mut roots[root_off..root_off + SPX_N],
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

    thash(pk, &roots, SPX_FORS_TREES as u32, ctx, &fors_pk_addr);
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    let leaf_s = core::slice::from_raw_parts_mut(leaf, SPX_N);
    fors_gen_leafx1(leaf_s, &*ctx, addr_idx, &mut *info);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let sig_s = core::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES);
    let pk_s = core::slice::from_raw_parts_mut(pk, SPX_N);
    let m_s = core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
    let addr = &*(fors_addr as *const [u32; 8]);
    fors_sign(sig_s, pk_s, m_s, &*ctx, addr);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let pk_s = core::slice::from_raw_parts_mut(pk, SPX_N);
    let sig_s = core::slice::from_raw_parts(sig, SPX_FORS_BYTES);
    let m_s = core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES);
    let addr = &*(fors_addr as *const [u32; 8]);
    fors_pk_from_sig(pk_s, sig_s, m_s, &*ctx, addr);
}
