use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::ptr;

type ProcessDecisions = unsafe extern "C" fn(*mut u8, usize, i32, i32) -> i32;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libdriver_c.so");
        let rust_path = rust_library_path();

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            Self {
                c: Library::new(c_path).expect("load C shared library"),
                rust: Library::new(rust_path).expect("load Rust shared library"),
            }
        }
    }

    unsafe fn call(library: &Library, data: *mut u8, len: usize, op: i32, param: i32) -> i32 {
        let function: Symbol<ProcessDecisions> = library
            .get(b"process_decisions\0")
            .expect("resolve process_decisions");
        function(data, len, op, param)
    }

    fn compare(&self, input: &[u8], op: i32, param: i32, row: &str) -> i32 {
        let mut c_input = input.to_vec();
        let mut rust_input = input.to_vec();

        let c_result =
            unsafe { Self::call(&self.c, c_input.as_mut_ptr(), c_input.len(), op, param) };
        let rust_result = unsafe {
            Self::call(
                &self.rust,
                rust_input.as_mut_ptr(),
                rust_input.len(),
                op,
                param,
            )
        };

        assert_eq!(
            rust_result, c_result,
            "{row}: result mismatch for op={op}, param={param}, input={input:?}"
        );
        assert_eq!(
            rust_input, c_input,
            "{row}: output-buffer mismatch for op={op}, param={param}, input={input:?}"
        );
        c_result
    }

    fn compare_raw(
        &self,
        c_data: *mut u8,
        rust_data: *mut u8,
        len: usize,
        op: i32,
        param: i32,
        row: &str,
    ) -> i32 {
        let c_result = unsafe { Self::call(&self.c, c_data, len, op, param) };
        let rust_result = unsafe { Self::call(&self.rust, rust_data, len, op, param) };
        assert_eq!(rust_result, c_result, "{row}: raw-call result mismatch");
        c_result
    }
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory");
    profile_dir.join(format!(
        "{}driver{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }
}

fn encoded(value: bool, rng: &mut Rng) -> u8 {
    const TRUE_BYTES: &[u8] = b"yY";
    const FALSE_BYTES: &[u8] = &[b'n', b'N', b'x', 0, 0xff];
    let choices = if value { TRUE_BYTES } else { FALSE_BYTES };
    choices[rng.usize(choices.len())]
}

fn encode(values: &[bool], rng: &mut Rng) -> Vec<u8> {
    values.iter().map(|&value| encoded(value, rng)).collect()
}

fn max_equal_run(values: &[bool]) -> usize {
    let mut maximum = 0;
    let mut current = 0;
    let mut previous = None;
    for &value in values {
        if previous == Some(value) {
            current += 1;
        } else {
            current = 1;
            previous = Some(value);
        }
        maximum = maximum.max(current);
    }
    maximum
}

fn transitions(values: &[bool]) -> usize {
    values.windows(2).filter(|pair| pair[0] != pair[1]).count()
}

#[test]
fn operation_0_permission_rows_c001_through_c008() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc001_c008_5eed);

    for mask in 0_u8..8 {
        let row = format!("C{:03}", usize::from(mask) + 1);
        for iteration in 0..96 {
            let length = if iteration % 16 == 0 {
                1025 + rng.usize(1024)
            } else {
                3 + rng.usize(126)
            };
            let mut input: Vec<u8> = (0..length).map(|_| rng.byte()).collect();
            for (index, byte) in input.iter_mut().take(3).enumerate() {
                *byte = encoded(mask & (1 << (2 - index)) != 0, &mut rng);
            }
            libraries.compare(&input, 0, rng.next_u64() as i32, &row);
        }
    }
}

#[test]
fn operation_1_condition_rows_c009_through_c040() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc009_c040_5eed);

    for param in 0_i32..=3 {
        for mask in 0_u8..8 {
            let row_number = 9 + param as usize * 8 + mask as usize;
            let row = format!("C{row_number:03}");
            for iteration in 0..96 {
                let length = if iteration % 16 == 0 {
                    1025 + rng.usize(1024)
                } else {
                    3 + rng.usize(126)
                };
                let mut input: Vec<u8> = (0..length).map(|_| rng.byte()).collect();
                for (index, byte) in input.iter_mut().take(3).enumerate() {
                    *byte = encoded(mask & (1 << (2 - index)) != 0, &mut rng);
                }
                libraries.compare(&input, 1, param, &row);
            }
        }
    }
}

#[test]
fn operation_2_flag_rows_c041_through_c050() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc041_c050_5eed);

    for _ in 0..128 {
        let count = 1 + rng.usize(32);
        libraries.compare(&encode(&vec![false; count], &mut rng), 2, 0, "C041");

        let count = 1 + rng.usize(32);
        libraries.compare(&encode(&vec![true; count], &mut rng), 2, 0, "C042");

        let count = 2 + rng.usize(31);
        let mut values = vec![false; count];
        values[rng.usize(count)] = true;
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C043");

        let count = 2 + rng.usize(31);
        let mut values = vec![true; count];
        values[rng.usize(count)] = false;
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C044");

        let count = 4 + rng.usize(29);
        let values: Vec<bool> = (0..count).map(|index| index % 2 == 0).collect();
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C045");

        let count = 4 + rng.usize(29);
        let values: Vec<bool> = (0..count).map(|index| index % 2 != 0).collect();
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C046");

        let count = 6 + rng.usize(27);
        let start = rng.usize(count - 2);
        let mut values = vec![false; count];
        values[start..start + 3].fill(true);
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C047");

        let count = 6 + rng.usize(27);
        let values: Vec<bool> = (0..count).map(|index| index % 4 < 2).collect();
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C048");

        let values: Vec<bool> = (0..32).map(|_| rng.usize(2) != 0).collect();
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C049");

        let count = 33 + rng.usize(224);
        let values: Vec<bool> = (0..count).map(|_| rng.usize(2) != 0).collect();
        libraries.compare(&encode(&values, &mut rng), 2, 0, "C050");
    }
}

