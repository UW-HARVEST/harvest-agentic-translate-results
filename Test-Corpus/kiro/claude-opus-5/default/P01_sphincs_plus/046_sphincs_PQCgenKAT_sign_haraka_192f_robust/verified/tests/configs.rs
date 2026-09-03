//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Both implementations are reached only through `dlopen`/`dlsym`, so the
//! `#[no_mangle]` export wrappers are part of what is under test.

mod common;

use common::*;
use std::ffi::{c_uint, c_ulonglong};
use std::sync::atomic::{AtomicUsize, Ordering};

/* ================================================================== */
/* row 0 — the harness itself: no symbol interposition                */
/* ================================================================== */

/// The Rust `.so` and the C `.so`s export the *same* names. If the dynamic
/// linker bound the Rust library's internal calls to the C definitions, every
/// test below would degenerate into "C vs C" and pass vacuously.
///
/// This re-runs the same Rust-only fingerprint in a child process where the C
/// libraries are never loaded at all, and requires the two to be identical.
#[test]
fn cfg00_no_symbol_interposition() {
    let (libs, p) = env();
    // rs_fingerprint seeds and then consumes the DRBG, so it must not race
    // with the other tests that reseed it.
    let in_process = {
        let _g = drbg_lock();
        unsafe { rs_fingerprint(&libs.rs, p) }
    };

    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "zz_rs_only_fingerprint", "--ignored", "--nocapture"])
        .env("SPX_RS_ONLY", "1")
        .output()
        .expect("cannot re-exec the test binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let isolated = stdout
        .lines()
        .find_map(|l| l.strip_prefix("RS_FINGERPRINT="))
        .unwrap_or_else(|| panic!("child produced no fingerprint:\n{stdout}"))
        .trim();

    assert_eq!(
        in_process, isolated,
        "the Rust .so behaves differently when the C .so's are loaded \
         alongside it -- symbol interposition is corrupting the comparison"
    );
}

/// Helper for `cfg00`: opens **only** the Rust `.so` and prints its
/// fingerprint. `#[ignore]`d so it never runs as part of a normal pass.
#[test]
#[ignore]
fn zz_rs_only_fingerprint() {
    let (lib, p) = rs_only();
    println!("RS_FINGERPRINT={}", unsafe { rs_fingerprint(&lib, &p) });
}

/* ================================================================== */
/* row 1 — the four size getters                                      */
/* ================================================================== */

#[test]
fn cfg01_size_getters() {
    let (l, p) = env();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", p.sk_bytes()),
        ("crypto_sign_publickeybytes", p.pk_bytes()),
        ("crypto_sign_bytes", p.spx_bytes()),
        ("crypto_sign_seedbytes", p.seed_bytes()),
    ] {
        unsafe {
            let c: FnSizes = *l.c(name);
            let r: FnSizes = *l.r(name);
            let cv = c();
            let rv = r();
            same_u(name, cv, rv);
            assert_eq!(cv as usize, expect, "{name} disagrees with dump_params.c");
        }
    }
}

/* ================================================================== */
/* rows 2-6 — address setters / copiers                               */
/* ================================================================== */

/// Runs an `fn(addr, u32)` setter against both libraries over a value grid and
/// asserts the whole 32-byte address matches, so a write to the wrong offset or
/// a clobbered neighbouring byte is caught.
fn addr_u32_setter(name: &str, extra: &[u32]) {
    let (l, _p) = env();
    let mut rng = Rng::new(0xA00D_0000 ^ name.len() as u64);
    let mut values: Vec<u32> = vec![0, 1, 2, 6, 7, 8, 254, 255, 256, 257, 259, 0xFFFF, u32::MAX];
    values.extend_from_slice(extra);
    for _ in 0..256 {
        values.push(rng.next_u32());
    }

    unsafe {
        let cf: FnAddrU32 = *l.c(name);
        let rf: FnAddrU32 = *l.r(name);
        for base in [[0u32; 8], [0xFFFF_FFFFu32; 8], rng.addr(), rng.addr()] {
            for &v in &values {
                let mut ca = base;
                let mut ra = base;
                cf(ca.as_mut_ptr(), v);
                rf(ra.as_mut_ptr(), v);
                same(
                    &format!("{name}(base={:08x?}, {v:#x})", base[0]),
                    bytemuck_u32(&ca),
                    bytemuck_u32(&ra),
                );
            }
        }
    }
}

fn bytemuck_u32(a: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a.as_ptr() as *const u8, 32) }
}

#[test]
fn cfg02_set_layer_addr() {
    let (_l, p) = env();
    addr_u32_setter("SPX_set_layer_addr", &[p.d() as u32 - 1, p.d() as u32]);
}

#[test]
fn cfg03_set_tree_addr() {
    let (l, p) = env();
    let tree_bits = p.tree_height() * (p.d() - 1);
    let max_tree = if tree_bits >= 64 {
        u64::MAX
    } else {
        (1u64 << tree_bits) - 1
    };
    let mut rng = Rng::new(0x7EEE_0003);
    let mut values = vec![0u64, 1, 2, max_tree, max_tree.wrapping_add(1), u64::MAX];
    for _ in 0..256 {
        values.push(rng.next_u64());
    }
    unsafe {
        let cf: FnAddrU64 = *l.c("SPX_set_tree_addr");
        let rf: FnAddrU64 = *l.r("SPX_set_tree_addr");
        for base in [[0u32; 8], [0xFFFF_FFFFu32; 8], rng.addr()] {
            for &v in &values {
                let mut ca = base;
                let mut ra = base;
                cf(ca.as_mut_ptr(), v);
                rf(ra.as_mut_ptr(), v);
                same(
                    &format!("SPX_set_tree_addr({v:#x})"),
                    bytemuck_u32(&ca),
                    bytemuck_u32(&ra),
                );
            }
        }
    }
}

#[test]
fn cfg04_set_type_all_variants_and_beyond() {
    // SPX_ADDR_TYPE_WOTS..FORSPRF == 0..6 are the documented variants; the
    // parameter is a bare uint32_t, so 7.. are legal inputs the C truncates.
    addr_u32_setter("SPX_set_type", &[0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn cfg05_other_addr_setters() {
    let (_l, p) = env();
    addr_u32_setter("SPX_set_keypair_addr", &[]);
    addr_u32_setter("SPX_set_chain_addr", &[p.wots_len() as u32]);
    addr_u32_setter("SPX_set_hash_addr", &[p.wots_w() as u32 - 1, p.wots_w() as u32]);
    addr_u32_setter(
        "SPX_set_tree_height",
        &[p.tree_height() as u32, p.fors_height() as u32],
    );
    addr_u32_setter("SPX_set_tree_index", &[]);
}

#[test]
fn cfg06_addr_copiers() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x0C06);
    unsafe {
        for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
            let cf: FnAddrCopy = *l.c(name);
            let rf: FnAddrCopy = *l.r(name);
            for _ in 0..256 {
                let src = rng.addr();
                let base = rng.addr();
                let mut ca = base;
                let mut ra = base;
                cf(ca.as_mut_ptr(), src.as_ptr());
                rf(ra.as_mut_ptr(), src.as_ptr());
                same(name, bytemuck_u32(&ca), bytemuck_u32(&ra));
            }
        }
    }
}

