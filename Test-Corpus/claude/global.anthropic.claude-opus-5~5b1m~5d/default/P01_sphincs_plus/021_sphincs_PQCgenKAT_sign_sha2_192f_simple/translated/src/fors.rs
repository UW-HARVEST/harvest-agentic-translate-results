//! Translation of `app/src/fors.c` (plus `app/include/fors.h` and
//! `app/include/forsx1.h`).

use crate::address::{
    copy_keypair_addr, set_tree_height, set_tree_index, set_type, SPX_ADDR_TYPE_FORSPK,
    SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE,
};
use crate::backend::{prf_addr, thash};
use crate::context::SpxCtx;
use crate::params::{SPX_FORS_HEIGHT, SPX_FORS_TREES, SPX_N};
use crate::utils::compute_root;

/// `typedef struct fors_gen_leaf_info { uint32_t leaf_addrx[8]; } fors_gen_leaf_info;`
#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

impl ForsGenLeafInfo {
    pub fn new() -> Self {
        ForsGenLeafInfo { leaf_addrx: [0u32; 8] }
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

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    let fors_leaf_addr = &mut info.leaf_addrx;

    /* Only set the parts that the caller doesn't set */
    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(&mut leaf[..SPX_N], ctx, fors_leaf_addr);

    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    /* C calls `fors_sk_to_leaf(leaf, leaf, ...)`, i.e. thash() with identical
     * in/out pointers.  thash() reads all of its input before writing the
     * output, so a temporary copy is behaviourally identical. */
    let mut sk = [0u8; SPX_N];
    sk.copy_from_slice(&leaf[..SPX_N]);
    fors_sk_to_leaf(&mut leaf[..SPX_N], &sk, ctx, fors_leaf_addr);
}

/**
 * Interprets m as SPX_FORS_HEIGHT-bit unsigned integers.
 * Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
 * Assumes indices has space for SPX_FORS_TREES integers.
 */
fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset: u32 = 0;

    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[(offset >> 3) as usize] >> (offset & 0x7)) & 1u8) as u32) << j;
            offset = offset.wrapping_add(1);
        }
    }
}

/**
 * Signs a message m, deriving the secret key from sk_seed and the FTS address.
 * Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
 */
pub fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo::new();
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);

    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    /* `sig` advances by SPX_N + SPX_N * SPX_FORS_HEIGHT per tree in C. */
    let mut sig_off: usize = 0;

    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32).wrapping_mul(1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(
            &mut fors_tree_addr,
            indices[i].wrapping_add(idx_offset),
        );
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        /* Include the secret key part that produces the selected leaf node. */
        fors_gen_sk(&mut sig[sig_off..sig_off + SPX_N], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        /* Compute the authentication path for this leaf node. */
        crate::utilsx1::fors_treehashx1(
            &mut roots[i * SPX_N..i * SPX_N + SPX_N],
            &mut sig[sig_off..sig_off + SPX_N * SPX_FORS_HEIGHT],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );

        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    thash(
        &mut pk[..SPX_N],
        &roots,
        SPX_FORS_TREES as u32,
        ctx,
        &mut fors_pk_addr,
    );
}

/**
 * Derives the FORS public key from a signature.
 * This can be used for verification by comparing to a known public key, or to
 * subsequently verify a signature on the derived public key. The latter is the
 * typical use-case when used as an FTS below an OTS in a hypertree.
 * Assumes m contains at least SPX_FORS_HEIGHT * SPX_FORS_TREES bits.
 */
pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);

    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off: usize = 0;

    for i in 0..SPX_FORS_TREES {
        let idx_offset: u32 = (i as u32).wrapping_mul(1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(
            &mut fors_tree_addr,
            indices[i].wrapping_add(idx_offset),
        );

        /* Derive the leaf from the included secret key part. */
        fors_sk_to_leaf(
            &mut leaf,
            &sig[sig_off..sig_off + SPX_N],
            ctx,
            &mut fors_tree_addr,
        );
        sig_off += SPX_N;

        /* Derive the corresponding root node of this tree. */
        compute_root(
            &mut roots[i * SPX_N..i * SPX_N + SPX_N],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sig_off..sig_off + SPX_N * SPX_FORS_HEIGHT],
            SPX_FORS_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    /* Hash horizontally across all tree roots to derive the public key. */
    thash(
        &mut pk[..SPX_N],
        &roots,
        SPX_FORS_TREES as u32,
        ctx,
        &mut fors_pk_addr,
    );
}

// ---------------------------------------------------------------------------
// C ABI wrappers (exported linker symbols carry the `SPX_` namespace prefix)
// ---------------------------------------------------------------------------

use crate::params::{SPX_FORS_BYTES, SPX_FORS_MSG_BYTES};

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    unsafe {
        fors_gen_leafx1(
            core::slice::from_raw_parts_mut(leaf, SPX_N),
            &*ctx,
            addr_idx,
            &mut *info,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    unsafe {
        let addr = *(fors_addr as *const [u32; 8]);
        fors_sign(
            core::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES),
            core::slice::from_raw_parts_mut(pk, SPX_N),
            core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES),
            &*ctx,
            &addr,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    unsafe {
        let addr = *(fors_addr as *const [u32; 8]);
        fors_pk_from_sig(
            core::slice::from_raw_parts_mut(pk, SPX_N),
            core::slice::from_raw_parts(sig, SPX_FORS_BYTES),
            core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES),
            &*ctx,
            &addr,
        );
    }
}
