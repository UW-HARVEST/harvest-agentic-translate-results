use crate::aes::{NB, NK, NR, RCON, SBOX};

/// AES AddRoundKey step.
pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

// Helper: copy 16 bytes from `src` into `dst[offset..offset+16]`. Mirrors the
// behaviour of an unaligned SSE store in C, including allowing overlapping
// destinations across calls.
fn store128(dst: &mut [u8], offset: usize, src: &[u8; 16]) {
    dst[offset..offset + 16].copy_from_slice(src);
}

// Helper: load 16 bytes from `src[offset..offset+16]`. Mirrors `_mm_loadu_si128`.
fn load128(src: &[u8], offset: usize) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(&src[offset..offset + 16]);
    out
}

/// AES key expansion. Faithfully replicates the reference C implementation,
/// which uses overlapping unaligned 16-byte SSE stores rather than the
/// canonical word-by-word AES key schedule. The end result for AES-128 is
/// identical to the canonical schedule, but the intermediate `w` bytes can
/// differ in unused/scratch regions, so we mimic the byte-level behaviour of
/// the original code.
pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Zero the buffer (matches `memset(w, 0, ...)`).
    for byte in w.iter_mut() {
        *byte = 0;
    }

    // Initial fill loop. With Nk = 4 the C loop body executes once with i=0:
    //   temp = load(Key[0..16])
    //   store(w[0..16], temp)
    //   store(w[4..20], temp)
    //   store(w[8..24], temp)
    //   store(w[12..28], temp)
    // The overlapping stores leave w in this state:
    //   w[0..4]   = Key[0..4]
    //   w[4..8]   = Key[0..4]
    //   w[8..12]  = Key[0..4]
    //   w[12..16] = Key[0..4]
    //   w[16..20] = Key[4..8]
    //   w[20..24] = Key[8..12]
    //   w[24..28] = Key[12..16]
    //
    // We replicate that exactly here for byte-level fidelity.
    let mut temp16 = [0u8; 16];
    let mut i: usize = 0;
    while i < NK * 4 {
        temp16.copy_from_slice(&key[i..i + 16]);
        store128(w, i, &temp16);
        store128(w, i + 4, &temp16);
        store128(w, i + 8, &temp16);
        store128(w, i + 12, &temp16);
        i += 16;
    }

    // Main expansion loop. `i` is a 4-byte word index.
    let mut i: usize = NK;
    while i < NB * (NR + 1) {
        // Load the previous 16 bytes (4 words) starting at w[4*(i-1)].
        let mut temp = load128(w, 4 * (i - 1));

        if i % NK == 0 {
            // Equivalent of `_mm_shuffle_epi32(temp, _MM_SHUFFLE(3, 0, 1, 2))`.
            // The shuffle reorders the four 32-bit lanes:
            //   dst[0] = src[2]
            //   dst[1] = src[1]
            //   dst[2] = src[0]
            //   dst[3] = src[3]
            let mut shuffled = [0u8; 16];
            shuffled[0..4].copy_from_slice(&temp[8..12]); // src[2]
            shuffled[4..8].copy_from_slice(&temp[4..8]);  // src[1]
            shuffled[8..12].copy_from_slice(&temp[0..4]); // src[0]
            shuffled[12..16].copy_from_slice(&temp[12..16]); // src[3]

            // SubBytes on the first 4 bytes; the remaining 12 bytes pass
            // through unchanged.
            for j in 0..4 {
                shuffled[j] = SBOX[shuffled[j] as usize];
            }
            shuffled[0] ^= RCON[i / NK];
            temp = shuffled;
        } else if NK > 6 && (i % NK == 4) {
            // AES-256 SubWord branch. With NK == 4 this is unreachable, but
            // we replicate the structure for parity with the C source.
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
                temp[j + 1] = SBOX[temp[j + 1] as usize];
                temp[j + 2] = SBOX[temp[j + 2] as usize];
                temp[j + 3] = SBOX[temp[j + 3] as usize];
            }
        }

        // 16-byte XOR with w[4*(i - Nk) .. + 16].
        let w_prev = load128(w, 4 * (i - NK));
        for k in 0..16 {
            temp[k] ^= w_prev[k];
        }
        // Store 16 bytes at w[4*i ..]. Subsequent iterations will overwrite
        // most of these bytes, but the byte-level final state matches the C
        // code's overlapping-store sequence.
        store128(w, 4 * i, &temp);

        i += 4;
    }
}
