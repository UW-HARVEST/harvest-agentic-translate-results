use crate::address;
use crate::context::SpxCtx;
use crate::params::{SPX_N, SPX_WOTS_LEN, SPX_WOTS_LEN1, SPX_WOTS_LEN2, SPX_WOTS_LOGW, SPX_WOTS_W};
use crate::thash::thash;
use crate::utils::ull_to_bytes;

/// Computes the chaining function.
fn gen_chain(
    out: &mut [u8],
    in_buf: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    out[..SPX_N].copy_from_slice(&in_buf[..SPX_N]);

    let mut i = start;
    let end = start + steps;
    while i < end && (i as usize) < SPX_WOTS_W {
        address::set_hash_addr(addr, i);
        let in_clone = out[..SPX_N].to_vec();
        thash(&mut out[..SPX_N], &in_clone, 1, ctx, addr);
        i += 1;
    }
}

/// base_w algorithm: interprets bytes as integers in base w.
fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0usize;
    let mut out_idx = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    for _ in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) as u32) & ((SPX_WOTS_W - 1) as u32);
        out_idx += 1;
    }
}

/// Computes the WOTS+ checksum over a message (in base_w).
fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    const CSUM_BYTES_LEN: usize = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; CSUM_BYTES_LEN];

    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W - 1) as u32 - msg_base_w[i];
    }

    let shift = (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    csum <<= shift;
    ull_to_bytes(&mut csum_bytes, CSUM_BYTES_LEN, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

/// Take a message and derive the matching chain lengths.
pub fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let (left, right) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(right, left);
}

/// Take a WOTS signature and an n-byte message, compute a WOTS public key.
pub fn wots_pk_from_sig(
    pk: &mut [u8],
    sig: &[u8],
    msg: &[u8],
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);

    for i in 0..SPX_WOTS_LEN {
        address::set_chain_addr(addr, i as u32);
        let start = lengths[i];
        let steps = (SPX_WOTS_W - 1) as u32 - lengths[i];
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..(i + 1) * SPX_N],
            start,
            steps,
            ctx,
            addr,
        );
    }
}
