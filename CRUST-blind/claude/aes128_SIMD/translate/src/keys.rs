use crate::aes::{NB, NK, NR};
use crate::cipher_utils::{RCON, SBOX};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Mirror the C code:
    //   for i in 0..Nk*4 step 16: replicate 16-byte block of Key into w[i..i+16],
    //     w[i+4..i+20], w[i+8..i+24], w[i+12..i+28]
    // For Nk=4 (Nk*4 = 16), only i=0 runs.
    // After this: w[0..16] = key[0..16]. Other writes go to w[4..20], w[8..24],
    // w[12..28]. Each subsequent write overwrites previous, so:
    //   w[0..4]  = key[0..4]   (from i=0 at i)
    //   w[4..8]  = key[0..4]   (overwritten by i+4 write... wait)
    // Actually trace more carefully. Each store is 16 bytes. The last store
    // (at i+12) writes key[0..16] to w[12..28]. Before that, w[8..24] was
    // written. Before that, w[4..20]. Before that, w[0..16].
    // Net result for w[0..28]:
    //   w[0..4]   = key[0..4]   (only set by i+0 store)
    //   w[4..8]   = key[0..4]   (set by i+0, then overwritten by i+4 -> key[0..4])
    //   wait, i+4 store at offset 4 writes 16 bytes of `temp` (which is key[0..16]).
    //   So w[4..8] = key[0..4], w[8..12]=key[4..8] etc.
    //   But then i+8 store writes key[0..16] at offset 8. So w[8..12]=key[0..4],
    //   etc.
    //   And i+12 store writes key[0..16] at offset 12. So:
    //     w[0..4]   = key[0..4]   (from i+0)
    //     w[4..8]   = key[0..4]   (from i+4, overwrote prior)
    //     w[8..12]  = key[0..4]   (from i+8, overwrote prior)
    //     w[12..16] = key[0..4]   (from i+12, overwrote prior)
    //     w[16..20] = key[4..8]   (from i+12)
    //     w[20..24] = key[8..12]  (from i+12)
    //     w[24..28] = key[12..16] (from i+12)

    // Zero out w
    for byte in w.iter_mut() {
        *byte = 0;
    }

    // Replicate the first 4 bytes of key into w[0..4], w[4..8], w[8..12], w[12..16]
    // Then place key[4..16] into w[16..28]. This matches the analysis above.
    // Then continue with the standard expansion loop.
    // Actually for proper AES-128, we want w[0..16] = key[0..16], not the
    // strange replicate. But the C code does this odd thing.
    //
    // Wait — let me re-read the C code. The loop is:
    //   for (i = 0; i < Nk * 4; i += 16)
    // Nk=4, so Nk*4 = 16. Loop runs once with i=0.
    // Inside:
    //   temp = load key[0..16]
    //   store temp at w[0..16]    (offset i=0)
    //   store temp at w[4..20]    (offset i+4=4)
    //   store temp at w[8..24]    (offset i+8=8)
    //   store temp at w[12..28]   (offset i+12=12)
    // The stores happen sequentially. Final w[0..28]:
    //   w[0..4]  = key[0..4]    (from i+0, not overwritten)
    //   w[4..8]  = key[0..4]    (from i+4)
    //   w[8..12] = key[0..4]    (from i+8)
    //   w[12..16]= key[0..4]    (from i+12)
    //   w[16..20]= key[4..8]    (from i+12)
    //   w[20..24]= key[8..12]   (from i+12)
    //   w[24..28]= key[12..16]  (from i+12)

    // Apply the same pattern
    // i+0 store: w[0..16] = key
    w[0..16].copy_from_slice(&key[0..16]);
    // i+4 store: w[4..20] = key
    w[4..20].copy_from_slice(&key[0..16]);
    // i+8 store: w[8..24] = key
    w[8..24].copy_from_slice(&key[0..16]);
    // i+12 store: w[12..28] = key
    w[12..28].copy_from_slice(&key[0..16]);

    // Now run the main expansion loop.
    let mut i = NK;
    while i < NB * (NR + 1) {
        // temp = load 16 bytes from w[4*(i-1) .. 4*(i-1) + 16]
        let mut temp = [0u8; 16];
        let src_start = 4 * (i - 1);
        // Be careful: src_start + 16 may exceed array bounds. But let's check.
        // Max i = NB*(NR+1) - 1 = 4*11 - 1 = 43. src_start = 168. +16 = 184.
        // But w array has 4*NB*(NR+1) = 176 bytes. So we'd read out of bounds.
        // Need to handle this — in C, this is undefined behavior but in
        // practice reads beyond. In our case, we should only read what's
        // needed. For the standard expansion algorithm, only the first 4 bytes
        // of temp matter (since temp represents word w[i-1]).
        // Actually, looking more carefully, the algorithm uses _mm_loadu_si128
        // which loads 16 bytes, but really only the first 4 bytes constitute
        // the "previous word". The rest of `temp`'s bytes get used by the
        // _mm_xor_si128 with w_i_Nk (also 16 bytes), and stored to
        // w[4*i .. 4*i + 16]. This means each iteration writes 4 words
        // (16 bytes), not just 1. The C code is doing 4 words at a time.
        //
        // But that doesn't match standard AES key expansion which derives
        // one word at a time. Let me re-examine.
        //
        // OK: after the pre-loop, w[0..28] is filled with the strange pattern.
        // The main loop starts at i=Nk=4 and increments by 4 each iteration.
        // So it processes positions w[16..32], w[20..36], w[24..40], etc.
        //
        // Each iteration:
        //   temp = w[4*(i-1) .. 4*(i-1)+16]  (16 bytes from offset 4*(i-1))
        //   if i % Nk == 0 (true when i=4,8,12,...): apply RotWord+SubWord+Rcon
        //   xor with w_i_Nk = w[4*(i-Nk) .. 4*(i-Nk)+16]
        //   store at w[4*i .. 4*i+16]
        //
        // Last iteration: i = 40 (when NR=10, NB=4, NR+1=11, so loop while
        // i < 44; last i = 40).
        // Wait: i starts at 4 and goes to < 44, stepping by 4. So i =
        // 4,8,12,16,20,24,28,32,36,40. That's 10 iterations.
        // For i=40: src_start = 4*39 = 156, +16 = 172. w has 176 bytes. OK.
        // For i=40: stores at w[160..176]. OK.
        //
        // So we never read past w[172] or write past w[176]. Good.

        for k in 0..16 {
            if src_start + k < w.len() {
                temp[k] = w[src_start + k];
            }
        }

        if i % NK == 0 {
            // _mm_shuffle_epi32 with _MM_SHUFFLE(3, 0, 1, 2)
            // _MM_SHUFFLE(z, y, x, w) = (z<<6) | (y<<4) | (x<<2) | w
            // For (3,0,1,2): output[0] = input[2], output[1] = input[1],
            //                output[2] = input[0], output[3] = input[3]
            // (32-bit lanes)
            // Each lane is 4 bytes, so:
            //   bytes 0..4   = old bytes 8..12
            //   bytes 4..8   = old bytes 4..8
            //   bytes 8..12  = old bytes 0..4
            //   bytes 12..16 = old bytes 12..16
            let old = temp;
            temp[0..4].copy_from_slice(&old[8..12]);
            temp[4..8].copy_from_slice(&old[4..8]);
            temp[8..12].copy_from_slice(&old[0..4]);
            temp[12..16].copy_from_slice(&old[12..16]);

            // SubWord on first 4 bytes
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
            }
            // XOR with Rcon[i/Nk]
            temp[0] ^= RCON[i / NK];
        } else if NK > 6 && i % NK == 4 {
            // Not applicable for NK=4
            for j in 0..4 {
                temp[j] = SBOX[temp[j] as usize];
                if j + 3 < 16 {
                    temp[j + 1] = SBOX[temp[j + 1] as usize];
                    temp[j + 2] = SBOX[temp[j + 2] as usize];
                    temp[j + 3] = SBOX[temp[j + 3] as usize];
                }
            }
        }

        // XOR temp with w[4*(i-Nk) .. 4*(i-Nk)+16]
        let xor_start = 4 * (i - NK);
        for k in 0..16 {
            temp[k] ^= w[xor_start + k];
        }

        // Store temp at w[4*i .. 4*i + 16]
        let dst_start = 4 * i;
        for k in 0..16 {
            if dst_start + k < w.len() {
                w[dst_start + k] = temp[k];
            }
        }

        i += 4;
    }
}