/* ================================================================== */
/* rows 7-9, 33 — the byte/integer conversion leaves                  */
/* ================================================================== */

#[test]
fn cfg07_ull_to_bytes() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x0771);
    let mut vals = vec![0u64, 1, 0xFF, 0x100, 0xFFFF_FFFF, 0x1_0000_0000, u64::MAX];
    for _ in 0..64 {
        vals.push(rng.next_u64());
    }
    unsafe {
        let cf: FnUllToBytes = *l.c("SPX_ull_to_bytes");
        let rf: FnUllToBytes = *l.r("SPX_ull_to_bytes");
        for outlen in [0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16, 32] {
            for &v in &vals {
                // 32 guard bytes so an over-long write is detected
                let mut cb = vec![0xA5u8; outlen + 32];
                let mut rb = cb.clone();
                cf(cb.as_mut_ptr(), outlen as c_uint, v);
                rf(rb.as_mut_ptr(), outlen as c_uint, v);
                same(&format!("SPX_ull_to_bytes(outlen={outlen}, {v:#x})"), &cb, &rb);
            }
        }
    }
}

#[test]
fn cfg08_u32_to_bytes() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x0832);
    unsafe {
        let cf: FnU32ToBytes = *l.c("SPX_u32_to_bytes");
        let rf: FnU32ToBytes = *l.r("SPX_u32_to_bytes");
        let mut vals = vec![0u32, 1, 0xFF, 0x100, u32::MAX];
        for _ in 0..256 {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let mut cb = vec![0xA5u8; 4 + 16];
            let mut rb = cb.clone();
            cf(cb.as_mut_ptr(), v);
            rf(rb.as_mut_ptr(), v);
            same(&format!("SPX_u32_to_bytes({v:#x})"), &cb, &rb);
        }
    }
}

#[test]
fn cfg09_bytes_to_ull() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x0944);
    unsafe {
        let cf: FnBytesToUll = *l.c("SPX_bytes_to_ull");
        let rf: FnBytesToUll = *l.r("SPX_bytes_to_ull");
        // inlen > 8 shifts by >= 64, which is undefined behaviour in C; see
        // ERRORS.md row 31.  Only the well-defined range is byte-compared.
        for inlen in 0usize..=8 {
            for _ in 0..64 {
                let b = rng.bytes(16);
                let cv = cf(b.as_ptr(), inlen as c_uint);
                let rv = rf(b.as_ptr(), inlen as c_uint);
                same_u(&format!("SPX_bytes_to_ull(inlen={inlen})"), cv, rv);
            }
            // all-zero and all-ones
            for fill in [0x00u8, 0xFF] {
                let b = vec![fill; 16];
                same_u(
                    &format!("SPX_bytes_to_ull(inlen={inlen}, fill={fill:#x})"),
                    cf(b.as_ptr(), inlen as c_uint),
                    rf(b.as_ptr(), inlen as c_uint),
                );
            }
        }
    }
}

#[test]
fn cfg33_ull_bytes_roundtrip() {
    let (l, _p) = env();
    let mut rng = Rng::new(0x3333);
    unsafe {
        let c_to: FnUllToBytes = *l.c("SPX_ull_to_bytes");
        let r_to: FnUllToBytes = *l.r("SPX_ull_to_bytes");
        let c_from: FnBytesToUll = *l.c("SPX_bytes_to_ull");
        let r_from: FnBytesToUll = *l.r("SPX_bytes_to_ull");
        for len in 1usize..=8 {
            let mask = if len == 8 { u64::MAX } else { (1u64 << (8 * len)) - 1 };
            for _ in 0..64 {
                let v = rng.next_u64() & mask;
                let mut cb = vec![0u8; len];
                let mut rb = vec![0u8; len];
                c_to(cb.as_mut_ptr(), len as c_uint, v);
                r_to(rb.as_mut_ptr(), len as c_uint, v);
                same(&format!("roundtrip encode len={len}"), &cb, &rb);
                let cv = c_from(cb.as_ptr(), len as c_uint);
                let rv = r_from(rb.as_ptr(), len as c_uint);
                same_u(&format!("roundtrip decode len={len}"), cv, rv);
                assert_eq!(cv, v, "C round-trip lost information at len={len}");
            }
        }
    }
}

/* ================================================================== */
/* rows 10-12 — the hash hooks                                        */
/* ================================================================== */

/// Builds a context in both libraries from the same seeds and asserts the
/// resulting `sizeof(spx_ctx)` bytes are identical.  Returns the (identical)
/// context bytes for reuse by later rows.
fn init_ctx(rng: &mut Rng) -> Vec<u8> {
    let (l, p) = env();
    let n = p.n_();
    let mut cctx = vec![0u8; p.ctx_size() + 32];
    let seeds = rng.bytes(2 * n);
    cctx[..2 * n].copy_from_slice(&seeds);
    let mut rctx = cctx.clone();
    unsafe {
        let cf: FnInitHash = *l.c("SPX_initialize_hash_function");
        let rf: FnInitHash = *l.r("SPX_initialize_hash_function");
        cf(cctx.as_mut_ptr());
        rf(rctx.as_mut_ptr());
    }
    same("SPX_initialize_hash_function", &cctx, &rctx);
    cctx.truncate(p.ctx_size());
    cctx
}

#[test]
fn cfg10_initialize_hash_function() {
    let (_l, p) = env();
    let mut rng = Rng::new(0x1010);
    for _ in 0..64 {
        let ctx = init_ctx(&mut rng);
        assert_eq!(ctx.len(), p.ctx_size());
    }
    // extreme seeds
    for fill in [0x00u8, 0xFF] {
        let (l, p) = env();
        let mut cctx = vec![0u8; p.ctx_size() + 32];
        cctx[..2 * p.n_()].fill(fill);
        let mut rctx = cctx.clone();
        unsafe {
            let cf: FnInitHash = *l.c("SPX_initialize_hash_function");
            let rf: FnInitHash = *l.r("SPX_initialize_hash_function");
            cf(cctx.as_mut_ptr());
            rf(rctx.as_mut_ptr());
        }
        same(&format!("initialize_hash_function(seed={fill:#x})"), &cctx, &rctx);
    }
}

