use crate::address::{set_tree_index, set_type};
use crate::context::SpxCtx;
use crate::fors::ForsGenLeafInfo;
use crate::params::{SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE};
use crate::thash::thash;
use crate::hash::prf_addr;

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    let fors_leaf_addr = &mut info.leaf_addrx;
    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    prf_addr(leaf, ctx, fors_leaf_addr);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    let out = thash(&leaf[..crate::params::SPX_N], 1, ctx, fors_leaf_addr);
    leaf[..crate::params::SPX_N].copy_from_slice(&out);
}
