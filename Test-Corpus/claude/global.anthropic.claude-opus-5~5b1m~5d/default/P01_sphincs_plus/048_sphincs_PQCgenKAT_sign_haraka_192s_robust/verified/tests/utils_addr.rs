//! Phase B rows 1-16: the size accessors, `utils.c` byte converters and the
//! whole of `address.c`, driven through both `.so`s.

mod common;
use common::*;
use libloading::Symbol;

// ---------------------------------------------------------------------------
// Row 1 -- the four size accessors
// ---------------------------------------------------------------------------
#[test]
fn row01_size_accessors() {
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let l = libs();
        let c: Symbol<unsafe extern "C" fn() -> u64> = sym(&l.c, name);
        let r: Symbol<unsafe extern "C" fn() -> u64> = sym(&l.r, name);
        let (cv, rv) = unsafe { (c(), r()) };
        eq(&format!("{name}() C-vs-Rust"), cv, rv);
        // Also pin the value against the params header transcribed in common.
        eq(&format!("{name}() vs params header"), cv, expect);
    }
}

// ---------------------------------------------------------------------------
// Row 2 -- SPX_ull_to_bytes
// ---------------------------------------------------------------------------
#[test]
fn row02_ull_to_bytes() {
    let (c, r) = both!(
        "SPX_ull_to_bytes",
        unsafe extern "C" fn(*mut u8, u32, u64)
    );
    let mut rng = Rng::new(RNG_SEED ^ 2);
    let mut vals: Vec<u64> = vec![0, 1, 0xFF, 0x100, 0x0102_0304_0506_0708, u64::MAX];
    for _ in 0..N_ITER {
        vals.push(rng.next_u64());
    }
    for &outlen in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16] {
        for &v in &vals {
            // Prefill with a marker so we also verify nothing outside
            // out[0..outlen] is touched.
            let mut cb = vec![0xA5u8; outlen + 8];
            let mut rb = vec![0xA5u8; outlen + 8];
            unsafe {
                c(cb.as_mut_ptr(), outlen as u32, v);
                r(rb.as_mut_ptr(), outlen as u32, v);
            }
            eq_bytes(&format!("ull_to_bytes(outlen={outlen}, in={v:#x})"), &cb, &rb);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 -- SPX_u32_to_bytes
// ---------------------------------------------------------------------------
#[test]
fn row03_u32_to_bytes() {
    let (c, r) = both!("SPX_u32_to_bytes", unsafe extern "C" fn(*mut u8, u32));
    let mut rng = Rng::new(RNG_SEED ^ 3);
    let mut vals: Vec<u32> = vec![0, 1, 0xFF, 0x100, 0x0102_0304, u32::MAX];
    for _ in 0..N_ITER {
        vals.push(rng.next_u32());
    }
    for &v in &vals {
        let mut cb = [0xA5u8; 12];
        let mut rb = [0xA5u8; 12];
        unsafe {
            c(cb.as_mut_ptr(), v);
            r(rb.as_mut_ptr(), v);
        }
        eq_bytes(&format!("u32_to_bytes({v:#x})"), &cb, &rb);
    }
}

// ---------------------------------------------------------------------------
// Row 4 -- SPX_bytes_to_ull  (incl. inlen > 8, where the C shift overflows)
// ---------------------------------------------------------------------------
#[test]
fn row04_bytes_to_ull() {
    let (c, r) = both!(
        "SPX_bytes_to_ull",
        unsafe extern "C" fn(*const u8, u32) -> u64
    );
    let mut rng = Rng::new(RNG_SEED ^ 4);
    for &inlen in &[0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16] {
        let mut inputs: Vec<Vec<u8>> = vec![vec![0x00; inlen.max(1)], vec![0xFF; inlen.max(1)]];
        for _ in 0..N_ITER {
            inputs.push(rng.bytes(inlen.max(1)));
        }
        for inp in &inputs {
            let (cv, rv) = unsafe {
                (
                    c(inp.as_ptr(), inlen as u32),
                    r(inp.as_ptr(), inlen as u32),
                )
            };
            eq(
                &format!("bytes_to_ull(inlen={inlen}, in={})", hex(inp)),
                cv,
                rv,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5 -- ull_to_bytes / bytes_to_ull round trip
// ---------------------------------------------------------------------------
#[test]
fn row05_ull_roundtrip() {
    let (cu, ru) = both!(
        "SPX_ull_to_bytes",
        unsafe extern "C" fn(*mut u8, u32, u64)
    );
    let (cb2, rb2) = both!(
        "SPX_bytes_to_ull",
        unsafe extern "C" fn(*const u8, u32) -> u64
    );
    let mut rng = Rng::new(RNG_SEED ^ 5);
    for len in 1usize..=8 {
        for _ in 0..N_ITER {
            let v = rng.next_u64() >> (64 - 8 * len as u32);
            let mut cbuf = vec![0u8; len];
            let mut rbuf = vec![0u8; len];
            unsafe {
                cu(cbuf.as_mut_ptr(), len as u32, v);
                ru(rbuf.as_mut_ptr(), len as u32, v);
            }
            eq_bytes(&format!("roundtrip encode len={len}"), &cbuf, &rbuf);
            let (cv, rv) =
                unsafe { (cb2(cbuf.as_ptr(), len as u32), rb2(rbuf.as_ptr(), len as u32)) };
            eq(&format!("roundtrip decode len={len}"), cv, rv);
            eq(&format!("roundtrip value len={len}"), cv, v);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 6-13 -- the single-field address setters
// ---------------------------------------------------------------------------

/// Drive a `void f(uint32_t addr[8], u32)` setter with a random address and a
/// list of field values; the whole 32-byte address is compared afterwards.
fn check_setter_u32(name: &str, vals: &[u32], seed: u64) {
    let l = libs();
    let c: Symbol<unsafe extern "C" fn(*mut u32, u32)> = sym(&l.c, name);
    let r: Symbol<unsafe extern "C" fn(*mut u32, u32)> = sym(&l.r, name);
    let mut rng = Rng::new(seed);
    for iter in 0..N_ITER {
        let base = if iter == 0 {
            [0u32; 8]
        } else if iter == 1 {
            [u32::MAX; 8]
        } else {
            rng.addr()
        };
        for &v in vals {
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            eq_bytes(
                &format!("{name}(base={base:08X?}, v={v:#x})"),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }
}

pub fn addr_bytes(a: &[u32; 8]) -> Vec<u8> {
    // The C code casts uint32_t[8] to unsigned char* -- so this is just the
    // native byte image of the array.
    let mut v = Vec::with_capacity(32);
    for w in a {
        v.extend_from_slice(&w.to_ne_bytes());
    }
    v
}

#[test]
fn row06_set_layer_addr() {
    check_setter_u32(
        "SPX_set_layer_addr",
        &[0, 1, 6, 7, 16, 21, 22, 254, 255, 256, 0x1FF, 0xFFFF, u32::MAX],
        RNG_SEED ^ 6,
    );
}

#[test]
fn row07_set_tree_addr() {
    let (c, r) = both!("SPX_set_tree_addr", unsafe extern "C" fn(*mut u32, u64));
    let mut rng = Rng::new(RNG_SEED ^ 7);
    let mut vals: Vec<u64> = vec![
        0,
        1,
        0xFF,
        0x1_0000_0000,
        1u64 << 62,
        1u64 << 63,
        u64::MAX,
    ];
    for _ in 0..N_ITER {
        vals.push(rng.next_u64());
    }
    for iter in 0..N_ITER {
        let base = if iter == 0 { [0u32; 8] } else { rng.addr() };
        for &v in &vals {
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            eq_bytes(
                &format!("set_tree_addr(v={v:#x})"),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }
}

#[test]
fn row08_set_type_all_valid() {
    check_setter_u32("SPX_set_type", &ADDR_TYPES, RNG_SEED ^ 8);
}

#[test]
fn row09_set_keypair_addr() {
    check_setter_u32(
        "SPX_set_keypair_addr",
        &[0, 1, 0xFF, 0x100, 0xFFFF, 0x1_0000, u32::MAX],
        RNG_SEED ^ 9,
    );
}

#[test]
fn row10_set_chain_addr() {
    check_setter_u32(
        "SPX_set_chain_addr",
        &[
            0,
            1,
            15,
            34,
            (SPX_WOTS_LEN - 1) as u32,
            SPX_WOTS_LEN as u32,
            66,
            67,
            255,
            256,
            u32::MAX,
        ],
        RNG_SEED ^ 10,
    );
}

#[test]
fn row11_set_hash_addr() {
    check_setter_u32(
        "SPX_set_hash_addr",
        &[0, 1, 14, 15, (SPX_WOTS_W - 1) as u32, 255, 256, u32::MAX],
        RNG_SEED ^ 11,
    );
}

#[test]
fn row12_set_tree_height() {
    check_setter_u32(
        "SPX_set_tree_height",
        &[
            0,
            1,
            2,
            3,
            SPX_TREE_HEIGHT as u32,
            SPX_FORS_HEIGHT as u32,
            14,
            255,
            256,
            u32::MAX,
        ],
        RNG_SEED ^ 12,
    );
}

#[test]
fn row13_set_tree_index() {
    check_setter_u32(
        "SPX_set_tree_index",
        &[0, 1, 1 << 13, 1 << 20, 0xFFFF, u32::MAX],
        RNG_SEED ^ 13,
    );
}

// ---------------------------------------------------------------------------
// Rows 14-15 -- the two address copy helpers
// ---------------------------------------------------------------------------

fn check_copy(name: &str, seed: u64) {
    let l = libs();
    let c: Symbol<unsafe extern "C" fn(*mut u32, *const u32)> = sym(&l.c, name);
    let r: Symbol<unsafe extern "C" fn(*mut u32, *const u32)> = sym(&l.r, name);
    let mut rng = Rng::new(seed);
    for iter in 0..N_ITER + 2 {
        let (inp, base) = match iter {
            0 => ([0u32; 8], [u32::MAX; 8]),
            1 => ([u32::MAX; 8], [0u32; 8]),
            _ => (rng.addr(), rng.addr()),
        };
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), inp.as_ptr());
            r(ra.as_mut_ptr(), inp.as_ptr());
        }
        eq_bytes(
            &format!("{name}(in={inp:08X?}, out={base:08X?})"),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
    }
}

#[test]
fn row14_copy_subtree_addr() {
    check_copy("SPX_copy_subtree_addr", RNG_SEED ^ 14);
}

#[test]
fn row15_copy_keypair_addr() {
    check_copy("SPX_copy_keypair_addr", RNG_SEED ^ 15);
}

// ---------------------------------------------------------------------------
// Row 16 -- all setters composed (catches offset aliasing, e.g. CHAIN_ADDR 27
// and TREE_HGT 27 are the SAME byte on the non-sha2 backends)
// ---------------------------------------------------------------------------
#[test]
fn row16_all_setters_composed() {
    let l = libs();
    macro_rules! pair32 {
        ($n:literal) => {{
            let c: Symbol<unsafe extern "C" fn(*mut u32, u32)> = sym(&l.c, $n);
            let r: Symbol<unsafe extern "C" fn(*mut u32, u32)> = sym(&l.r, $n);
            (c, r)
        }};
    }
    let (c_layer, r_layer) = pair32!("SPX_set_layer_addr");
    let (c_type, r_type) = pair32!("SPX_set_type");
    let (c_kp, r_kp) = pair32!("SPX_set_keypair_addr");
    let (c_chain, r_chain) = pair32!("SPX_set_chain_addr");
    let (c_hash, r_hash) = pair32!("SPX_set_hash_addr");
    let (c_hgt, r_hgt) = pair32!("SPX_set_tree_height");
    let (c_idx, r_idx) = pair32!("SPX_set_tree_index");
    let c_tree: Symbol<unsafe extern "C" fn(*mut u32, u64)> = sym(&l.c, "SPX_set_tree_addr");
    let r_tree: Symbol<unsafe extern "C" fn(*mut u32, u64)> = sym(&l.r, "SPX_set_tree_addr");

    let mut rng = Rng::new(RNG_SEED ^ 16);
    for _ in 0..N_ITER * 4 {
        let base = rng.addr();
        let layer = rng.next_u32();
        let tree = rng.next_u64();
        let ty = ADDR_TYPES[rng.below(7) as usize];
        let kp = rng.next_u32();
        let chain = rng.next_u32();
        let h = rng.next_u32();
        let hgt = rng.next_u32();
        let idx = rng.next_u32();

        let mut ca = base;
        let mut ra = base;
        unsafe {
            c_layer(ca.as_mut_ptr(), layer);
            c_tree(ca.as_mut_ptr(), tree);
            c_type(ca.as_mut_ptr(), ty);
            c_kp(ca.as_mut_ptr(), kp);
            c_chain(ca.as_mut_ptr(), chain);
            c_hash(ca.as_mut_ptr(), h);
            c_hgt(ca.as_mut_ptr(), hgt);
            c_idx(ca.as_mut_ptr(), idx);

            r_layer(ra.as_mut_ptr(), layer);
            r_tree(ra.as_mut_ptr(), tree);
            r_type(ra.as_mut_ptr(), ty);
            r_kp(ra.as_mut_ptr(), kp);
            r_chain(ra.as_mut_ptr(), chain);
            r_hash(ra.as_mut_ptr(), h);
            r_hgt(ra.as_mut_ptr(), hgt);
            r_idx(ra.as_mut_ptr(), idx);
        }
        eq_bytes("composed setters", &addr_bytes(&ca), &addr_bytes(&ra));
    }
}