#[test]
fn cfg11_prf_addr() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x1111);
    unsafe {
        let cf: FnPrfAddr = *l.c("SPX_prf_addr");
        let rf: FnPrfAddr = *l.r("SPX_prf_addr");
        for _ in 0..200 {
            let ctx = init_ctx(&mut rng);
            let addr = rng.addr();
            let mut co = vec![0xA5u8; n + 32];
            let mut ro = co.clone();
            cf(co.as_mut_ptr(), ctx.as_ptr(), addr.as_ptr());
            rf(ro.as_mut_ptr(), ctx.as_ptr(), addr.as_ptr());
            same("SPX_prf_addr", &co, &ro);
        }
    }
}

#[test]
fn cfg12_thash_all_inblocks() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x1212);
    let mut blocks: Vec<usize> = vec![0, 1, 2, 3, 4, 16, 64, p.wots_len(), p.fors_trees()];
    blocks.sort_unstable();
    blocks.dedup();
    unsafe {
        let cf: FnThash = *l.c("SPX_thash");
        let rf: FnThash = *l.r("SPX_thash");
        for &nb in &blocks {
            for _ in 0..24 {
                let ctx = init_ctx(&mut rng);
                let addr = rng.addr();
                let input = rng.bytes(nb * n);
                let mut co = vec![0xA5u8; n + 32];
                let mut ro = co.clone();
                let mut ca = addr;
                let mut ra = addr;
                cf(
                    co.as_mut_ptr(),
                    input.as_ptr(),
                    nb as c_uint,
                    ctx.as_ptr(),
                    ca.as_mut_ptr(),
                );
                rf(
                    ro.as_mut_ptr(),
                    input.as_ptr(),
                    nb as c_uint,
                    ctx.as_ptr(),
                    ra.as_mut_ptr(),
                );
                same(&format!("SPX_thash(inblocks={nb}) out"), &co, &ro);
                same(
                    &format!("SPX_thash(inblocks={nb}) addr"),
                    bytemuck_u32(&ca),
                    bytemuck_u32(&ra),
                );
            }
        }
    }
}

/* ================================================================== */
/* rows 13-14 — message hashing, all block-boundary shapes            */
/* ================================================================== */

