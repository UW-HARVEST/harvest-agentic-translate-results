use crate::address;
use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::thash::thash;

/// Converts the value of `in` to `outlen` bytes in big-endian byte order.
pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut input: u64) {
    if outlen == 0 {
        return;
    }
    let mut i = (outlen as i64) - 1;
    while i >= 0 {
        out[i as usize] = (input & 0xff) as u8;
        input >>= 8;
        i -= 1;
    }
}

pub fn u32_to_bytes(out: &mut [u8], input: u32) {
    out[0] = (input >> 24) as u8;
    out[1] = (input >> 16) as u8;
    out[2] = (input >> 8) as u8;
    out[3] = input as u8;
}

/// Converts inlen bytes from big-endian to an integer.
pub fn bytes_to_ull(input: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
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
    let n = SPX_N;
    let mut buffer = vec![0u8; 2 * n];
    let mut auth_pos: usize = 0;

    if leaf_idx & 1 != 0 {
        buffer[n..2 * n].copy_from_slice(&leaf[..n]);
        buffer[..n].copy_from_slice(&auth_path[..n]);
    } else {
        buffer[..n].copy_from_slice(&leaf[..n]);
        buffer[n..2 * n].copy_from_slice(&auth_path[..n]);
    }
    auth_pos += n;

    let mut i: u32 = 0;
    while i + 1 < tree_height {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        address::set_tree_height(addr, i + 1);
        address::set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp_in = buffer.clone();
            thash(&mut buffer[n..2 * n], &tmp_in, 2, ctx, addr);
            buffer[..n].copy_from_slice(&auth_path[auth_pos..auth_pos + n]);
        } else {
            let tmp_in = buffer.clone();
            thash(&mut buffer[..n], &tmp_in, 2, ctx, addr);
            buffer[n..2 * n].copy_from_slice(&auth_path[auth_pos..auth_pos + n]);
        }
        auth_pos += n;
        i += 1;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    address::set_tree_height(addr, tree_height);
    address::set_tree_index(addr, leaf_idx + idx_offset);
    let tmp_in = buffer.clone();
    thash(&mut root[..n], &tmp_in, 2, ctx, addr);
}

// ---------------------------------------------------------------------
// C-ABI exports (renamed to SPX_* to match the C linker symbols)
// ---------------------------------------------------------------------

#[unsafe(export_name = "SPX_ull_to_bytes")]
pub unsafe extern "C" fn spx_ull_to_bytes(
    out: *mut u8,
    outlen: core::ffi::c_uint,
    input: core::ffi::c_ulonglong,
) {
    let slice = unsafe { core::slice::from_raw_parts_mut(out, outlen as usize) };
    ull_to_bytes(slice, outlen as usize, input);
}

#[unsafe(export_name = "SPX_u32_to_bytes")]
pub unsafe extern "C" fn spx_u32_to_bytes(out: *mut u8, input: u32) {
    let slice = unsafe { core::slice::from_raw_parts_mut(out, 4) };
    u32_to_bytes(slice, input);
}

#[unsafe(export_name = "SPX_bytes_to_ull")]
pub unsafe extern "C" fn spx_bytes_to_ull(
    input: *const u8,
    inlen: core::ffi::c_uint,
) -> core::ffi::c_ulonglong {
    let slice = unsafe { core::slice::from_raw_parts(input, inlen as usize) };
    bytes_to_ull(slice, inlen as usize)
}
