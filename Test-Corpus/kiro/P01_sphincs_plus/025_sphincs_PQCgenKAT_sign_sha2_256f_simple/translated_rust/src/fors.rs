use crate::address::{
    copy_keypair_addr, set_tree_height, set_tree_index, set_type,
};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::utilsx1::fors_treehashx1;

#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: *mut u8, ctx: *const SpxCtx, fors_leaf_addr: *mut u32) {
    unsafe {
        prf_addr(sk, ctx, fors_leaf_addr as *const u32);
    }
}

fn fors_sk_to_leaf(leaf: *mut u8, sk: *const u8, ctx: *const SpxCtx, fors_leaf_addr: *mut u32) {
    unsafe {
        thash(leaf, sk, 1, ctx, fors_leaf_addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    unsafe {
        let fors_info = &mut *info;
        let fors_leaf_addr = fors_info.leaf_addrx.as_mut_ptr();

        set_tree_index(fors_leaf_addr, addr_idx);
        set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
        fors_gen_sk(leaf, ctx, fors_leaf_addr);

        set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
        fors_sk_to_leaf(leaf, leaf, ctx, fors_leaf_addr);
    }
}

fn message_to_indices_exact(indices: *mut u32, m: *const u8) {
    unsafe {
        let mut offset: u32 = 0;
        for i in 0..SPX_FORS_TREES {
            *indices.add(i) = 0;
            for j in 0..SPX_FORS_HEIGHT as u32 {
                *indices.add(i) ^=
                    (((*m.add((offset >> 3) as usize) >> (offset & 0x7)) & 1) as u32) << j;
                offset += 1;
            }
        }
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
        let mut indices = [0u32; SPX_FORS_TREES];
        let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
        let mut fors_tree_addr = [0u32; 8];
        let mut fors_info = ForsGenLeafInfo {
            leaf_addrx: [0u32; 8],
        };
        let fors_leaf_addr = fors_info.leaf_addrx.as_mut_ptr();
        let mut fors_pk_addr = [0u32; 8];

        copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
        copy_keypair_addr(fors_leaf_addr, fors_addr);
        copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);
        set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

        message_to_indices_exact(indices.as_mut_ptr(), m);

        let mut sig_ptr = sig;
        for i in 0..SPX_FORS_TREES {
            let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

            set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
            set_tree_index(fors_tree_addr.as_mut_ptr(), indices[i] + idx_offset);
            set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPRF);

            fors_gen_sk(sig_ptr, ctx, fors_tree_addr.as_mut_ptr());
            set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
            sig_ptr = sig_ptr.add(SPX_N);

            fors_treehashx1(
                roots.as_mut_ptr().add(i * SPX_N),
                sig_ptr,
                ctx,
                indices[i],
                idx_offset,
                SPX_FORS_HEIGHT as u32,
                fors_tree_addr.as_mut_ptr(),
                &mut fors_info as *mut ForsGenLeafInfo as *mut u8,
            );

            sig_ptr = sig_ptr.add(SPX_N * SPX_FORS_HEIGHT);
        }

        thash(
            pk,
            roots.as_ptr(),
            SPX_FORS_TREES as u32,
            ctx,
            fors_pk_addr.as_mut_ptr(),
        );
    }
}

pub fn fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    SPX_fors_sign(sig, pk, m, ctx, fors_addr);
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
        let mut indices = [0u32; SPX_FORS_TREES];
        let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
        let mut leaf = [0u8; SPX_N];
        let mut fors_tree_addr = [0u32; 8];
        let mut fors_pk_addr = [0u32; 8];

        copy_keypair_addr(fors_tree_addr.as_mut_ptr(), fors_addr);
        copy_keypair_addr(fors_pk_addr.as_mut_ptr(), fors_addr);

        set_type(fors_tree_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSTREE);
        set_type(fors_pk_addr.as_mut_ptr(), SPX_ADDR_TYPE_FORSPK);

        message_to_indices_exact(indices.as_mut_ptr(), m);

        let mut sig_ptr = sig;
        for i in 0..SPX_FORS_TREES {
            let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

            set_tree_height(fors_tree_addr.as_mut_ptr(), 0);
            set_tree_index(fors_tree_addr.as_mut_ptr(), indices[i] + idx_offset);

            fors_sk_to_leaf(
                leaf.as_mut_ptr(),
                sig_ptr,
                ctx,
                fors_tree_addr.as_mut_ptr(),
            );
            sig_ptr = sig_ptr.add(SPX_N);

            compute_root(
                roots.as_mut_ptr().add(i * SPX_N),
                leaf.as_ptr(),
                indices[i],
                idx_offset,
                sig_ptr,
                SPX_FORS_HEIGHT as u32,
                ctx,
                fors_tree_addr.as_mut_ptr(),
            );
            sig_ptr = sig_ptr.add(SPX_N * SPX_FORS_HEIGHT);
        }

        thash(
            pk,
            roots.as_ptr(),
            SPX_FORS_TREES as u32,
            ctx,
            fors_pk_addr.as_mut_ptr(),
        );
    }
}

pub fn fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    SPX_fors_pk_from_sig(pk, sig, m, ctx, fors_addr);
}
