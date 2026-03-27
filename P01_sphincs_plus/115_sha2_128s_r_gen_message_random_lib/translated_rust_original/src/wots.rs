use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::ull_to_bytes;

extern "C" {
    fn SPX_thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32);
}

unsafe fn thash(out: *mut u8, in_: *const u8, inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    SPX_thash(out, in_, inblocks, ctx as *const SpxCtx, addr.as_mut_ptr());
}

/// Computes the chaining function.
fn gen_chain(
    out: &mut [u8],
    in_: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    out[..SPX_N].copy_from_slice(&in_[..SPX_N]);
    let mut i = start;
    while i < start + steps && i < SPX_WOTS_W as u32 {
        set_hash_addr(addr, i);
        unsafe { thash(out.as_mut_ptr(), out.as_ptr(), 1, ctx, addr) };
        i += 1;
    }
}

/// base_w algorithm - interprets bytes as integers in base w.
fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0;
    let mut out_idx = 0;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    for _ in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
        out_idx += 1;
    }
}

/// Computes the WOTS+ checksum over a message (in base_w).
fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = vec![0u8; csum_bytes_len];

    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    unsafe {
        ull_to_bytes(csum_bytes.as_mut_ptr(), csum_bytes_len as u32, csum as u64);
    }
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

/// Takes a message and derives the matching chain lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_chain_lengths(lengths: *mut u32, msg: *const u8) {
    let lengths_slice = core::slice::from_raw_parts_mut(lengths, SPX_WOTS_LEN);
    let msg_slice = core::slice::from_raw_parts(msg, SPX_WOTS_LEN1 * SPX_WOTS_LOGW / 8);
    base_w(lengths_slice, SPX_WOTS_LEN1, msg_slice);
    // Copy msg part to avoid borrow conflict
    let msg_part: Vec<u32> = lengths_slice[..SPX_WOTS_LEN1].to_vec();
    wots_checksum(&mut lengths_slice[SPX_WOTS_LEN1..], &msg_part);
}

pub fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let msg_part: Vec<u32> = lengths[..SPX_WOTS_LEN1].to_vec();
    wots_checksum(&mut lengths[SPX_WOTS_LEN1..], &msg_part);
}

/// Takes a WOTS signature and an n-byte message, computes a WOTS public key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    msg: *const u8,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    let msg_slice = core::slice::from_raw_parts(msg, SPX_N);
    chain_lengths(&mut lengths, msg_slice);

    let addr_ref = &mut *(addr as *mut [u32; 8]);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr_ref, i as u32);
        let pk_slice = core::slice::from_raw_parts_mut(pk.add(i * SPX_N), SPX_N);
        let sig_slice = core::slice::from_raw_parts(sig.add(i * SPX_N), SPX_N);
        gen_chain(
            pk_slice,
            sig_slice,
            lengths[i],
            SPX_WOTS_W as u32 - 1 - lengths[i],
            &*ctx,
            addr_ref,
        );
    }
}
