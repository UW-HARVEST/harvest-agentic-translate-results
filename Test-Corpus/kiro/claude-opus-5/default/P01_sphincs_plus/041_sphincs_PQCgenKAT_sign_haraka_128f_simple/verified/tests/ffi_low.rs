//! Differential tests for the lowest layer: `app/src/utils.c`,
//! `app/src/address.c` and the backends' `thash` / `hash` functions.
//!
//! Every call is dispatched through `dlsym` on the C and the Rust shared
//! object, so the exported ABI wrappers are part of what is being tested.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::os::raw::{c_uint, c_ulong};

// ---------------------------------------------------------------------------
// app/src/utils.c
// ---------------------------------------------------------------------------

type FnUllToBytes = unsafe extern "C" fn(*mut u8, c_uint, u64);
type FnU32ToBytes = unsafe extern "C" fn(*mut u8, u32);
type FnBytesToUll = unsafe extern "C" fn(*const u8, c_uint) -> u64;

#[test]
fn ull_to_bytes() {
    let l = libs();
    let c = unsafe { l.c::<FnUllToBytes>("SPX_ull_to_bytes") };
    let r = unsafe { l.r::<FnUllToBytes>("SPX_ull_to_bytes") };

    let mut rng = Rng::new(0x1234);
    for outlen in 0..=16u32 {
        let mut values = vec![0u64, 1, 0xff, 0x100, u64::MAX, 0x0123_4567_89ab_cdef];
        for _ in 0..8 {
            values.push(rng.next_u64());
        }
        for v in values {
            // Over-allocate so an out-of-range write would be caught.
            let mut cb = vec![0xAAu8; 24];
            let mut rb = vec![0xAAu8; 24];
            unsafe {
                c(cb.as_mut_ptr(), outlen, v);
                r(rb.as_mut_ptr(), outlen, v);
            }
            assert_bytes_eq(&format!("ull_to_bytes(outlen={outlen}, in={v:#x})"), &cb, &rb);
        }
    }
}

#[test]
fn u32_to_bytes() {
    let l = libs();
    let c = unsafe { l.c::<FnU32ToBytes>("SPX_u32_to_bytes") };
    let r = unsafe { l.r::<FnU32ToBytes>("SPX_u32_to_bytes") };

    let mut rng = Rng::new(0x2345);
    let mut values = vec![0u32, 1, 0xff, 0x100, u32::MAX, 0xdead_beef];
    for _ in 0..16 {
        values.push(rng.next_u32());
    }
    for v in values {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        unsafe {
            c(cb.as_mut_ptr(), v);
            r(rb.as_mut_ptr(), v);
        }
        assert_bytes_eq(&format!("u32_to_bytes({v:#x})"), &cb, &rb);
    }
}

#[test]
fn bytes_to_ull() {
    let l = libs();
    let c = unsafe { l.c::<FnBytesToUll>("SPX_bytes_to_ull") };
    let r = unsafe { l.r::<FnBytesToUll>("SPX_bytes_to_ull") };

    let mut rng = Rng::new(0x3456);
    for inlen in 0..=8u32 {
        for _ in 0..16 {
            let buf = rng.vec(8);
            let cv = unsafe { c(buf.as_ptr(), inlen) };
            let rv = unsafe { r(buf.as_ptr(), inlen) };
            assert_eq!(cv, rv, "bytes_to_ull(inlen={inlen}, {buf:02x?})");
        }
    }
}

// ---------------------------------------------------------------------------
// app/src/address.c
// ---------------------------------------------------------------------------

type FnAddrU32 = unsafe extern "C" fn(*mut u32, u32);
type FnAddrU64 = unsafe extern "C" fn(*mut u32, u64);
type FnAddrCopy = unsafe extern "C" fn(*mut u32, *const u32);

fn check_addr_u32(name: &str, values: &[u32]) {
    let l = libs();
    let c = unsafe { l.c::<FnAddrU32>(name) };
    let r = unsafe { l.r::<FnAddrU32>(name) };
    let mut rng = Rng::new(0x4567);
    for &v in values {
        let mut base = [0u32; 8];
        for w in base.iter_mut() {
            *w = rng.next_u32();
        }
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        assert_eq!(ca, ra, "{name}({v:#x}) base={base:08x?}");
    }
}

