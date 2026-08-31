//! Translation of c_src/libsodium/crypto_generichash/blake2b/ref/blake2b-compress-ref.c

use crate::common::{load64_le, rotr64};
use core::ffi::c_int;

// blake2b_state, packed (see blake2.h `#pragma pack(push, 1)`).
#[repr(C, packed)]
struct blake2b_state {
    h: [u64; 8],
    t: [u64; 2],
    f: [u64; 2],
    buf: [u8; 2 * 128],
    buflen: usize,
    last_node: u8,
}

// BLAKE2B_BLOCKBYTES == 128
const BLAKE2B_BLOCKBYTES: usize = 128;

// CRYPTO_ALIGN(64) static const uint64_t blake2b_IV[8]
static blake2b_IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

static blake2b_sigma: [[u8; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

// quirks.h: blake2b_compress_ref -> _sodium_blake2b_compress_ref
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_compress_ref(
    S: *mut blake2b_state,
    block: *const u8,
) -> c_int {
    let mut m: [u64; 16] = [0; 16];
    let mut v: [u64; 16] = [0; 16];
    let mut i: usize;

    i = 0;
    while i < 16 {
        m[i] = load64_le(block.add(i * core::mem::size_of::<u64>()));
        i += 1;
    }
    i = 0;
    while i < 8 {
        v[i] = core::ptr::addr_of!((*S).h[i]).read_unaligned();
        i += 1;
    }
    let t = core::ptr::addr_of!((*S).t).read_unaligned();
    let f = core::ptr::addr_of!((*S).f).read_unaligned();
    v[8] = blake2b_IV[0];
    v[9] = blake2b_IV[1];
    v[10] = blake2b_IV[2];
    v[11] = blake2b_IV[3];
    v[12] = t[0] ^ blake2b_IV[4];
    v[13] = t[1] ^ blake2b_IV[5];
    v[14] = f[0] ^ blake2b_IV[6];
    v[15] = f[1] ^ blake2b_IV[7];

    // G(r, i, a, b, c, d) expanded, operating on v indices.
    macro_rules! g {
        ($r:expr, $ii:expr, $a:expr, $b:expr, $c:expr, $d:expr) => {{
            v[$a] = v[$a]
                .wrapping_add(v[$b])
                .wrapping_add(m[blake2b_sigma[$r][2 * $ii + 0] as usize]);
            v[$d] = rotr64(v[$d] ^ v[$a], 32);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotr64(v[$b] ^ v[$c], 24);
            v[$a] = v[$a]
                .wrapping_add(v[$b])
                .wrapping_add(m[blake2b_sigma[$r][2 * $ii + 1] as usize]);
            v[$d] = rotr64(v[$d] ^ v[$a], 16);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotr64(v[$b] ^ v[$c], 63);
        }};
    }
    macro_rules! round {
        ($r:expr) => {{
            g!($r, 0, 0, 4, 8, 12);
            g!($r, 1, 1, 5, 9, 13);
            g!($r, 2, 2, 6, 10, 14);
            g!($r, 3, 3, 7, 11, 15);
            g!($r, 4, 0, 5, 10, 15);
            g!($r, 5, 1, 6, 11, 12);
            g!($r, 6, 2, 7, 8, 13);
            g!($r, 7, 3, 4, 9, 14);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    round!(7);
    round!(8);
    round!(9);
    round!(10);
    round!(11);

    i = 0;
    while i < 8 {
        let hi = core::ptr::addr_of!((*S).h[i]).read_unaligned();
        core::ptr::addr_of_mut!((*S).h[i]).write_unaligned(hi ^ v[i] ^ v[i + 8]);
        i += 1;
    }

    let _ = BLAKE2B_BLOCKBYTES;
    0
}