/// The `mlen` values the C branches on, for both possible SHA/BLAKE block
/// sizes plus generic small/large cases.
fn message_lengths(p: &Params) -> Vec<usize> {
    let n = p.n_();
    let mut v: Vec<usize> = vec![0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 1000, 5000];
    for block in [64usize, 128] {
        for delta in [-2i64, -1, 0, 1, 2] {
            // gen_message_random: branch at SPX_N + mlen == block
            let base = block as i64 - n as i64 + delta;
            if base >= 0 {
                v.push(base as usize);
            }
            // hash_message: branch at SPX_N + SPX_PK_BYTES + mlen == k*block
            for k in 1..=2usize {
                let b2 = (k * block) as i64 - n as i64 - p.pk_bytes() as i64 + delta;
                if b2 >= 0 {
                    v.push(b2 as usize);
                }
            }
            let b3 = block as i64 + delta;
            if b3 >= 0 {
                v.push(b3 as usize);
            }
            let b4 = 2 * block as i64 + delta;
            if b4 >= 0 {
                v.push(b4 as usize);
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn cfg13_gen_message_random() {
    let (l, p) = env();
    let n = p.n_();
    // NOTE on the output size: only the BLAKE backend writes the *whole* digest
    // into R -- `hash_blake.c` ends with `blakeX_final(&S, R)`, which emits 32
    // bytes (BLAKE-256) or 64 bytes (BLAKE-512, i.e. SPX_N >= 24), whereas
    // sha2/shake/haraka emit exactly SPX_N.  `sign.c` gets away with it because
    // it passes `sig`, which is SPX_BYTES long.  The buffer here is sized for
    // the largest possible write, and the *whole* buffer is compared, so the two
    // implementations must also agree on how many bytes they touch.
    let outbuf = n.max(64) + 64;
    let mut rng = Rng::new(0x1313);
    unsafe {
        let cf: FnGenMsgRandom = *l.c("SPX_gen_message_random");
        let rf: FnGenMsgRandom = *l.r("SPX_gen_message_random");
        for mlen in message_lengths(p) {
            for _ in 0..3 {
                let ctx = init_ctx(&mut rng);
                let sk_prf = rng.bytes(n);
                let optrand = rng.bytes(n);
                // hash_sha2.c's gen_message_random reads m only forward, but
                // the doc comment demands slack in front; allocate it so the
                // C implementation can never read unmapped memory.
                let mut mbuf = vec![0u8; 256 + mlen];
                rng.fill(&mut mbuf);
                let m = mbuf.as_ptr().add(256);
                let mut co = vec![0xA5u8; outbuf];
                let mut ro = co.clone();
                cf(
                    co.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m,
                    mlen as c_ulonglong,
                    ctx.as_ptr(),
                );
                rf(
                    ro.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m,
                    mlen as c_ulonglong,
                    ctx.as_ptr(),
                );
                same(&format!("SPX_gen_message_random(mlen={mlen})"), &co, &ro);
            }
        }
    }
}

#[test]
fn cfg14_hash_message() {
    let (l, p) = env();
    let n = p.n_();
    let dgst = p.fors_msg_bytes();
    let mut rng = Rng::new(0x1414);
    unsafe {
        let cf: FnHashMessage = *l.c("SPX_hash_message");
        let rf: FnHashMessage = *l.r("SPX_hash_message");
        for mlen in message_lengths(p) {
            for _ in 0..3 {
                let ctx = init_ctx(&mut rng);
                let r = rng.bytes(n);
                let pk = rng.bytes(p.pk_bytes());
                let mut mbuf = vec![0u8; 256 + mlen];
                rng.fill(&mut mbuf);
                let m = mbuf.as_ptr().add(256);

                let mut cd = vec![0xA5u8; dgst + 32];
                let mut rd = cd.clone();
                let mut ct: u64 = 0xDEAD_BEEF_DEAD_BEEF;
                let mut rt: u64 = 0xDEAD_BEEF_DEAD_BEEF;
                let mut cl: u32 = 0xDEAD_BEEF;
                let mut rl: u32 = 0xDEAD_BEEF;
                cf(
                    cd.as_mut_ptr(),
                    &mut ct,
                    &mut cl,
                    r.as_ptr(),
                    pk.as_ptr(),
                    m,
                    mlen as c_ulonglong,
                    ctx.as_ptr(),
                );
                rf(
                    rd.as_mut_ptr(),
                    &mut rt,
                    &mut rl,
                    r.as_ptr(),
                    pk.as_ptr(),
                    m,
                    mlen as c_ulonglong,
                    ctx.as_ptr(),
                );
                same(&format!("SPX_hash_message(mlen={mlen}) digest"), &cd, &rd);
                same_u(&format!("SPX_hash_message(mlen={mlen}) tree"), ct, rt);
                same_u(
                    &format!("SPX_hash_message(mlen={mlen}) leaf_idx"),
                    cl as u64,
                    rl as u64,
                );
                // the C masks leaf_idx down to SPX_TREE_HEIGHT bits
                assert!(
                    (cl as u64) < (1u64 << p.tree_height()),
                    "leaf_idx {cl} exceeds 2^TREE_HEIGHT"
                );
            }
        }
    }
}

/* ================================================================== */
/* rows 15-16 — compute_root and treehash (function-pointer entry)     */
/* ================================================================== */

#[test]
fn cfg15_compute_root_shapes() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x1515);
    let mut heights: Vec<u32> = vec![1, 2, 3, p.fors_height() as u32, p.tree_height() as u32];
    heights.sort_unstable();
    heights.dedup();
    unsafe {
        let cf: FnComputeRoot = *l.c("SPX_compute_root");
        let rf: FnComputeRoot = *l.r("SPX_compute_root");
        for &h in &heights {
            let leaf_idxs: Vec<u32> = vec![
                0,
                1,
                2,
                3,
                (1u32 << h.min(30)) - 1,
                (1u32 << h.min(30)) / 2,
                rng.next_u32() & ((1u32 << h.min(30)) - 1),
            ];
            for &leaf_idx in &leaf_idxs {
                for &idx_offset in &[0u32, 1, 2, 0xFFFF_FFFE, rng.next_u32()] {
                    let ctx = init_ctx(&mut rng);
                    let leaf = rng.bytes(n);
                    let auth = rng.bytes(h as usize * n);
                    let addr = rng.addr();
                    let mut co = vec![0xA5u8; n + 32];
                    let mut ro = co.clone();
                    let mut ca = addr;
                    let mut ra = addr;
                    cf(
                        co.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        auth.as_ptr(),
                        h,
                        ctx.as_ptr(),
                        ca.as_mut_ptr(),
                    );
                    rf(
                        ro.as_mut_ptr(),
                        leaf.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        auth.as_ptr(),
                        h,
                        ctx.as_ptr(),
                        ra.as_mut_ptr(),
                    );
                    let what = format!("SPX_compute_root(h={h}, leaf={leaf_idx}, off={idx_offset})");
                    same(&format!("{what} root"), &co, &ro);
                    same(&format!("{what} addr"), bytemuck_u32(&ca), bytemuck_u32(&ra));
                }
            }
        }
    }
}

/// `SPX_N` for the neutral `gen_leaf` callback below.
static LEAF_N: AtomicUsize = AtomicUsize::new(0);

/// A `gen_leaf` implementation that lives in the **test**, not in either
/// library, so both `treehash`es are fed identical leaves and only their own
/// traversal + `thash` differ.  It mixes in `tree_addr`, so a divergence in the
/// address state the caller has set up also shows up in the output.
unsafe extern "C" fn neutral_gen_leaf(
    leaf: *mut u8,
    _ctx: *const u8,
    addr_idx: u32,
    tree_addr: *const u32,
) {
    let n = LEAF_N.load(Ordering::Relaxed);
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ addr_idx as u64;
    for i in 0..8 {
        h = (h ^ *tree_addr.add(i) as u64).wrapping_mul(0x100_0000_01b3);
    }
    for i in 0..n {
        h = (h ^ i as u64).wrapping_mul(0x100_0000_01b3);
        *leaf.add(i) = (h >> 29) as u8;
    }
}

#[test]
fn cfg16_treehash_shapes() {
    let (l, p) = env();
    let n = p.n_();
    LEAF_N.store(n, Ordering::Relaxed);
    let mut rng = Rng::new(0x1616);
    // treehash is O(2^h); cap the height so the test stays fast.
    let mut heights: Vec<u32> = vec![1, 2, 3, (p.fors_height() as u32).min(10)];
    heights.push((p.tree_height() as u32).min(10));
    heights.sort_unstable();
    heights.dedup();
    unsafe {
        let cf: FnTreehash = *l.c("SPX_treehash");
        let rf: FnTreehash = *l.r("SPX_treehash");
        for &h in &heights {
            let last = (1u32 << h) - 1;
            for &leaf_idx in &[0u32, 1, last / 2, last, u32::MAX] {
                for &idx_offset in &[0u32, 1u32 << h, 3u32 << h, rng.next_u32()] {
                    let ctx = init_ctx(&mut rng);
                    let addr = rng.addr();
                    let mut croot = vec![0xA5u8; n + 32];
                    let mut rroot = croot.clone();
                    let mut cauth = vec![0xA5u8; h as usize * n + 32];
                    let mut rauth = cauth.clone();
                    let mut ca = addr;
                    let mut ra = addr;
                    cf(
                        croot.as_mut_ptr(),
                        cauth.as_mut_ptr(),
                        ctx.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        neutral_gen_leaf,
                        ca.as_mut_ptr(),
                    );
                    rf(
                        rroot.as_mut_ptr(),
                        rauth.as_mut_ptr(),
                        ctx.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        neutral_gen_leaf,
                        ra.as_mut_ptr(),
                    );
                    let what =
                        format!("SPX_treehash(h={h}, leaf={leaf_idx}, off={idx_offset})");
                    same(&format!("{what} root"), &croot, &rroot);
                    same(&format!("{what} auth"), &cauth, &rauth);
                    same(&format!("{what} addr"), bytemuck_u32(&ca), bytemuck_u32(&ra));
                }
            }
        }
    }
}

/* ================================================================== */
/* rows 17-18 — the x1 treehash variants (lowest-level tree entry)      */
/* ================================================================== */

/// Owns a C-layout `leaf_info_x1` (88 bytes on x86-64) plus the buffers its two
/// pointer fields refer to.
struct LeafInfo {
    raw: Vec<u8>,
    sig: Vec<u8>,
    steps: Vec<u32>,
}

impl LeafInfo {
    fn new(p: &Params, sign_leaf: u32, steps: Vec<u32>, addr: [u32; 8]) -> Self {
        let mut li = LeafInfo {
            raw: vec![0u8; p.leaf_info_size()],
            sig: vec![0xA5u8; p.wots_bytes() + 32],
            steps,
        };
        let sig_ptr = li.sig.as_mut_ptr();
        let steps_ptr = li.steps.as_mut_ptr();
        unsafe {
            let b = li.raw.as_mut_ptr();
            std::ptr::write_unaligned(b.add(0) as *mut *mut u8, sig_ptr);
            std::ptr::write_unaligned(b.add(8) as *mut u32, sign_leaf);
            std::ptr::write_unaligned(b.add(16) as *mut *mut u32, steps_ptr);
            std::ptr::copy_nonoverlapping(addr.as_ptr() as *const u8, b.add(24), 32);
            std::ptr::copy_nonoverlapping(addr.as_ptr() as *const u8, b.add(56), 32);
        }
        li
    }
    /// The struct bytes with the two pointer fields blanked, so the two
    /// libraries' copies are comparable.
    fn comparable(&self) -> Vec<u8> {
        let mut v = self.raw.clone();
        v[0..8].fill(0);
        v[16..24].fill(0);
        v
    }
}

#[test]
fn cfg17_wots_treehashx1() {
    let (l, p) = env();
    let n = p.n_();
    let h = p.tree_height() as u32;
    let mut rng = Rng::new(0x1717);
    let last = (1u32 << h) - 1;
    unsafe {
        let cf: FnTreehashX1 = *l.c("SPX_wots_treehashx1");
        let rf: FnTreehashX1 = *l.r("SPX_wots_treehashx1");
        for &leaf_idx in &[0u32, 1, last / 2, last, u32::MAX] {
            // both sides of wotsx1.c's `leaf_idx == info->wots_sign_leaf`
            for &sign_leaf in &[leaf_idx, u32::MAX, leaf_idx.wrapping_add(1)] {
                for &idx_offset in &[0u32, 1u32 << h, rng.next_u32() & !last] {
                    let ctx = init_ctx(&mut rng);
                    let addr = rng.addr();
                    let steps: Vec<u32> = (0..p.wots_len())
                        .map(|_| rng.next_u32() % p.wots_w() as u32)
                        .collect();
                    let mut ci = LeafInfo::new(p, sign_leaf, steps.clone(), addr);
                    let mut ri = LeafInfo::new(p, sign_leaf, steps, addr);
                    let mut croot = vec![0xA5u8; n + 32];
                    let mut rroot = croot.clone();
                    let mut cauth = vec![0xA5u8; h as usize * n + 32];
                    let mut rauth = cauth.clone();
                    let mut ca = addr;
                    let mut ra = addr;
                    cf(
                        croot.as_mut_ptr(),
                        cauth.as_mut_ptr(),
                        ctx.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        ca.as_mut_ptr(),
                        ci.raw.as_mut_ptr(),
                    );
                    rf(
                        rroot.as_mut_ptr(),
                        rauth.as_mut_ptr(),
                        ctx.as_ptr(),
                        leaf_idx,
                        idx_offset,
                        h,
                        ra.as_mut_ptr(),
                        ri.raw.as_mut_ptr(),
                    );
                    let what = format!(
                        "SPX_wots_treehashx1(leaf={leaf_idx}, sign_leaf={sign_leaf}, off={idx_offset})"
                    );
                    same(&format!("{what} root"), &croot, &rroot);
                    same(&format!("{what} auth"), &cauth, &rauth);
                    same(&format!("{what} addr"), bytemuck_u32(&ca), bytemuck_u32(&ra));
                    same(&format!("{what} wots_sig"), &ci.sig, &ri.sig);
                    same(&format!("{what} info"), &ci.comparable(), &ri.comparable());
                }
            }
        }
    }
}

#[test]
fn cfg18_fors_treehashx1() {
    let (l, p) = env();
    let n = p.n_();
    let h = (p.fors_height() as u32).min(10);
    let mut rng = Rng::new(0x1818);
    let last = (1u32 << h) - 1;
    unsafe {
        let cf: FnTreehashX1 = *l.c("SPX_fors_treehashx1");
        let rf: FnTreehashX1 = *l.r("SPX_fors_treehashx1");
        for &leaf_idx in &[0u32, 1, last / 2, last] {
            for i in [0u32, 1, 3] {
                let idx_offset = i * (1 << h);
                let ctx = init_ctx(&mut rng);
                let addr = rng.addr();
                // fors_treehashx1 is handed a fors_gen_leaf_info (32 bytes),
                // not a leaf_info_x1 -- exactly what fors.c does.
                let mut cinfo = vec![0u8; p.n("sizeof_fors_gen_leaf_info")];
                cinfo.copy_from_slice(bytemuck_u32(&addr));
                let mut rinfo = cinfo.clone();
                let mut croot = vec![0xA5u8; n + 32];
                let mut rroot = croot.clone();
                let mut cauth = vec![0xA5u8; h as usize * n + 32];
                let mut rauth = cauth.clone();
                let mut ca = addr;
                let mut ra = addr;
                cf(
                    croot.as_mut_ptr(),
                    cauth.as_mut_ptr(),
                    ctx.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    ca.as_mut_ptr(),
                    cinfo.as_mut_ptr(),
                );
                rf(
                    rroot.as_mut_ptr(),
                    rauth.as_mut_ptr(),
                    ctx.as_ptr(),
                    leaf_idx,
                    idx_offset,
                    h,
                    ra.as_mut_ptr(),
                    rinfo.as_mut_ptr(),
                );
                let what = format!("SPX_fors_treehashx1(leaf={leaf_idx}, off={idx_offset})");
                same(&format!("{what} root"), &croot, &rroot);
                same(&format!("{what} auth"), &cauth, &rauth);
                same(&format!("{what} addr"), bytemuck_u32(&ca), bytemuck_u32(&ra));
                same(&format!("{what} info"), &cinfo, &rinfo);
            }
        }
    }
}

/* ================================================================== */
/* rows 19-22 — WOTS / FORS leaves                                     */
/* ================================================================== */

#[test]
fn cfg19_chain_lengths() {
    let (l, p) = env();
    let len = p.wots_len();
    let mut rng = Rng::new(0x1919);
    unsafe {
        let cf: FnChainLengths = *l.c("SPX_chain_lengths");
        let rf: FnChainLengths = *l.r("SPX_chain_lengths");
        let mut msgs: Vec<Vec<u8>> = vec![vec![0u8; p.n_()], vec![0xFFu8; p.n_()]];
        for _ in 0..200 {
            msgs.push(rng.bytes(p.n_()));
        }
        for msg in msgs {
            let mut co = vec![0xAAAA_AAAAu32; len + 8];
            let mut ro = co.clone();
            cf(co.as_mut_ptr() as *mut c_uint, msg.as_ptr());
            rf(ro.as_mut_ptr() as *mut c_uint, msg.as_ptr());
            let cb =
                std::slice::from_raw_parts(co.as_ptr() as *const u8, co.len() * 4);
            let rb =
                std::slice::from_raw_parts(ro.as_ptr() as *const u8, ro.len() * 4);
            same("SPX_chain_lengths", cb, rb);
            for i in 0..len {
                assert!(
                    co[i] < p.wots_w() as u32,
                    "chain_lengths[{i}] = {} >= W",
                    co[i]
                );
            }
        }
    }
}

#[test]
fn cfg20_wots_pk_from_sig() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x2020);
    unsafe {
        let cf: FnWotsPkFromSig = *l.c("SPX_wots_pk_from_sig");
        let rf: FnWotsPkFromSig = *l.r("SPX_wots_pk_from_sig");
        // all-zero and all-ones messages drive chain lengths to the extremes
        // (0 -> gen_chain copies only, W-1 -> full chain)
        let mut msgs: Vec<Vec<u8>> = vec![vec![0u8; n], vec![0xFFu8; n]];
        for _ in 0..38 {
            msgs.push(rng.bytes(n));
        }
        for msg in msgs {
            let ctx = init_ctx(&mut rng);
            let sig = rng.bytes(p.wots_bytes());
            let addr = rng.addr();
            let mut cpk = vec![0xA5u8; p.wots_bytes() + 32];
            let mut rpk = cpk.clone();
            let mut ca = addr;
            let mut ra = addr;
            cf(
                cpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                ctx.as_ptr(),
                ca.as_mut_ptr(),
            );
            rf(
                rpk.as_mut_ptr(),
                sig.as_ptr(),
                msg.as_ptr(),
                ctx.as_ptr(),
                ra.as_mut_ptr(),
            );
            same("SPX_wots_pk_from_sig pk", &cpk, &rpk);
            same(
                "SPX_wots_pk_from_sig addr",
                bytemuck_u32(&ca),
                bytemuck_u32(&ra),
            );
        }
    }
}

