use crate::aes::{NB, NR, NK};
use crate::cipher_utils::{SBOX, RCON};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Zero out w
    for b in w.iter_mut() {
        *b = 0;
    }

    // Mimic the C behavior: for i = 0; i < Nk * 4; i += 16
    // It loads key[0..16] and stores it 4 times overlapping: w[0..16], w[4..20], w[8..24], w[12..28]
    // Final state of w[0..28] after these stores (each 16-byte store overwrites previous):
    //   w[0..4]   = key[0..4]   (from store at offset 0)
    //   w[4..8]   = key[0..4]   (last store touching here was at offset 4)
    //   w[8..12]  = key[0..4]   (last store touching here was at offset 8)
    //   w[12..28] = key[0..16]  (from store at offset 12)
    // Since Nk = 4, Nk*4 = 16, so the loop runs once with i=0.
    // After this: w[0..4]=key[0..4], w[4..8]=key[0..4], w[8..12]=key[0..4], w[12..28]=key[0..16].
    // BUT: subsequent iterations (i=Nk=4 onwards) overwrite from w[16] onward.
    // So effectively w[0..16] before main loop = key[0..4] repeated 3 times then key[0..4].
    // Wait, let me re-check the C code:
    //   _mm_storeu_si128(&w[i], temp);     // writes w[i..i+16]
    //   _mm_storeu_si128(&w[i + 4], temp); // writes w[i+4..i+20]
    //   _mm_storeu_si128(&w[i + 8], temp); // writes w[i+8..i+24]
    //   _mm_storeu_si128(&w[i + 12], temp);// writes w[i+12..i+28]
    // After all four stores at i=0:
    //   w[0..4]   = key[0..4]   (first store, untouched after)
    //   w[4..8]   = key[0..4]   (overwritten by store at offset 4: writes key[0..16] to w[4..20])
    //   w[8..12]  = key[0..4]   (overwritten by store at offset 8: writes key[0..16] to w[8..24])
    //   w[12..28] = key[0..16]  (last store)
    // So w[0..4] = key[0..4]  (first 4 bytes of key)
    //    w[4..8] = key[0..4]   (still first 4 bytes - because the second store is identical key)
    //    w[8..12] = key[0..4]
    //    w[12..16] = key[0..4]
    //    w[16..20] = key[4..8]
    //    w[20..24] = key[8..12]
    //    w[24..28] = key[12..16]
    // Hmm wait, the second store at offset 4 writes key[0..16] to w[4..20]. So w[4..20] = key[0..16].
    // Then w[4..8]=key[0..4], w[8..12]=key[4..8], w[12..16]=key[8..12], w[16..20]=key[12..16].
    // Then store at offset 8 writes key[0..16] to w[8..24]. So w[8..12]=key[0..4], w[12..16]=key[4..8], etc.
    // Then store at offset 12 writes key[0..16] to w[12..28]. So w[12..16]=key[0..4], w[16..20]=key[4..8], w[20..24]=key[8..12], w[24..28]=key[12..16].
    // Final w[0..28]:
    //   w[0..4]   = key[0..4]
    //   w[4..8]   = key[0..4]   (untouched after first store at offset 4 wrote key[0..4] there)
    //   w[8..12]  = key[0..4]   (last store at offset 12 didn't touch this; store at offset 8 wrote key[0..4] here)
    //   w[12..16] = key[0..4]
    //   w[16..20] = key[4..8]
    //   w[20..24] = key[8..12]
    //   w[24..28] = key[12..16]
    // But Nk*(Nr+1) = 4*11 = 44, so total w length is 4*44 = 176 bytes.
    // The main loop continues from i=Nk=4 (i is in 4-byte words) and overwrites w[16..] anyway.
    // So we only care about w[0..16] from this initial setup.
    // Effective w[0..16] = key[0..4] || key[0..4] || key[0..4] || key[0..4]  -- no wait, that's wrong.
    // Let me redo: the initial state of w[0..16] after all 4 stores:
    //   Stores in order:
    //     1. offset 0: writes key[0..16] to w[0..16]
    //     2. offset 4: writes key[0..16] to w[4..20]
    //     3. offset 8: writes key[0..16] to w[8..24]
    //     4. offset 12: writes key[0..16] to w[12..28]
    //   For w[0..4]: only touched by store 1 -> key[0..4]
    //   For w[4..8]: touched by stores 1, 2. Store 2 writes key[0..4] here -> key[0..4]
    //   For w[8..12]: touched by stores 1, 2, 3. Store 3 writes key[0..4] here -> key[0..4]
    //   For w[12..16]: touched by stores 1, 2, 3, 4. Store 4 writes key[0..4] here -> key[0..4]
    //   For w[16..20]: touched by stores 2, 3, 4. Store 4 writes key[4..8] here -> key[4..8]
    //   For w[20..24]: touched by stores 3, 4. Store 4 writes key[8..12] here -> key[8..12]
    //   For w[24..28]: touched by store 4. Store 4 writes key[12..16] here -> key[12..16]
    // So w[0..16] = [key[0..4], key[0..4], key[0..4], key[0..4]] -> 4 copies of key[0..4]
    // That's clearly wrong for AES key expansion!
    // But the main loop starts with i=Nk=4 (word index), so 4*i = 16 (byte index).
    // The main loop reads from w[4*(i-1)] = w[12..16] which is key[0..4].
    // And w[4*(i-Nk)] = w[0..4] which is key[0..4].
    // Hmm. Let me re-read the C carefully.

    // Actually, looking more carefully:
    // for (i = 0; i < Nk * 4; i += 16)
    // Nk*4 = 16, so this loop runs ONCE (i=0).
    // But notice: this whole pre-loop seems buggy. The standard key expansion just copies the key
    // into w[0..16]. The C code has weird stores that overlap.
    //
    // Actually wait, the AES test passes in C, so this MUST result in w[0..16] = key[0..16].
    // Let me reconsider. In the C, _mm_storeu_si128(&w[i + 4], temp) — temp is loaded as
    // _mm_loadu_si128(&Key[i]) which is Key[0..16]. So it writes Key[0..16] starting at w[i+4]=w[4].
    //
    // Wait, this is buggy. Unless the test passes by coincidence... let me check what the test would
    // produce: with key[0..16] = {0x2B,0x7E,0x15,0x16,0x28,0xAE,0xD2,0xA6,0xAB,0xF7,0x15,0x88,0x09,0xCF,0x4F,0x3C}
    // After the pre-loop, w[0..16] would be:
    //   w[0..4]   = key[0..4] = {2B, 7E, 15, 16}
    //   w[4..8]   = key[0..4] = {2B, 7E, 15, 16}  (from store at offset 4)
    //   w[8..12]  = key[0..4] = {2B, 7E, 15, 16}  (from store at offset 8)
    //   w[12..16] = key[0..4] = {2B, 7E, 15, 16}  (from store at offset 12)
    // Then the main loop at i=4 reads w[12..16+] = key[0..4] = {2B, 7E, 15, 16}.
    // Then the main loop XORs with w[0..4] = key[0..4] = {2B, 7E, 15, 16}.
    //
    // But the standard AES test vector expects expanded_key[16..32] = {a0fafe17, 88542cb1, 23a33939, 2a6c7605}
    // which means w[16..20] = {a0, fa, fe, 17}.
    // If we follow the buggy C code: w[16..20] = sub_word(rot_word(w[12..16])) ^ rcon ^ w[0..4]
    //   rot_word({2B, 7E, 15, 16}) -> shuffle_epi32(_, _MM_SHUFFLE(3,0,1,2))
    //   shuffle_epi32 with (3,0,1,2) means: dst[0]=src[2], dst[1]=src[0], dst[2]=src[1], dst[3]=src[3]
    //   Wait actually _MM_SHUFFLE(3,0,1,2) selects 32-bit words:
    //   dst[31:0]   = src[3:2] (word index 2)
    //   dst[63:32]  = src[2:0] -- hmm let me look up.
    //   _MM_SHUFFLE(d, c, b, a) -> dst[31:0]=src[a*32:..], dst[63:32]=src[b*32:..], etc.
    //   So _MM_SHUFFLE(3,0,1,2) gives a=2, b=1, c=0, d=3
    //   dst[31:0]   = src word 2
    //   dst[63:32]  = src word 1
    //   dst[95:64]  = src word 0
    //   dst[127:96] = src word 3
    // Hmm, but temp is initially _mm_loadu_si128(&w[4*(i-1)]) = w[12..28].
    // With Nk*4=16 pre-loop, w[12..28]:
    //   w[12..16] = key[0..4]  = {2B, 7E, 15, 16}
    //   w[16..20] = key[4..8]  = {28, AE, D2, A6}  (last store at offset 12 wrote key[0..16] to w[12..28])
    //   w[20..24] = key[8..12] = {AB, F7, 15, 88}
    //   w[24..28] = key[12..16]= {09, CF, 4F, 3C}
    // So temp = the full key[0..16]!
    //
    // OK that makes sense now. Let me trace:
    //   word 0 of temp = key[0..4]  = {2B, 7E, 15, 16}
    //   word 1 of temp = key[4..8]  = {28, AE, D2, A6}
    //   word 2 of temp = key[8..12] = {AB, F7, 15, 88}
    //   word 3 of temp = key[12..16]= {09, CF, 4F, 3C}
    // After shuffle(3,0,1,2):
    //   word 0 = old word 2 = {AB, F7, 15, 88}
    //   word 1 = old word 1 = {28, AE, D2, A6}
    //   word 2 = old word 0 = {2B, 7E, 15, 16}
    //   word 3 = old word 3 = {09, CF, 4F, 3C}
    // Then sbox to first 4 bytes (word 0) = {AB, F7, 15, 88}:
    //   sbox[AB]=62, sbox[F7]=68, sbox[15]=59, sbox[88]=C4
    //   Hmm. But standard key expansion: rot_word(09 CF 4F 3C) = CF 4F 3C 09, sbox -> 8A 84 EB 01
    //   xor rcon -> 8B 84 EB 01, xor key[0..4] = 2B 7E 15 16 -> A0 FA FE 17. CORRECT.
    //   So the C code SHOULD produce {8B, 84, EB, 01} after sbox+rcon, matching standard.
    //   But my trace says first 4 bytes of temp after shuffle = {AB, F7, 15, 88}, not {CF, 4F, 3C, 09}.
    //
    // Let me re-check the load: temp = _mm_loadu_si128(&w[4*(i-1)]) where i=4, so &w[12].
    //   w[12..28] should be loaded as 16 bytes. With pre-loop in C:
    //   w[12..16] = key[0..4]  (because last store at offset 12 wrote key[0..16] to w[12..28])
    //   So w[12..16] = {2B, 7E, 15, 16}, NOT key[12..16] = {09, CF, 4F, 3C}.
    //   And w[16..20] = key[4..8] = {28, AE, D2, A6}.
    //   And w[20..24] = key[8..12] = {AB, F7, 15, 88}.
    //   And w[24..28] = key[12..16]= {09, CF, 4F, 3C}.
    //
    // So temp loaded from &w[12]:
    //   word 0 = w[12..16] = {2B, 7E, 15, 16}
    //   word 1 = w[16..20] = {28, AE, D2, A6}
    //   word 2 = w[20..24] = {AB, F7, 15, 88}
    //   word 3 = w[24..28] = {09, CF, 4F, 3C}
    //
    // Hmm that's just the original key shifted by 4 bytes. OK now shuffle(3,0,1,2):
    //   Actually wait, _MM_SHUFFLE(z, y, x, w) selects: dst[0..32]=src[w*32:(w+1)*32], dst[32..64]=src[x*32:(x+1)*32], dst[64..96]=src[y*32:(y+1)*32], dst[96..128]=src[z*32:(z+1)*32]
    //   So _MM_SHUFFLE(3, 0, 1, 2) means: dst word 0 = src word 2, dst word 1 = src word 1, dst word 2 = src word 0, dst word 3 = src word 3.
    //   dst word 0 = src word 2 = {AB, F7, 15, 88}
    //   dst word 1 = src word 1 = {28, AE, D2, A6}
    //   dst word 2 = src word 0 = {2B, 7E, 15, 16}
    //   dst word 3 = src word 3 = {09, CF, 4F, 3C}
    //
    // First 4 bytes (word 0): {AB, F7, 15, 88}. Apply sbox: sbox[AB]=62, sbox[F7]=68, sbox[15]=59, sbox[88]=C4.
    //   Result: {62, 68, 59, C4}. XOR with Rcon[1]=0x01 -> {63, 68, 59, C4}.
    //
    // Then load temp = {63, 68, 59, C4, 28, AE, D2, A6, 2B, 7E, 15, 16, 09, CF, 4F, 3C}.
    // XOR with w[4*(i-Nk)] = w[0..16] = {2B, 7E, 15, 16, 2B, 7E, 15, 16, 2B, 7E, 15, 16, 2B, 7E, 15, 16}.
    // Result: {48, 16, 4C, D2, 03, D0, C7, B0, 00, 00, 00, 00, 22, B1, 5A, 35}.
    // Store at &w[4*i] = &w[16].
    // So w[16..32] = {48, 16, 4C, D2, 03, D0, C7, B0, 00, 00, 00, 00, 22, B1, 5A, 35}.
    //
    // But the EXPECTED expanded key is: w[16..20] = {a0, fa, fe, 17}.
    // So this C code is BUGGY... or my understanding is wrong.

    // Let me re-read the test in the Rust code. The test_cipher uses key {0x2b...0x3c} and verifies cipher output.
    // If the key expansion is the standard AES key expansion, then we get the standard ciphertext.
    // If the C key expansion is buggy as I analyzed, the cipher output wouldn't match.

    // Actually, the Rust tests test the OUTPUT of cipher with a known-good test vector,
    // which assumes STANDARD AES. So the implementation should be standard AES key expansion,
    // not whatever buggy thing the C code does.

    // Let me implement standard AES-128 key expansion.

    let nk = NK;
    let nb = NB;
    let nr = NR;

    // Copy key into first Nk words of w
    for i in 0..(4 * nk) {
        w[i] = key[i];
    }

    let mut i = nk;
    while i < nb * (nr + 1) {
        let mut t = [0u8; 4];
        t[0] = w[4 * (i - 1)];
        t[1] = w[4 * (i - 1) + 1];
        t[2] = w[4 * (i - 1) + 2];
        t[3] = w[4 * (i - 1) + 3];

        if i % nk == 0 {
            // RotWord
            let tmp = t[0];
            t[0] = t[1];
            t[1] = t[2];
            t[2] = t[3];
            t[3] = tmp;
            // SubWord
            for j in 0..4 {
                t[j] = SBOX[t[j] as usize];
            }
            // XOR Rcon
            t[0] ^= RCON[i / nk];
        } else if nk > 6 && i % nk == 4 {
            for j in 0..4 {
                t[j] = SBOX[t[j] as usize];
            }
        }

        for j in 0..4 {
            w[4 * i + j] = w[4 * (i - nk) + j] ^ t[j];
        }

        i += 1;
    }
}
