mod common;

use common::{AllocationHarness, Rng, assert_covered, assert_i32_bytes, load_both, mark};
use std::ffi::{c_char, c_int};

const SAMPLES: usize = 96;

#[test]
fn configuration_rows_1_through_15_low_level_entry_points() {
    let (c, rust) = unsafe { load_both() };
    let mut rng = Rng::new(0x8d26_5f91_12aa_4c03);
    let mut covered = [false; 54];

    for row in 1..=4 {
        mark(&mut covered, row);
        for sample in 0..SAMPLES {
            let (size, positions) = match row {
                1 => (2, 1),
                2 => (rng.i32_between(3, 48), 1),
                3 => {
                    let size = rng.i32_between(4, 48);
                    (size, rng.i32_between(2, size - 2))
                }
                4 => {
                    let size = rng.i32_between(3, 48);
                    (size, size - 1)
                }
                _ => unreachable!(),
            };
            let original: Vec<c_int> = (0..size)
                .map(|_| rng.i32_between(-100_000, 100_000))
                .collect();
            let mut c_values = original.clone();
            let mut rust_values = original;
            unsafe {
                (c.shift_array)(c_values.as_mut_ptr(), size, positions);
                (rust.shift_array)(rust_values.as_mut_ptr(), size, positions);
            }
            assert_eq!(c_values, rust_values, "row {row}, sample {sample}");
        }
    }

    for row in 5..=7 {
        mark(&mut covered, row);
        for sample in 0..SAMPLES {
            let visible_length = match row {
                5 => 0,
                6 => 1,
                7 => rng.i32_between(2, 96) as usize,
                _ => unreachable!(),
            };
            let mut bytes: Vec<u8> = (0..visible_length).map(|_| rng.nonzero_u8()).collect();
            bytes.push(0);
            for _ in 0..rng.i32_between(0, 8) {
                bytes.push(rng.nonzero_u8());
            }
            let c_value = unsafe { (c.process_string)(bytes.as_ptr().cast::<c_char>()) };
            let rust_value = unsafe { (rust.process_string)(bytes.as_ptr().cast::<c_char>()) };
            assert_i32_bytes(row, sample, c_value, rust_value);
        }
    }

    let boundaries = [i32::MIN, -65_536, -1, 0, 1, 65_535, i32::MAX];
    for row in 8..=12 {
        mark(&mut covered, row);
        for sample in 0..SAMPLES {
            let value = if sample < boundaries.len() {
                boundaries[sample]
            } else {
                rng.next_u32() as i32
            };
            let operation = match row {
                8..=11 => (row - 8) as i32,
                12 => {
                    let candidates = [i32::MIN, -100, -1, 4, 5, 100, i32::MAX];
                    candidates[sample % candidates.len()]
                }
                _ => unreachable!(),
            };
            let c_value = unsafe { (c.apply_bitmask)(value, operation) };
            let rust_value = unsafe { (rust.apply_bitmask)(value, operation) };
            assert_i32_bytes(row, sample, c_value, rust_value);
        }
    }

    mark(&mut covered, 13);
    for sample in 0..SAMPLES {
        let mut c_matrix = [[0; 4]; 3];
        for row in &mut c_matrix {
            for value in row {
                *value = rng.next_u32() as i32;
            }
        }
        let mut rust_matrix = c_matrix;
        unsafe {
            (c.init_matrix)(c_matrix.as_mut_ptr());
            (rust.init_matrix)(rust_matrix.as_mut_ptr());
        }
        assert_eq!(c_matrix, rust_matrix, "row 13, sample {sample}");
    }

    let mut allocations = AllocationHarness::new();
    for row in 14..=15 {
        mark(&mut covered, row);
        for sample in 0..SAMPLES {
            let val1 = if row == 14 {
                rng.i32_between(-100_000, 0)
            } else {
                rng.i32_between(1, 100_000)
            };
            let val2 = rng.i32_between(-100_000, 100_000);
            allocations.compare(
                row,
                sample,
                || unsafe { (c.compare_allocations)(val1, val2) },
                || unsafe { (rust.compare_allocations)(val1, val2) },
            );
        }
    }

    assert_covered(&covered, 1, 15);
}

