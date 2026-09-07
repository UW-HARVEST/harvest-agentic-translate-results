//! CONFIGS.md rows 1-9: `utils.c` scalar helpers and every `address.c` setter.

mod common;
use common::*;

type FUll = unsafe extern "C" fn(*mut u8, u32, u64);
type FU32 = unsafe extern "C" fn(*mut u8, u32);
type FB2U = unsafe extern "C" fn(*const u8, u32) -> u64;
type FAddrU32 = unsafe extern "C" fn(*mut u32, u32);
type FAddrU64 = unsafe extern "C" fn(*mut u32, u64);
type FCopy = unsafe extern "C" fn(*mut u32, *const u32);

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

/// row 1 -- `SPX_ull_to_bytes`, every `outlen` incl. 0
#[test]
fn row01_ull_to_bytes() {
    let p = libs();
    let cf = f!(p.c, "SPX_ull_to_bytes", FUll);
    let rf = f!(p.rust, "SPX_ull_to_bytes", FUll);
    let mut rng = Rng::new(SEED);

    for &outlen in &[0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16] {
        for iter in 0..32 {
            let v = match iter {
                0 => 0u64,
                1 => u64::MAX,
                2 => 1,
                3 => 0xFF,
                4 => 0x100,
                _ => rng.next_u64(),
            };
            // pre-fill with a marker so untouched bytes are compared too
            let mut a = vec![0xA5u8; 24];
            let mut b = vec![0xA5u8; 24];
            unsafe {
                cf(a.as_mut_ptr(), outlen, v);
                rf(b.as_mut_ptr(), outlen, v);
            }
            eq_bytes(&format!("ull_to_bytes(outlen={outlen},in={v:#x})"), &a, &b);
        }
    }
}

/// row 2 -- `SPX_u32_to_bytes`
#[test]
fn row02_u32_to_bytes() {
    let p = libs();
    let cf = f!(p.c, "SPX_u32_to_bytes", FU32);
    let rf = f!(p.rust, "SPX_u32_to_bytes", FU32);
    let mut rng = Rng::new(SEED ^ 2);

    for iter in 0..64 {
        let v = match iter {
            0 => 0u32,
            1 => u32::MAX,
            2 => 1,
            3 => 0x8000_0000,
            _ => rng.next_u32(),
        };
        let mut a = vec![0x5Au8; 8];
        let mut b = vec![0x5Au8; 8];
        unsafe {
            cf(a.as_mut_ptr(), v);
            rf(b.as_mut_ptr(), v);
        }
        eq_bytes(&format!("u32_to_bytes({v:#x})"), &a, &b);
    }
}

/// row 3 -- `SPX_bytes_to_ull`, `inlen` 0..=8
#[test]
fn row03_bytes_to_ull() {
    let p = libs();
    let cf = f!(p.c, "SPX_bytes_to_ull", FB2U);
    let rf = f!(p.rust, "SPX_bytes_to_ull", FB2U);
    let mut rng = Rng::new(SEED ^ 3);

    for inlen in 0u32..=8 {
        for iter in 0..32 {
            let mut buf = rng.bytes(8);
            match iter {
                0 => buf.iter_mut().for_each(|x| *x = 0),
                1 => buf.iter_mut().for_each(|x| *x = 0xFF),
                _ => {}
            }
            let (x, y) = unsafe { (cf(buf.as_ptr(), inlen), rf(buf.as_ptr(), inlen)) };
            eq(&format!("bytes_to_ull(inlen={inlen},{})", hex(&buf)), x, y);
        }
    }
}

/// Runs a `void f(uint32_t addr[8], uint32_t v)` setter over many values.
fn addr_setter_u32(name: &str, values: &[u32], salt: u64) {
    let p = libs();
    let cf = f!(p.c, name, FAddrU32);
    let rf = f!(p.rust, name, FAddrU32);
    let mut rng = Rng::new(SEED ^ salt);

    for &v in values {
        for _ in 0..8 {
            let mut a = [0u32; 8];
            let mut b;
            for w in a.iter_mut() {
                *w = rng.next_u32();
            }
            b = a;
            unsafe {
                cf(a.as_mut_ptr(), v);
                rf(b.as_mut_ptr(), v);
            }
            eq(&format!("{name}({v:#x})"), a, b);
        }
    }
}

