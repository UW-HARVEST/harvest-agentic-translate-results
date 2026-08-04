use crate::address::*;
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;
use crate::utils::compute_root_rs;
use crate::utilsx1::{fors_treehashx1_rs, ForsGenLeafInfo};

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    let leaf_slice = unsafe { core::slice::from_raw_parts_mut(leaf, SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let info_ref = unsafe { &mut *info };

    set_tree_index(&mut info_ref.leaf_addrx, addr_idx);
    set_type(&mut info_ref.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf_slice, ctx_ref, &info_ref.leaf_addrx);

    set_type(&mut info_ref.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = vec![0u8; SPX_N];
    tmp.copy_from_slice(leaf_slice);
    fors_sk_to_leaf(leaf_slice, &tmp, ctx_ref, &mut info_ref.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1u8) as u32) << j;
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
    fors_addr: *const [u32; 8],
) {
    let m_slice = unsafe { core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let ctx_ref = unsafe { &*ctx };
    let fors_addr_ref = unsafe { &*fors_addr };
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, SPX_N) };

    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo {
        leaf_addrx: [0u32; 8],
    };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr_ref);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr_ref);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr_ref);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m_slice);

    let mut sig_offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i * (1usize << SPX_FORS_HEIGHT)) as u32;

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        // Write sk part to sig
        let sig_sk = unsafe { core::slice::from_raw_parts_mut(sig.add(sig_offset), SPX_N) };
        fors_gen_sk(sig_sk, ctx_ref, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_offset += SPX_N;

        let auth_path = unsafe {
            core::slice::from_raw_parts_mut(sig.add(sig_offset), SPX_N * SPX_FORS_HEIGHT)
        };
        fors_treehashx1_rs(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            auth_path,
            ctx_ref,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );

        sig_offset += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk_slice, &roots, SPX_FORS_TREES as u32, ctx_ref, &mut fors_pk_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const [u32; 8],
) {
    let m_slice = unsafe { core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let ctx_ref = unsafe { &*ctx };
    let fors_addr_ref = unsafe { &*fors_addr };
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, SPX_N) };

    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = vec![0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr_ref);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr_ref);
    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m_slice);

    let mut sig_offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i * (1usize << SPX_FORS_HEIGHT)) as u32;

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        let sk = unsafe { core::slice::from_raw_parts(sig.add(sig_offset), SPX_N) };
        fors_sk_to_leaf(&mut leaf, sk, ctx_ref, &mut fors_tree_addr);
        sig_offset += SPX_N;

        let auth_path = unsafe {
            core::slice::from_raw_parts(sig.add(sig_offset), SPX_N * SPX_FORS_HEIGHT)
        };
        compute_root_rs(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &leaf,
            indices[i],
            idx_offset,
            auth_path,
            SPX_FORS_HEIGHT as u32,
            ctx_ref,
            &mut fors_tree_addr,
        );
        sig_offset += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk_slice, &roots, SPX_FORS_TREES as u32, ctx_ref, &mut fors_pk_addr);
}