#[test]
fn cfg21_wots_gen_leafx1() {
    let (l, p) = env();
    let n = p.n_();
    let w = p.wots_w() as u32;
    let mut rng = Rng::new(0x2121);
    unsafe {
        let cf: FnWotsGenLeafX1 = *l.c("SPX_wots_gen_leafx1");
        let rf: FnWotsGenLeafX1 = *l.r("SPX_wots_gen_leafx1");
        let step_sets: Vec<Vec<u32>> = vec![
            vec![0u32; p.wots_len()],
            vec![w - 1; p.wots_len()],
            (0..p.wots_len()).map(|i| (i as u32) % w).collect(),
            (0..p.wots_len()).map(|_| rng.next_u32() % w).collect(),
        ];
        for steps in step_sets {
            for &leaf_idx in &[0u32, 1, 7, u32::MAX] {
                // signing leaf (wots_k_mask = 0) and non-signing (~0)
                for &sign_leaf in &[leaf_idx, leaf_idx.wrapping_add(1), u32::MAX] {
                    let ctx = init_ctx(&mut rng);
                    let addr = rng.addr();
                    let mut ci = LeafInfo::new(p, sign_leaf, steps.clone(), addr);
                    let mut ri = LeafInfo::new(p, sign_leaf, steps.clone(), addr);
                    let mut cd = vec![0xA5u8; n + 32];
                    let mut rd = cd.clone();
                    cf(cd.as_mut_ptr(), ctx.as_ptr(), leaf_idx, ci.raw.as_mut_ptr());
                    rf(rd.as_mut_ptr(), ctx.as_ptr(), leaf_idx, ri.raw.as_mut_ptr());
                    let what =
                        format!("SPX_wots_gen_leafx1(leaf={leaf_idx}, sign_leaf={sign_leaf})");
                    same(&format!("{what} dest"), &cd, &rd);
                    same(&format!("{what} wots_sig"), &ci.sig, &ri.sig);
                    same(&format!("{what} info"), &ci.comparable(), &ri.comparable());
                }
            }
        }
    }
}

