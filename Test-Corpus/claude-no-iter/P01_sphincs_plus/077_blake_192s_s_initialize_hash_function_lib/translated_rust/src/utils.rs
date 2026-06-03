// Translation of c_src/app/src/utils.c

use crate::context::SpxCtx;
use crate::params::{SPX_N, SPX_FORS_TREES};
use crate::address::{set_tree_height, set_tree_index};
use crate::thash::thash;

/// Converts the value of `in_val` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut in_val: u64) {
    // iterate decreasingly for big-endian
    let mut i = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (in_val & 0xff) as u8;
        in_val >>= 8;
        i -= 1;
    }
}

pub fn u32_to_bytes(out: &mut [u8], in_val: u32) {
    out[0] = (in_val >> 24) as u8;
    out[1] = (in_val >> 16) as u8;
    out[2] = (in_val >> 8) as u8;
    out[3] = in_val as u8;
}

/// Converts `inlen` bytes in `in_val` from big-endian byte order to an integer.
pub fn bytes_to_ull(in_val: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (in_val[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

/// Computes a root node given a leaf and an auth path.
pub fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut auth_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    auth_off += SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            // hash buffer -> buffer[N..2N], then copy auth into buffer[..N]
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, &buffer, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&tmp);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            // hash buffer -> buffer[..N], copy auth into buffer[N..2N]
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, &buffer, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&tmp);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }

    // Last iteration: do not copy an auth_path node.
    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    let mut tmp = [0u8; SPX_N];
    thash(&mut tmp, &buffer, 2, ctx, addr);
    root[..SPX_N].copy_from_slice(&tmp);
}

// `treehash` is also defined in utils.c but is unused at the C call sites in
// this project (everything goes through the x1 variants). We omit it.

/// Helper used when the FORS leaf-info struct is passed through.
/// We provide both `wots_treehashx1` and `fors_treehashx1` in `utilsx1.rs`.
///
/// (Used by callers to size the auth-path / VLA buffers.)
#[allow(dead_code)]
pub const _UTILS_DUMMY: usize = SPX_FORS_TREES; // suppress unused warnings when feature stripping