#[test]
fn address_setters_u32() {
    let mut rng = Rng::new(0x5678);
    let mut values = vec![0u32, 1, 0x7f, 0xff, 0x100, 0xffff_ffff, 0x1234_5678];
    for _ in 0..16 {
        values.push(rng.next_u32());
    }
    for name in [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_keypair_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
        "SPX_set_tree_index",
    ] {
        check_addr_u32(name, &values);
    }
}

#[test]
fn set_tree_addr() {
    let l = libs();
    let c = unsafe { l.c::<FnAddrU64>("SPX_set_tree_addr") };
    let r = unsafe { l.r::<FnAddrU64>("SPX_set_tree_addr") };
    let mut rng = Rng::new(0x6789);
    let mut values = vec![0u64, 1, u64::MAX, 0x0123_4567_89ab_cdef];
    for _ in 0..16 {
        values.push(rng.next_u64());
    }
    for v in values {
        let mut base = [0u32; 8];
        for w in base.iter_mut() {
            *w = rng.next_u32();
        }
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        assert_eq!(ca, ra, "set_tree_addr({v:#x})");
    }
}

#[test]
fn address_copies() {
    let l = libs();
    let mut rng = Rng::new(0x789a);
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        let c = unsafe { l.c::<FnAddrCopy>(name) };
        let r = unsafe { l.r::<FnAddrCopy>(name) };
        for _ in 0..32 {
            let mut src = [0u32; 8];
            let mut dst = [0u32; 8];
            for w in src.iter_mut() {
                *w = rng.next_u32();
            }
            for w in dst.iter_mut() {
                *w = rng.next_u32();
            }
            let mut ca = dst;
            let mut ra = dst;
            unsafe {
                c(ca.as_mut_ptr(), src.as_ptr());
                r(ra.as_mut_ptr(), src.as_ptr());
            }
            assert_eq!(ca, ra, "{name} src={src:08x?} dst={dst:08x?}");
        }
    }
}

/// Probes the address offsets the C build actually uses, and checks that the
/// Rust build agrees with the values transcribed in `common`.
#[test]
fn address_offsets_match_headers() {
    let l = libs();
    let probes: [(&str, usize); 5] = [
        ("SPX_set_layer_addr", SPX_OFFSET_LAYER),
        ("SPX_set_type", SPX_OFFSET_TYPE),
        ("SPX_set_chain_addr", SPX_OFFSET_CHAIN_ADDR),
        ("SPX_set_hash_addr", SPX_OFFSET_HASH_ADDR),
        ("SPX_set_tree_height", SPX_OFFSET_TREE_HGT),
    ];
    for (name, expect) in probes {
        for lib_is_c in [true, false] {
            let f = if lib_is_c {
                unsafe { l.c::<FnAddrU32>(name) }
            } else {
                unsafe { l.r::<FnAddrU32>(name) }
            };
            let mut addr = [0u32; 8];
            unsafe { f(addr.as_mut_ptr(), 0xA5) };
            let bytes: [u8; 32] = unsafe { std::mem::transmute(addr) };
            let touched: Vec<usize> = (0..32).filter(|&i| bytes[i] != 0).collect();
            assert_eq!(
                touched,
                vec![expect],
                "{name} in {} touched {touched:?}, expected offset {expect}",
                if lib_is_c { "C" } else { "Rust" }
            );
        }
    }
}

// ---------------------------------------------------------------------------
// <backend>/thash_*.c
// ---------------------------------------------------------------------------

type FnThash = unsafe extern "C" fn(*mut u8, *const u8, c_uint, *const u8, *mut u32);
type FnInitHash = unsafe extern "C" fn(*mut u8);

/// Prepares a pair of contexts (C, Rust) with identical seeds and runs
/// `initialize_hash_function` on both, comparing the resulting state.
pub fn seeded_ctx_pair(tag: u8) -> (Box<Ctx>, Box<Ctx>) {
    let l = libs();
    let ci = unsafe { l.c::<FnInitHash>("SPX_initialize_hash_function") };
    let ri = unsafe { l.r::<FnInitHash>("SPX_initialize_hash_function") };
    let mut cc = Ctx::seeded(tag);
    let mut rc = Ctx::seeded(tag);
    unsafe {
        ci(cc.as_mut_ptr());
        ri(rc.as_mut_ptr());
    }
    (cc, rc)
}

