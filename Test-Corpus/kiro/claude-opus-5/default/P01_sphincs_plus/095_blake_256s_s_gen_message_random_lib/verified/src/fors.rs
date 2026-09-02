//! Translation of `app/src/fors.c`, `app/include/fors.h` and
//! `app/include/forsx1.h`.

use crate::address::{
    addr_ref, copy_keypair_addr, set_tree_height, set_tree_index, set_type, Addr, ZERO_ADDR,
    SPX_ADDR_TYPE_FORSPK, SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE,
};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::utilsx1::fors_treehashx1;

/// `fors_gen_leaf_info`
#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: Addr,
}

impl ForsGenLeafInfo {
    /// `struct fors_gen_leaf_info fors_info = {0};`
    pub const fn zeroed() -> Self {
        ForsGenLeafInfo {
            leaf_addrx: ZERO_ADDR,
        }
    }
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &Addr) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &Addr) {
    thash(&mut leaf[..SPX_N], sk, 1, ctx, fors_leaf_addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    let fors_leaf_addr = &mut info.leaf_addrx;

    /* Only set the parts that the caller doesn't set */
    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);

    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    /* fors_sk_to_leaf(leaf, leaf, ...): the C code hashes in place. */
    let mut sk = [0u8; SPX_N];
    sk.copy_from_slice(&leaf[..SPX_N]);
    fors_sk_to_leaf(leaf, &sk, ctx, fors_leaf_addr);
}

/// Interprets `m` as `SPX_FORS_HEIGHT`-bit unsigned integers.
///
/// Assumes `m` contains at least `SPX_FORS_HEIGHT * SPX_FORS_TREES` bits.
fn message_to_indices(indices: &mut [u32; SPX_FORS_TREES], m: &[u8]) {
    let mut offset: usize = 0;

    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1u8) as u32) << j;
            offset += 1;
        }
    }
}

/// Signs a message `m`, deriving the secret key from `sk_seed` and the FTS
/// address.
///
/// Assumes `m` contains at least `SPX_FORS_HEIGHT * SPX_FORS_TREES` bits.
pub unsafe fn fors_sign(
    sig: &mut [u8],
    pk: &mut [u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &Addr,
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = ZERO_ADDR;
    let mut fors_info = ForsGenLeafInfo::zeroed();
    let mut fors_pk_addr = ZERO_ADDR;

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);

    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sigp = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        /* Include the secret key part that produces the selected leaf node. */
        fors_gen_sk(&mut sig[sigp..sigp + SPX_N], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sigp += SPX_N;

        /* Compute the authentication path for this leaf node. */
        fors_treehashx1(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &mut sig[sigp..sigp + SPX_N * SPX_FORS_HEIGHT],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );

        sigp += SPX_N * SPX_FORS_HEIGHT;
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    thash(
        &mut pk[..SPX_N],
        &roots,
        SPX_FORS_TREES as u32,
        ctx,
        &fors_pk_addr,
    );
}

/// Derives the FORS public key from a signature.
///
/// This can be used for verification by comparing to a known public key, or to
/// subsequently verify a signature on the derived public key.
/// Assumes `m` contains at least `SPX_FORS_HEIGHT * SPX_FORS_TREES` bits.
pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &Addr) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = ZERO_ADDR;
    let mut fors_pk_addr = ZERO_ADDR;

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);

    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sigp = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        /* Derive the leaf from the included secret key part. */
        fors_sk_to_leaf(&mut leaf, &sig[sigp..sigp + SPX_N], ctx, &fors_tree_addr);
        sigp += SPX_N;

        /* Derive the corresponding root node of this tree. */
        compute_root(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sigp..sigp + SPX_N * SPX_FORS_HEIGHT],
            SPX_FORS_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        sigp += SPX_N * SPX_FORS_HEIGHT;
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    thash(
        &mut pk[..SPX_N],
        &roots,
        SPX_FORS_TREES as u32,
        ctx,
        &fors_pk_addr,
    );
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    let leaf_s = core::slice::from_raw_parts_mut(leaf, SPX_N);
    fors_gen_leafx1(leaf_s, &*ctx, addr_idx, &mut *info);
}

#[unsafe(no_mangle)]
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
    fors_sign(sig_s, pk_s, m_s, &*ctx, addr_ref(fors_addr));
}

#[unsafe(no_mangle)]
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
    fors_pk_from_sig(pk_s, sig_s, m_s, &*ctx, addr_ref(fors_addr));
}
