use libloading::Library;
use std::path::PathBuf;

type EncodeQuant = unsafe extern "C" fn(i32, i32, i32, i32, i32, i32) -> i32;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c: EncodeQuant,
    rust: EncodeQuant,
}

impl Implementations {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-VZy3X1.so");
        let rust_path = root.join("target/release/libencode_quant_lib.so");

        assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {rust_path:?}"
        );

        unsafe {
            let c_library = Library::new(&c_path).expect("load the C shared library");
            let rust_library = Library::new(&rust_path).expect("load the Rust shared library");
            let c = *c_library
                .get::<EncodeQuant>(b"encode_quant\0")
                .expect("load C encode_quant");
            let rust = *rust_library
                .get::<EncodeQuant>(b"encode_quant\0")
                .expect("load Rust encode_quant");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }

    fn compare(&self, args: [i32; 6], context: &str) {
        let c_result = unsafe { (self.c)(args[0], args[1], args[2], args[3], args[4], args[5]) };
        let rust_result =
            unsafe { (self.rust)(args[0], args[1], args[2], args[3], args[4], args[5]) };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "{context}: args={args:?}, C={c_result}, Rust={rust_result}"
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum LsbitMode {
    Zero,
    Four,
    OddNonzero,
    EvenNonzeroNotFour,
}

#[derive(Clone, Copy, Debug)]
enum OctetPosition {
    Low,
    Interior,
    High,
}

#[derive(Clone, Copy, Debug)]
struct Configuration {
    row: usize,
    lsbit: LsbitMode,
    position: OctetPosition,
    bit_eight_set: bool,
    comparisons: (bool, bool),
}

fn configurations() -> Vec<Configuration> {
    let mut result = Vec::new();
    let modes = [
        LsbitMode::Zero,
        LsbitMode::Four,
        LsbitMode::OddNonzero,
        LsbitMode::EvenNonzeroNotFour,
    ];
    let positions = [
        OctetPosition::Low,
        OctetPosition::Interior,
        OctetPosition::High,
    ];

    for lsbit in modes {
        for position in positions {
            for bit_eight_set in [false, true] {
                let outcomes: &[(bool, bool)] = match (lsbit, position) {
                    (LsbitMode::Zero, OctetPosition::Low) => &[(false, false), (true, false)],
                    (LsbitMode::Zero, OctetPosition::Interior) => {
                        &[(false, false), (false, true), (true, false)]
                    }
                    (LsbitMode::Zero, OctetPosition::High) => &[(false, false), (false, true)],
                    (_, OctetPosition::Interior) => &[(false, false), (true, false)],
                    _ => &[(false, false)],
                };

                for &comparisons in outcomes {
                    result.push(Configuration {
                        row: result.len() + 1,
                        lsbit,
                        position,
                        bit_eight_set,
                        comparisons,
                    });
                }
            }
        }
    }

    assert_eq!(result.len(), 38);
    result
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    fn range_inclusive(&mut self, low: i32, high: i32) -> i32 {
        let width = (i64::from(high) - i64::from(low) + 1) as u64;
        i32::try_from(i64::from(low) + (self.next_u64() % width) as i64).unwrap()
    }
}

fn generated_lsbit(mode: LsbitMode, rng: &mut Rng) -> i32 {
    match mode {
        LsbitMode::Zero => 0,
        LsbitMode::Four => 4,
        LsbitMode::OddNonzero => rng.next_i32() | 1,
        LsbitMode::EvenNonzeroNotFour => {
            let value = rng.next_i32() & !1;
            if value == 0 || value == 4 { 2 } else { value }
        }
    }
}

fn generated_uni(position: OctetPosition, bit_eight_set: bool, rng: &mut Rng) -> i32 {
    let low = match position {
        OctetPosition::Low => 0,
        OctetPosition::Interior => rng.range_inclusive(1, 6),
        OctetPosition::High => 7,
    };
    (rng.next_i32() & !15) | low | if bit_eight_set { 8 } else { 0 }
}

fn candidate_state(mut uni: i32, lsbit: i32) -> [i32; 3] {
    let mut uni1 = uni.wrapping_add(1);
    let mut uni2 = uni.wrapping_sub(1);

    if (uni ^ uni1) & !7 != 0 {
        uni1 = uni;
    }
    if (uni ^ uni2) & !7 != 0 {
        uni2 = uni;
    }

    if lsbit != 0 {
        if lsbit == 4 {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if lsbit & 1 != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
        }
    }

    [uni, uni1, uni2]
}

fn prediction(uni: i32, step: i32, pred: i32) -> i32 {
    let scale = (2 * (uni & 7) + 1).wrapping_mul(step);
    let mut diff = scale / 8;
    if uni & 8 != 0 {
        diff = diff.wrapping_neg();
    }
    pred.wrapping_add(diff)
}

fn folded_difference(target: i32, prediction: i32) -> i32 {
    let difference = target.wrapping_sub(prediction);
    difference ^ (difference >> 31)
}

fn comparison_outcomes(args: [i32; 6]) -> (bool, bool) {
    let [uni, step, pred, tgt, tgt2, lsbit] = args;
    let candidates = candidate_state(uni, lsbit);
    let predictions = candidates.map(|candidate| prediction(candidate, step, pred));
    let distances = predictions.map(|value| {
        folded_difference(tgt, value).wrapping_add(folded_difference(tgt2, value) >> 5)
    });

    (distances[1] < distances[0], distances[2] < distances[0])
}

fn generated_args(config: Configuration, rng: &mut Rng) -> [i32; 6] {
    [
        generated_uni(config.position, config.bit_eight_set, rng),
        rng.range_inclusive(-65_536, 65_536),
        rng.range_inclusive(-2_000_000, 2_000_000),
        rng.range_inclusive(-2_000_000, 2_000_000),
        rng.range_inclusive(-2_000_000, 2_000_000),
        generated_lsbit(config.lsbit, rng),
    ]
}

#[test]
fn every_configuration_row_matches_on_randomized_inputs() {
    const SAMPLES_PER_ROW: usize = 256;
    const MAX_ATTEMPTS_PER_ROW: usize = 1_000_000;

    let implementations = Implementations::load();

    for config in configurations() {
        let mut rng = Rng::new(0x6a09_e667_f3bc_c909_u64 ^ config.row as u64);
        let mut matched = 0;

        for _ in 0..MAX_ATTEMPTS_PER_ROW {
            let args = generated_args(config, &mut rng);
            if comparison_outcomes(args) != config.comparisons {
                continue;
            }

            implementations.compare(args, &format!("CONFIGS.md row {} ({config:?})", config.row));
            matched += 1;
            if matched == SAMPLES_PER_ROW {
                break;
            }
        }

        assert_eq!(
            matched, SAMPLES_PER_ROW,
            "could not generate enough inputs for CONFIGS.md row {} ({config:?})",
            config.row
        );
    }
}

#[test]
fn broad_fixed_seed_random_inputs_match() {
    let implementations = Implementations::load();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    for case in 0..200_000 {
        let args = [
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        ];
        implementations.compare(args, &format!("broad random case {case}"));
    }
}

#[test]
fn integer_boundary_inputs_match() {
    let implementations = Implementations::load();
    let boundary = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let baseline = [3, 64, 100, -20, 40, 0];

    implementations.compare(baseline, "boundary baseline");

    for argument in 0..6 {
        for &value in &boundary {
            let mut args = baseline;
            args[argument] = value;
            implementations.compare(
                args,
                &format!("boundary argument {argument}, value {value}"),
            );
        }
    }

    for &value in &boundary {
        implementations.compare([value; 6], &format!("all arguments {value}"));
    }

    let mixed = [
        [i32::MIN, 0, 0, 0, 0, 0],
        [i32::MAX, 0, 0, 0, 0, 0],
        [0, i32::MIN, i32::MAX, i32::MIN, i32::MAX, 4],
        [7, i32::MAX, i32::MIN, i32::MAX, i32::MIN, -1],
        [8, i32::MIN, i32::MAX, i32::MIN, i32::MAX, 2],
        [15, i32::MAX, i32::MAX, i32::MIN, i32::MIN, i32::MAX],
    ];

    for (case, args) in mixed.into_iter().enumerate() {
        implementations.compare(args, &format!("mixed boundary case {case}"));
    }
}
