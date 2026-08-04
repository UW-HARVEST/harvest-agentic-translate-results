use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct TflacBitwriter {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: *mut u8,
}

type BitwriterAddFn =
    unsafe extern "C" fn(*mut TflacBitwriter, u32, u64) -> c_int;

fn lib_paths() -> (String, String) {
    // Allow override via env, otherwise default to known build paths.
    let c_path = std::env::var("C_LIB_PATH").unwrap_or_else(|_| {
        format!(
            "{}/c_src/build/libtranslated_rust.so",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let rust_path = std::env::var("RUST_LIB_PATH").unwrap_or_else(|_| {
        format!(
            "{}/target/debug/libbitwriter_add_lib.so",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    (c_path, rust_path)
}

fn load_libs() -> (Library, Library) {
    let (c, r) = lib_paths();
    unsafe {
        let cl = Library::new(&c).unwrap_or_else(|e| panic!("load C lib {c}: {e}"));
        let rl =
            Library::new(&r).unwrap_or_else(|e| panic!("load Rust lib {r}: {e}"));
        (cl, rl)
    }
}

fn run_one(
    c_fn: &Symbol<BitwriterAddFn>,
    r_fn: &Symbol<BitwriterAddFn>,
    initial: &TflacBitwriter,
    bits: u32,
    val: u64,
) {
    let mut a = initial.clone();
    let mut b = initial.clone();
    let ra = unsafe { c_fn(&mut a as *mut _, bits, val) };
    let rb = unsafe { r_fn(&mut b as *mut _, bits, val) };
    assert_eq!(ra, rb, "return code mismatch for bits={bits}, val={val:#x}");
    assert_eq!(
        a.val, b.val,
        "val mismatch: c={:#x} rust={:#x} (bits={bits}, val={val:#x}, init={:?})",
        a.val, b.val, initial
    );
    assert_eq!(
        a.bits, b.bits,
        "bits mismatch: c={} rust={} (bits={bits}, val={val:#x}, init={:?})",
        a.bits, b.bits, initial
    );
    assert_eq!(
        a.pos, b.pos,
        "pos mismatch (bits={bits}, val={val:#x}, init={:?})",
        initial
    );
    assert_eq!(
        a.len, b.len,
        "len mismatch (bits={bits}, val={val:#x}, init={:?})",
        initial
    );
    assert_eq!(
        a.tot, b.tot,
        "tot mismatch: c={} rust={} (bits={bits}, val={val:#x}, init={:?})",
        a.tot, b.tot, initial
    );
}

fn make_bw(val: u64, bits: u32, pos: u32, len: u32, tot: u32) -> TflacBitwriter {
    TflacBitwriter {
        val,
        bits,
        pos,
        len,
        tot,
        buffer: std::ptr::null_mut(),
    }
}

#[test]
fn bitwriter_add_simple_cases() {
    let (cl, rl) = load_libs();
    let c_fn: Symbol<BitwriterAddFn> = unsafe { cl.get(b"bitwriter_add").unwrap() };
    let r_fn: Symbol<BitwriterAddFn> = unsafe { rl.get(b"bitwriter_add").unwrap() };

    // Simple: empty bitwriter, write a few bits.
    let init = make_bw(0, 0, 0, 0, 0);
    for bits in 1..=63u32 {
        for &val in &[0u64, 1, 0xff, 0xdead_beef, u64::MAX] {
            run_one(&c_fn, &r_fn, &init, bits, val);
        }
    }
}

#[test]
fn bitwriter_add_full_buffer_then_some() {
    let (cl, rl) = load_libs();
    let c_fn: Symbol<BitwriterAddFn> = unsafe { cl.get(b"bitwriter_add").unwrap() };
    let r_fn: Symbol<BitwriterAddFn> = unsafe { rl.get(b"bitwriter_add").unwrap() };

    // Vary initial bw.bits across the whole range.
    for init_bits in 0..=63u32 {
        for &init_val in &[0u64, 0x1, 0xa5a5_a5a5_a5a5_a5a5, 0xfffffffffffffffe] {
            let init = make_bw(init_val & ((!0u64) << 1), init_bits, 7, 9, 11);
            for bits in 1..=63u32 {
                for &val in &[0u64, 1, 0xdeadbeefcafebabe, (1u64 << 32) - 1] {
                    run_one(&c_fn, &r_fn, &init, bits, val);
                }
            }
        }
    }
}

#[test]
fn bitwriter_add_bits_64() {
    // bits=64 is a borderline case (val << (64 - 64) = no shift, while
    // val << 0 in C is well-defined).
    let (cl, rl) = load_libs();
    let c_fn: Symbol<BitwriterAddFn> = unsafe { cl.get(b"bitwriter_add").unwrap() };
    let r_fn: Symbol<BitwriterAddFn> = unsafe { rl.get(b"bitwriter_add").unwrap() };

    for init_bits in 0..=63u32 {
        for &val in &[0u64, 1, 0x8000_0000_0000_0000, u64::MAX, 0xdeadbeefcafebabe] {
            let init = make_bw(0xa5a5_a5a5_a5a5_a5a4u64, init_bits, 0, 0, 0);
            run_one(&c_fn, &r_fn, &init, 64, val);
        }
    }
}

#[test]
fn bitwriter_add_pseudorandom() {
    let (cl, rl) = load_libs();
    let c_fn: Symbol<BitwriterAddFn> = unsafe { cl.get(b"bitwriter_add").unwrap() };
    let r_fn: Symbol<BitwriterAddFn> = unsafe { rl.get(b"bitwriter_add").unwrap() };

    // Simple xorshift PRNG for deterministic coverage.
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..2000 {
        let init_val = next() & ((!0u64) << 1);
        let init_bits = (next() % 64) as u32;
        let pos = next() as u32;
        let len = next() as u32;
        let tot = next() as u32;
        let init = make_bw(init_val, init_bits, pos, len, tot);
        let bits = 1 + (next() % 63) as u32; // 1..=63
        let val = next();
        run_one(&c_fn, &r_fn, &init, bits, val);
    }
}