#[test]
fn initialize_hash_function() {
    for tag in [0u8, 1, 0x5a, 0xff] {
        let (cc, rc) = seeded_ctx_pair(tag);
        assert_bytes_eq(
            &format!("initialize_hash_function(tag={tag})"),
            &cc.bytes,
            &rc.bytes,
        );
    }
}

#[test]
fn thash() {
    let l = libs();
    let c = unsafe { l.c::<FnThash>("SPX_thash") };
    let r = unsafe { l.r::<FnThash>("SPX_thash") };
    let (cc, rc) = seeded_ctx_pair(0x33);

    let mut rng = Rng::new(0x8abc);
    // `thash` is called with 1, 2, SPX_WOTS_LEN and SPX_FORS_TREES blocks.
    let mut blocks = vec![1usize, 2, 3, SPX_WOTS_LEN, SPX_FORS_TREES];
    blocks.sort_unstable();
    blocks.dedup();
    for inblocks in blocks {
        for _ in 0..4 {
            let inp = rng.vec(inblocks * SPX_N);
            let mut addr = [0u32; 8];
            for w in addr.iter_mut() {
                *w = rng.next_u32();
            }
            let mut ca = addr;
            let mut ra = addr;
            let mut co = vec![0xAAu8; SPX_N + 8];
            let mut ro = vec![0xAAu8; SPX_N + 8];
            unsafe {
                c(
                    co.as_mut_ptr(),
                    inp.as_ptr(),
                    inblocks as c_uint,
                    cc.as_ptr(),
                    ca.as_mut_ptr(),
                );
                r(
                    ro.as_mut_ptr(),
                    inp.as_ptr(),
                    inblocks as c_uint,
                    rc.as_ptr(),
                    ra.as_mut_ptr(),
                );
            }
            assert_bytes_eq(&format!("thash(inblocks={inblocks})"), &co, &ro);
            assert_eq!(ca, ra, "thash(inblocks={inblocks}) addr side effect");
        }
    }
}

// ---------------------------------------------------------------------------
// <backend>/hash_*.c
// ---------------------------------------------------------------------------

type FnPrfAddr = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type FnGenMsgRandom = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
type FnHashMessage =
    unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8);

#[test]
fn prf_addr() {
    let l = libs();
    let c = unsafe { l.c::<FnPrfAddr>("SPX_prf_addr") };
    let r = unsafe { l.r::<FnPrfAddr>("SPX_prf_addr") };
    let (cc, rc) = seeded_ctx_pair(0x44);

    let mut rng = Rng::new(0x9bcd);
    for _ in 0..32 {
        let mut addr = [0u32; 8];
        for w in addr.iter_mut() {
            *w = rng.next_u32();
        }
        let mut co = vec![0xAAu8; SPX_N + 8];
        let mut ro = vec![0xAAu8; SPX_N + 8];
        unsafe {
            c(co.as_mut_ptr(), cc.as_ptr(), addr.as_ptr());
            r(ro.as_mut_ptr(), rc.as_ptr(), addr.as_ptr());
        }
        assert_bytes_eq("prf_addr", &co, &ro);
    }
}

#[test]
fn gen_message_random() {
    let l = libs();
    let c = unsafe { l.c::<FnGenMsgRandom>("SPX_gen_message_random") };
    let r = unsafe { l.r::<FnGenMsgRandom>("SPX_gen_message_random") };
    let (cc, rc) = seeded_ctx_pair(0x55);

    let mut rng = Rng::new(0xacde);
    for mlen in [0usize, 1, 15, 32, 63, 64, 65, 127, 128, 129, 200, 1000] {
        let sk_prf = rng.vec(SPX_N);
        let optrand = rng.vec(SPX_N);
        let m = rng.vec(mlen);
        // `hash_blake.c` hands the raw `R` pointer to `blakeX_final`, which
        // writes SPX_BLAKE{256,512}_OUTPUT_BYTES rather than SPX_N bytes.  The
        // buffer is oversized and compared in full so that the number of bytes
        // written is part of the comparison.
        let mut co = vec![0xAAu8; 128];
        let mut ro = vec![0xAAu8; 128];
        unsafe {
            c(
                co.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                cc.as_ptr(),
            );
            r(
                ro.as_mut_ptr(),
                sk_prf.as_ptr(),
                optrand.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                rc.as_ptr(),
            );
        }
        assert_bytes_eq(&format!("gen_message_random(mlen={mlen})"), &co, &ro);
    }
}

