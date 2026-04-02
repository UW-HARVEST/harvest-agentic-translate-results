use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::compute_root;
use crate::utilsx1::fors_treehashx1;

extern "C" {
    fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32);
    fn SPX_thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32);
}

unsafe fn prf_addr(out: *mut u8, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    SPX_prf_addr(out, ctx as *const SpxCtx, addr.as_ptr());
}

unsafe fn thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    SPX_thash(out, in_, inblocks, ctx as *const SpxCtx, addr.as_mut_ptr());
}

#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    unsafe { prf_addr(sk.as_mut_ptr(), ctx, fors_leaf_addr) };
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    unsafe { thash(leaf.as_mut_ptr(), sk.as_ptr(), 1, ctx, fors_leaf_addr) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    let fors_info = &mut *info;
    let fors_leaf_addr = &mut fors_info.leaf_addrx;

    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    let leaf_slice = core::slice::from_raw_parts_mut(leaf, SPX_N);
    fors_gen_sk(leaf_slice, &*ctx, fors_leaf_addr);

    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    let leaf_slice = core::slice::from_raw_parts_mut(leaf, SPX_N);
    let leaf_copy: Vec<u8> = leaf_slice.to_vec();
    fors_sk_to_leaf(leaf_slice, &leaf_copy, &*ctx, fors_leaf_addr);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_sign(
    mut sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];

    let fors_addr_ref = &*(fors_addr as *const [u32; 8]);
    copy_keypair_addr(&mut fors_tree_addr, fors_addr_ref);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr_ref);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr_ref);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    let m_slice = core::slice::from_raw_parts(m, (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8);
    message_to_indices(&mut indices, m_slice);

    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        let sig_slice = core::slice::from_raw_parts_mut(sig, SPX_N);
        fors_gen_sk(sig_slice, &*ctx, &mut fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig = sig.add(SPX_N);

        fors_treehashx1(
            roots.as_mut_ptr().add(i * SPX_N),
            sig,
            &*ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info as *mut ForsGenLeafInfo as *mut crate::wotsx1::LeafInfoX1,
        );

        sig = sig.add(SPX_N * SPX_FORS_HEIGHT);
    }

    thash(pk, roots.as_ptr(), SPX_FORS_TREES as u32, &*ctx, &mut fors_pk_addr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    mut sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    let fors_addr_ref = &*(fors_addr as *const [u32; 8]);
    copy_keypair_addr(&mut fors_tree_addr, fors_addr_ref);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr_ref);

    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    let m_slice = core::slice::from_raw_parts(m, (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8);
    message_to_indices(&mut indices, m_slice);

    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        let sig_slice = core::slice::from_raw_parts(sig, SPX_N);
        fors_sk_to_leaf(&mut leaf, sig_slice, &*ctx, &mut fors_tree_addr);
        sig = sig.add(SPX_N);

        compute_root(
            roots.as_mut_ptr().add(i * SPX_N),
            leaf.as_ptr(),
            indices[i],
            idx_offset,
            sig,
            SPX_FORS_HEIGHT as u32,
            &*ctx,
            &mut fors_tree_addr,
        );
        sig = sig.add(SPX_N * SPX_FORS_HEIGHT);
    }

    thash(pk, roots.as_ptr(), SPX_FORS_TREES as u32, &*ctx, &mut fors_pk_addr);
}