#[test]
fn cfg22_fors_gen_leafx1() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x2222);
    unsafe {
        let cf: FnForsGenLeafX1 = *l.c("SPX_fors_gen_leafx1");
        let rf: FnForsGenLeafX1 = *l.r("SPX_fors_gen_leafx1");
        let mut idxs: Vec<u32> = vec![0, 1, (1u32 << p.fors_height()) - 1, u32::MAX];
        for _ in 0..64 {
            idxs.push(rng.next_u32());
        }
        for addr_idx in idxs {
            let ctx = init_ctx(&mut rng);
            let addr = rng.addr();
            let mut cinfo = vec![0u8; p.n("sizeof_fors_gen_leaf_info")];
            cinfo.copy_from_slice(bytemuck_u32(&addr));
            let mut rinfo = cinfo.clone();
            let mut cd = vec![0xA5u8; n + 32];
            let mut rd = cd.clone();
            cf(cd.as_mut_ptr(), ctx.as_ptr(), addr_idx, cinfo.as_mut_ptr());
            rf(rd.as_mut_ptr(), ctx.as_ptr(), addr_idx, rinfo.as_mut_ptr());
            same(&format!("SPX_fors_gen_leafx1({addr_idx}) leaf"), &cd, &rd);
            same(&format!("SPX_fors_gen_leafx1({addr_idx}) info"), &cinfo, &rinfo);
        }
    }
}

/* ================================================================== */
/* rows 23-25 — FORS                                                   */
/* ================================================================== */

#[test]
fn cfg23_fors_sign() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x2323);
    unsafe {
        let cf: FnForsSign = *l.c("SPX_fors_sign");
        let rf: FnForsSign = *l.r("SPX_fors_sign");
        let mut msgs: Vec<Vec<u8>> = vec![
            vec![0u8; p.fors_msg_bytes()],
            vec![0xFFu8; p.fors_msg_bytes()],
        ];
        for _ in 0..18 {
            msgs.push(rng.bytes(p.fors_msg_bytes()));
        }
        for m in msgs {
            let ctx = init_ctx(&mut rng);
            let addr = rng.addr();
            let mut csig = vec![0xA5u8; p.fors_bytes() + 32];
            let mut rsig = csig.clone();
            let mut cpk = vec![0xA5u8; n + 32];
            let mut rpk = cpk.clone();
            cf(
                csig.as_mut_ptr(),
                cpk.as_mut_ptr(),
                m.as_ptr(),
                ctx.as_ptr(),
                addr.as_ptr(),
            );
            rf(
                rsig.as_mut_ptr(),
                rpk.as_mut_ptr(),
                m.as_ptr(),
                ctx.as_ptr(),
                addr.as_ptr(),
            );
            same("SPX_fors_sign sig", &csig, &rsig);
            same("SPX_fors_sign pk", &cpk, &rpk);
        }
    }
}

#[test]
fn cfg24_fors_pk_from_sig() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x2424);
    unsafe {
        let cf: FnForsPkFromSig = *l.c("SPX_fors_pk_from_sig");
        let rf: FnForsPkFromSig = *l.r("SPX_fors_pk_from_sig");
        for _ in 0..20 {
            let ctx = init_ctx(&mut rng);
            let addr = rng.addr();
            let sig = rng.bytes(p.fors_bytes());
            let m = rng.bytes(p.fors_msg_bytes());
            let mut cpk = vec![0xA5u8; n + 32];
            let mut rpk = cpk.clone();
            cf(
                cpk.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                ctx.as_ptr(),
                addr.as_ptr(),
            );
            rf(
                rpk.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                ctx.as_ptr(),
                addr.as_ptr(),
            );
            same("SPX_fors_pk_from_sig", &cpk, &rpk);
        }
    }
}

