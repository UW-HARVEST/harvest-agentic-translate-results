//! Phase C — error-path differential tests.  One test per row of ERRORS.md
//! (the rng.c rows live in `diff_rng.rs`), plus the generic FFI boundaries:
//! zero / oversized lengths, values one step past a documented range, and
//! out-of-range enum values crossing the FFI boundary.

mod common;
use common::*;
use std::ffi::c_void;

type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Signature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type SignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type Thash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const c_void, *mut u32);
type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;
type Set1 = unsafe extern "C" fn(*mut u32, u32);
type PrfAddr = unsafe extern "C" fn(*mut u8, *const c_void, *const u32);
type GenLeaf = unsafe extern "C" fn(*mut u8, *const c_void, u32, *const u32);
type Treehash =
    unsafe extern "C" fn(*mut u8, *mut u8, *const c_void, u32, u32, u32, GenLeaf, *mut u32);

/// Builds a valid (pk, sk, message, signature) quadruple.
struct Kat {
    pk: Vec<u8>,
    #[allow(dead_code)]
    sk: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
}

fn make_kat(libs: &Libs, mlen: usize, seed_byte: u8) -> Kat {
    let _g = drbg_lock();
    let kp = libs.c::<SeedKeypair>("crypto_sign_seed_keypair");
    let sg = libs.c::<Signature>("crypto_sign_signature");
    let ri = libs.c::<RandombytesInit>("randombytes_init");
    let mut ent = vec![seed_byte; 48];
    unsafe { ri(ent.as_mut_ptr(), std::ptr::null_mut()) };

    let seed = vec![seed_byte; SEED_BYTES];
    let mut pk = vec![0u8; PK_BYTES];
    let mut sk = vec![0u8; SK_BYTES];
    unsafe { kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
    let mut rng = Rng::new(0x2000 + seed_byte as u64);
    let m = rng.bytes(mlen);
    let mut sig = vec![0u8; SPX_BYTES];
    let mut siglen = 0usize;
    unsafe { sg(sig.as_mut_ptr(), &mut siglen, m.as_ptr(), mlen, sk.as_ptr()) };
    assert_eq!(siglen, SPX_BYTES);
    Kat { pk, sk, m, sig }
}

// ==================================================================
// sign.c:179  `crypto_sign_verify`: siglen != SPX_BYTES  ->  -1
// ==================================================================
#[test]
fn err_verify_wrong_siglen() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<Verify>("crypto_sign_verify");
    let k = make_kat(&libs, 33, 0x01);

    let mut lens: Vec<usize> = vec![0, 1, SPX_BYTES - 1, SPX_BYTES + 1, 2 * SPX_BYTES];
    lens.push(usize::MAX);
    for siglen in lens {
        let cv = unsafe { c(k.sig.as_ptr(), siglen, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        let rv = unsafe { r(k.sig.as_ptr(), siglen, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        assert_eq!(cv, -1, "C must reject siglen={}", siglen);
        assert_eq!(rv, cv, "crypto_sign_verify(siglen={})", siglen);
    }
    // the exact length is accepted
    let cv = unsafe { c(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
    let rv = unsafe { r(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
    assert_eq!((cv, rv), (0, 0), "the correct siglen must be accepted");
}

// ==================================================================
// sign.c:235  `crypto_sign_verify`: recomputed root != pk root  ->  -1
// ==================================================================
#[test]
fn err_verify_root_mismatch() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<Verify>("crypto_sign_verify");
    let k = make_kat(&libs, 33, 0x02);
    let mut rng = Rng::new(0x2100);

    // (a) flip one bit of the signature, at many different offsets
    for &off in &[
        0usize,
        N - 1,
        N,
        N + 1,
        N + FORS_BYTES - 1,
        N + FORS_BYTES,
        SPX_BYTES / 2,
        SPX_BYTES - 1,
    ] {
        let mut bad = k.sig.clone();
        bad[off] ^= 0x01;
        let cv = unsafe { c(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        let rv = unsafe { r(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        assert_eq!(cv, -1, "C must reject a signature corrupted at {}", off);
        assert_eq!(rv, cv, "corrupted signature at offset {}", off);
    }

    // (b) fully random signature
    for _ in 0..3 {
        let bad = rng.bytes(SPX_BYTES);
        let cv = unsafe { c(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        let rv = unsafe { r(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        assert_eq!(cv, -1);
        assert_eq!(rv, cv, "random signature");
    }

    // (c) wrong message.
    //
    // NOTE: whether the C *rejects* a modified message depends on the backend.
    // `hash_blake.c` calls `blakeX_update` with byte counts where the routine
    // expects BIT counts, so for short messages the BLAKE state never reaches a
    // full block, `blakeX_final` performs no compression at all and the message
    // has no influence on the digest.  That is the reference behaviour, so the
    // assertion here is that C and Rust *agree*, and it is checked separately
    // (below) that the error path itself is reached.
    let mut m2 = k.m.clone();
    m2[0] ^= 0x80;
    let cv = unsafe { c(k.sig.as_ptr(), SPX_BYTES, m2.as_ptr(), m2.len(), k.pk.as_ptr()) };
    let rv = unsafe { r(k.sig.as_ptr(), SPX_BYTES, m2.as_ptr(), m2.len(), k.pk.as_ptr()) };
    assert_eq!(rv, cv, "modified message");

    let cv = unsafe { c(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len() - 1, k.pk.as_ptr()) };
    let rv = unsafe { r(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len() - 1, k.pk.as_ptr()) };
    assert_eq!(rv, cv, "truncated message");

    // A long message, where every backend really does absorb the message, so
    // the rejection path is guaranteed to be taken.
    {
        let k2 = make_kat(&libs, 4096, 0x12);
        let mut m3 = k2.m.clone();
        m3[0] ^= 0x01;
        let cv = unsafe { c(k2.sig.as_ptr(), SPX_BYTES, m3.as_ptr(), m3.len(), k2.pk.as_ptr()) };
        let rv = unsafe { r(k2.sig.as_ptr(), SPX_BYTES, m3.as_ptr(), m3.len(), k2.pk.as_ptr()) };
        assert_eq!(cv, -1, "C must reject a modified long message");
        assert_eq!(rv, cv, "modified long message");
        let cv = unsafe { c(k2.sig.as_ptr(), SPX_BYTES, k2.m.as_ptr(), k2.m.len(), k2.pk.as_ptr()) };
        let rv = unsafe { r(k2.sig.as_ptr(), SPX_BYTES, k2.m.as_ptr(), k2.m.len(), k2.pk.as_ptr()) };
        assert_eq!((cv, rv), (0, 0), "the intact long message must verify");
    }

    // (d) wrong public key: bad root half, then bad seed half
    for off in [0usize, N] {
        let mut pk2 = k.pk.clone();
        pk2[off] ^= 0x01;
        let cv = unsafe { c(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), pk2.as_ptr()) };
        let rv = unsafe { r(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), pk2.as_ptr()) };
        assert_eq!(cv, -1, "C must reject pk corrupted at {}", off);
        assert_eq!(rv, cv, "corrupted pk at {}", off);
    }

    // (e) mlen == 0 against a signature over a non-empty message
    let cv = unsafe { c(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), 0, k.pk.as_ptr()) };
    let rv = unsafe { r(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), 0, k.pk.as_ptr()) };
    assert_eq!(rv, cv, "mlen=0");
}

// ==================================================================
// sign.c:269  `crypto_sign_open`: smlen < SPX_BYTES
//   -> memset(m, 0, smlen); *mlen = 0; return -1
// ==================================================================
#[test]
fn err_sign_open_short_smlen() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<SignOpen>("crypto_sign_open");
    let k = make_kat(&libs, 33, 0x03);

    for smlen in [0u64, 1, 2, (SPX_BYTES / 2) as u64, (SPX_BYTES - 1) as u64] {
        let mut cm = vec![0xA5u8; SPX_BYTES + 64];
        let mut rm = vec![0xA5u8; SPX_BYTES + 64];
        let mut cml = 0xDEAD_BEEFu64;
        let mut rml = 0xDEAD_BEEFu64;
        let cv = unsafe { c(cm.as_mut_ptr(), &mut cml, k.sig.as_ptr(), smlen, k.pk.as_ptr()) };
        let rv = unsafe { r(rm.as_mut_ptr(), &mut rml, k.sig.as_ptr(), smlen, k.pk.as_ptr()) };
        assert_eq!(cv, -1, "C must reject smlen={}", smlen);
        assert_eq!(rv, cv, "crypto_sign_open(smlen={}) rc", smlen);
        assert_eq!(cml, 0, "C sets *mlen = 0");
        assert_eq!(rml, cml, "crypto_sign_open(smlen={}) *mlen", smlen);
        // the C zeroes exactly `smlen` bytes of m and leaves the rest alone
        assert_bytes_eq(&format!("m after short smlen={}", smlen), &cm, &rm);
        assert!(cm[..smlen as usize].iter().all(|&b| b == 0));
        assert!(cm[smlen as usize..].iter().all(|&b| b == 0xA5));
    }
}

// ==================================================================
// sign.c:277  `crypto_sign_open`: crypto_sign_verify fails
//   -> memset(m, 0, smlen); *mlen = 0; return -1
// ==================================================================
#[test]
fn err_sign_open_bad_signature() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<SignOpen>("crypto_sign_open");
    let k = make_kat(&libs, 33, 0x04);
    let mlen = k.m.len();

    let mut sm = vec![0u8; SPX_BYTES + mlen];
    sm[..SPX_BYTES].copy_from_slice(&k.sig);
    sm[SPX_BYTES..].copy_from_slice(&k.m);

    // sanity: the untouched sm opens successfully in both
    {
        let mut cm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut rm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut cml = 0u64;
        let mut rml = 0u64;
        let smlen = (SPX_BYTES + mlen) as u64;
        let cv = unsafe { c(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), smlen, k.pk.as_ptr()) };
        let rv = unsafe { r(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), smlen, k.pk.as_ptr()) };
        assert_eq!((cv, rv), (0, 0));
        assert_eq!((cml, rml), (mlen as u64, mlen as u64));
        assert_bytes_eq("crypto_sign_open success m", &cm, &rm);
    }

    // Corrupting the signature part always makes verification fail; corrupting
    // the *message* part only does so for backends/lengths where the message
    // actually reaches the hash (see the note in err_verify_root_mismatch), so
    // the message offsets only assert C/Rust agreement.
    let mut saw_reject = false;
    for &off in &[0usize, 1, N, SPX_BYTES - 1, SPX_BYTES, SPX_BYTES + mlen - 1] {
        let in_sig = off < SPX_BYTES;
        let mut bad = sm.clone();
        bad[off] ^= 0x01;
        let smlen = (SPX_BYTES + mlen) as u64;
        let mut cm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut rm = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut cml = 0xDEAD_BEEFu64;
        let mut rml = 0xDEAD_BEEFu64;
        let cv = unsafe { c(cm.as_mut_ptr(), &mut cml, bad.as_ptr(), smlen, k.pk.as_ptr()) };
        let rv = unsafe { r(rm.as_mut_ptr(), &mut rml, bad.as_ptr(), smlen, k.pk.as_ptr()) };
        if in_sig {
            assert_eq!(cv, -1, "C must reject sm corrupted at {}", off);
            saw_reject = true;
        }
        assert_eq!(rv, cv, "crypto_sign_open corrupted at {} rc", off);
        assert_eq!(rml, cml, "crypto_sign_open corrupted at {} *mlen", off);
        assert_bytes_eq(&format!("m after failure at {}", off), &cm, &rm);
        if cv == -1 {
            assert_eq!(cml, 0, "C sets *mlen = 0 on failure");
            assert!(cm[..smlen as usize].iter().all(|&b| b == 0));
            assert!(cm[smlen as usize..].iter().all(|&b| b == 0xA5));
        }
    }
    assert!(saw_reject, "the rejection path was never exercised");

    // wrong pk
    let mut pk2 = k.pk.clone();
    pk2[N] ^= 0x40;
    let smlen = (SPX_BYTES + mlen) as u64;
    let mut cm = vec![0xA5u8; SPX_BYTES + mlen + 8];
    let mut rm = vec![0xA5u8; SPX_BYTES + mlen + 8];
    let mut cml = 1u64;
    let mut rml = 1u64;
    let cv = unsafe { c(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), smlen, pk2.as_ptr()) };
    let rv = unsafe { r(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), smlen, pk2.as_ptr()) };
    assert_eq!((cv, rv), (-1, -1));
    assert_eq!((cml, rml), (0, 0));
    assert_bytes_eq("m after wrong-pk failure", &cm, &rm);
}

// `crypto_sign_open` with smlen == SPX_BYTES exactly (mlen becomes 0) — the
// boundary immediately above the rejection threshold.
#[test]
fn sign_open_smlen_exactly_spx_bytes() {
    let _g = drbg_lock();
    let libs = Libs::load();
    let (co, ro) = libs.pair::<SignOpen>("crypto_sign_open");
    let kp = libs.c::<SeedKeypair>("crypto_sign_seed_keypair");
    let sg = libs.c::<Signature>("crypto_sign_signature");
    let ri = libs.c::<RandombytesInit>("randombytes_init");

    let mut ent = vec![0x77u8; 48];
    unsafe { ri(ent.as_mut_ptr(), std::ptr::null_mut()) };
    let seed = vec![0x66u8; SEED_BYTES];
    let mut pk = vec![0u8; PK_BYTES];
    let mut sk = vec![0u8; SK_BYTES];
    unsafe { kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
    let mut sig = vec![0u8; SPX_BYTES];
    let mut siglen = 0usize;
    // sign the EMPTY message, so sm == sig and smlen == SPX_BYTES
    unsafe { sg(sig.as_mut_ptr(), &mut siglen, std::ptr::null(), 0, sk.as_ptr()) };

    let mut cm = vec![0xA5u8; SPX_BYTES + 8];
    let mut rm = vec![0xA5u8; SPX_BYTES + 8];
    let mut cml = 0xDEADu64;
    let mut rml = 0xDEADu64;
    let cv = unsafe { co(cm.as_mut_ptr(), &mut cml, sig.as_ptr(), SPX_BYTES as u64, pk.as_ptr()) };
    let rv = unsafe { ro(rm.as_mut_ptr(), &mut rml, sig.as_ptr(), SPX_BYTES as u64, pk.as_ptr()) };
    assert_eq!(cv, rv, "crypto_sign_open(smlen == SPX_BYTES) rc");
    assert_eq!(cml, rml, "crypto_sign_open(smlen == SPX_BYTES) *mlen");
    assert_bytes_eq("m at smlen == SPX_BYTES", &cm, &rm);
}

// ==================================================================
// Generic FFI boundaries
// ==================================================================

/// `thash` with `inblocks == 0` (the C builds a zero-length input region) and
/// with the largest count the library ever uses.
#[test]
fn boundary_thash_inblocks() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(0x2200);
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let biggest = std::cmp::max(WOTS_LEN, FORS_TREES) as u32;
    for nb in [0u32, 1, 2, biggest, biggest + 1] {
        let inp = rng.bytes(std::cmp::max(nb as usize, 1) * N);
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut co = vec![0xEEu8; N + 16];
        let mut ro = vec![0xEEu8; N + 16];
        unsafe {
            c(co.as_mut_ptr(), inp.as_ptr(), nb, cc.as_ptr(), ca.as_mut_ptr());
            r(ro.as_mut_ptr(), inp.as_ptr(), nb, rc.as_ptr(), ra.as_mut_ptr());
        }
        assert_bytes_eq(&format!("SPX_thash(inblocks={})", nb), &co, &ro);
        assert_eq!(ca, ra);
    }
}

/// `ull_to_bytes(out, 0, v)` must write nothing; `bytes_to_ull(in, 0)` must
/// return 0.  Also the largest in-range width (8).
#[test]
fn boundary_zero_and_max_lengths() {
    let libs = Libs::load();
    let (cu, ru) = libs.pair::<UllToBytes>("SPX_ull_to_bytes");
    let (cb, rb) = libs.pair::<BytesToUll>("SPX_bytes_to_ull");
    for v in [0u64, 1, u64::MAX, 0x0123_4567_89ab_cdef] {
        let mut co = vec![0x5Au8; 16];
        let mut ro = vec![0x5Au8; 16];
        unsafe {
            cu(co.as_mut_ptr(), 0, v);
            ru(ro.as_mut_ptr(), 0, v);
        }
        assert_bytes_eq("ull_to_bytes(outlen=0)", &co, &ro);
        assert!(co.iter().all(|&b| b == 0x5A), "outlen=0 must write nothing");
    }
    let data = vec![0xffu8; 16];
    for inlen in [0u32, 1, 8] {
        let cv = unsafe { cb(data.as_ptr(), inlen) };
        let rv = unsafe { rb(data.as_ptr(), inlen) };
        assert_eq!(cv, rv, "bytes_to_ull(inlen={})", inlen);
    }
    assert_eq!(unsafe { cb(data.as_ptr(), 0) }, 0);
}

/// Out-of-range "enum" values across the FFI boundary: `set_type` takes the
/// `SPX_ADDR_TYPE_*` constants (0..=6) but is a plain `uint32_t`, so any int is
/// a real input.  The C truncates it to one byte; the whole address, and a
/// following `prf_addr`/`thash`, must agree.
#[test]
fn boundary_out_of_range_addr_type() {
    let libs = Libs::load();
    let (cst, rst) = libs.pair::<Set1>("SPX_set_type");
    let (cp, rp) = libs.pair::<PrfAddr>("SPX_prf_addr");
    let (ct, rt) = libs.pair::<Thash>("SPX_thash");
    let mut rng = Rng::new(0x2300);
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    let mut types: Vec<u32> = vec![
        0, 1, 2, 3, 4, 5, 6, // the documented variants
        7, 8, 9, 100, 255, 256, 257, 0x1_0006, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff,
    ];
    for _ in 0..64 {
        types.push(rng.next_u32());
    }

    for ty in types {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            cst(ca.as_mut_ptr(), ty);
            rst(ra.as_mut_ptr(), ty);
        }
        assert_eq!(ca, ra, "SPX_set_type({:#x}) address bytes", ty);

        let mut co = vec![0xEEu8; N + 16];
        let mut ro = vec![0xEEu8; N + 16];
        unsafe {
            cp(co.as_mut_ptr(), cc.as_ptr(), ca.as_ptr());
            rp(ro.as_mut_ptr(), rc.as_ptr(), ra.as_ptr());
        }
        assert_bytes_eq(&format!("prf_addr with type={:#x}", ty), &co, &ro);

        let inp = rng.bytes(2 * N);
        let mut ca2 = ca;
        let mut ra2 = ra;
        let mut co = vec![0xEEu8; N + 16];
        let mut ro = vec![0xEEu8; N + 16];
        unsafe {
            ct(co.as_mut_ptr(), inp.as_ptr(), 2, cc.as_ptr(), ca2.as_mut_ptr());
            rt(ro.as_mut_ptr(), inp.as_ptr(), 2, rc.as_ptr(), ra2.as_mut_ptr());
        }
        assert_bytes_eq(&format!("thash with type={:#x}", ty), &co, &ro);
    }
}

/// Every other address setter is also a `uint32_t` that the C truncates to one
/// byte (layer / chain / hash / tree_height); values past the byte range are
/// valid FFI inputs.
#[test]
fn boundary_out_of_range_addr_fields() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x2400);
    for name in [
        "SPX_set_layer_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
    ] {
        let (c, r) = libs.pair::<Set1>(name);
        let mut vals: Vec<u32> = vec![
            0, 1, 254, 255, 256, 257, 511, 512, 0xffff, 0x1_0000, 0xffff_ff00, 0xffff_ffff,
        ];
        for _ in 0..64 {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            assert_eq!(ca, ra, "{}({:#x})", name, v);
        }
    }
}

/// `treehash` with `tree_height == 0`: the C loop runs exactly once
/// (`1 << 0 == 1`) and no `thash` is performed.
#[test]
fn boundary_treehash_zero_height() {
    unsafe extern "C" fn gl(leaf: *mut u8, _c: *const c_void, idx: u32, a: *const u32) {
        let out = std::slice::from_raw_parts_mut(leaf, N);
        let addr = std::slice::from_raw_parts(a, 8);
        for (i, b) in out.iter_mut().enumerate() {
            *b = (idx as u8) ^ (i as u8) ^ (addr[i % 8] as u8);
        }
    }

    let libs = Libs::load();
    let (c, r) = libs.pair::<Treehash>("SPX_treehash");
    let mut rng = Rng::new(0x2500);
    let ps = rng.bytes(N);
    let ss = rng.bytes(N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);

    for leaf_idx in [0u32, 1, 0xffff_ffff] {
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut croot = vec![0xEEu8; N + 16];
        let mut rroot = vec![0xEEu8; N + 16];
        let mut cauth = vec![0xEEu8; N + 16];
        let mut rauth = vec![0xEEu8; N + 16];
        unsafe {
            c(croot.as_mut_ptr(), cauth.as_mut_ptr(), cc.as_ptr(), leaf_idx, 0, 0, gl, ca.as_mut_ptr());
            r(rroot.as_mut_ptr(), rauth.as_mut_ptr(), rc.as_ptr(), leaf_idx, 0, 0, gl, ra.as_mut_ptr());
        }
        assert_bytes_eq("treehash(h=0) root", &croot, &rroot);
        assert_bytes_eq("treehash(h=0) auth", &cauth, &rauth);
        assert_eq!(ca, ra);
    }
}

/// `crypto_sign_signature` / `crypto_sign` with `mlen == 0` (and a NULL message
/// pointer, which the C never dereferences for a zero length).
#[test]
fn boundary_zero_length_message() {
    let _g = drbg_lock();
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<RandombytesInit>("randombytes_init");
    let (cs, rs) = libs.pair::<Signature>("crypto_sign_signature");
    let (cv, rv) = libs.pair::<Verify>("crypto_sign_verify");
    let kp = libs.c::<SeedKeypair>("crypto_sign_seed_keypair");

    let seed = vec![0x5Cu8; SEED_BYTES];
    let mut pk = vec![0u8; PK_BYTES];
    let mut sk = vec![0u8; SK_BYTES];
    unsafe { kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    let mut ent = vec![0x0Fu8; 48];
    let mut csig = vec![0xEEu8; SPX_BYTES + 8];
    let mut rsig = vec![0xEEu8; SPX_BYTES + 8];
    let mut cl = 0usize;
    let mut rl = 0usize;
    unsafe {
        ci(ent.as_mut_ptr(), std::ptr::null_mut());
        let crc = cs(csig.as_mut_ptr(), &mut cl, std::ptr::null(), 0, sk.as_ptr());
        ri(ent.as_mut_ptr(), std::ptr::null_mut());
        let rrc = rs(rsig.as_mut_ptr(), &mut rl, std::ptr::null(), 0, sk.as_ptr());
        assert_eq!(crc, rrc);
    }
    assert_eq!(cl, rl);
    assert_bytes_eq("signature over the empty message", &csig, &rsig);

    let a = unsafe { cv(csig.as_ptr(), SPX_BYTES, std::ptr::null(), 0, pk.as_ptr()) };
    let b = unsafe { rv(csig.as_ptr(), SPX_BYTES, std::ptr::null(), 0, pk.as_ptr()) };
    assert_eq!((a, b), (0, 0), "empty-message signature must verify in both");
}
