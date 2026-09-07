//! CONFIGS.md rows 10-14: the `hash.h` / `thash.h` backend interface.

mod common;
use common::*;

type FInit = unsafe extern "C" fn(*mut u8);
type FPrf = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type FThash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);
type FGenR = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
type FHashMsg = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u32,
    *const u8,
    *const u8,
    *const u8,
    u64,
    *const u8,
);

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

/// Message lengths that straddle every absorb/block boundary of every backend
/// (sha2: 64/128 byte blocks; shake: 136-byte rate; haraka: 32-byte rate;
/// blake: 64/128-byte buffers).
pub const MLENS: &[usize] = &[
    0, 1, 15, 16, 17, 31, 32, 33, 47, 48, 63, 64, 65, 71, 72, 79, 80, 103, 104, 111, 112, 127, 128,
    135, 136, 137, 167, 168, 169, 255, 256, 1000,
];

/// Build two identical, freshly initialised ctx buffers (C and Rust side).
fn init_pair(rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let p = libs();
    let ci = f!(p.c, "SPX_initialize_hash_function", FInit);
    let ri = f!(p.rust, "SPX_initialize_hash_function", FInit);
    let mut a = new_ctx();
    seed_ctx(&mut a, rng);
    let mut b = a.clone();
    unsafe {
        ci(a.as_mut_ptr());
        ri(b.as_mut_ptr());
    }
    eq_bytes("initialize_hash_function(ctx)", &a[..CTX_LIVE], &b[..CTX_LIVE]);
    (a, b)
}

/// row 10 -- `SPX_initialize_hash_function` (no-op / seed_state / tweak_constants)
#[test]
fn row10_initialize_hash_function() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..16 {
        let _ = init_pair(&mut rng);
    }
    // all-zero and all-0xFF seeds
    let p = libs();
    let ci = f!(p.c, "SPX_initialize_hash_function", FInit);
    let ri = f!(p.rust, "SPX_initialize_hash_function", FInit);
    for fillv in [0x00u8, 0xFFu8] {
        let mut a = new_ctx();
        for x in a[..2 * SPX_N].iter_mut() {
            *x = fillv;
        }
        let mut b = a.clone();
        unsafe {
            ci(a.as_mut_ptr());
            ri(b.as_mut_ptr());
        }
        eq_bytes(
            &format!("initialize_hash_function(fill={fillv:#x})"),
            &a[..CTX_LIVE],
            &b[..CTX_LIVE],
        );
    }
}

/// row 11 -- `SPX_prf_addr`
#[test]
fn row11_prf_addr() {
    let p = libs();
    let cf = f!(p.c, "SPX_prf_addr", FPrf);
    let rf = f!(p.rust, "SPX_prf_addr", FPrf);
    let mut rng = Rng::new(SEED ^ 11);

    for _ in 0..32 {
        let (ca, ra) = init_pair(&mut rng);
        let mut addr = [0u32; 8];
        for w in addr.iter_mut() {
            *w = rng.next_u32();
        }
        let mut o1 = vec![0xCCu8; 64];
        let mut o2 = vec![0xCCu8; 64];
        unsafe {
            cf(o1.as_mut_ptr(), ca.as_ptr(), addr.as_ptr());
            rf(o2.as_mut_ptr(), ra.as_ptr(), addr.as_ptr());
        }
        eq_bytes("prf_addr", &o1, &o2);
    }
}

