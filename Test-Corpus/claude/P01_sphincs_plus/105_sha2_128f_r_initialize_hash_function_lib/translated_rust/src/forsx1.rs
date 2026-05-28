use crate::address::{set_tree_index, set_type, SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE};
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::SPX_N;
use crate::thash::thash;

pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    let sk_copy = sk[..SPX_N].to_vec();
    thash(leaf, &sk_copy, 1, ctx, fors_leaf_addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    let fors_leaf_addr = &mut info.leaf_addrx;
    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);

    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    let leaf_copy = leaf[..SPX_N].to_vec();
    let mut local_addr = *fors_leaf_addr;
    fors_sk_to_leaf(leaf, &leaf_copy, ctx, &mut local_addr);
    *fors_leaf_addr = local_addr;
}

// ---------- C-ABI exports ----------

#[unsafe(export_name = "SPX_fors_gen_leafx1")]
pub unsafe extern "C" fn spx_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    let leaf_slice = unsafe { core::slice::from_raw_parts_mut(leaf, SPX_N) };
    let info_ref = unsafe { &mut *info };
    fors_gen_leafx1(leaf_slice, unsafe { &*ctx }, addr_idx, info_ref);
}