#[test]
fn cfg25_fors_sign_then_recover() {
    // Composed pipeline: sign with one library, recover with the other.  A bug
    // that shifts a pointer by the same amount in both halves of one library is
    // invisible to per-function tests but shows up here.
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x2525);
    unsafe {
        let c_sign: FnForsSign = *l.c("SPX_fors_sign");
        let r_sign: FnForsSign = *l.r("SPX_fors_sign");
        let c_rec: FnForsPkFromSig = *l.c("SPX_fors_pk_from_sig");
        let r_rec: FnForsPkFromSig = *l.r("SPX_fors_pk_from_sig");
        for _ in 0..10 {
            let ctx = init_ctx(&mut rng);
            let addr = rng.addr();
            let m = rng.bytes(p.fors_msg_bytes());

            let mut csig = vec![0u8; p.fors_bytes()];
            let mut rsig = vec![0u8; p.fors_bytes()];
            let mut cpk = vec![0u8; n];
            let mut rpk = vec![0u8; n];
            c_sign(csig.as_mut_ptr(), cpk.as_mut_ptr(), m.as_ptr(), ctx.as_ptr(), addr.as_ptr());
            r_sign(rsig.as_mut_ptr(), rpk.as_mut_ptr(), m.as_ptr(), ctx.as_ptr(), addr.as_ptr());
            same("fors_sign sig (pipeline)", &csig, &rsig);

            // cross: C signature recovered by Rust and vice versa
            let mut a = vec![0u8; n];
            let mut b = vec![0u8; n];
            r_rec(a.as_mut_ptr(), csig.as_ptr(), m.as_ptr(), ctx.as_ptr(), addr.as_ptr());
            c_rec(b.as_mut_ptr(), rsig.as_ptr(), m.as_ptr(), ctx.as_ptr(), addr.as_ptr());
            same("rust recovers C fors signature", &cpk, &a);
            same("C recovers rust fors signature", &rpk, &b);
        }
    }
}

/* ================================================================== */
/* rows 26-27 — Merkle                                                 */
/* ================================================================== */

#[test]
fn cfg26_merkle_sign() {
    let (l, p) = env();
    let n = p.n_();
    let h = p.tree_height();
    let mut rng = Rng::new(0x2626);
    let sig_len = p.wots_bytes() + h * n;
    unsafe {
        let cf: FnMerkleSign = *l.c("SPX_merkle_sign");
        let rf: FnMerkleSign = *l.r("SPX_merkle_sign");
        let last = (1u32 << h) - 1;
        for &idx_leaf in &[0u32, 1, last / 2, last, u32::MAX] {
            for _ in 0..3 {
                let ctx = init_ctx(&mut rng);
                let root = rng.bytes(n);
                let wots_addr = rng.addr();
                let tree_addr = rng.addr();
                let mut csig = vec![0xA5u8; sig_len + 32];
                let mut rsig = csig.clone();
                let mut croot = root.clone();
                let mut rroot = root.clone();
                let (mut cw, mut rw) = (wots_addr, wots_addr);
                let (mut ct, mut rt) = (tree_addr, tree_addr);
                cf(
                    csig.as_mut_ptr(),
                    croot.as_mut_ptr(),
                    ctx.as_ptr(),
                    cw.as_mut_ptr(),
                    ct.as_mut_ptr(),
                    idx_leaf,
                );
                rf(
                    rsig.as_mut_ptr(),
                    rroot.as_mut_ptr(),
                    ctx.as_ptr(),
                    rw.as_mut_ptr(),
                    rt.as_mut_ptr(),
                    idx_leaf,
                );
                let what = format!("SPX_merkle_sign(idx_leaf={idx_leaf})");
                same(&format!("{what} sig"), &csig, &rsig);
                same(&format!("{what} root"), &croot, &rroot);
                same(&format!("{what} wots_addr"), bytemuck_u32(&cw), bytemuck_u32(&rw));
                same(&format!("{what} tree_addr"), bytemuck_u32(&ct), bytemuck_u32(&rt));
            }
        }
    }
}

#[test]
fn cfg27_merkle_gen_root() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0x2727);
    unsafe {
        let cf: FnMerkleGenRoot = *l.c("SPX_merkle_gen_root");
        let rf: FnMerkleGenRoot = *l.r("SPX_merkle_gen_root");
        for _ in 0..4 {
            let ctx = init_ctx(&mut rng);
            let mut co = vec![0xA5u8; n + 32];
            let mut ro = co.clone();
            cf(co.as_mut_ptr(), ctx.as_ptr());
            rf(ro.as_mut_ptr(), ctx.as_ptr());
            same("SPX_merkle_gen_root", &co, &ro);
        }
    }
}

/* ================================================================== */
/* rows 28-32 — the public signing API                                 */
/* ================================================================== */

