use libloading::{Library, Symbol};

const C_SO: &str = "c_src/build/libtranslated_rust.so";
const RUST_SO: &str = "target/release/libfloat2half_lib.so";

type Float2HalfFn = unsafe extern "C" fn(f32) -> u16;

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(C_SO).expect("failed to load C .so");
        let r = Library::new(RUST_SO).expect("failed to load Rust .so");
        (c, r)
    }
}

fn cmp(c: &Library, r: &Library, x: f32) {
    unsafe {
        let cf: Symbol<Float2HalfFn> = c.get(b"float2half").unwrap();
        let rf: Symbol<Float2HalfFn> = r.get(b"float2half").unwrap();
        let cv = cf(x);
        let rv = rf(x);
        assert_eq!(
            cv, rv,
            "mismatch for input {:?} (bits=0x{:08x}): C=0x{:04x} Rust=0x{:04x}",
            x,
            x.to_bits(),
            cv,
            rv
        );
    }
}

#[test]
fn test_basic_values() {
    let (c, r) = load_libs();
    let inputs: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        0.5,
        -0.5,
        65504.0,    // max half
        -65504.0,
        65520.0,    // overflows to inf
        6.103515625e-5, // min normal half
        5.9604645e-8,   // min subnormal half
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1e-10,
        1e10,
        3.14159265,
        -3.14159265,
    ];
    for &x in inputs {
        cmp(&c, &r, x);
    }
}

#[test]
fn test_subnormals() {
    let (c, r) = load_libs();
    // Sweep small magnitudes near subnormal boundary
    for i in 0..1000u32 {
        let bits = i;
        let f = f32::from_bits(bits);
        cmp(&c, &r, f);
    }
}

#[test]
fn test_random_pattern_bits() {
    let (c, r) = load_libs();
    // Walk through a deterministic stride of 32-bit patterns
    let mut bits: u32 = 0;
    for _ in 0..200_000 {
        let f = f32::from_bits(bits);
        cmp(&c, &r, f);
        bits = bits.wrapping_add(0x9E3779B1); // golden ratio stride
    }
}

#[test]
fn test_exponent_boundaries() {
    let (c, r) = load_libs();
    // For each of 512 indexes for j ((bits>>23)&0x1ff), pick representative bit patterns
    for j in 0..512u32 {
        for mantissa in [
            0u32,
            1,
            0x400000,
            0x7fffff,
            0x123456,
            0x55_5555,
        ] {
            let bits = (j << 23) | mantissa;
            let f = f32::from_bits(bits);
            cmp(&c, &r, f);
        }
    }
}