/// row 12 -- `SPX_thash` over every `inblocks` the library ever uses, plus 0
#[test]
fn row12_thash() {
    let p = libs();
    let cf = f!(p.c, "SPX_thash", FThash);
    let rf = f!(p.rust, "SPX_thash", FThash);
    let mut rng = Rng::new(SEED ^ 12);

    let mut blocks = vec![0u32, 1, 2, 3, SPX_WOTS_LEN as u32, SPX_FORS_TREES as u32];
    blocks.sort_unstable();
    blocks.dedup();

    for &inblocks in &blocks {
        for _ in 0..6 {
            let (ca, ra) = init_pair(&mut rng);
            let input = rng.bytes((inblocks as usize) * SPX_N + 1);
            let mut addr = [0u32; 8];
            for w in addr.iter_mut() {
                *w = rng.next_u32();
            }
            let mut a1 = addr;
            let mut a2 = addr;
            let mut o1 = vec![0xDDu8; 64];
            let mut o2 = vec![0xDDu8; 64];
            unsafe {
                cf(
                    o1.as_mut_ptr(),
                    input.as_ptr(),
                    inblocks,
                    ca.as_ptr(),
                    a1.as_mut_ptr(),
                );
                rf(
                    o2.as_mut_ptr(),
                    input.as_ptr(),
                    inblocks,
                    ra.as_ptr(),
                    a2.as_mut_ptr(),
                );
            }
            eq_bytes(&format!("thash(inblocks={inblocks})"), &o1, &o2);
            eq(&format!("thash addr(inblocks={inblocks})"), a1, a2);
        }
    }
}

/// row 13 -- `SPX_gen_message_random` over every message-length boundary
#[test]
fn row13_gen_message_random() {
    let p = libs();
    let cf = f!(p.c, "SPX_gen_message_random", FGenR);
    let rf = f!(p.rust, "SPX_gen_message_random", FGenR);
    let mut rng = Rng::new(SEED ^ 13);

    for &mlen in MLENS {
        let (ca, ra) = init_pair(&mut rng);
        let sk_prf = rng.bytes(SPX_N);
        let optrand = rng.bytes(SPX_N);
        let m = rng.bytes(mlen.max(1));
        // The blake backend's `blakeX_final` writes 32 (or 64) bytes into R,
        // more than SPX_N -- compare a generous window so that is covered.
        let mut o1 = vec![0xEEu8; 160];
        let mut o2 = vec![0xEEu8; 160];
        unsafe {
            cf(
                o1.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                ca.as_ptr(),
            );
            rf(
                o2.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                ra.as_ptr(),
            );
        }
        eq_bytes(&format!("gen_message_random(mlen={mlen})"), &o1, &o2);
    }
}

/// row 14 -- `SPX_hash_message`, incl. the `tree` / `leaf_idx` out-params
#[test]
fn row14_hash_message() {
    let p = libs();
    let cf = f!(p.c, "SPX_hash_message", FHashMsg);
    let rf = f!(p.rust, "SPX_hash_message", FHashMsg);
    let mut rng = Rng::new(SEED ^ 14);

    for &mlen in MLENS {
        let (ca, ra) = init_pair(&mut rng);
        let r = rng.bytes(SPX_N);
        let pk = rng.bytes(SPX_PK_BYTES);
        let m = rng.bytes(mlen.max(1));
        let mut d1 = vec![0x11u8; SPX_FORS_MSG_BYTES + 16];
        let mut d2 = vec![0x11u8; SPX_FORS_MSG_BYTES + 16];
        let mut t1 = 0xDEAD_BEEF_DEAD_BEEFu64;
        let mut t2 = t1;
        let mut l1 = 0xDEAD_BEEFu32;
        let mut l2 = l1;
        unsafe {
            cf(
                d1.as_mut_ptr(),
                &mut t1,
                &mut l1,
                r.as_ptr(),
                pk.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                ca.as_ptr(),
            );
            rf(
                d2.as_mut_ptr(),
                &mut t2,
                &mut l2,
                r.as_ptr(),
                pk.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                ra.as_ptr(),
            );
        }
        eq_bytes(&format!("hash_message digest(mlen={mlen})"), &d1, &d2);
        eq(&format!("hash_message tree(mlen={mlen})"), t1, t2);
        eq(&format!("hash_message leaf_idx(mlen={mlen})"), l1, l2);
    }
}