#[test]
fn cfg28_seed_keypair() {
    let (l, p) = env();
    let mut rng = Rng::new(0x2828);
    unsafe {
        let cf: FnSeedKeypair = *l.c("crypto_sign_seed_keypair");
        let rf: FnSeedKeypair = *l.r("crypto_sign_seed_keypair");
        let mut seeds: Vec<Vec<u8>> = vec![
            vec![0u8; p.seed_bytes()],
            vec![0xFFu8; p.seed_bytes()],
            (0..p.seed_bytes()).map(|i| i as u8).collect(),
        ];
        for _ in 0..8 {
            seeds.push(rng.bytes(p.seed_bytes()));
        }
        for seed in seeds {
            let mut cpk = vec![0xA5u8; p.pk_bytes() + 32];
            let mut csk = vec![0xA5u8; p.sk_bytes() + 32];
            let mut rpk = cpk.clone();
            let mut rsk = csk.clone();
            let cr = cf(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let rr = rf(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            same_i("crypto_sign_seed_keypair ret", cr, rr);
            assert_eq!(cr, 0);
            same("crypto_sign_seed_keypair pk", &cpk, &rpk);
            same("crypto_sign_seed_keypair sk", &csk, &rsk);
        }
    }
}

/// Re-seeds both DRBGs from the same 48-byte entropy input and returns the
/// (identical) `DRBG_ctx` bytes.  Caller must hold the `DRBG` lock.
unsafe fn reseed_both(entropy: &mut [u8; 48], pers: Option<&mut [u8; 48]>) {
    let (l, _p) = env();
    let ci: FnRandombytesInit = *l.c("randombytes_init");
    let ri: FnRandombytesInit = *l.r("randombytes_init");
    match pers {
        Some(ps) => {
            ci(entropy.as_mut_ptr(), ps.as_mut_ptr());
            ri(entropy.as_mut_ptr(), ps.as_mut_ptr());
        }
        None => {
            ci(entropy.as_mut_ptr(), std::ptr::null_mut());
            ri(entropy.as_mut_ptr(), std::ptr::null_mut());
        }
    }
    let cd = std::slice::from_raw_parts(l.c_data("DRBG_ctx"), 52);
    let rd = std::slice::from_raw_parts(l.r_data("DRBG_ctx"), 52);
    same("DRBG_ctx after randombytes_init", cd, rd);
}

#[test]
fn cfg29_keypair_from_drbg() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0x2929);
    unsafe {
        let cf: FnKeypair = *l.c("crypto_sign_keypair");
        let rf: FnKeypair = *l.r("crypto_sign_keypair");
        for _ in 0..4 {
            let mut e: [u8; 48] = rng.bytes(48).try_into().unwrap();
            let mut cpk = vec![0xA5u8; p.pk_bytes() + 32];
            let mut csk = vec![0xA5u8; p.sk_bytes() + 32];
            let mut rpk = cpk.clone();
            let mut rsk = csk.clone();

            // each library draws its seed from its own DRBG, so both must be
            // re-seeded to the same state immediately before the call
            reseed_both(&mut e, None);
            let cr = cf(cpk.as_mut_ptr(), csk.as_mut_ptr());
            let cstate = std::slice::from_raw_parts(l.c_data("DRBG_ctx"), 52).to_vec();

            reseed_both(&mut e, None);
            let rr = rf(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            let rstate = std::slice::from_raw_parts(l.r_data("DRBG_ctx"), 52).to_vec();

            same_i("crypto_sign_keypair ret", cr, rr);
            same("crypto_sign_keypair pk", &cpk, &rpk);
            same("crypto_sign_keypair sk", &csk, &rsk);
            same("DRBG_ctx after crypto_sign_keypair", &cstate, &rstate);
        }
    }
}

fn api_message_lengths() -> Vec<usize> {
    vec![0, 1, 2, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000, 5000]
}

#[test]
fn cfg30_signature_and_verify() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0x3030);
    unsafe {
        let c_sig: FnSignature = *l.c("crypto_sign_signature");
        let r_sig: FnSignature = *l.r("crypto_sign_signature");
        let c_ver: FnVerify = *l.c("crypto_sign_verify");
        let r_ver: FnVerify = *l.r("crypto_sign_verify");
        let c_kp: FnSeedKeypair = *l.c("crypto_sign_seed_keypair");

        let seed = rng.bytes(p.seed_bytes());
        let mut pk = vec![0u8; p.pk_bytes()];
        let mut sk = vec![0u8; p.sk_bytes()];
        c_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());

        for mlen in api_message_lengths() {
            let m = rng.bytes(mlen);
            let mut e: [u8; 48] = rng.bytes(48).try_into().unwrap();

            // crypto_sign_signature draws optrand from randombytes
            reseed_both(&mut e, None);
            let mut csig = vec![0xA5u8; p.spx_bytes() + 32];
            let mut cl: usize = usize::MAX;
            let cr = c_sig(csig.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, sk.as_ptr());

            reseed_both(&mut e, None);
            let mut rsig = vec![0xA5u8; p.spx_bytes() + 32];
            let mut rl: usize = usize::MAX;
            let rr = r_sig(rsig.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, sk.as_ptr());

            same_i(&format!("crypto_sign_signature(mlen={mlen}) ret"), cr, rr);
            assert_eq!(cl, p.spx_bytes(), "siglen");
            assert_eq!(cl, rl, "siglen mismatch");
            same(&format!("crypto_sign_signature(mlen={mlen})"), &csig, &rsig);

            // row 32: cross-library verification
            for (who, sig) in [("C sig", &csig), ("Rust sig", &rsig)] {
                let cv = c_ver(sig.as_ptr(), cl, m.as_ptr(), mlen, pk.as_ptr());
                let rv = r_ver(sig.as_ptr(), cl, m.as_ptr(), mlen, pk.as_ptr());
                same_i(&format!("verify {who} (mlen={mlen})"), cv, rv);
                assert_eq!(cv, 0, "valid signature rejected: {who}, mlen={mlen}");
            }
        }
    }
}

#[test]
fn cfg31_sign_and_open() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0x3131);
    unsafe {
        let c_sign: FnSign = *l.c("crypto_sign");
        let r_sign: FnSign = *l.r("crypto_sign");
        let c_open: FnOpen = *l.c("crypto_sign_open");
        let r_open: FnOpen = *l.r("crypto_sign_open");
        let c_kp: FnSeedKeypair = *l.c("crypto_sign_seed_keypair");

        let seed = rng.bytes(p.seed_bytes());
        let mut pk = vec![0u8; p.pk_bytes()];
        let mut sk = vec![0u8; p.sk_bytes()];
        c_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());

        for mlen in api_message_lengths() {
            let m = rng.bytes(mlen);
            let mut e: [u8; 48] = rng.bytes(48).try_into().unwrap();
            let smlen_expect = (p.spx_bytes() + mlen) as u64;

            reseed_both(&mut e, None);
            let mut csm = vec![0xA5u8; p.spx_bytes() + mlen + 32];
            let mut csl: c_ulonglong = 0;
            let cr = c_sign(
                csm.as_mut_ptr(),
                &mut csl,
                m.as_ptr(),
                mlen as c_ulonglong,
                sk.as_ptr(),
            );

            reseed_both(&mut e, None);
            let mut rsm = vec![0xA5u8; p.spx_bytes() + mlen + 32];
            let mut rsl: c_ulonglong = 0;
            let rr = r_sign(
                rsm.as_mut_ptr(),
                &mut rsl,
                m.as_ptr(),
                mlen as c_ulonglong,
                sk.as_ptr(),
            );

            same_i(&format!("crypto_sign(mlen={mlen}) ret"), cr, rr);
            same_u(&format!("crypto_sign(mlen={mlen}) smlen"), csl, rsl);
            assert_eq!(csl, smlen_expect);
            same(&format!("crypto_sign(mlen={mlen}) sm"), &csm, &rsm);

            for (who, sm) in [("C sm", &csm), ("Rust sm", &rsm)] {
                let mut cm = vec![0xA5u8; p.spx_bytes() + mlen + 32];
                let mut rm = cm.clone();
                let mut cml: c_ulonglong = u64::MAX;
                let mut rml: c_ulonglong = u64::MAX;
                let cv = c_open(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), csl, pk.as_ptr());
                let rv = r_open(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), csl, pk.as_ptr());
                same_i(&format!("open {who} (mlen={mlen}) ret"), cv, rv);
                assert_eq!(cv, 0, "valid sm rejected: {who}");
                same_u(&format!("open {who} (mlen={mlen}) mlen"), cml, rml);
                assert_eq!(cml as usize, mlen);
                same(&format!("open {who} (mlen={mlen}) m"), &cm, &rm);
                assert_eq!(&cm[..mlen], &m[..], "recovered message differs");
            }
        }
    }
}
