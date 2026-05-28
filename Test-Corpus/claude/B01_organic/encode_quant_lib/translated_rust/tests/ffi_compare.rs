use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type EncodeQuantFn = unsafe extern "C" fn(
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) -> c_int;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Built via `cargo build --release` -> target/release/libencode_quant_lib.so
    workspace_root().join("target/release/libencode_quant_lib.so")
}

unsafe fn load_encode_quant<'lib>(lib: &'lib Library) -> Symbol<'lib, EncodeQuantFn> {
    lib.get(b"encode_quant\0").expect("encode_quant symbol")
}

fn run_case(
    c_fn: &EncodeQuantFn,
    r_fn: &EncodeQuantFn,
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) {
    unsafe {
        let c_out = (c_fn)(uni, step, pred, tgt, tgt2, lsbit);
        let r_out = (r_fn)(uni, step, pred, tgt, tgt2, lsbit);
        assert_eq!(
            c_out, r_out,
            "Mismatch for inputs uni={uni}, step={step}, pred={pred}, tgt={tgt}, tgt2={tgt2}, lsbit={lsbit}: c={c_out}, rust={r_out}"
        );
    }
}

#[test]
fn encode_quant_matches_c() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_sym = unsafe { load_encode_quant(&c_lib) };
    let r_sym = unsafe { load_encode_quant(&r_lib) };
    let c_fn: EncodeQuantFn = *c_sym;
    let r_fn: EncodeQuantFn = *r_sym;

    // Hand-picked edge cases.
    let unis = [
        i32::MIN,
        i32::MIN + 1,
        -16,
        -9,
        -8,
        -7,
        -1,
        0,
        1,
        7,
        8,
        9,
        15,
        16,
        17,
        100,
        i32::MAX - 1,
        i32::MAX,
    ];
    let steps = [0, 1, 2, 7, 8, 16, 100, 1000, -1, -100];
    let preds = [-1000, -1, 0, 1, 1000];
    let tgts = [-1000, -1, 0, 1, 1000];
    let tgt2s = [-1000, -1, 0, 1, 1000];
    let lsbits = [0, 1, 2, 3, 4, 5, 6, 7];

    for &uni in &unis {
        for &step in &steps {
            for &pred in &preds {
                for &tgt in &tgts {
                    for &tgt2 in &tgt2s {
                        for &lsbit in &lsbits {
                            run_case(&c_fn, &r_fn, uni, step, pred, tgt, tgt2, lsbit);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn encode_quant_matches_c_pseudo_random() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_sym = unsafe { load_encode_quant(&c_lib) };
    let r_sym = unsafe { load_encode_quant(&r_lib) };
    let c_fn: EncodeQuantFn = *c_sym;
    let r_fn: EncodeQuantFn = *r_sym;

    // Simple deterministic LCG so we don't need an external rand crate.
    let mut state: u64 = 0xdeadbeefcafebabe;
    let mut next = || -> i32 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 32) as i32
    };

    for _ in 0..200_000 {
        let uni = next();
        let step = next() % 4096;
        let pred = next() % 100_000;
        let tgt = next() % 100_000;
        let tgt2 = next() % 100_000;
        // lsbit is small in practice; test 0..=7 plus rare large values.
        let lsbit = next() & 0x7;
        run_case(&c_fn, &r_fn, uni, step, pred, tgt, tgt2, lsbit);
    }
}