/// row 4 -- `SPX_set_layer_addr`
#[test]
fn row04_set_layer_addr() {
    let mut vals: Vec<u32> = vec![0, 1, (SPX_D - 1) as u32, 255, 256, 0x1FF, 0xFFFF_FFFF];
    let mut rng = Rng::new(SEED ^ 41);
    for _ in 0..8 {
        vals.push(rng.next_u32());
    }
    addr_setter_u32("SPX_set_layer_addr", &vals, 4);
}

/// row 5 -- `SPX_set_tree_addr` (64-bit argument)
#[test]
fn row05_set_tree_addr() {
    let p = libs();
    let cf = f!(p.c, "SPX_set_tree_addr", FAddrU64);
    let rf = f!(p.rust, "SPX_set_tree_addr", FAddrU64);
    let mut rng = Rng::new(SEED ^ 5);

    let mut vals: Vec<u64> = vec![0, 1, 0xFF, 0x1_0000_0000, 1u64 << 56, u64::MAX];
    for _ in 0..16 {
        vals.push(rng.next_u64());
    }
    for &v in &vals {
        let mut a = [0u32; 8];
        for w in a.iter_mut() {
            *w = rng.next_u32();
        }
        let mut b = a;
        unsafe {
            cf(a.as_mut_ptr(), v);
            rf(b.as_mut_ptr(), v);
        }
        eq(&format!("set_tree_addr({v:#x})"), a, b);
    }
}

/// row 6 -- `SPX_set_type` including out-of-range "enum" values
#[test]
fn row06_set_type() {
    let vals: Vec<u32> = vec![
        0, 1, 2, 3, 4, 5, 6, // all valid SPX_ADDR_TYPE_*
        7, 8, 100, 255, 256, 0x103, 0xFFFF_FFFF,
    ];
    addr_setter_u32("SPX_set_type", &vals, 6);
}

/// row 7 -- the 4-byte big-endian fields
#[test]
fn row07_u32_fields() {
    let mut vals: Vec<u32> = vec![0, 1, 0xFF, 0x100, 0xFFFF, 0xFFFF_FFFF];
    let mut rng = Rng::new(SEED ^ 71);
    for _ in 0..16 {
        vals.push(rng.next_u32());
    }
    addr_setter_u32("SPX_set_keypair_addr", &vals, 7);
    addr_setter_u32("SPX_set_tree_index", &vals, 8);
}

/// row 8 -- the single-byte fields, incl. values >= 256
#[test]
fn row08_byte_fields() {
    let mut vals: Vec<u32> = vec![
        0,
        1,
        SPX_WOTS_LEN as u32,
        (SPX_WOTS_W - 1) as u32,
        SPX_FULL_HEIGHT as u32,
        255,
        256,
        257,
        0xFFFF_FFFF,
    ];
    let mut rng = Rng::new(SEED ^ 81);
    for _ in 0..8 {
        vals.push(rng.next_u32());
    }
    addr_setter_u32("SPX_set_chain_addr", &vals, 9);
    addr_setter_u32("SPX_set_hash_addr", &vals, 10);
    addr_setter_u32("SPX_set_tree_height", &vals, 11);
}

/// row 9 -- the two partial-copy helpers (also checks the untouched bytes)
#[test]
fn row09_copy_addr() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 9);
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        let cf = f!(p.c, name, FCopy);
        let rf = f!(p.rust, name, FCopy);
        for _ in 0..64 {
            let mut inp = [0u32; 8];
            for w in inp.iter_mut() {
                *w = rng.next_u32();
            }
            let mut a = [0u32; 8];
            for w in a.iter_mut() {
                *w = rng.next_u32();
            }
            let mut b = a;
            unsafe {
                cf(a.as_mut_ptr(), inp.as_ptr());
                rf(b.as_mut_ptr(), inp.as_ptr());
            }
            eq(name, a, b);
        }
    }
}
