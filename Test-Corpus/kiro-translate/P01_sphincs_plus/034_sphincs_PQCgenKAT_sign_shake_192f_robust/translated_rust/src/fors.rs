use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::hash::prf_addr;
use crate::thash::thash;
use crate::utils::compute_root;
use crate::utilsx1::fors_treehashx1;

#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

pub fn fors_gen_leafx1(
    leaf: &mut [u8],
    ctx: &SpxCtx,
    addr_idx: u32,
    info: &mut ForsGenLeafInfo,
) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);

    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp = leaf[..SPX_N].to_vec();
    fors_sk_to_leaf(leaf, &tmp, ctx, &mut info.leaf_addrx);
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

pub fn fors_sign(
    sig: &mut [u8],
    pk: &mut [u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off: usize = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..sig_off + SPX_N], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        fors_treehashx1(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &mut sig[sig_off..],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info as *mut ForsGenLeafInfo as *mut u8,
        );

        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}

pub fn fors_pk_from_sig(
    pk: &mut [u8],
    sig: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = vec![0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);

    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off: usize = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..sig_off + SPX_N], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        compute_root(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
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

    thash(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}

// --- extern "C" wrappers ---

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES) };
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) };
    let m = unsafe { std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    fors_sign(sig, pk, m, ctx, fors_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) };
    let sig = unsafe { std::slice::from_raw_parts(sig, SPX_FORS_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    fors_pk_from_sig(pk, sig, m, ctx, fors_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    let leaf = unsafe { std::slice::from_raw_parts_mut(leaf, SPX_N) };
    let ctx = unsafe { &*ctx };
    let info = unsafe { &mut *info };
    fors_gen_leafx1(leaf, ctx, addr_idx, info);
}