#[test]
fn operation_3_sequence_rows_c051_through_c058() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc051_c058_5eed);

    for iteration in 0..96 {
        libraries.compare(&encode(&[true], &mut rng), 3, 0, "C051");
        libraries.compare(&encode(&[true, false], &mut rng), 3, 0, "C052");

        let short = if rng.usize(2) == 0 {
            [true, true, false]
        } else {
            [true, false, false]
        };
        libraries.compare(&encode(&short, &mut rng), 3, 0, "C053");

        libraries.compare(
            &encode(&[true, true, true, false, false, false], &mut rng),
            3,
            0,
            "C054",
        );

        let many_length = 4 + 2 * rng.usize(4);
        let many: Vec<bool> = (0..many_length).map(|index| index % 2 == 0).collect();
        libraries.compare(&encode(&many, &mut rng), 3, 0, "C055");

        let middle = loop {
            let length = 4 + rng.usize(7);
            let mut values: Vec<bool> = (0..length).map(|_| rng.usize(2) != 0).collect();
            values[0] = true;
            values[length - 1] = false;
            let count = transitions(&values);
            if max_equal_run(&values) <= 3 && count >= length / 3 && count <= length / 2 {
                break values;
            }
        };
        libraries.compare(&encode(&middle, &mut rng), 3, 0, "C056");

        let many_length = if iteration % 8 == 0 {
            1026
        } else {
            12 + 2 * rng.usize(58)
        };
        let many: Vec<bool> = (0..many_length).map(|index| index % 2 == 0).collect();
        libraries.compare(&encode(&many, &mut rng), 3, 0, "C057");

        let groups = if iteration % 8 == 0 {
            257
        } else {
            3 + rng.usize(30)
        };
        let middle: Vec<bool> = (0..groups * 4).map(|index| index % 4 < 2).collect();
        libraries.compare(&encode(&middle, &mut rng), 3, 0, "C058");
    }
}

#[test]
fn error_rows_e01_through_e09_and_generic_boundaries() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xe001_e009_5eed);

    for operation in [0, 1, 2, 3, -1, 4] {
        let result =
            libraries.compare_raw(ptr::null_mut(), ptr::null_mut(), 1, operation, 0, "E01");
        assert_eq!(result, -1);
    }

    for operation in [0, 1, 2, 3, -1, 4] {
        let mut c_byte = rng.byte();
        let mut rust_byte = c_byte;
        let result = libraries.compare_raw(&mut c_byte, &mut rust_byte, 0, operation, 0, "E02");
        assert_eq!(result, -1);
    }

    for length in [1, 2] {
        for _ in 0..64 {
            let input: Vec<u8> = (0..length).map(|_| rng.byte()).collect();
            assert_eq!(libraries.compare(&input, 0, 0, "E03"), -2);
            assert_eq!(libraries.compare(&input, 1, 0, "E04"), -2);
        }
    }

    for operation in [i32::MIN, -100, -1, 4, 100, i32::MAX] {
        for length in [1, 3, 33, 1025] {
            let input: Vec<u8> = (0..length).map(|_| rng.byte()).collect();
            assert_eq!(libraries.compare(&input, operation, 0, "E05"), -3);
        }
    }
    let mut c_byte = b'y';
    let mut rust_byte = b'y';
    assert_eq!(
        libraries.compare_raw(
            &mut c_byte,
            &mut rust_byte,
            usize::MAX,
            i32::MAX,
            0,
            "E05 oversized length",
        ),
        -3
    );

    for param in [i32::MIN, -100, -1, 4, 100, i32::MAX] {
        for _ in 0..64 {
            let input = encode(
                &[rng.usize(2) != 0, rng.usize(2) != 0, rng.usize(2) != 0],
                &mut rng,
            );
            assert_eq!(libraries.compare(&input, 1, param, "E06"), -1);
        }
    }

    for length in 1..=64 {
        let mut values: Vec<bool> = (0..length).map(|_| rng.usize(2) != 0).collect();
        values[0] = false;
        assert_eq!(
            libraries.compare(&encode(&values, &mut rng), 3, 0, "E07"),
            -10
        );
    }

    for length in 2..=64 {
        let mut values: Vec<bool> = (0..length).map(|index| index % 2 == 0).collect();
        values[0] = true;
        values[length - 1] = true;
        assert_eq!(
            libraries.compare(&encode(&values, &mut rng), 3, 0, "E08"),
            -11
        );
    }

    for run_length in 4..=64 {
        let mut values = vec![true; run_length];
        values.push(false);
        assert_eq!(
            libraries.compare(&encode(&values, &mut rng), 3, 0, "E09"),
            -12
        );
    }
}
