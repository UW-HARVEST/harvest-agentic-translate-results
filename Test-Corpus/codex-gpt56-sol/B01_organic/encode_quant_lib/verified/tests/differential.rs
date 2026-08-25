use libloading::Library;
use std::ffi::c_int;
use std::path::Path;

type EncodeQuant = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

struct ComparisonLibraries {
    _c_library: Library,
    _rust_library: Library,
    c_encode_quant: EncodeQuant,
    rust_encode_quant: EncodeQuant,
}

impl ComparisonLibraries {
    fn load() -> Self {
        let c_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so");
        let test_executable = std::env::current_exe().expect("locate test executable");
        let profile_dir = test_executable
            .parent()
            .and_then(Path::parent)
            .expect("locate Cargo profile directory");
        let rust_path = profile_dir.join(format!(
            "{}encode_quant_lib{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));

        assert_library_exists(&c_path);
        assert_library_exists(&rust_path);

        // Symbols are copied as function pointers while both libraries remain
        // owned by this structure for the duration of every call.
        unsafe {
            let c_library = Library::new(&c_path).expect("load C shared library");
            let rust_library = Library::new(&rust_path).expect("load Rust shared library");
            let c_encode_quant = *c_library
                .get::<EncodeQuant>(b"encode_quant\0")
                .expect("load C encode_quant");
            let rust_encode_quant = *rust_library
                .get::<EncodeQuant>(b"encode_quant\0")
                .expect("load Rust encode_quant");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_encode_quant,
                rust_encode_quant,
            }
        }
    }

    fn compare(&self, args: [c_int; 6], context: &str) {
        unsafe {
            let c_result =
                (self.c_encode_quant)(args[0], args[1], args[2], args[3], args[4], args[5]);
            let rust_result =
                (self.rust_encode_quant)(args[0], args[1], args[2], args[3], args[4], args[5]);
            assert_eq!(c_result, rust_result, "{context}: encode_quant({args:?})");
        }
    }
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn next_i32(&mut self) -> i32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as i32
    }
}

fn lsbit_representatives(mode: usize) -> &'static [i32] {
    match mode {
        0 => &[0],
        1 => &[4],
        2 => &[1, -1, 3, i32::MIN + 1, i32::MAX],
        3 => &[2, -2, 6, i32::MIN, i32::MAX - 1],
        _ => unreachable!("four lsbit modes"),
    }
}

fn uni_for_shape(high_bits: i32, residue: i32, quant_sign: i32) -> i32 {
    (high_bits & !15) | quant_sign | residue
}

#[test]
fn all_configuration_rows_match_across_ffi() {
    const RANDOM_CASES_PER_REPRESENTATIVE: usize = 2_048;
    const SEED: u64 = 0xd1ff_e2e0_5eed_1234;
    const HIGH_BIT_BOUNDARIES: [i32; 5] = [0, 16, -16, i32::MIN, i32::MAX & !15];
    const SCALAR_BOUNDARIES: [[i32; 4]; 10] = [
        [0, 0, 0, 0],
        [1, 1, 1, 1],
        [-1, -1, -1, -1],
        [8, 0, 0, 0],
        [-8, 0, 0, 0],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [i32::MIN, i32::MAX, i32::MIN, i32::MAX],
        [i32::MIN, -1, 0, i32::MAX],
    ];

    let libraries = ComparisonLibraries::load();
    let mut rng = XorShift64(SEED);
    let mut row = 0;

    for mode in 0..4 {
        for quant_sign in [0, 8] {
            for residue in 0..8 {
                row += 1;

                for &lsbit in lsbit_representatives(mode) {
                    for &high_bits in &HIGH_BIT_BOUNDARIES {
                        let uni = uni_for_shape(high_bits, residue, quant_sign);
                        for [step, pred, tgt, tgt2] in SCALAR_BOUNDARIES {
                            libraries.compare(
                                [uni, step, pred, tgt, tgt2, lsbit],
                                &format!("CONFIGS.md row {row}, boundary case"),
                            );
                        }
                    }

                    for case in 0..RANDOM_CASES_PER_REPRESENTATIVE {
                        let uni = uni_for_shape(rng.next_i32(), residue, quant_sign);
                        libraries.compare(
                            [
                                uni,
                                rng.next_i32(),
                                rng.next_i32(),
                                rng.next_i32(),
                                rng.next_i32(),
                                lsbit,
                            ],
                            &format!("CONFIGS.md row {row}, randomized case {case}"),
                        );
                    }
                }
            }
        }
    }

    assert_eq!(row, 64, "every CONFIGS.md runtime row must run");
}

#[test]
fn scalar_argument_extrema_match_across_ffi() {
    let libraries = ComparisonLibraries::load();
    let baseline = [5, 17, -23, 41, -59, 0];

    for argument in 0..baseline.len() {
        for boundary in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            let mut args = baseline;
            args[argument] = boundary;
            libraries.compare(args, &format!("scalar boundary for argument {argument}"));
        }
    }
}