#[test]
fn hash_message() {
    let l = libs();
    let c = unsafe { l.c::<FnHashMessage>("SPX_hash_message") };
    let r = unsafe { l.r::<FnHashMessage>("SPX_hash_message") };
    let (cc, rc) = seeded_ctx_pair(0x66);

    let mut rng = Rng::new(0xbdef);
    for mlen in [0usize, 1, 31, 32, 33, 64, 100, 137, 256, 1000] {
        let rr = rng.vec(SPX_N);
        let pk = rng.vec(SPX_PK_BYTES);
        let m = rng.vec(mlen);

        let mut cd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
        let mut rd = vec![0xAAu8; SPX_FORS_MSG_BYTES + 8];
        let mut ct: u64 = 0xdead_beef_dead_beef;
        let mut rt: u64 = 0xdead_beef_dead_beef;
        let mut cl: u32 = 0xdead_beef;
        let mut rl: u32 = 0xdead_beef;
        unsafe {
            c(
                cd.as_mut_ptr(),
                &mut ct,
                &mut cl,
                rr.as_ptr(),
                pk.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                cc.as_ptr(),
            );
            r(
                rd.as_mut_ptr(),
                &mut rt,
                &mut rl,
                rr.as_ptr(),
                pk.as_ptr(),
                m.as_ptr(),
                mlen as u64,
                rc.as_ptr(),
            );
        }
        assert_bytes_eq(&format!("hash_message digest (mlen={mlen})"), &cd, &rd);
        assert_eq!(ct, rt, "hash_message tree (mlen={mlen})");
        assert_eq!(cl, rl, "hash_message leaf_idx (mlen={mlen})");
    }
}

// ---------------------------------------------------------------------------
// The parameter sizes the C library reports must match the Rust ones.
// ---------------------------------------------------------------------------

type FnSizes = unsafe extern "C" fn() -> u64;

#[test]
fn crypto_sign_sizes() {
    let l = libs();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", 3 * SPX_N as u64),
    ] {
        let c = unsafe { l.c::<FnSizes>(name) };
        let r = unsafe { l.r::<FnSizes>(name) };
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, rv, "{name}");
        assert_eq!(cv, expect, "{name} does not match the transcribed params");
    }
}

/// `mgf1` / raw hash helpers exported by the SHA-2 and BLAKE backends.
#[test]
fn backend_mgf1() {
    let l = libs();
    let names: &[&str] = match BACKEND {
        "blake" => &["SPX_blake256_mgf1", "SPX_blake512_mgf1"],
        "sha2" => &["SPX_mgf1_256", "SPX_mgf1_512"],
        _ => &[],
    };
    type FnMgf1 = unsafe extern "C" fn(*mut u8, c_ulong, *const u8, c_ulong);
    let mut rng = Rng::new(0xce01);
    for name in names {
        let c = unsafe { l.c_backend::<FnMgf1>(name) };
        let r = unsafe { l.r::<FnMgf1>(name) };
        for inlen in [1usize, 16, SPX_N + SPX_ADDR_BYTES, 2 * SPX_N + 64] {
            for outlen in [1usize, 31, 32, 33, 64, 100, 200] {
                let inp = rng.vec(inlen);
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                unsafe {
                    c(co.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                    r(ro.as_mut_ptr(), outlen as c_ulong, inp.as_ptr(), inlen as c_ulong);
                }
                assert_bytes_eq(&format!("{name}(outlen={outlen}, inlen={inlen})"), &co, &ro);
            }
        }
    }
}