fn param1_for(class: usize, rng: &mut Rng) -> i32 {
    let magnitude = rng.i32_between(1, 250);
    match class {
        0 => 4 * magnitude,
        1 => 0,
        2 => -4 * magnitude,
        3 => 4 * magnitude + 1,
        4 => 4 * magnitude + 2,
        5 => 4 * magnitude + 3,
        6 => -(4 * magnitude + rng.i32_between(1, 3)),
        _ => unreachable!(),
    }
}

fn random_params(class: usize, p3_set: bool, p4_set: bool, rng: &mut Rng) -> [i32; 4] {
    [
        param1_for(class, rng),
        rng.i32_between(-10_000, 10_000),
        if p3_set { rng.nonzero_i32(-20, 20) } else { 0 },
        if p4_set {
            rng.nonzero_i32(-10_000, 10_000)
        } else {
            0
        },
    ]
}

#[test]
fn configuration_rows_16_through_43_arity4_cross_product() {
    let (c, rust) = unsafe { load_both() };
    let mut rng = Rng::new(0xa97c_e135_613f_9b20);
    let mut allocations = AllocationHarness::new();
    let mut covered = [false; 54];

    for class in 0..7 {
        for toggle in 0..4 {
            let row = 16 + class * 4 + toggle;
            let p3_set = toggle == 1 || toggle == 3;
            let p4_set = toggle == 2 || toggle == 3;
            mark(&mut covered, row);
            for sample in 0..SAMPLES {
                let [p1, p2, p3, p4] = random_params(class, p3_set, p4_set, &mut rng);
                allocations.compare(
                    row,
                    sample,
                    || unsafe { (c.arity4)(p1, p2, p3, p4) },
                    || unsafe { (rust.arity4)(p1, p2, p3, p4) },
                );
            }
        }
    }

    assert_covered(&covered, 16, 43);
}

#[test]
fn configuration_rows_44_through_53_wrappers_and_dispatch() {
    let (c, rust) = unsafe { load_both() };
    let mut rng = Rng::new(0xdcb4_17c9_53e2_806f);
    let mut allocations = AllocationHarness::new();
    let mut covered = [false; 54];

    mark(&mut covered, 44);
    for sample in 0..SAMPLES {
        let params = random_params(sample % 7, false, false, &mut rng);
        allocations.compare(
            44,
            sample,
            || unsafe { (c.arity2)(params[0], params[1]) },
            || unsafe { (rust.arity2)(params[0], params[1]) },
        );
    }

    for row in 45..=46 {
        mark(&mut covered, row);
        for sample in 0..SAMPLES {
            let params = random_params(sample % 7, row == 46, false, &mut rng);
            allocations.compare(
                row,
                sample,
                || unsafe { (c.arity3)(params[0], params[1], params[2]) },
                || unsafe { (rust.arity3)(params[0], params[1], params[2]) },
            );
        }
    }

    for row in 47..=53 {
        mark(&mut covered, row);
        for sample in 0..SAMPLES {
            let class = sample % 7;
            let p3_set = row == 49 || (row >= 50 && sample % 2 == 1);
            let mut params = random_params(class, p3_set, sample % 3 == 0, &mut rng);
            let len = match row {
                47 => 2,
                48 | 49 => 3,
                50 => rng.i32_between(4, 255),
                51 => {
                    let turns = rng.i32_between(1, 1_000);
                    if sample % 2 == 0 {
                        2 + 256 * turns
                    } else {
                        2 - 256 * turns
                    }
                }
                52 => {
                    let turns = rng.i32_between(1, 1_000);
                    if sample % 2 == 0 {
                        3 + 256 * turns
                    } else {
                        3 - 256 * turns
                    }
                }
                53 => {
                    let low_byte = rng.i32_between(4, 255);
                    let turns = rng.i32_between(1, 1_000);
                    if sample % 2 == 0 {
                        low_byte + 256 * turns
                    } else {
                        low_byte - 256 * turns
                    }
                }
                _ => unreachable!(),
            };
            if row == 48 {
                params[2] = 0;
            } else if row == 49 && params[2] == 0 {
                params[2] = 1;
            }
            let params_pointer = params.as_mut_ptr();
            allocations.compare(
                row,
                sample,
                || unsafe { (c.arity)(len, params_pointer) },
                || unsafe { (rust.arity)(len, params_pointer) },
            );
        }
    }

    assert_covered(&covered, 44, 53);
}
